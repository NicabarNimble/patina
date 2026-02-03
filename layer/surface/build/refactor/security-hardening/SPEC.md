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
| 2 | `src/commands/serve/internal.rs` | No request body size limit (rouille accepts unbounded) | Add `Content-Length` check, reject >1MB |
| 3 | `src/commands/serve/internal.rs` | `limit` field accepts any `usize` value | Cap at 1000 in `handle_scry()` before query execution |
| 4 | `src/commands/serve/internal.rs` | No query execution timeout | Wrap query in `std::thread` with timeout, return 504 on expiry |
| 5 | `src/commands/serve/internal.rs` | No CORS headers | Add `Access-Control-Allow-Origin: null` (deny all cross-origin) |
| 6 | `src/commands/serve/internal.rs:98` | No warning when binding to 0.0.0.0 | Print security warning to stderr when `--host` is not 127.0.0.1 |

### P1: Local Privilege Escalation / Secrets Hygiene

| # | File | Issue | Fix |
|---|------|-------|-----|
| 7 | `src/secrets/vault.rs:183-191` | Vault files written with default permissions (0o644) | `fs::set_permissions(path, Permissions::from_mode(0o600))` after write |
| 8 | `src/secrets/registry.rs:81-96` | Secrets registry world-readable (maps names to env vars) | Same: 0o600 on `secrets.toml` |
| 9 | `src/secrets/mod.rs:296` | SSH variant puts decrypted secrets in process argv | Use stdin pipe or `SSH_ASKPASS` protocol instead of command-line env prefix |
| 10 | `src/embeddings/onnx.rs:73` | ONNX model loaded without integrity verification | Add SHA-256 checksum verification against hardcoded expected hash |

### P2: Defense-in-Depth

| # | File | Issue | Fix |
|---|------|-------|-----|
| 11 | `src/secrets/mod.rs:482-491` | Secret input not masked (echoed to terminal) | Use `rpassword` crate for sensitive prompts |
| 12 | `src/secrets/identity.rs` | Identity strings not zeroized after use | Add `zeroize` crate, use `Zeroizing<String>` for key material |
| 13 | `src/commands/secrets.rs:258` | `--export-key` prints private key to stdout | Write to file with 0o600 permissions, or require `--stdout` flag for pipe use |
| 14 | `src/commands/repo/internal.rs:70-71` | Registry YAML path field not validated on load | Canonicalize and validate against `~/.patina/cache/` prefix during deserialization |

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

All changes in `src/commands/serve/internal.rs`:

**1a. Bearer token auth:**
```rust
fn check_auth(request: &Request, token: &str) -> bool {
    request.header("Authorization")
        .map(|h| h == format!("Bearer {}", token))
        .unwrap_or(false)
}

// In run_server():
let token = std::env::var("PATINA_SERVE_TOKEN")
    .unwrap_or_else(|_| {
        let t = generate_token();  // random 32-byte hex
        eprintln!("Generated auth token: {}", t);
        eprintln!("  Set PATINA_SERVE_TOKEN={} to use a fixed token", t);
        t
    });
```

**1b. Request limits:**
```rust
fn handle_scry(request: &Request, token: &str) -> Response {
    // Body size check
    if let Some(len) = request.header("Content-Length") {
        if len.parse::<usize>().unwrap_or(0) > 1_048_576 {
            return Response::text("Request too large").with_status_code(413);
        }
    }

    let mut body: ScryRequest = /* parse */;
    body.limit = body.limit.min(1000);  // Cap limit
}
```

**1c. CORS deny + bind warning:**
```rust
// Add to all responses:
.with_additional_header("Access-Control-Allow-Origin", "null")

// In run_server():
if options.host != "127.0.0.1" && options.host != "localhost" {
    eprintln!("WARNING: Binding to {} exposes the server to the network.", options.host);
    eprintln!("  The server has no encryption (HTTP only). Use a reverse proxy for production.");
}
```

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
- [ ] `patina serve` requires `Authorization: Bearer <token>` on `/api/scry`
- [ ] `/health` and `/version` remain open (healthcheck compatibility)
- [ ] Request body >1MB rejected with 413
- [ ] `limit` capped at 1000 regardless of input
- [ ] CORS deny header on all responses
- [ ] Warning printed when `--host` is not localhost

**Phase 2 (P1):**
- [ ] `~/.patina/vault.age` created with 0o600 permissions
- [ ] `~/.patina/secrets.toml` created with 0o600 permissions
- [ ] ONNX model verified via SHA-256 before loading
- [ ] `patina secrets run` over SSH does not expose secrets in `ps auxe`

**Phase 3 (P2):**
- [ ] Secret prompts use masked input (no echo)
- [ ] Key material zeroized after use (`Zeroizing<String>`)
- [ ] `--export-key` writes to file (0o600), not stdout
- [ ] Registry paths canonicalized and validated on load

---

## Verification

```bash
# Phase 1: HTTP daemon
curl http://127.0.0.1:50051/api/scry -d '{"query":"test"}' -H "Content-Type: application/json"
# → 401 Unauthorized (no token)

curl http://127.0.0.1:50051/api/scry -d '{"query":"test"}' -H "Authorization: Bearer $TOKEN" -H "Content-Type: application/json"
# → 200 OK (valid token)

curl http://127.0.0.1:50051/api/scry -d '{"query":"test","limit":99999}' -H "Authorization: Bearer $TOKEN" -H "Content-Type: application/json"
# → Results with max 1000 (capped)

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
