---
type: feat
id: patina-connect
status: draft
created: 2026-03-06
blocked_by:
- pipe-protocol-types
sessions:
  origin: 20260306-171859
related:
- pipe-architecture
beliefs:
- safety-boundaries
exit_criteria:
- id: oauth-flow-works
  text: '`patina connect github` completes OAuth device flow — opens browser, user approves, token stored in vault'
  checked: false
- id: connection-config-created
  text: Connection config created at ~/.patina/connections/github.toml — links credential (vault reference) to connector child
  checked: false
- id: connect-status-works
  text: '`patina connect status` shows connection health — connected/expired/missing for each configured connection'
  checked: false
- id: credentials-via-pipe
  text: github-connector receives credentials via pipe/initialize config delivery, not via environment variables or files
  checked: false
---
# feat: Connection Model — `patina connect` with OAuth Device Flow

> One command links auth to connector. `patina connect github` creates
> credential + connector child config. Replaces four manual steps
> (create PAT, store secret, grant plugin, configure source) with one
> user-approved flow.

## Problem

Current setup for GitHub data ingestion requires:
1. Create a Personal Access Token on GitHub web UI
2. `patina secrets add github-token <paste>`
3. Edit `~/.patina/plugin-config/secret-grants.toml` to grant forge
4. Run `patina scrape forge` and hope it works

This is 4 manual steps, error-prone, and doesn't support token
refresh or expiry. Users who aren't developers struggle with PATs.

## Solution

`patina connect <provider>` replaces all manual steps with one
OAuth device flow:

```
$ patina connect github
  Opening browser for GitHub authorization...
  Enter code: ABCD-1234

  Waiting for approval... approved!
  Token stored in vault: github:user
  Connection configured: ~/.patina/connections/github.toml
  Done. GitHub data flows on next mother run.
```

Connection management commands:
- `patina connect github` — setup via OAuth device flow
- `patina connect status` — show all connections and health
- `patina connect refresh github` — re-authorize if expired
- `patina connect remove github` — remove credential + config

### Connection Config

```toml
# ~/.patina/connections/github.toml
[connection]
name = "github"
provider = "github"
credential = "github:user"      # references vault secret
child = "github-connector"      # which child binary to use
created = "2026-03-06T00:00:00Z"

[oauth]
client_id = "Iv1.xxxxxxxx"      # Patina's registered OAuth app
scopes = ["repo", "read:org"]
```

This is the evolution of `patina secrets` for external sources. The
vault stays the same (age-encrypted, Keychain + Touch ID). The
addition: one command creates both the credential AND the connector
configuration.

## Steps

1. Register Patina as a GitHub OAuth App (device flow enabled)
2. Create `src/connect/` module — connection management logic
3. Implement OAuth device flow (RFC 8628): device authorization
   request, user code display, polling for token
4. Store acquired token in vault via existing secrets infrastructure
5. Create connection config at `~/.patina/connections/<name>.toml`
6. Add `patina connect` CLI commands (connect, status, refresh, remove)
7. Wire credential delivery: Mother reads connection config, decrypts
   credential from vault, passes via pipe/initialize to child
8. Verify: `patina connect github` → `patina mother run github`
   works end-to-end

## Key Files

**Build on:**
- `src/secrets/mod.rs` — vault, identity, session caching (reuse)
- `src/secrets/vault.rs` — age encryption (reuse for token storage)
- [[spec-pipe-architecture]] DESIGN.md §3 (Connection Model)

**New:**
- `src/connect/mod.rs` — connection management
- `src/connect/oauth.rs` — OAuth device flow
- `src/commands/connect.rs` — CLI commands

## Non-Goals

- Supporting providers beyond GitHub initially (Slack, etc. come later
  using the same connection infrastructure)
- Building the routing engine (that's [[spec-mother-broker]])
- Token refresh automation (manual `patina connect refresh` first,
  Mother-automated refresh is future work)
- Replacing `patina secrets` entirely — secrets remains for non-OAuth
  credentials (API keys, manual tokens). Connect is specifically for
  OAuth-capable providers.
