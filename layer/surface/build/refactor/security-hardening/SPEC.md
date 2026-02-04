---
type: refactor
id: security-hardening
status: ready
created: 2026-02-03
sessions:
  origin: 20260203-134222
  work:
    - 20260203-192041
related:
  - layer/core/build.md
  - layer/surface/build/feat/mother/SPEC.md
  - src/commands/serve/internal.rs
  - src/secrets/vault.rs
  - src/secrets/mod.rs
  - src/embeddings/onnx.rs
  - src/mcp/server.rs
---

# refactor: Security Hardening

> Close the safety holes before Mother goes multi-user or networked.

**Problem:** Security audit (session 20260203-134222, Litchfield/Davidoff style review) identified 10 findings across the HTTP daemon, secrets management, and model loading. SQL injection and command injection are clean (parameterized queries, no shell=true). The real exposure is the unauthenticated HTTP daemon and secrets hygiene.

**Solution:** Phased hardening — P0 fixes prevent remote exploitation, P1 fixes close local privilege escalation, P2 fixes improve defense-in-depth.

---

## Critical Bug (Fix First)

`patina serve` binds an HTTP server with **zero authentication** on port 50051. If run with `--host 0.0.0.0` (suggested in help text for containers), the entire knowledge base is queryable by anyone on the network. No CORS headers means browser-based attacks work against localhost too.

---

## Complete Inventory

### P0: Remote Exploitation Prevention (HTTP Daemon)

| # | File | Issue | Fix |
|---|------|-------|-----|
| 1 | `src/commands/serve/internal.rs` | No authentication on any endpoint | Add bearer token auth (read from `PATINA_SERVE_TOKEN` or generate on start, print to stderr) |
| 2 | `src/commands/serve/internal.rs` | No request body size limit (rouille accepts unbounded) | Read cap via `Read::take()` — do not trust Content-Length header |
| 3 | `src/commands/serve/internal.rs` | `limit` field accepts any `usize` value | Cap at 1000 in `handle_scry()` before query execution |
| 4 | `src/commands/serve/internal.rs` | No security headers | `X-Content-Type-Options: nosniff`, `X-Frame-Options: DENY` (omit CORS origin entirely) |
| 5 | `src/commands/serve/internal.rs` | No warning when binding to 0.0.0.0 | Print security warning to stderr when `--host` is not 127.0.0.1 |
| 6 | `src/commands/serve/internal.rs` | Error responses mix text and JSON formats | All errors return `{"error": "..."}` with correct status code |

### Deferred: Execution Bounding (separate spec)

| # | File | Issue | Needs |
|---|------|-------|-------|
| — | `src/commands/serve/internal.rs` | No query execution timeout or concurrency bound | Proper design: RAII drop guard, atomic increment-then-check, panic safety. Naive `thread::spawn` + `recv_timeout` leaks threads on timeout and a bare semaphore has TOCTOU race + panic counter leak. Design direction: session-aware budgets + rate limiting. Spec separately. |

### P1: Local Privilege Escalation / Secrets Hygiene

| # | File | Issue | Fix |
|---|------|-------|-----|
| 7 | `src/secrets/vault.rs:183-191` | Vault files written with default permissions (0o644) | `fs::set_permissions(path, Permissions::from_mode(0o600))` after write |
| 8 | `src/secrets/registry.rs:81-96` | Secrets registry world-readable (maps names to env vars) | Same: 0o600 on `secrets.toml` |
| 9 | `src/secrets/mod.rs:296` | SSH variant puts decrypted secrets in process argv | Use stdin pipe or `SSH_ASKPASS` protocol instead of command-line env prefix |
| 10 | `src/embeddings/onnx.rs:73` | ONNX model loaded without integrity verification | Add SHA-256 checksum verification against hardcoded expected hash |
| 11 | `src/commands/serve/internal.rs` | Serve token bypasses Patina's own secrets infrastructure | Load token from vault via `patina secrets`, fall back to `PATINA_SERVE_TOKEN` env, generate-and-print as last resort |
| 16 | `src/commands/serve/internal.rs:150-151` | Generated auth token printed to stderr in plain text | Secrets never in outputs. Token printed to stderr leaks to terminal scrollback, log capture, process supervision. With Phase 2 UDS, localhost callers don't need a token at all. For TCP path: write token to `~/.patina/run/serve.token` (0o600), print path not value. |

