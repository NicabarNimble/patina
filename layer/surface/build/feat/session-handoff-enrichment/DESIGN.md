# Design: Session Handoff Enrichment

## Why This Design

Session continuity is broken at the boundary between sessions. When a tmux lane
dies, a terminal closes, or a session ends without a manual handoff, the next
instance reconstructs state from scratch — reading a blank `## Handoff` section
and improvising from the git log.

Pi-mono's handoff/compaction approach demonstrates that LLM-generated structured
context transfer is the right primitive. Patina's philosophy is sharper: **handoff
+ fresh context beats degraded context continuation**. This design wires that
philosophy into `session end` and adds an explicit `session handoff <goal>` path.

## Build Target

Four concrete changes to `src/commands/session/`:

1. `parent_session` populated in new session frontmatter
2. `<modified-files>` list written into `## Handoff` at session-end
3. Structured handoff section template (Goal / Constraints / Progress / Decisions / Next Steps / Critical Context / modified-files)
4. LLM-generated handoff via Anthropic API at session-end, with graceful fallback
5. `session handoff <goal>` CLI command: generate → end → start new

## Resolved Decisions

- **Model**: `claude-haiku-4-5-20251001` — fast and cheap, sufficient for summarisation, not user-configurable in v1.
- **API key resolution**: `ANTHROPIC_API_KEY` env var first, then `patina secrets get anthropic_api_key`, then skip with a one-line warning. No error. Session-end still succeeds.
- **Modified-files source**: `git::files_changed_since(&session_tag)` — already called in `end_session_document_value`, already returns `Vec<String>`. Reuse it; write file paths verbatim into `<modified-files>` block.
- **`parent_session` source**: read session ID from `last-session.md` at `start_session_value` time. The field `SessionFrontmatter.parent_session: Option<String>` already exists; it is never populated. Fix that.
- **LLM owns prose, Patina owns files**: the `<modified-files>` block is always written by Patina from git truth. The LLM is instructed to include it verbatim. Patina post-processes to inject or verify it is present.
- **Gjengset principle**: `files_changed_since` returns `Vec<String>` (existing API). These are path strings at a serialization boundary (going into markdown) — `String` is correct here per `boundary-string-internal-enum`. No type change needed.
- **No compaction**: explicit non-goal. Not in scope.
- **`session handoff` ends then starts**: calls `end_session_document_value` then `start_session_value` with `parent_session` set. The user gets a new session opened with the handoff already in place.

## Commit Slices

### Slice 1 — `parent_session` frontmatter
`feat(session): populate parent_session from last-session.md at session start`

**Where**: `src/commands/session/internal.rs`, `start_session_value()` (~line 425).

After resolving the adapter and before writing the new session artifact, read
`.patina/local/last-session.md`. Extract the session ID from the pointer line:

```
# Last Session: <title>
See: layer/sessions/<id>.md
```

Parse `<id>` from the `See:` line. Set `SessionFrontmatter.parent_session = Some(id)`.

No new dependencies. No new tests beyond asserting the field is set when
last-session.md exists and absent when it does not.

```rust
fn read_parent_session_id(project_root: &Path) -> Option<String> {
    let path = project_root.join(LAST_SESSION_PATH);
    let content = fs::read_to_string(path).ok()?;
    content.lines()
        .find(|l| l.starts_with("See: layer/sessions/"))
        .and_then(|l| l.strip_prefix("See: layer/sessions/"))
        .and_then(|l| l.strip_suffix(".md"))
        .map(|s| s.to_string())
}
```

In `start_session_value`, after resolving adapter:

```rust
let parent_session_id = read_parent_session_id(project_root);
// when building SessionFrontmatter:
parent_session: parent_session_id,
```

---

### Slice 2 — Modified-files list + structured handoff template
`feat(session): write structured handoff with modified-files list at session end`

**Where**: `src/commands/session/internal.rs`, `end_session_document_value()` (~line 819).

