---
type: refactor
id: security-hardening
status: ready
created: 2026-02-03
sessions:
  origin: 20260203-134222
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

### Deferred: Execution Bounding (needs design)

| # | File | Issue | Needs |
|---|------|-------|-------|
| — | `src/commands/serve/internal.rs` | No query execution timeout or concurrency bound | Proper design: RAII drop guard, atomic increment-then-check, panic safety. Naive `thread::spawn` + `recv_timeout` leaks threads on timeout and a bare semaphore has TOCTOU race + panic counter leak. Spec separately. |

### P1: Local Privilege Escalation / Secrets Hygiene

| # | File | Issue | Fix |
|---|------|-------|-----|
| 7 | `src/secrets/vault.rs:183-191` | Vault files written with default permissions (0o644) | `fs::set_permissions(path, Permissions::from_mode(0o600))` after write |
| 8 | `src/secrets/registry.rs:81-96` | Secrets registry world-readable (maps names to env vars) | Same: 0o600 on `secrets.toml` |
| 9 | `src/secrets/mod.rs:296` | SSH variant puts decrypted secrets in process argv | Use stdin pipe or `SSH_ASKPASS` protocol instead of command-line env prefix |
| 10 | `src/embeddings/onnx.rs:73` | ONNX model loaded without integrity verification | Add SHA-256 checksum verification against hardcoded expected hash |
| 11 | `src/commands/serve/internal.rs` | Serve token bypasses Patina's own secrets infrastructure | Load token from vault via `patina secrets`, fall back to `PATINA_SERVE_TOKEN` env, generate-and-print as last resort |

Item 11 rationale: The serve token is currently a plain env var — visible in `ps auxe`,
shell history, terminal scrollback, and container environment. On localhost this is
acceptable, but on `0.0.0.0` (container use case) the token is the sole network
boundary and should use the same vault/keychain infrastructure that protects other
Patina secrets. Load order: vault → env var → generate. This keeps the simple
`PATINA_SERVE_TOKEN=xxx patina serve` workflow for local dev while giving the
container path a proper secret.

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

### Phase 2: P1 — File Permissions + Model Integrity (4 changes, 4 files)

**2a. Vault permissions** (`src/secrets/vault.rs`):
```rust
use std::os::unix::fs::PermissionsExt;

fs::write(vault_path, encrypted)?;
fs::set_permissions(vault_path, fs::Permissions::from_mode(0o600))?;
```

**2b. Registry permissions** (`src/secrets/registry.rs`):
Same pattern — `fs::set_permissions` after `fs::write`.

**2c. ONNX checksum** (`src/embeddings/onnx.rs`):
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

**2d. SSH secrets via stdin** (`src/secrets/mod.rs`):
Replace argv-based env prefix with stdin pipe to remote shell.

### Phase 3: P2 — Defense-in-Depth (4 changes, 4 files)

Add `rpassword` and `zeroize` crates. Update prompts and key handling. Validate registry paths.

---

## New Dependencies

| Crate | Purpose | Phase |
|-------|---------|-------|
| `sha2` | ONNX model checksum | Phase 2 |
| `rpassword` | Masked input for secrets | Phase 3 |
| `zeroize` | Zero key material on drop | Phase 3 |

Check if `sha2` is already a transitive dependency (likely via `age`).

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

**Phase 2 (P1):**
- [ ] `~/.patina/vault.age` created with 0o600 permissions
- [ ] `~/.patina/secrets.toml` created with 0o600 permissions
- [ ] ONNX model verified via SHA-256 before loading
- [ ] `patina secrets run` over SSH does not expose secrets in `ps auxe`
- [ ] Serve token loaded from vault when available, env var as fallback

**Phase 3 (P2):**
- [ ] Secret prompts use masked input (no echo)
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

# Phase 2: File permissions
stat -f "%Lp" ~/.patina/vault.age
# → 600

# Phase 2: ONNX integrity
echo "tampered" >> resources/models/all-MiniLM-L6-v2-int8.onnx
patina scrape
# → Error: ONNX model integrity check failed

# Phase 3: Masked input
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
| 2026-02-03 | in-progress | P0 shipped: 6 fixes (auth, read cap, limit cap, security headers, bind warning, consistent errors). Execution bounding (timeout + concurrency gate) deferred — naive thread::spawn + semaphore had TOCTOU race, panic leak, and thread abandonment. Needs proper design with RAII guard. |
