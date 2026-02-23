---
type: fix
id: spec-launcher-auth
status: active
created: 2026-02-18
related:
- layer/surface/build/feat/spec-launcher-tmux/SPEC.md
beliefs:
- patina-identity
- transport-security-by-trust-boundary
- storage-encryption-vs-runtime-isolation
- defense-in-depth-over-perfect-isolation
---

# fix: Launcher Auth — Long-Lived Token via Patina Secrets

> Patch the launcher to inject a long-lived Claude OAuth token from
> `patina secrets` so Max subscription works in headless/SSH/tmux
> sessions without a browser.

## Problem

Claude Code authenticates via OAuth browser flow. The resulting token
expires in 8-12 hours. When `patina` launches Claude in tmux and the
user reconnects from a phone or remote SSH, three failure modes exist:

1. **Token expired** — Claude prompts for re-auth, which opens a
   browser that doesn't exist in the headless SSH session. Dead end.
2. **Fresh launch after crash** — same browser problem.
3. **New machine via Desktop App SSH** — Claude Code runs on the remote
   host but has no cached credentials. Login flow fails headless.

The yolo devcontainer system solved a related problem (credential
persistence across container rebuilds) but used a different mechanism
(`.credentials.json` bind-mount + `--dangerously-skip-permissions`).
That approach is container-specific and bundles auth with permissions
bypass. This spec solves auth for the native launcher without touching
permissions.

## Solution

Use `claude setup-token` to generate a **1-year OAuth token**
(`sk-ant-oat01-...`) scoped to Claude Code, stored in patina's
age-encrypted global vault, and injected as `CLAUDE_CODE_OAUTH_TOKEN`
when the launcher execs the Claude adapter.

### Why This Works

- `claude setup-token` produces a long-lived token that bills against
  the user's Max subscription — same billing as normal `claude login`
- `CLAUDE_CODE_OAUTH_TOKEN` env var is Claude Code's standard mechanism
  for headless auth (used in CI, containers, GitHub Actions)
- Patina secrets already handles age encryption, Touch ID unlock, and
  env var injection via `patina secrets run`
- The token is **Claude-adapter-specific** — Gemini and OpenCode have
  their own auth; this doesn't touch them

### What This Is NOT

- **Not `--dangerously-skip-permissions`** — that stays in yolo
  territory. This spec is about authentication, not permission bypass.
  The launcher does not add any permission flags.
- **Not a replacement for `claude login`** — users who work locally
  with a browser can keep using OAuth. This is an opt-in escape hatch
  for remote/headless workflows.
- **Not a credential file copy** — unlike yolo's `.credentials.json`
  bind-mount, the token lives in patina's encrypted vault and is
  injected as an env var. No plaintext files on disk.

## Insertion Point

`src/commands/launch/internal.rs` — `launch_adapter_cli()`.

Today (after spec-launcher-tmux), the Command is built inline:
```rust
let err = Command::new("tmux")
    .args(["new-session", "-A", "-D", "-s", session_name, "-c"])
    .arg(project_path.as_os_str())
    .arg(adapter_name)           // bare "claude"
    .current_dir(project_path)
    .exec();
```

This chains `.exec()` directly — no opportunity to add `.env()` calls.
The refactor splits Command construction from exec:

```rust
// Build the adapter command (tmux-wrapped or direct)
let mut cmd = match decision {
    TmuxDecision::Auto => {
        let mut c = Command::new("tmux");
        c.args(["new-session", "-A", "-D", "-s", session_name, "-c"]);
        c.arg(project_path.as_os_str());
        c.arg(adapter_name);
        c.current_dir(project_path);
        c
    }
    TmuxDecision::Off(_) => {
        let mut c = Command::new(adapter_name);
        c.current_dir(project_path);
        c
    }
};

// Inject Claude auth token if available (adapter-gated).
// All warnings print HERE — before exec/tmux takes over stderr.
if adapter_name == "claude" {
    if let Some(token) = try_get_claude_token() {
        cmd.env("CLAUDE_CODE_OAUTH_TOKEN", &token);
    }
}
io::stderr().flush().ok();

// exec replaces process — only returns on error
let err = cmd.exec();
```

