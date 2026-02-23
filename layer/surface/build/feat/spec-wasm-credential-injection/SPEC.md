---
type: feat
id: spec-wasm-credential-injection
status: draft
created: 2026-02-23
related:
- layer/surface/build/fix/spec-launcher-auth/SPEC.md
beliefs:
- wasm-host-boundary-hides-credentials
- local-first-credential-isolation
- defense-in-depth-over-perfect-isolation
- storage-encryption-vs-runtime-isolation
- bearer-token-forces-plaintext-exposure
- two-layer-capability-grants
sessions:
- 20260222-165738
- 20260222-200024
- 20260223-061011
---

# feat: WASM Host-Boundary Credential Injection

> Wire the secrets vault to the WASM plugin host boundary so plugins can make
> authenticated API calls without ever seeing credentials. Credentials are
> decrypted only inside the host, injected into HTTP requests, and scrubbed
> from responses.

## Problem

WASM plugins with `host_http` access can make HTTP calls through the host boundary.
The host already validates domains against an allowlist (`plugin.toml` capabilities).
But there is no way to make **authenticated** calls — the host sends requests without
credentials, and plugins have no access to the secrets vault.

Today, authenticated external API calls (GitHub, Gitea, Google) require either:
- Shelling out to CLI tools (`gh`, `gcloud`) — impossible from WASM sandbox
- Environment variable injection — exposes credentials to LLM process

Neither works for credential isolation. The host boundary is the right place to
inject credentials because:
- Host controls the `reqwest` client (owns the request before it hits the wire)
- Domain allowlisting already validates at load-time AND call-time
- Secrets vault already stores and decrypts credentials
- Plugin physically cannot access host memory (WASM sandbox guarantee)

## Solution

### Credential Mapping in Plugin Manifests

Extend `plugin.toml` to declare credential needs per domain:

```toml
[capabilities]
host_http = ["api.github.com"]

[capabilities.host_secrets]
"api.github.com" = { secret = "github-token", location = "bearer" }
```

Fields:
- **secret**: Name in patina secrets vault (e.g., `github-token`)
- **location**: How to inject. Start with `bearer` (Authorization: Bearer header)

Future injection locations (not in this spec):
- `basic` — Authorization: Basic (base64 encoded)
- `header` — Custom header name (e.g., X-API-Key)
- `query` — URL query parameter

### Host-Boundary Injection

When a plugin calls `host_http_post` or `host_http_get`:

```
Plugin: host_http_post("https://api.github.com/repos/owner/repo/pulls", body, "application/json")
                                    │
Host (host_support.rs):             │
  1. Extract domain ─────────────── api.github.com
  2. Validate domain allowlist ──── ✓ in grants.http_domains
  3. Look up credential mapping ─── host_secrets["api.github.com"]
  4. Decrypt secret from vault ──── secrets::get_global_secret("github-token")
  5. Inject auth header ─────────── Authorization: Bearer ghp_xxx
  6. Send request ───────────────── reqwest POST with injected header
  7. Receive response
  8. Leak-detect response body ──── scan for "ghp_xxx" in body
  9. Return sanitized response ──── to plugin
```

The credential exists in host memory only for steps 4-6. It is never passed to
the plugin, never stored in WASM linear memory, never returned in any result.

### Leak Detection

Before returning HTTP response bodies to the plugin, scan for known secret values:

```rust
fn leak_check(body: &str, secrets: &[(String, String)]) -> String {
    let mut sanitized = body.to_string();
    for (name, value) in secrets {
        if sanitized.contains(value) {
            log::warn!(
                "Credential leak detected in response: secret '{}' found in body, redacting",
                name
            );
            sanitized = sanitized.replace(value, "[REDACTED]");
        }
    }
    sanitized
}
```

This catches APIs that echo credentials in responses (some OAuth endpoints,
misconfigured services).

### Capability Validation

Credential mappings are validated at **two points** (defense in depth):

**Load-time** (plugin manifest parsing in `check_capabilities`):
- Every domain in `host_secrets` must also be in `host_http` allowlist
- Secret names probed against vault via `get_global_secret()` — warn if
  missing or decryption fails, don't block load. Surfaces misconfig at
  plugin load rather than first HTTP call.
- Injection location must be a supported type

**Call-time** (host_support.rs):
- Domain extracted from URL must match `host_secrets` mapping
- Secret decryption attempted only for mapped domains
- If decryption fails, request proceeds WITHOUT credentials (log warning)
- Plugin is never told whether injection happened or not

## Implementation