Item 11 rationale: The serve token is currently a plain env var — visible in `ps auxe`,
shell history, terminal scrollback, and container environment. On localhost this is
acceptable, but on `0.0.0.0` (container use case) the token is the sole network
boundary and should use the same vault/keychain infrastructure that protects other
Patina secrets. Load order: vault → env var → generate. This keeps the simple
`PATINA_SERVE_TOKEN=xxx patina serve` workflow for local dev while giving the
container path a proper secret.

Item 16 rationale: Principle — secrets never appear in CLI output by default. The
generated token is currently printed to stderr (`eprintln!("Generated auth token: {}",
t)`). This leaks to terminal scrollback, tmux capture, log files, and process
supervisors. Phase 2 eliminates the need entirely for localhost (UDS = no token).
For the TCP opt-in path, write the token to a file and print the file path.

### P2: Defense-in-Depth

| # | File | Issue | Fix |
|---|------|-------|-----|
| 12 | `src/secrets/mod.rs:482-491` | Secret input not masked (echoed to terminal) | Use `rpassword` crate for sensitive prompts |
| 13 | `src/secrets/identity.rs` | Identity strings not zeroized after use | Add `zeroize` crate, use `Zeroizing<String>` for key material |
| 14 | `src/commands/secrets.rs:258` | `--export-key` prints private key to stdout | Write to file with 0o600 permissions, or require `--stdout` flag for pipe use |
| 15 | `src/commands/repo/internal.rs:70-71` | Registry YAML path field not validated on load | Canonicalize and validate against `~/.patina/cache/` prefix during deserialization |

---

## What's Already Secure (No Changes Needed)

| Area | Status | Evidence |
|------|--------|---------|
| SQL injection | Clean | All paths use `rusqlite::params![]` parameterized binding |
| Command injection | Clean | All `Command::new()` calls, no shell=true |
| FTS5 injection | Clean | User input bound via MATCH `?` parameter, not concatenated |
| Age encryption | Strong | X25519 via `age` 0.11 crate, well-audited |
| macOS key storage | Strong | Keychain with Touch ID via `security-framework` crate |
| Secret scanning | Good | 11 high-severity + 2 medium patterns, `.secretsignore` support |
| Mother graph | Clean | `EdgeType` enum validates via `parse()` before SQL |

---

## Implementation Plan

### Phase 1: P0 — HTTP Daemon (6 changes, 1 file)

All changes in `src/commands/serve/internal.rs`. Shipped.

**1. Bearer token auth** — `PATINA_SERVE_TOKEN` env or random 32-byte hex on start.
`check_auth()` validates `Authorization: Bearer <token>` header.

**2. Body size enforcement (read cap)** — `Read::take(MAX_BODY_SIZE + 1)` on the
actual body stream, then check `buf.len() > MAX_BODY_SIZE`. Does not trust
Content-Length (can be absent or wrong).

**3. Limit cap** — `body.limit = body.limit.min(1000)` before query execution.

**4. Security headers** — `X-Content-Type-Options: nosniff`, `X-Frame-Options: DENY`
on all responses. No `Access-Control-Allow-Origin` header (omission is safer than `null`).

**5. Bind warning** — stderr warning when `--host` is not `127.0.0.1` or `localhost`.

**6. Consistent JSON errors** — `json_error()` helper, all paths return `{"error": "..."}`.

### Phase 2: Transport Security by Trust Boundary (3 files)

Anchored in [[transport-security-by-trust-boundary]]. Design principle (djb): default
safe, insecure path hard. Prior art: PostgreSQL, Docker.

**Transport model:**
- `Transport::Uds(PathBuf)` — local path, file permissions are auth, no token
- `Transport::Http { base_url, token }` — network path, explicit opt-in, bearer token required

#### 2a. Server dual listener (`serve/internal.rs`, `serve/mod.rs`)

