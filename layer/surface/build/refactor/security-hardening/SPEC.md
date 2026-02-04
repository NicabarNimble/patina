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

## Threat Model

Patina runs inside an LLM's execution environment (Claude Code, Gemini, etc). The
LLM has the same UID, filesystem access, and network access as the user. This
shapes what security can and cannot do.

### The LLM is the exfiltration channel, not the attacker

The LLM runs as you — file permissions, socket permissions, and bearer tokens don't
stop it. It can run any command you can run. But every token of its context is
transmitted to the API provider (Anthropic, OpenAI, Google). The LLM doesn't need to
be malicious. Structurally, everything it sees leaves the machine.

A secret printed to stderr → LLM sees it → sent to the cloud.
A secret in `ps auxe` → LLM sees it → sent to the cloud.
A secret in a file the LLM reads → sent to the cloud.

**Design principle: keep secrets out of the context window.** Secrets flow through
injection paths (`secrets run -- cmd`), never through output paths the LLM observes.

### Who we protect against

| Actor | Threat | Mitigation |
|-------|--------|------------|
| Network observers | Sniff bearer tokens, query data on wire | UDS for local (no wire). TCP is explicit opt-in. |
| Other local users | Read socket, files, connect to daemon | Socket dir 0o700, socket 0o600, vault 0o600 |
| Other local processes | Probe known ports, read predictable paths | UDS (no TCP port by default), strict dir perms |
| Containers | Reach host daemon across network boundary | TCP path requires explicit flag + bearer token |
| Prompt injection | Malicious file tricks LLM into reading secrets | Vault encrypted at rest (ciphertext is useless). Touch ID gates decryption (human in the loop). |
| LLM context leakage | Secrets appear in conversation → transmitted to API provider | Never print secrets to stdout/stderr. Token to file, not terminal. UDS eliminates token for localhost entirely. |

### Who we do NOT protect against

| Actor | Why not |
|-------|---------|
| The LLM itself (locally) | It runs as you. Same UID. Same access. By design. |
| The user | They're the operator. |
| API provider with conversation access | The conversation is transmitted. We minimize what enters it, but can't prevent the LLM from running commands that produce secret output. |

### What each layer protects