### Modified Files

**`src/plugin/internal/host_support.rs`** — Core change:
- Add `credentials: HashMap<String, CredentialMapping>` to function signatures
- Look up credential mapping by domain in `http_get` and `http_post`
- Decrypt secret via `crate::secrets::get_global_secret()`
- Inject Authorization header into reqwest request
- Leak-detect response body before returning

**`src/plugin/internal/mod.rs`** — Manifest parsing:
- Parse `[capabilities.host_secrets]` from plugin.toml
- Validate: every host_secrets domain must be in host_http list
- Store credential mappings in `GrantedCapabilities`

**`src/plugin/internal/mother_child.rs`** and **`src/plugin/internal/command.rs`**:
- Pass credential mappings through to host_support HTTP functions

### New Types

```rust
/// How to inject a credential into an HTTP request
pub enum InjectionLocation {
    Bearer,  // Authorization: Bearer {value}
}

/// Maps a domain to a vault secret and injection method
pub struct CredentialMapping {
    pub secret_name: String,
    pub location: InjectionLocation,
}
```

### Dependencies

None new. Uses existing:
- `crate::secrets::get_global_secret()` — already implemented
- `reqwest::blocking::Client` — already used in host_support
- Plugin manifest parsing — already in internal/mod.rs

### What This Does NOT Change

- **WIT interface**: `host_http_get` and `host_http_post` signatures stay the same.
  Plugins don't know credentials are being injected. That's the point.
- **Plugin SDK**: No changes needed. Plugins call the same HTTP functions.
- **Existing plugins**: Unaffected. No `host_secrets` = no injection = same behavior.
- **MCP server**: Not touched. MCP→WASM bridge is a separate spec.
- **Secrets vault**: No changes. Uses existing `get_global_secret()`.

## Testing

### Exit Criteria

**1. Credential injection works**
```
Given: A mother-child plugin with host_secrets mapping for api.github.com
When: Plugin calls host_http_get("https://api.github.com/user")
Then: Request includes Authorization: Bearer header with decrypted secret
And: Plugin receives response body (user info JSON)
And: Plugin never receives the credential value
```

**2. Missing secret degrades gracefully**
```
Given: A plugin with host_secrets mapping for a secret not in vault
When: Plugin calls host_http_get for that domain
Then: Request is sent WITHOUT credentials
And: Warning logged: "secret 'xxx' not found in vault, sending unauthenticated"
And: Plugin receives response (likely 401)
And: No crash, no error propagation to plugin
```

**3. Leak detection works**
```
Given: A plugin making an HTTP call to a service that echoes the credential
When: Response body contains the secret value
Then: Secret value is replaced with [REDACTED] before returning to plugin
And: Warning logged: "credential leak detected"
```

**4. Manifest validation works**
```
Given: A plugin.toml with host_secrets domain not in host_http list
When: Plugin is loaded
Then: Load fails with clear error: "domain 'x' in host_secrets but not in host_http"
```

**5. No injection without mapping**
```
Given: A plugin with host_http but no host_secrets
When: Plugin calls host_http_get for any domain
Then: No credential injection attempted (existing behavior preserved)
```

**6. Domain mismatch is safe**
```
Given: A plugin with host_secrets for api.github.com only
When: Plugin calls host_http_get("https://api.other.com/...")
Then: No credential injection for api.other.com
And: Request sent without credentials for unmapped domain
```

## Non-Goals

- **New WIT interface for secrets** — Plugins should NOT be able to request
  secret values. The whole point is they never see them. A `get-secret` host
  function would defeat the architecture.
- **MCP bridge** — Exposing WASM plugins as MCP tools is a separate spec.
  This spec wires the credential plumbing that future MCP-callable plugins
  will use.
- **Skills generation** — Teaching LLMs to use credential-isolated tools via
  skills is a separate concern. This spec builds the foundation.
- **OAuth flow management** — This spec injects stored tokens. Acquiring tokens
  (OAuth flows, refresh) is out of scope. Users store tokens via
  `patina secrets add`.
- **Multiple injection types** — Start with `bearer` only. Add `basic`, `header`,
  `query` when a real plugin needs them.
- **Per-request credential override** — Plugin cannot choose which credential to
  use. The mapping is declared in the manifest, enforced by the host.
- **New plugins** — No github-tools or gitea-tools plugin in this spec. The exit
  criteria use an existing or minimal test plugin. Real service plugins come later.

## Security Review