`changed_files: Vec<String>` is already computed at line 849 via
`git::files_changed_since(&session_tag)`. Reuse it.

Replace the blank `## Handoff\n\n` section with the structured block by finding
`## Handoff` in the session content and replacing everything after it until the
next `##` heading or EOF.

**Structured template** (written when no LLM, or as base before LLM call):

```markdown
## Handoff

## Goal
<!-- What this session was trying to accomplish -->

## Constraints & Preferences
<!-- Requirements or constraints that were active -->

## Progress
### Done
<!-- Completed items -->

### In Progress
<!-- Unfinished work -->

### Blocked
<!-- Any blockers -->

## Key Decisions
<!-- Decision: Rationale -->

## Next Steps
<!-- What the next session should do first -->

## Critical Context
<!-- Data, state, or caveats needed to continue -->

<modified-files>
src/path/to/file.rs
</modified-files>
```

The `<modified-files>` block is populated from `changed_files`. If empty, write
`<!-- no files changed this session -->` inside the block.

```rust
fn build_handoff_section(changed_files: &[String]) -> String {
    let files_block = if changed_files.is_empty() {
        "<!-- no files changed this session -->".to_string()
    } else {
        changed_files.join("\n")
    };
    format!(
        "## Handoff\n\n\
         ## Goal\n<!-- What this session was trying to accomplish -->\n\n\
         ## Constraints & Preferences\n<!-- Requirements or constraints -->\n\n\
         ## Progress\n### Done\n<!-- Completed items -->\n\n\
         ### In Progress\n<!-- Unfinished work -->\n\n\
         ### Blocked\n<!-- Any blockers -->\n\n\
         ## Key Decisions\n<!-- Decision: Rationale -->\n\n\
         ## Next Steps\n<!-- What the next session should do first -->\n\n\
         ## Critical Context\n<!-- Data, state, or caveats needed to continue -->\n\n\
         <modified-files>\n{files_block}\n</modified-files>\n",
    )
}

fn inject_handoff_section(content: &str, handoff: &str) -> String {
    // Replace from "\n## Handoff" to the next "\n## " heading or EOF
    if let Some(start) = content.find("\n## Handoff") {
        let tail = &content[start + 1 + "## Handoff".len()..];
        let end_offset = tail.find("\n## ")
            .map(|i| i + 1)
            .unwrap_or(tail.len());
        let end = start + 1 + "## Handoff".len() + end_offset;
        format!("\n{}{}", handoff, &content[end..])
            .replace(&content[..start + 1], &content[..start + 1])
        // simpler: rebuild from parts
    } else {
        format!("{}\n{}", content, handoff)
    }
}
```

Note: write a clean implementation — the pseudocode above shows intent, not the
exact string arithmetic. Use byte-offset slicing carefully; Patina has a
`truncate` helper that ensures UTF-8 safety (`0c4ab9f4`).

In `end_session_document_value`, after computing `changed_files` and before the
appendix write:

```rust
let handoff = build_handoff_section(&changed_files);
session_content = inject_handoff_section(&session_content, &handoff);
```

---

### Slice 3 — LLM-generated handoff
`feat(session): call claude-haiku to generate structured handoff at session end`

**Where**: `src/commands/session/internal.rs` — new private functions.

**System prompt** (constant `HANDOFF_SYSTEM_PROMPT`):

```
You are a session context transfer assistant. Given a session artifact and git
summary, generate a structured handoff that lets the next AI session immediately
orient and continue work.

Output ONLY the handoff block — no preamble, no explanation. Use this exact format:

## Goal
[What this session was trying to accomplish]

## Constraints & Preferences
- [Requirements or constraints]

## Progress
### Done
- [x] [Completed items]

### In Progress
- [ ] [Unfinished items]

### Blocked
- [Blockers, or "None"]

## Key Decisions
- **[Decision]**: [Rationale]

## Next Steps
1. [First thing the next session should do]

## Critical Context
- [Any data, state, or caveats the next session must know]

<modified-files>
[INCLUDE THE MODIFIED FILES LIST EXACTLY AS PROVIDED — DO NOT INVENT OR OMIT]
</modified-files>
```