Default: listen on `~/.patina/run/serve.sock` (unix domain socket). No TCP unless
`--host`/`--port` explicitly passed. Flip the djb switch — safe by default.

**Socket directory permissions (non-negotiable):** On startup:
- Create `~/.patina/run/` if it doesn't exist, with 0o700
- Verify `~/.patina/run/` is 0o700 and owned by current user. Refuse to start if
  world/group accessible.
- Set socket file to 0o600 after bind.
- This is the auth — if you can open the socket, you're the right user.

**Stale socket cleanup (safe unlink):** On startup, if `serve.sock` exists:
- Unlink only if: it is a socket AND owned by current user
- Otherwise refuse to start (do not unlink arbitrary files)
- Delete socket on clean shutdown

**UDS wire protocol — length-prefix framing:**

```
[4 bytes: big-endian u32 payload length][JSON payload]
```

Frame size limit: 1 MB (same as HTTP body cap). Reject before reading payload
if length > MAX_FRAME_SIZE. Big-endian per network byte order convention.

**Request/response shape:** Every request includes an `id`. Every response echoes it.

Request:
```json
{"id":1,"action":"hello","client":"session","version":1}
```

Response:
```json
{"id":1,"status":200,"body":{"server":"patina-serve","version":1}}
```

**Actions:** `hello`, `health`, `scry`, `cache_get`, `cache_set`, `cache_clear`

**Handshake (recommended):** First message from client:
```
→ {"id":1,"action":"hello","client":"session","version":1}
← {"id":1,"status":200,"body":{"server":"patina-serve","version":1}}
```

Then request/response pairs:
```
→ {"id":2,"action":"scry","query":"test","limit":10}
← {"id":2,"status":200,"body":{"results":[...],"count":5}}

→ {"id":3,"action":"cache_get"}
← {"id":3,"status":200,"body":{"secrets":{"key":"value"}}}

→ {"id":4,"action":"cache_set","secrets":{"k":"v"},"ttl_secs":600}
← {"id":4,"status":200,"body":{"cached":true}}

→ {"id":5,"action":"cache_clear"}
← {"id":5,"status":200,"body":{"cleared":true}}
```

**Shared handler logic:** Extract current HTTP handlers into standalone functions.
UDS and HTTP MUST call the same core handlers. No duplicated business logic.

```rust
fn handle_health(state: &ServerState) -> ActionResponse { ... }
fn handle_scry(state: &ServerState, req: ScryRequest) -> ActionResponse { ... }
fn handle_cache_get(state: &ServerState) -> ActionResponse { ... }
fn handle_cache_set(state: &ServerState, req: CacheSetRequest) -> ActionResponse { ... }
fn handle_cache_clear(state: &ServerState) -> ActionResponse { ... }
```

rouille stays for TCP/HTTP path. `std::os::unix::net::UnixListener` for UDS. One
thread per listener. No new dependencies — stdlib + existing `libc`.

**Why not HTTP over UDS?** rouille/tiny-http accept `TcpListener` only. Supporting
HTTP over UDS would require switching frameworks (often async) or hacking the HTTP
server internals. We don't need HTTP for local IPC — the protocol is simply "send
JSON, receive JSON."

#### 2b. Session client transport-aware (`secrets/session.rs`)

Try UDS first (no auth needed). Fall back to HTTP + token if no socket.
```rust
fn connect() -> Transport {
    let sock = paths::serve::socket_path();
    if sock.exists() { Transport::Uds(sock) }
    else { Transport::Http { base_url: serve_url(), token: serve_token() } }
}
```

#### 2c. Mother client transport-aware (`mother/internal.rs`)

Accept transport in constructor. Local mother: uses UDS (no token). Container
mother: uses TCP + bearer token passed in constructor.

### Phase 3: P1 — File Permissions + Model Integrity (4 changes, 4 files)

**3a. Vault permissions** (`src/secrets/vault.rs`):
```rust
use std::os::unix::fs::PermissionsExt;

fs::write(vault_path, encrypted)?;
fs::set_permissions(vault_path, fs::Permissions::from_mode(0o600))?;
```

