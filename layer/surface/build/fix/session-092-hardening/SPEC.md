---
type: fix
id: session-092-hardening
status: in_progress
created: 2026-01-31
affects_since: 0.9.2
sessions:
  origin: 20260131-093100
related:
  - feat/v1-release
  - refactor/version-semver-alignment
---

# fix: Session System 0.9.2 Hardening

Post-release testing of 0.9.2 (Session System & Adapter Parity) revealed bugs in integration points that weren't covered by the original spec's exit criteria. The session lifecycle works, but downstream consumers read stale paths and parse legacy formats.

## Context

Deep review of session system identified 9 issues across 9 commands that touch sessions (`session`, `doctor`, `scrape`, `rebuild`, `scry`, `version`, `adapter`, `report`, `serve`). This review also revealed a gap in the version system (no patch release mechanism), leading to the `refactor/version-semver-alignment` spec.

## Issues

| # | Issue | Severity | Fix Complexity | Status |
|---|-------|----------|----------------|--------|
| 1 | Scry/MCP reads `**ID**:` but YAML format has `id:` in frontmatter; also reads wrong path (`.claude/context/` vs `.patina/local/`) | Medium | Small | **Fixed** |
| 2 | `patina doctor` counts sessions from `.claude/context/sessions/` (doesn't exist) instead of `layer/sessions/` | Medium | Small | **Fixed** |
| 3 | `patina adapter refresh` fails when adapter dir is gitignored | Medium | Small | **Fixed** |
| 4 | Rapid session start creates duplicate tags (same-second collision) | Low | Small | **Fixed** |
| 5 | Starting commit captured before branch switch (metrics wrong if work branch diverged) | Medium | Small | **Fixed** |
| 6 | Incomplete session archive retains `status: active` in YAML | Low | Trivial | **Fixed** |
| 7 | `patterns_modified` counts all `.md` files (inflates classification) | Low | Small | **Fixed** |
| 8 | Dual-format dispatch table maintenance risk (`read_session_field` prefix matching) | Low | Design debt | Deferred |
| 9 | User prompts extraction is Claude-specific (`~/.claude/history.jsonl`) | Low | By design | Deferred |

## Fixes Applied

### Fix 1: Scry/MCP session ID lookup
- `src/commands/scry/internal/logging.rs`: `get_active_session_id()` now reads from `.patina/local/active-session.md` (correct path), parses YAML frontmatter `id:` field with legacy `**ID**:` fallback
- `src/mcp/server.rs`: Two inline duplicates replaced with calls to shared function
- Verified: scry queries now log `session_id: "20260131-093100"` correctly

### Fix 2: Doctor session count
- `src/commands/doctor.rs`: Count from `layer/sessions/` (canonical) instead of adapter-specific `.claude/context/sessions/`
- Verified: 545 sessions now visible vs 0 before

### Fix 3: Adapter refresh gitignored dirs
- `src/git/operations.rs`: `add_paths()` checks `git check-ignore -q` before staging; added `has_staged_changes()` utility
- `src/commands/adapter.rs`: Skip commit when no trackable changes exist (prints informational message)

### Fix 4: Session tag collision guard
- `src/commands/session/internal.rs`: Check `tag_exists()` before creating session tag; bail with clear error instead of silent failure

### Fix 5: Starting commit after branch switch
- `src/commands/session/internal.rs`: Moved `head_sha()` from before branch switch to after, so `starting_commit` reflects the actual branch HEAD
- Impact: Only affects `main` → `work` auto-switch path where `work` already exists and has diverged

### Fix 6: Incomplete session archive status
- `src/commands/session/internal.rs`: Read file content, `replacen("status: active", "status: archived", 1)`, write to archive instead of raw `fs::copy`

### Fix 7: Tighten patterns_modified filter
- `src/commands/session/internal.rs`: Changed filter to only count files under `layer/core/`, `layer/surface/`, `layer/topics/`
- Excludes: `CLAUDE.md`, `README.md`, session files, spec files from pattern count

## Deferred Items

- **#8 (dual-format dispatch)**: The `read_session_field` prefix-matching dispatch table works but is fragile. Will naturally resolve when legacy sessions age out or when a proper frontmatter parser replaces the dispatch table.
- **#9 (user prompts Claude-specific)**: By design — Gemini and OpenCode don't have equivalent history files. Would need adapter trait method `fn get_prompt_history_path()` to generalize. Low priority until those adapters gain real usage.

## Exit Criteria

- [x] `patina doctor` shows correct session count
- [x] `patina adapter refresh claude` succeeds with gitignored adapter dir
- [x] Scry queries log session_id from YAML frontmatter
- [x] MCP server uses shared session ID function
- [x] Duplicate session start is caught and rejected
- [x] Starting commit captured after branch switch
- [x] Incomplete session archive updates status to archived
- [x] patterns_modified filter excludes non-pattern .md files
- [ ] All fixes pass `cargo fmt`, `cargo clippy`, `cargo test`
- [ ] Committed and pushed