| Threat | Mitigation | Status |
|--------|------------|--------|
| Plugin reads credential from WASM memory | Credential never enters WASM linear memory | Mitigated |
| Plugin extracts credential from response | Leak detection scans and redacts | Mitigated |
| Plugin calls unmapped domain with credential | Domain must be in both host_http AND host_secrets | Mitigated |
| Manifest declares secret it shouldn't access | Secret grants gate: `resolve_credential` checks `~/.patina/plugin-config/secret-grants.toml` before decrypting. Ungated plugins get no secrets. | Mitigated |
| Credential lingers in host memory | Decrypted only for request duration, Rust ownership drops it after | Mitigated |
| Multiple plugins share same secret | Each plugin has independent mapping, host decrypts per-call | No cross-plugin leakage |

## Addendum: Post-Review Fixes (2026-02-23)

Three findings from agent review, all addressed in this spec:

### A1. Secret Grants Gate (P1 — supply-chain risk)

`resolve_credential` must check a user-maintained allowlist before decrypting
any secret. Without this, a malicious plugin manifest can name any well-known
vault key and exfiltrate it to any declared domain.

**Fix:** `resolve_credential` reads `~/.patina/plugin-config/secret-grants.toml`
before calling `get_global_secret()`. Format:

```toml
[patina-github]
secrets = ["github-token"]

[my-custom-plugin]
secrets = ["slack-webhook", "openai-key"]
```

Rules:
- If the file doesn't exist: deny all secret access, log actionable message
  (`"no secret-grants.toml — run 'patina plugin grant <plugin> <secret>' to allow"`)
- If the file exists but the plugin isn't listed: deny, warn
- If the plugin is listed but the secret isn't in its `secrets` array: deny, warn
- Only when plugin + secret both match: proceed to `get_global_secret()`

This implements the second layer of [[two-layer-capability-grants]]: manifest
declares what the plugin wants, host decides what to allow.

### A2. Load-Time Vault Probe (P2 — silent misconfig)

The spec already stated "Secret names validated against vault (warn if missing,
don't block load)" under Capability Validation, but the implementation skipped
this. Fix: `check_capabilities` calls `get_global_secret()` for each secret
in `host_secrets` and emits a warning if the secret is missing or decryption
fails. This surfaces misconfigurations at plugin load time, not at first HTTP
call.

### A3. HTTP Injection Path Test (P3 — regression risk)

The test suite covers manifest parsing and the pure `leak_check` helper but
nothing exercises the actual `http_get`/`http_post` injection path. A regression
(forgetting to call `inject_credential` or `leak_check`) would compile and
all tests would still pass.

**Fix:** Add unit tests that construct `GrantedCapabilities` with credential
mappings and call `http_get`/`http_post` against a localhost test server (or
verify the request builder state). At minimum, test that:
- `http_get` with a credential mapping adds the Authorization header
- `http_get` with a credential mapping leak-checks the response
- `http_get` without a mapping sends no Authorization header

### Exit Criteria (addendum)

**7. Secret grants gate blocks ungated plugins**
```
Given: A plugin with host_secrets for "github-token"
And: No secret-grants.toml exists (or plugin not listed)
When: Plugin calls host_http_get for the mapped domain
Then: No credential is injected (request sent unauthenticated)
And: Warning logged with actionable message about granting
```

**8. Secret grants gate allows gated plugins**
```
Given: A plugin "my-plugin" with host_secrets for "github-token"
And: secret-grants.toml lists my-plugin with secrets = ["github-token"]
When: Plugin calls host_http_get for the mapped domain
Then: Credential is injected normally
```

**9. Load-time vault probe warns on missing secret**
```
Given: A plugin with host_secrets referencing "nonexistent-secret"
When: Plugin is loaded (check_capabilities)
Then: Warning logged: "secret 'nonexistent-secret' not found in vault"
And: Plugin loads successfully (not blocked)
```

**10. HTTP injection path has automated test coverage**
```
Given: Unit test constructs GrantedCapabilities with credential mapping
When: http_get is called against a test endpoint
Then: Test verifies Authorization header was added to the request
And: Test verifies leak_check was applied to response body
```

## Reference

- **IronClaw `credential_injector.rs`**: Production implementation of same pattern
- **IronClaw `SharedCredentialRegistry`**: Thread-safe credential mapping aggregation
- **Belief: [[wasm-host-boundary-hides-credentials]]**: Architecture rationale
- **Belief: [[local-first-credential-isolation]]**: Why local-first beats server-dependent
- **Completed: spec-secrets-dual-storage (v0.28.0)**: Vault infrastructure this builds on