**User message**:

```
## Session Artifact

{full session markdown content}

## Git Summary

Commits this session: {commits_made}
Files changed: {files_changed_count}

## Modified Files (authoritative — include verbatim in <modified-files> block)

{changed_files joined by \n}
```

**API call**:

```rust
fn generate_llm_handoff(
    session_content: &str,
    commits_made: usize,
    changed_files: &[String],
    api_key: &str,
) -> Result<String> {
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(60))
        .build()?;

    let files_list = changed_files.join("\n");
    let user_msg = format!(
        "## Session Artifact\n\n{session_content}\n\n\
         ## Git Summary\n\nCommits: {commits_made}\nFiles changed: {count}\n\n\
         ## Modified Files (include verbatim)\n\n{files_list}",
        count = changed_files.len(),
    );

    let body = serde_json::json!({
        "model": "claude-haiku-4-5-20251001",
        "max_tokens": 1024,
        "system": HANDOFF_SYSTEM_PROMPT,
        "messages": [{"role": "user", "content": user_msg}]
    });

    let resp = client
        .post("https://api.anthropic.com/v1/messages")
        .header("x-api-key", api_key)
        .header("anthropic-version", "2023-06-01")
        .header("content-type", "application/json")
        .json(&body)
        .send()?;

    if !resp.status().is_success() {
        bail!("Anthropic API error: {}", resp.status());
    }

    let json: serde_json::Value = resp.json()?;
    json["content"][0]["text"]
        .as_str()
        .context("No text in response")
        .map(|s| s.to_string())
}
```

**API key resolution**:

```rust
fn resolve_anthropic_api_key() -> Option<String> {
    // 1. Env var
    if let Ok(key) = std::env::var("ANTHROPIC_API_KEY") {
        if !key.is_empty() { return Some(key); }
    }
    // 2. patina secrets
    std::process::Command::new("patina")
        .args(["secrets", "get", "anthropic_api_key"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}
```

**Post-processing** — Patina always owns the `<modified-files>` block:

```rust
fn ensure_modified_files_block(llm_output: &str, changed_files: &[String]) -> String {
    let files_block = if changed_files.is_empty() {
        "<!-- no files changed this session -->".to_string()
    } else {
        changed_files.join("\n")
    };
    let replacement = format!("<modified-files>\n{files_block}\n</modified-files>");

    if let Some(start) = llm_output.find("<modified-files>") {
        if let Some(end) = llm_output.find("</modified-files>") {
            return format!(
                "{}{}{}",
                &llm_output[..start],
                replacement,
                &llm_output[end + "</modified-files>".len()..]
            );
        }
    }
    // Not present — append
    format!("{}\n\n{}", llm_output, replacement)
}
```

