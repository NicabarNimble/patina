---
type: feat
id: session-handoff-enrichment
status: draft
created: 2026-03-17
sessions:
  origin: 20260317-171514-193735000
exit_criteria:
- id: parent-session-field-populated
  text: New session frontmatter always carries `parent_session` ID when a previous session exists on disk
  checked: false
- id: modified-files-list-at-session-end
  text: "`## Handoff` section contains a `<modified-files>` block listing every file touched since `starting_commit`, derived from `git diff --name-only`"
  checked: false
- id: structured-handoff-format-adopted
  text: "`## Handoff` section follows the structured schema (Goal / Constraints / Progress / Key Decisions / Next Steps / modified-files) instead of blank or freeform prose"
  checked: false
- id: llm-generated-handoff-at-session-end
  text: "`patina ai session end` calls the Anthropic API and writes an LLM-generated structured handoff into the `## Handoff` section before archiving"
  checked: false
- id: session-handoff-command-exists
  text: "`patina ai session handoff <goal>` command exists, generates a focused transfer prompt via LLM, writes it to `## Handoff`, ends the current session, and starts a new one with `parent_session` set"
  checked: false
- id: graceful-degradation-without-api-key
  text: When no API key is available, session-end writes the structured template with git-derived `<modified-files>` populated and skips the LLM call — no error, no data loss
  checked: false
---
# feat: Session Handoff Enrichment — LLM-generated handoff, parent linking, modified-files

> Replace blank/freeform session handoffs with LLM-generated structured context
> transfer so every fresh session starts with full orientation rather than
> reconstructed prose.

## Problem

Patina session continuity is fragile at the boundary between sessions:

1. **`parent_session` field exists but is never populated.** The struct has it;
   nothing writes it. Session lineage is untraversable by machine.

2. **`## Handoff` is always blank.** The section template emits a comment
   placeholder. In practice the agent fills it at end-of-session — but only if
   it remembers to, and the quality varies. When a session is auto-archived
   (tmux lane death), the handoff is empty.

3. **Modified files are a count, not a list.** `SessionEndResult.modified_files`
   is a `usize`. The next session has to re-derive which files were touched from
   the git log or memory.

4. **Handoff is freeform prose optimised for human reading, not LLM consumption.**
   The next agent reads a narrative paragraph and improvises context. Pi's
   compaction/handoff format (Goal / Progress / Decisions / Next Steps /
   modified-files) is structured for direct LLM intake.

Today when a lane dies or a session ends without a manual handoff, the next
instance gets nothing actionable. It has to reconstruct state from scratch.

## Goal

At `session-end` (and on-demand via `session handoff <goal>`):

1. Populate `parent_session` in the new session's frontmatter automatically.
2. Collect modified files from `git diff --name-only <starting_commit>..HEAD`
   and embed them in the handoff as `<modified-files>`.
3. Generate a structured handoff block via LLM from the session artifact content
   and git diff summary. Fall back to the structured template (with files
   populated) if no API key is available.
4. Provide `patina ai session handoff <goal>` as an explicit mid-session
   transition: generate a focused transfer prompt for the stated goal, end this
   session, start a new one with `parent_session` set.

## Status

Draft. Designed in session `20260317-171514-193735000` by studying pi-mono's
compaction/handoff/branch-summarisation approach and mapping gaps to Patina's
session infrastructure.

## Non-Goals

- In-session compaction (intentional — the philosophy is handoff + fresh context,
  not degraded context continuation).
- Multi-provider LLM support — Anthropic Claude is the only target for the
  generated handoff call. Other providers are follow-on.
- UI/TUI changes — all output is CLI text.
- Changing the session artifact format beyond the `## Handoff` section and
  frontmatter `parent_session` field.

## Target Shape

### New session frontmatter

```yaml
parent_session: 20260316-142102-842339000   # populated when previous session exists
```

### `## Handoff` section (structured, LLM-generated or template)

```markdown
## Handoff

## Goal
[What this session was trying to accomplish]

## Constraints & Preferences
- [Requirements or constraints that were active]

## Progress
### Done
- [x] [Completed items]

### In Progress
- [ ] [Unfinished work]

### Blocked
- [Any blockers]

## Key Decisions
- **[Decision]**: [Rationale]

## Next Steps
1. [What the next session should do first]

## Critical Context
- [Data, state, or caveats needed to continue]

<modified-files>
src/commands/session/internal.rs
src/commands/session/mod.rs
</modified-files>
```