**3b. Registry permissions** (`src/secrets/registry.rs`):
Same pattern — `fs::set_permissions` after `fs::write`.

**3c. ONNX checksum** (`src/embeddings/onnx.rs`):
```rust
const EXPECTED_SHA256: &str = "abc123...";  // computed from known-good model

fn verify_model(path: &Path) -> Result<()> {
    let bytes = fs::read(path)?;
    let hash = sha2::Sha256::digest(&bytes);
    let hex = format!("{:x}", hash);
    if hex != EXPECTED_SHA256 {
        bail!("ONNX model integrity check failed.\n  Expected: {}\n  Got: {}", EXPECTED_SHA256, hex);
    }
    Ok(())
}
```

**3d. SSH secrets via stdin** (`src/secrets/mod.rs`):
Replace argv-based env prefix with stdin pipe to remote shell.

### Phase 4: P2 — Defense-in-Depth (4 changes, 4 files)

**4a. Masked secret input** (`src/secrets/mod.rs`, `src/commands/secrets.rs`):
- `patina secrets add <name>` prompts with masked input (no echo)
- `patina secrets add <name> --stdin` reads from stdin (scripting/piping)
- MUST NOT accept secret values as positional CLI arguments
- Use `console::Term::read_secure_line()` for masked prompts (already in tree)

**4b. Key material zeroization** (`src/secrets/identity.rs`):
- Add `zeroize` crate, use `Zeroizing<String>` for identity strings after use

**4c. Export key safety** (`src/commands/secrets.rs`):
- `--export-key` writes to file with 0o600, or requires `--stdout` for pipe use

**4d. Registry path validation** (`src/commands/repo/internal.rs`):
- Canonicalize and validate against `~/.patina/cache/` prefix during deserialization

---

## New Dependencies

None. All required crates are already in the dependency tree via `age` and `console`:

| Need | Crate | Already via |
|------|-------|-------------|
| ONNX checksum | `sha2` | `age` → `sha2 0.10` |
| Zero key material | `zeroize` | `age` → `zeroize 1.8` |
| Masked input | `console` | `console 0.15` (has `read_secure_line()`) |

---

## Exit Criteria

**Phase 1 (P0):**
- [x] `patina serve` requires `Authorization: Bearer <token>` on `/api/scry`
- [x] `/health` and `/version` remain open (healthcheck compatibility)
- [x] Request body >1MB rejected with 413 (read cap via `Read::take`, not Content-Length trust)
- [x] `limit` capped at 1000 regardless of input
- [x] Security headers on all responses (`X-Content-Type-Options: nosniff`, `X-Frame-Options: DENY`)
- [x] Warning printed when `--host` is not localhost
- [x] All error responses use consistent JSON format (`{"error": "..."}`)
- [ ] Execution bounding (timeout + concurrency gate) — deferred, needs proper design

**Phase 2 (Transport Security):**
- [ ] `patina serve` defaults to unix socket at `~/.patina/run/serve.sock`
- [ ] TCP listener only starts with explicit `--host`/`--port`
- [ ] Socket dir `~/.patina/run/` created 0o700, verified on startup, refuse if open
- [ ] Socket file set to 0o600 after bind
- [ ] Stale socket: unlink only if is-socket AND owned-by-current-user, else refuse
- [ ] UDS uses length-prefix framing (4-byte BE u32 + JSON payload)
- [ ] Frame size limit: 1 MB, reject before reading if exceeded
- [ ] Every request carries `id`, every response echoes it
- [ ] Client hello/version handshake on connect
- [ ] Socket deleted on clean shutdown
- [ ] `session.rs` connects via UDS first, HTTP+token fallback
- [ ] `mother/internal.rs` accepts transport enum (Uds or Http+token)
- [ ] Handler functions shared between UDS and HTTP paths — no duplicated logic
- [ ] Generated token never printed to stderr — write to file, print path

**Phase 3 (P1 — File Permissions):**
- [ ] `~/.patina/vault.age` created with 0o600 permissions
- [ ] `~/.patina/secrets.toml` created with 0o600 permissions
- [ ] ONNX model verified via SHA-256 before loading
- [ ] `patina secrets run` over SSH does not expose secrets in `ps auxe`
- [ ] Serve token loaded from vault when available, env var as fallback