| Layer | Protects against |
|-------|-----------------|
| Age vault (encrypted at rest) | LLM reads file → sees ciphertext. Other users → sees ciphertext. Disk theft → sees ciphertext. |
| Touch ID / Keychain | Silent decryption. LLM can't decrypt without human's finger on the sensor. |
| Session cache (serve daemon) | Repeated Touch ID prompts. Secrets stay in daemon memory, not in files the LLM can cat. |
| UDS transport (Phase 2) | No token generated for localhost → nothing enters LLM context. No TCP port → no network exposure. |
| File permissions (Phase 3) | Other users reading vault, registry, socket. |
| No secrets in output (Phase 2, #16) | Token never printed to stderr → never in terminal scrollback → never in LLM context. |

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
| 12 | `src/secrets/mod.rs:482-491` | Secret input not masked (echoed to terminal) | Use `console::Term::read_secure_line()` (already in tree) |
| 13 | `src/secrets/identity.rs` | Identity strings not zeroized after use | Use `zeroize::Zeroizing<String>` (already in tree via `age`) |
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

### Phase 2: Transport Security by Trust Boundary

Anchored in [[transport-security-by-trust-boundary]], [[use-whats-in-the-tree]].
Design principle (djb): default safe, insecure path hard. Prior art: PostgreSQL, Docker.

#### Design: Replace rouille with blocking HTTP-over-UDS microserver

**Key insight:** rouille only accepts `TcpListener`. Rather than running two protocols
(HTTP for TCP, custom framing for UDS), replace rouille with a minimal blocking HTTP
server that handles both `UnixListener` and `TcpListener`. One protocol (HTTP)
everywhere. Same wire format. Same client code. Same `curl` for testing.

**What we drop:** `rouille` (pulls in `tiny-http` + its own HTTP parser).
**What we add:** `httparse` (~1000 lines, zero deps, zero-copy HTTP/1.x parser).
Net reduction in dependency surface.

rouille currently touches 3 lines: one import, one `start_server` call, one `router!`
macro. The business logic (structs, handlers, query engine) is already pure Rust with
serde. The refactor is surgical.

#### Transport model

- **Default:** `UnixListener` at `~/.patina/run/serve.sock` — file permissions are auth
- **Opt-in:** `TcpListener` at `--host`/`--port` — bearer token required
- Both speak HTTP/1.1. Both call the same handlers. Clients don't know the difference.

#### 2a. Microserver (`serve/internal.rs`)

**Intentionally minimal HTTP surface:**
- One request per connection. Read request, write response, close stream.
- POST requires Content-Length. Read body with `Read::take(MAX_BODY_SIZE + 1)` —
  do not trust Content-Length for size enforcement (lesson from P0 Gotcha A).
- Header cap: 32 KiB. Body cap: 1 MiB.
- No chunked encoding (reject). No keep-alive (close after response).
- Sufficient for: `/health`, `/version`, `/api/scry`, `/secrets/cache`, `/secrets/lock`.

**Internal request/response types (transport-free):**
```rust
struct HttpRequest {
    method: String,
    path: String,
    headers: Vec<(String, String)>,
    body: Vec<u8>,
}

struct HttpResponse {
    status: u16,
    body: Vec<u8>,  // JSON
}
```

**Router:** Replace `router!` macro with plain match:
```rust
match (request.method.as_str(), request.path.as_str()) {
    ("GET", "/health")    => handle_health(state),
    ("GET", "/version")   => handle_version(state),
    ("POST", "/api/scry") => handle_scry(state, &request),
    _                     => json_error(404, "Not found"),
}
```

**Transport-free handlers:**
```rust
fn handle_health(state: &ServerState) -> HttpResponse { ... }
fn handle_scry(state: &ServerState, req: &HttpRequest) -> HttpResponse { ... }
```

Auth check, body parsing, size enforcement happen in the per-connection handler
before dispatch. Business logic never sees HTTP.

**Accept loop:** `std::thread::spawn` per connection (same spirit as rouille).
Both `UnixStream` and `TcpStream` implement `Read + Write` — the per-connection
handler is generic over the stream type.

#### 2b. Socket safety (`serve/mod.rs`)

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

**Peer identity (future enhancement):**
- macOS: `getpeereid(fd, &uid, &gid)`
- Linux: `getsockopt(SO_PEERCRED)`
- Can add later without changing external API.

#### 2c. Session client (`secrets/session.rs`)

Try UDS first (no auth needed). Fall back to HTTP + token if no socket.

UDS client is a small function (~40 lines): open `UnixStream`, write HTTP request
bytes, read response, parse with `httparse`. No `reqwest` needed for local path.
`reqwest` stays for the TCP fallback (containers).

```rust
fn connect() -> Transport {
    let sock = paths::serve::socket_path();
    if sock.exists() { Transport::Uds(sock) }
    else { Transport::Http { base_url: serve_url(), token: serve_token() } }
}
```

#### 2d. Mother client (`mother/internal.rs`)

UDS-first for localhost addresses, TCP+bearer token for remote. Same pattern as
`session.rs`: `Client::new(address)` detects localhost via `is_localhost()`, loads
token via `serve_token()` (file → env resolution). Each method (`health`, `scry`)
tries UDS first when `try_uds` is true, falls back to TCP+token. `reqwest` stays
for remote/container path.

#### 2e. Secrets never in output (#16)

Generated token never printed to stderr. For TCP opt-in path: write token to
`~/.patina/run/serve.token` (0o600), print path not value. UDS path needs no
token at all.

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

## Dependencies

**Phase 2:** Replace `rouille` with `httparse`. Net dependency reduction.

| Change | Crate | Notes |
|--------|-------|-------|
| Remove | `rouille` | Pulls in `tiny-http` + HTTP parser. Only touched 3 lines. |
| Add | `httparse` | ~1000 lines, zero deps, zero-copy. HTTP/1.x request parser only. |

**Phase 3-4:** All crates already in tree via `age` and `console`:

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
- [x] `rouille` removed from Cargo.toml, replaced by `httparse` + blocking microserver
- [x] `patina serve` defaults to unix socket at `~/.patina/run/serve.sock`
- [x] TCP listener only starts with explicit `--host`/`--port`
- [x] Both transports speak HTTP/1.1 — one protocol, same handlers
- [x] Socket dir `~/.patina/run/` created 0o700, verified on startup, refuse if open
- [x] Socket file set to 0o600 after bind
- [x] Stale socket: unlink only if is-socket AND owned-by-current-user, else refuse
- [x] Header cap: 32 KiB. Body cap: 1 MiB (read cap via `Read::take`, not Content-Length trust)
- [x] No chunked encoding, no keep-alive (one request per connection)
- [x] Socket deleted on clean shutdown
- [x] `session.rs` connects via UDS first (small HTTP-over-UDS client), HTTP+token fallback
- [x] `mother/internal.rs` UDS-first for localhost, TCP+bearer token for remote
- [x] Handlers are transport-free — no rouille types, no HTTP types in business logic
- [x] Generated token never printed to stderr — write to file, print path
- [x] `curl --unix-socket` works for testing UDS endpoints

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

# UDS works with curl — same HTTP, no token needed for local
curl -s --unix-socket ~/.patina/run/serve.sock http://localhost/health
# → {"status":"ok","version":"0.10.0","uptime_secs":123}

curl -s --unix-socket ~/.patina/run/serve.sock http://localhost/api/scry \
  -d '{"query":"test"}' -H "Content-Type: application/json"
# → 200 OK (no bearer token required over UDS)

# TCP still requires auth when explicitly enabled
patina serve --host 127.0.0.1 --port 50051
curl -s http://127.0.0.1:50051/api/scry -d '{"query":"test"}'
# → 401 Unauthorized

# Body size enforcement (same read-cap approach as P0)
dd if=/dev/zero bs=1048577 count=1 2>/dev/null | curl -s -X POST \
  --unix-socket ~/.patina/run/serve.sock http://localhost/api/scry -d @-
# → {"error":"Request too large"} 413

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
| 2026-02-03 | in-progress | Phase 2 revised: replace rouille with blocking HTTP-over-UDS microserver. Drop custom length-prefix protocol — one protocol (HTTP) over both transports. `httparse` replaces `rouille` (net dep reduction). `curl --unix-socket` for testing. Transport-free handlers. Anchored in [[transport-security-by-trust-boundary]], [[use-whats-in-the-tree]]. |
| 2026-02-04 | in-progress | Phase 2 complete: `mother/internal.rs` updated — UDS-first for localhost (same pattern as session.rs), TCP+bearer token for remote. Fixes pre-existing bug: mother client sent no auth header → 401 since Phase 1. Added `is_localhost()`, `serve_token()`, UDS client functions, 7 unit tests. All 15 Phase 2 exit criteria checked. |