### `patina ai session handoff <goal>`

```
$ patina ai session handoff "implement the structured handoff format"

Generating handoff prompt...
→ Reading session artifact and git diff
→ Calling Claude API (claude-haiku-4-5)
→ Handoff written to ## Handoff section
→ Ending session 20260317-171514 (tagged, archived)
→ Starting new session 20260318-090000 (parent: 20260317-171514)

Handoff ready. Next session opens with full context.
```

## Solution

### 1. Populate `parent_session` at session-start

In `internal.rs`, `start_session_value` already reads `last-session.md`. Extract
the session ID from the pointer and write it as `parent_session` in the new
session's frontmatter. The field already exists in `SessionFrontmatter`; it just
needs to be set.

### 2. Collect modified-files list at session-end

`end_session_document_value` already reads `starting_commit` from the artifact.
Replace the `modified_files: usize` derivation with a `git diff --name-only
<starting_commit>..HEAD` call that returns `Vec<String>`. Both the count and the
list are available from one command.

### 3. Write structured handoff template

Change the `## Handoff\n\n` template string to emit the full structured schema
with `<modified-files>` populated from step 2. This gives every session a usable
handoff even without LLM generation.

### 4. LLM-generated handoff at session-end

After writing the structured template, if an Anthropic API key is available
(from `ANTHROPIC_API_KEY` env var or `patina secrets get anthropic_api_key`),
call `claude-haiku-4-5` (fast, cheap) with:

- **System prompt**: the handoff generation prompt (see DESIGN.md)
- **User message**: serialized session artifact body + `<git-diff-stat>` summary

Parse the response and replace the `## Handoff` section content with the
generated block. The `<modified-files>` list from git remains authoritative and
is injected into or verified in the generated output.

### 5. `session handoff <goal>` command

New `SessionCommands::Handoff { goal: String }` variant in `mod.rs`.

Implementation in `internal.rs`:
1. Read active session artifact.
2. Collect modified files and diff stat from git.
3. Call Claude API with the session content + goal.
4. Write generated focused transfer prompt into `## Handoff`.
5. Call `end_session_document_value` (archives, tags, writes `last-session.md`).
6. Call `start_session_value` with `parent_session` set from the just-ended session ID.
7. Print the new session artifact path and handoff content.

## Implementation Order

1. `parent_session` population (frontmatter, no LLM, no risk) — commit slice 1
2. `modified-files` list from git diff — commit slice 2
3. Structured handoff template (replaces blank section) — commit slice 2 (same PR)
4. LLM-generated handoff at `session-end` with graceful fallback — commit slice 3
5. `session handoff <goal>` command — commit slice 4

## Resolved Decisions

1. **Model**: `claude-haiku-4-5` for handoff generation. Fast, cheap, sufficient
   for summarisation. Not configurable in v1 — follow-on if needed.
2. **Fallback**: If no API key, write structured template with git-derived files.
   No error. No interruption of session-end flow.
3. **Modified-files source**: `git diff --name-only <starting_commit>..HEAD`.
   Authoritative. Always written by Patina, not left to LLM to invent.
4. **`<modified-files>` in LLM output**: The LLM is instructed to include the
   list verbatim from what Patina provides. Patina post-processes to verify and
   inject if absent.
5. **No compaction**: Explicit design choice. Handoff + fresh session is the
   continuity model. Compaction is not a goal.
6. **API key resolution order**: `ANTHROPIC_API_KEY` env var first, then
   `patina secrets get anthropic_api_key`, then skip with warning.

## Verification

```
cargo test -q -p patina-ai session_handoff_parent_session_populated
cargo test -q -p patina-ai session_handoff_modified_files_list
cargo test -q -p patina-ai session_handoff_structured_template
cargo test -q -p patina-ai session_handoff_llm_generated_fallback_without_key
cargo test -q -p patina-ai session_handoff_command_end_and_start
```

Manual smoke:
```
patina ai session end   # should write structured handoff + modified-files
patina ai session handoff "implement X"   # should end + start new with parent_session
```

## Exit Criteria

See frontmatter.

## Build Readiness

Not ready. DESIGN.md must be written before implementation begins.