**Phase 4 (P2 — Defense-in-Depth):**
- [ ] Secret prompts use masked input (no echo via `console`)
- [ ] `patina secrets add` with `--stdin` flag for scripting
- [ ] Secret values NEVER accepted as positional CLI arguments
- [ ] Key material zeroized after use (`Zeroizing<String>`)
- [ ] `--export-key` writes to file (0o600), not stdout
- [ ] Registry paths canonicalized and validated on load

---

## Verification

```bash
# Phase 1: HTTP daemon
curl -s http://127.0.0.1:50051/api/scry -d '{"query":"test"}' -H "Content-Type: application/json"
# → {"error":"Unauthorized"} 401 (no token)

curl -s http://127.0.0.1:50051/api/scry -d '{"query":"test"}' -H "Authorization: Bearer $TOKEN" -H "Content-Type: application/json"
# → 200 OK (valid token)

curl -s http://127.0.0.1:50051/api/scry -d '{"query":"test","limit":99999}' -H "Authorization: Bearer $TOKEN" -H "Content-Type: application/json"
# → Results with max 1000 (capped)

# Body size — enforced by read cap, not Content-Length header:
dd if=/dev/zero bs=1048577 count=1 2>/dev/null | curl -s -X POST http://127.0.0.1:50051/api/scry -d @- -H "Authorization: Bearer $TOKEN" -H "Content-Type: application/json"
# → {"error":"Request too large"} 413

# Security headers present on all responses:
curl -sD - -o /dev/null http://127.0.0.1:50051/health | grep -i "x-content-type-options\|x-frame-options"
# → X-Content-Type-Options: nosniff
# → X-Frame-Options: DENY

# Phase 2: Transport security
# Default: socket only, no TCP
patina serve
# → Listening on ~/.patina/run/serve.sock
# → No TCP listener (use --host/--port for network access)

# Verify socket-dir permissions
stat -f "%Lp" ~/.patina/run/
# → 700
stat -f "%Lp" ~/.patina/run/serve.sock
# → 600

# UDS client works without token (socat for manual testing)
printf '\x00\x00\x00\x2d{"id":1,"action":"hello","client":"test","version":1}' \
  | socat - UNIX-CONNECT:~/.patina/run/serve.sock
# → length-prefixed response with id:1 echoed

# TCP still requires auth when explicitly enabled
patina serve --host 127.0.0.1 --port 50051
curl -s http://127.0.0.1:50051/api/scry -d '{"query":"test"}'
# → 401 Unauthorized (same as before)

# Frame size enforcement
# Send >1MB length prefix over UDS → rejected before reading payload

# Phase 3: File permissions
stat -f "%Lp" ~/.patina/vault.age
# → 600

# Phase 3: ONNX integrity
echo "tampered" >> resources/models/all-MiniLM-L6-v2-int8.onnx
patina scrape
# → Error: ONNX model integrity check failed

# Phase 4: Masked input
patina secrets add test-secret
# → Value for test-secret: ******** (not echoed)
```

---

## Versioning

Security hardening is a patch release, not a feature milestone.

```
All phases complete → patina version patch → 0.10.1
                    → patina spec archive security-hardening → spec/security-hardening tag
```

Precedent: 0.9.3 (session hardening), 0.9.4 (spec archive + belief verification).

---

## Status Log

| Date | Status | Note |
|------|--------|------|
| 2026-02-03 | ready | Spec created from security audit (session 20260203-134222) |
| 2026-02-03 | in-progress | P0 shipped: 6 fixes (auth, read cap, limit cap, security headers, bind warning, consistent errors). Execution bounding deferred — needs RAII guard design. |
| 2026-02-03 | in-progress | Phase 2 spec: transport security by trust boundary. UDS at `~/.patina/run/serve.sock` default, TCP opt-in, length-prefix framing (BE u32), request-id echo, socket-dir 0o700 enforcement, safe stale-socket unlink, frame size limits. Phase 4: masked input, refuse secret values as positional args. Anchored in [[transport-security-by-trust-boundary]]. |