**Warning timing:** `try_get_claude_token()` prints conflict-guard
warnings and decryption-failure warnings to stderr internally, before
returning. The flush before exec guarantees the user sees them in the
terminal before tmux takes over. Once `exec()` fires, patina's stderr
is gone — anything printed after that point is lost. The existing
tmux reconnect hint (from spec-launcher-tmux) also prints before exec,
so both sets of messages share the same timing guarantee.

The `let mut cmd` pattern lets us add `.env()` between construction
and exec. The tmux-wrapping logic, fallback-on-error, and stderr
messaging from spec-launcher-tmux are preserved — only the Command
construction is restructured, not the control flow.

## Design

### Secret Name Convention

```
Secret name: claude-oauth
Env var:     CLAUDE_CODE_OAUTH_TOKEN
Vault:       global (~/.patina/vault.age)
```

Global vault because the token is tied to the user's Anthropic account,
not a specific project. All projects sharing the same Max subscription
use the same token.

### Token Lifecycle

```
One-time setup (requires a machine with a browser — Mac GUI, not SSH):
  1. claude setup-token          → opens browser OAuth once
                                 → generates sk-ant-oat01-... (1-year token)
  2. patina secrets add claude-oauth \
       --env CLAUDE_CODE_OAUTH_TOKEN \
       --global --stdin          → Touch ID to encrypt into vault

Every launch (works headless):
  3. patina launch detects adapter == "claude"
  4. Attempts vault decryption:
     - macOS: Touch ID (or session cache if patina serve running)
     - Headless/Linux: PATINA_IDENTITY env var
  5. If token found: set CLAUDE_CODE_OAUTH_TOKEN in exec environment
  6. If not found: launch without it (normal OAuth flow)
```

**Onboarding constraint:** Step 1 (`claude setup-token`) requires a
browser for the OAuth flow. This means the user must run it once from
a machine with a GUI — their Mac desktop, not a headless SSH session.
The token is then stored in the vault and works headless forever after.

If the user is SSH'd into their Mac from a phone and needs to set up
the token for the first time, they must either:
- Walk to the Mac and run `claude setup-token` there, or
- Use screen sharing / VNC to complete the browser flow remotely

This is a one-time cost (~1 year token lifetime). A future
`patina auth setup` convenience command could guide the user through
the flow and detect when a browser is unavailable, but that is out of
scope for this spec.

### Token Conflict Guard

The launcher checks three env vars before injection, in order:

1. **`ANTHROPIC_API_KEY` is set** — user has explicitly chosen API key
   billing. Do NOT inject `CLAUDE_CODE_OAUTH_TOKEN`. Warn to stderr:
   `"patina: ANTHROPIC_API_KEY set — skipping vault token injection (API key takes priority)"`
2. **`CLAUDE_CODE_OAUTH_TOKEN` is already set** — user (or wrapper
   script) provided their own token. Do NOT overwrite. Warn to stderr:
   `"patina: CLAUDE_CODE_OAUTH_TOKEN already set — skipping vault token injection"`
3. **Neither set** — inject from vault if available.

Warnings are user-visible (stderr, not debug-level) so the user
understands why their vault token wasn't used. Single line each,
prefixed with `patina:` for grep-ability.

### Adapter Gating

Only the `claude` adapter gets token injection. The check is a simple
string match on `adapter_name` in `launch_adapter_cli()`. Other
adapters (gemini, opencode) pass through unchanged. If future adapters
need similar auth injection, the pattern is clear but not pre-built —
no abstract "adapter auth" trait. YAGNI.

### Decryption Strategy

The launcher needs only `claude-oauth` from the global vault. It must
NOT call `load_merged_secrets()` or `load_all_secrets()` — those
decrypt both global and project vaults, which means:
- A corrupted/missing project vault would block launch
- Unnecessary Touch ID prompt for project secrets we don't need
- Decrypting all secrets when we need exactly one