**Integration in `end_session_document_value`** (after slice 2's template write):

```rust
if let Some(api_key) = resolve_anthropic_api_key() {
    match generate_llm_handoff(&session_content, commits_made, &changed_files, &api_key) {
        Ok(llm_body) => {
            let with_files = ensure_modified_files_block(&llm_body, &changed_files);
            let full_section = format!("## Handoff\n\n{with_files}");
            session_content = inject_handoff_section(&session_content, &full_section);
        }
        Err(e) => {
            eprintln!("patina: LLM handoff skipped ({e}) — structured template written");
        }
    }
} else {
    eprintln!("patina: no API key — structured handoff template written");
}
```

---

### Slice 4 — `session handoff <goal>` command
`feat(session): add session handoff command for explicit mid-session transfer`

**Where**: `src/commands/session/mod.rs` + `src/commands/session/internal.rs`.

**CLI addition** to `SessionCommands`:

```rust
/// Generate a focused handoff prompt, end this session, start a new one
Handoff {
    /// The goal for the next session
    goal: String,
    #[arg(long)]
    json: bool,
},
```

**Dispatch in `execute()`**:
```rust
SessionCommands::Handoff { goal, json } => handoff(&project_root, &goal, json),
```

**Goal-focused system prompt** (constant `HANDOFF_GOAL_SYSTEM_PROMPT`):

```
You are a context transfer assistant. Given a session artifact and a stated goal
for the next session, generate a focused, self-contained prompt that:

1. Summarises relevant context (decisions made, approaches taken, key findings)
2. Lists relevant files that were discussed or modified
3. Clearly states the next task from the user's goal
4. Is self-contained — the new session should proceed without the old conversation

Output ONLY the prompt. No preamble.

Format:
## Context
[Relevant context — decisions, findings, constraints]

Files involved:
- path/to/file.rs

## Task
[Clear statement of what to do next, derived from the user's goal]
```

**`handoff_session_value` implementation**:

1. Read active session artifact from `ACTIVE_SESSION_PATH`.
2. Read `session_tag` and `starting_commit` from frontmatter.
3. Run `git::files_changed_since(&session_tag)` for `changed_files`.
4. Resolve API key. If present, call `generate_llm_handoff_for_goal(content, goal, changed_files, api_key)`. If absent, build template with files and goal injected.
5. Write result into `## Handoff` section via `inject_handoff_section`.
6. Call `end_session_document_value` — this archives, tags, writes `last-session.md`.
7. Extract the just-archived session ID from the result.
8. Call `start_session_value` with a `SessionStartRequest` that includes `parent_session: Some(ended_session_id)`.
9. Print: old session ID, new session ID, `parent_session` link, and the handoff content.

**`generate_llm_handoff_for_goal`**: same structure as `generate_llm_handoff` but uses `HANDOFF_GOAL_SYSTEM_PROMPT` and includes `## User Goal\n\n{goal}` in the user message.

If no API key: write structured template with `## Task\n{goal}` injected into Next Steps, then end and start. Transfer still happens.

## Direct Code Targets

- `src/commands/session/internal.rs`
  - `start_session_value()` — add `read_parent_session_id()` call, set `parent_session` field
  - `end_session_document_value()` — add `build_handoff_section()`, `inject_handoff_section()`, LLM upgrade block
  - New private functions: `read_parent_session_id`, `build_handoff_section`, `inject_handoff_section`, `ensure_modified_files_block`, `resolve_anthropic_api_key`, `generate_llm_handoff`, `generate_llm_handoff_for_goal`, `handoff_session_value`

- `src/commands/session/mod.rs`
  - `SessionCommands` enum — add `Handoff { goal: String, json: bool }` variant
  - `execute()` — dispatch `Handoff` to `handoff_session_value`
  - `pub fn handoff(project_root: &Path, goal: &str, json: bool) -> Result<()>`

## Verification Plan

```
cargo test -q -p patina-ai session_handoff_parent_session_populated
cargo test -q -p patina-ai session_handoff_parent_session_absent_when_no_prior
cargo test -q -p patina-ai session_handoff_modified_files_written_to_section
cargo test -q -p patina-ai session_handoff_empty_files_writes_comment
cargo test -q -p patina-ai session_handoff_template_written_without_api_key
cargo test -q -p patina-ai session_handoff_ensure_modified_files_block_injects_missing
cargo test -q -p patina-ai session_handoff_ensure_modified_files_block_replaces_wrong
cargo test -q -p patina-ai session_handoff_command_ends_and_starts_new_session
```

All tests use fixture sessions and mock/stubbed API responses. No live API calls
in CI. Use `resolve_anthropic_api_key` being None in tests to exercise fallback path.

## Build Readiness

Ready. All code targets identified, function signatures sketched, data flow
verified against existing source. Implement slices in order — each is
independently committable and leaves the codebase in a working state.
