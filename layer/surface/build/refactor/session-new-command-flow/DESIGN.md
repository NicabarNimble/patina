# Design: Rename /session-start to /session-new and align auto-session naming

## Why This Design

Auto HITL launch already owns session resolution (attach-or-create). Keeping a `session-start` command name creates semantic mismatch and accidental double-session creation. Renaming explicit boundary creation to `session-new` clarifies intent and preserves auto flow as primary.

## Build Target

- Remove `start` from `patina ai session` lifecycle verbs.
- Add `new` lifecycle verb with identical create semantics.
- Rename all interface wrappers/prompts/templates from `session-start` to `session-new`.
- Add first-update title suggestion path for default auto-created titles.
- Keep git tag derivation unchanged.

## Resolved Decisions

1. No compatibility alias for `session-start` or `start`.
2. Auto launch remains attach-or-create source of truth.
3. `session-new` is explicit boundary creation.
4. First-update naming must persist to both artifact and Mother record.
5. Tags remain file-id/interface based, not title based.

## Commits

1. `refactor(ai): rename session start lifecycle verb to new`
   - Replace CLI surface and parser variants.
2. `refactor(interface): rename session-start wrappers/prompts/templates to session-new`
   - Apply to Claude/OpenCode/Gemini/PI generated surfaces.
3. `feat(session): add first-update title suggestion/persistence for auto default titles`
   - Persist rename to artifact + Mother state.
4. `test(ai/session): cover command surface rename and no-alias policy`
   - Assert `start` rejected and `new` accepted.

## Direct Code Targets

- `src/commands/ai/mod.rs` — subcommand enum rename (`Start` → `New`)
- `src/commands/ai/internal.rs` — dispatch rename (`start_session` path → `new_session` path)
- `src/commands/session/internal.rs` — first-update title hook + persistence integration
- `src/session/internal/live.rs` — Mother session title update helper(s)
- `src/interface/runtime/templates.rs` — generated wrapper/skill names (`session-new`)
- `src/interface/runtime/{claude,opencode,gemini}/mod.rs` — command list/help text updates
- `resources/{claude,opencode,gemini}/session-new*` + `.pi/prompts/session-new.md` — align prompt resources
- `.claude/.opencode/.gemini/.pi` command/prompt files — regenerated artifacts

## Verification Plan

1. **CLI surface**
   - `patina ai session --help` includes `new` and excludes `start`.
   - `patina ai session new "title"` works.
   - `patina ai session start "title"` fails with actionable message.

2. **Generated interface assets**
   - wrappers exist as `session-new.sh`.
   - command/prompt docs reference `/session-new` and corresponding wrappers.

3. **Auto flow integrity**
   - launch remains attach-or-create.
   - explicit `/session-new` creates boundary only when requested.

4. **First-update naming**
   - first update in default-title auto session proposes title.
   - confirm path writes artifact frontmatter/body + Mother session record.
   - skip path leaves title unchanged.

5. **Git tags**
   - start/end tags remain `session-<file_id>-<interface>-start/end`.
   - session range queries remain valid.

## Build Readiness

Ready for implementation. Command-surface changes are straightforward; first-update naming requires one new persistence seam for title updates in Mother/session artifact sync.

## Open Questions

1. Should first-update rename prompt be interactive-only, with non-TTY no-op?
2. Should title proposal be deterministic (rule-based) or assistant-authored suggestion only?
3. Should rename emit a dedicated event (`session.renamed`) for audit/queryability?