**New helper:** Add `get_global_secret(name: &str) -> Result<Option<String>>`
to `src/secrets/mod.rs`. This function:
1. Checks session cache first (via `patina serve` if running)
2. If miss, decrypts only `~/.patina/vault.age` (global)
3. Looks up `name` in the decrypted vault values
4. Returns `Ok(None)` if secret not found, `Err` only on decrypt failure

The launcher wraps this in `try_get_claude_token()` which catches all
errors and returns `Option<String>` — never propagates.

**Vault unlock timing:** Decryption (and any Touch ID prompt) happens
in the patina process *before* `exec()` replaces it. The token is set
in the `Command`'s env builder, then passed through exec to tmux, which
passes it to the adapter child process. Touch ID never fires inside
tmux.

If Touch ID is unavailable (detached SSH, Linux, CI), the identity
falls back to `PATINA_IDENTITY` env var. If neither is available,
`try_get_claude_token()` returns None and launch proceeds without
injection.

**Platform support:**

| Platform | Identity Source | Notes |
|---|---|---|
| macOS (GUI) | Encrypted file (Keychain fallback) | Primary path — see spec-secrets-dual-storage |
| macOS (SSH) | Encrypted file | Machine-bound ChaCha20-Poly1305 |
| Linux | Encrypted file | Machine-bound via /etc/machine-id |
| CI | `PATINA_IDENTITY` env var | No encrypted file; user exports identity |

Identity resolution uses encrypted file (`~/.patina/identity.enc`) as primary
on all platforms, with Keychain as legacy fallback with auto-migration on macOS.
See completed spec-secrets-dual-storage (v0.28.0) for details.

### Error Handling

| Scenario | Behavior |
|---|---|
| `claude-oauth` not in vault | Launch without token (normal OAuth) |
| Vault decryption fails | Warn to stderr, launch without token |
| `ANTHROPIC_API_KEY` set | Warn to stderr, skip injection |
| `CLAUDE_CODE_OAUTH_TOKEN` already set | Warn to stderr, skip injection |
| Token expired (server-side) | See "Token Expiry Detection" below |
| Wrong adapter (gemini, etc.) | No injection attempted |

No error is fatal. Every failure mode falls back to the current
behavior — launch proceeds, Claude handles its own auth.

### Token Expiry Detection

The setup-token lasts ~1 year but can be revoked or expire. When
Claude exits with an auth error after token injection, the launcher
cannot detect this directly — `exec()` has replaced the patina process.

**v1 approach (this spec):** No proactive detection. If the token
expires mid-session, Claude prompts for re-auth (which fails headless).
The user re-runs `claude setup-token` + `patina secrets add`. This is
acceptable because:
- 1-year lifetime means this happens at most once a year
- The failure is obvious (Claude says "please log in")
- The fix is the same two commands from initial setup

**Future improvement (out of scope):** A `patina doctor` check could
validate the token by calling `claude auth status` with the token in
env and warning if it returns unauthenticated. This would catch
expired tokens before they cause a mid-session failure. Not worth
building until someone actually hits the ~1-year expiry.

## Implementation

1. **New secret accessor** — Add `get_global_secret(name: &str) -> Result<Option<String>>`
   to `src/secrets/mod.rs`. Decrypts only the global vault
   (`~/.patina/vault.age`), looks up the named secret, returns
   `Ok(None)` if not found. Uses session cache when available.
   Does NOT touch project vaults. Errors only on actual decrypt
   failure (missing identity, corrupted vault).

2. **Token retrieval wrapper** — Add `try_get_claude_token() -> Option<String>`
   to `src/commands/launch/internal.rs`. Calls `secrets::get_global_secret("claude-oauth")`.
   Catches all errors (logs warn to stderr on decrypt failure),
   returns None on any failure. This function also checks the
   conflict guard:
   - If `ANTHROPIC_API_KEY` is set → warn + return None
   - If `CLAUDE_CODE_OAUTH_TOKEN` is set → warn + return None
   - Otherwise → attempt vault lookup

3. **Refactor Command construction** — Split `launch_adapter_cli()`
   so `Command` is built as `let mut cmd = ...` before `.exec()`.
   Both the tmux path and direct-exec path produce a `Command` that
   can receive `.env()` calls before exec. Preserve all existing
   tmux logic (fallback-on-error, stderr messages, `-D` flag).

4. **Inject token** — After building `cmd`, if `adapter_name == "claude"`
   and `try_get_claude_token()` returns Some, call
   `cmd.env("CLAUDE_CODE_OAUTH_TOKEN", &token)`. This applies to
   both the tmux-wrapped and direct-exec paths.

5. **Document onboarding** — Add setup instructions to the Claude
   adapter bootstrap (CLAUDE.md generation or patina doctor output):
   ```bash
   # One-time setup (requires browser):
   claude setup-token
   # Copy the token, then:
   patina secrets add claude-oauth --env CLAUDE_CODE_OAUTH_TOKEN --global --stdin
   ```

6. **Tests:**
   - `get_global_secret`: mock vault with secret present, missing,
     and vault-not-found cases. Verify project vault is never touched.
   - `try_get_claude_token`: test conflict guard (ANTHROPIC_API_KEY
     blocks, pre-existing CLAUDE_CODE_OAUTH_TOKEN blocks, clean env
     attempts vault).
   - `launch_adapter_cli`: test that `cmd.env()` is called only for
     claude adapter, not gemini/opencode. Test Command refactor
     preserves tmux args and fallback behavior.

## Non-Goals

- **`patina auth setup` convenience command** — future polish. The
  manual two-step (`claude setup-token` + `patina secrets add`) is
  sufficient for v1. A future wrapper could detect headless and
  guide the user, but not this spec.
- **Token rotation / expiry monitoring** — the token lasts ~1 year.
  If it expires, the user re-runs setup. No cron, no daemon. A
  `patina doctor` check could validate the token proactively — future.
- **Permission bypass (`--dangerously-skip-permissions`)** — stays in
  yolo. This spec is auth only.
- **Multi-account support** — one Max subscription per user. If
  someone has multiple accounts, they manage vault entries manually.
- **Adapter auth abstraction** — no generic "adapter credential"
  trait. Claude is the only adapter that needs this today. Extend
  when needed, not before.
- **Headless-only initial setup** — `claude setup-token` requires a
  browser. Users with no GUI access at all must use VNC/screen-share
  for the one-time setup. No workaround provided in this spec.

## Rollback & Safety

- Remove `claude-oauth` from vault: `patina secrets --remove claude-oauth --global`
- Token injection is additive — removing the secret restores the
  current behavior (Claude handles its own OAuth).
- The token is never written to disk in plaintext — only exists in
  the age-encrypted vault and in process memory during exec.
- Patina's secret scanner catches `sk-ant-` patterns if anyone
  accidentally commits the token.
- `CLAUDE_CODE_OAUTH_TOKEN` is scoped to Claude Code only — cannot
  be used for general API calls even if leaked.

## Exit Criteria

1. `patina secrets add claude-oauth --env CLAUDE_CODE_OAUTH_TOKEN --global`
   stores a token in the global vault. `patina secrets` shows it.
2. `patina` (Claude adapter) injects `CLAUDE_CODE_OAUTH_TOKEN` into the
   exec environment. Verified by checking Claude's auth status inside
   the launched session (`/status` shows Max subscription).
3. `patina gemini` does NOT inject the token — adapter gating works.
4. With `ANTHROPIC_API_KEY` set, token injection is skipped and a
   user-visible warning is printed to stderr.
5. With `CLAUDE_CODE_OAUTH_TOKEN` already set in env, vault injection
   is skipped and a user-visible warning is printed to stderr.
6. Without `claude-oauth` in vault, launch proceeds normally — no
   crash, no error, Claude handles its own auth.
7. `get_global_secret()` decrypts only the global vault. A corrupted
   or missing project vault does NOT prevent token injection.
8. From SSH (phone/remote): `tmux attach` reconnects to an already-
   authenticated session. If session crashed, `patina` re-launches
   with token injection — Max subscription works without a browser.
9. Token never appears in plaintext on disk outside the vault.
10. Command refactor preserves all spec-launcher-tmux behavior: tmux
    args, `-D` flag, fallback-on-error, stderr reconnect hint.
