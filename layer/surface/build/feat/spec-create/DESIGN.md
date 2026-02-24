# Design Walkthrough: spec-create

Step-by-step implementation guide. Read SPEC.md first for the full
rationale — this doc is the mechanical "how."

**Prerequisite:** spec-module-split must be complete. This walkthrough
assumes `src/commands/spec/internal/` directory exists. `types.rs`
(SpecType registry + body templates) is created as part of this spec
— it has no callers until `create.rs` uses it.

## Before You Start

```bash
# Verify prerequisite
ls src/commands/spec/internal/mod.rs     # must exist (from spec-module-split)
cargo test -p patina                     # all green

# Understand the pattern — read one existing mutation
cat src/commands/spec/internal/mutations.rs | head -80
```

Read these files in order:
1. `src/commands/spec/mod.rs` — where Create variant goes
2. `src/commands/spec/internal/mutations.rs` — `_value()` pattern to follow
3. `src/spec.rs` — `SpecFrontmatter` + `serialize_spec_file()`
4. `src/mcp/server.rs` — where MCP tool gets registered
5. `resources/claude/spec.md` — skill definition to update

## Step 1: Create internal/types.rs

New file: `src/commands/spec/internal/types.rs` — the SpecType registry.
This is created here (not in spec-module-split) because it has no
callers until `create.rs` uses it. See spec-module-split SPEC.md
section "Extract SpecType registry into types.rs" for the full design
(struct, constants, lookup function, body templates).

Add to `internal/mod.rs`:

```rust
pub(crate) mod types;
```

## Step 2: Add SpecCommands::Create to mod.rs

In `src/commands/spec/mod.rs`, add the new variant to `SpecCommands`:

```rust
/// Create a new spec draft
Create {
    /// Spec type: feat, fix, refactor, explore
    r#type: String,

    /// Spec identifier (kebab-case)
    id: String,

    /// Human title (defaults to "<type>: <id>")
    #[arg(long)]
    title: Option<String>,

    /// One-line problem statement for the blockquote
    #[arg(long)]
    description: Option<String>,

    /// Spec IDs this is blocked by
    #[arg(long)]
    blocked_by: Vec<String>,

    /// Related file paths
    #[arg(long)]
    related: Vec<String>,

    /// Output as JSON (for agent use)
    #[arg(long)]
    json: bool,
},
```

Add the public function:

```rust
/// Create a new spec draft
pub fn create(
    spec_type: &str,
    id: &str,
    title: Option<&str>,
    description: Option<&str>,
    blocked_by: Vec<String>,
    related: Vec<String>,
    json: bool,
) -> Result<()> {
    internal::create_spec(spec_type, id, title, description, blocked_by, related, json)
}
```

## Step 3: Dispatch in main.rs

Find the `SpecCommands` match block in `src/main.rs` and add:

```rust
SpecCommands::Create {
    r#type,
    id,
    title,
    description,
    blocked_by,
    related,
    json,
} => spec::create(
    &r#type,
    &id,
    title.as_deref(),
    description.as_deref(),
    blocked_by,
    related,
    json,
),
```

## Step 4: Create internal/create.rs

New file: `src/commands/spec/internal/create.rs`

### Imports

```rust
use anyhow::{Context, Result};
use rusqlite::Connection;
use std::path::Path;
use std::process::Command;

use patina::spec::{serialize_spec_file, Sessions, SpecFrontmatter};

use super::DB_PATH;
use super::types;
```

### Validation Helpers

```rust
/// Validate kebab-case identifier: lowercase, hyphens, starts with letter.
fn is_valid_id(id: &str) -> bool {
    let re = regex::Regex::new(r"^[a-z][a-z0-9-]*$").unwrap();
    re.is_match(id)
}

/// Read the active session ID from .patina/local/active-session.md
fn active_session_id() -> Option<String> {
    let content = std::fs::read_to_string(".patina/local/active-session.md").ok()?;
    // Parse YAML frontmatter for id field
    let content = content.strip_prefix("---")?;
    let end = content.find("\n---")?;
    let frontmatter: serde_yaml::Value = serde_yaml::from_str(&content[..end]).ok()?;
    frontmatter.get("id")?.as_str().map(|s| s.to_string())
}
```

### Core Implementation

```rust
/// Create a new spec draft and return structured result.
///
/// Flow: validate → mkdir → write SPEC.md → git commit → update DB
pub fn create_spec_value(
    spec_type: &str,
    id: &str,
    title: Option<&str>,
    description: Option<&str>,
    blocked_by: Vec<String>,
    related: Vec<String>,
) -> Result<serde_json::Value> {
    // 1. Validate type via registry
    let st = types::lookup(spec_type).ok_or_else(|| {
        let valid: Vec<_> = types::SPEC_TYPES.iter().map(|t| t.name).collect();
        anyhow::anyhow!(
            "Unknown spec type '{}'. Valid types: {}",
            spec_type,
            valid.join(", ")
        )
    })?;

    // 2. Validate id
    if !is_valid_id(id) {
        anyhow::bail!(
            "Invalid spec id '{}'. Must be kebab-case: lowercase letters, \
             digits, hyphens. Must start with a letter.",
            id
        );
    }

    // 3. Check directory doesn't exist
    let spec_dir = format!("layer/surface/build/{}/{}", st.directory, id);
    if Path::new(&spec_dir).exists() {
        anyhow::bail!(
            "Directory already exists: {}\n  \
             A spec with this id may already exist.",
            spec_dir
        );
    }

    // 4. Check archived tag doesn't exist
    let archive_tag = format!("spec/{}", id);
    if patina::git::tag_exists(&archive_tag)? {
        anyhow::bail!(
            "Tag '{}' already exists — a spec with this id was previously \
             archived.\n  View it: git show {}",
            archive_tag,
            archive_tag
        );
    }

    // 5. Create directory
    std::fs::create_dir_all(&spec_dir)
        .with_context(|| format!("Failed to create directory {}", spec_dir))?;

    // 6. Build frontmatter
    let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
    let title_str = title.unwrap_or_else(|| {
        // Can't return a reference to a local, so we'll handle below
        id
    });
    // Build a proper title if none provided
    let display_title = match title {
        Some(t) => t.to_string(),
        None => format!("{}: {}", spec_type, id),
    };

    let sessions = active_session_id().map(|sid| Sessions::Structured {
        origin: Some(sid),
        work: vec![],
        updated: None,
    });

    let frontmatter = SpecFrontmatter {
        r#type: spec_type.to_string(),
        id: id.to_string(),
        status: Some("draft".to_string()),
        created: Some(today.clone()),
        sessions,
        blocked_by,
        related,
        ..Default::default()
    };

    // 7. Build body from template
    let desc_line = description.unwrap_or("TODO: problem statement");
    let body = format!(
        "\n\n# {}\n\n> {}\n\n{}",
        display_title, desc_line, st.body_template
    );

    // 8. Write SPEC.md
    let spec_path = format!("{}/SPEC.md", spec_dir);
    let content = serialize_spec_file(&frontmatter, &body)?;
    std::fs::write(&spec_path, &content)
        .with_context(|| format!("Failed to write {}", spec_path))?;

    // 9. Git commit
    let output = Command::new("git")
        .args(["add", &spec_path])
        .output()
        .context("Failed to stage spec file")?;
    if !output.status.success() {
        anyhow::bail!(
            "git add failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    let commit_msg = format!("spec: draft {}", id);
    let output = Command::new("git")
        .args(["commit", "-m", &commit_msg])
        .output()
        .context("Failed to commit")?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        if !stderr.contains("nothing to commit") {
            anyhow::bail!("git commit failed: {}", stderr);
        }
    }

    // 10. Update database
    // Minimal INSERT — only columns needed for spec queries (list, ready, blocked).
    // Scrape fills in remaining columns (created, tags, refs, purpose) on next run.
    // INSERT OR REPLACE is safe: directory-exists check above guarantees no prior row
    // (only stale DB rows could conflict, and those get cleaned up by scrape).
    let db_path = Path::new(DB_PATH);
    if db_path.exists() {
        if let Ok(conn) = Connection::open(db_path) {
            let _ = conn.execute(
                "INSERT OR REPLACE INTO patterns (id, file_path, status, title, layer) \
                 VALUES (?1, ?2, ?3, ?4, 'surface')",
                rusqlite::params![
                    id,
                    spec_path,
                    "draft",
                    display_title,
                ],
            );
        }
    }

    Ok(serde_json::json!({
        "command": "create",
        "spec_id": id,
        "spec_type": spec_type,
        "status": "draft",
        "path": spec_path,
        "directory": spec_dir,
        "session": active_session_id(),
    }))
}
```

### CLI Wrapper

```rust
/// Create a new spec draft (human-readable or JSON output).
pub fn create_spec(
    spec_type: &str,
    id: &str,
    title: Option<&str>,
    description: Option<&str>,
    blocked_by: Vec<String>,
    related: Vec<String>,
    json: bool,
) -> Result<()> {
    let result = create_spec_value(spec_type, id, title, description, blocked_by, related)?;

    if json {
        println!("{}", serde_json::to_string_pretty(&result)?);
    } else {
        let path = result["path"].as_str().unwrap_or("");
        let session = result["session"].as_str().unwrap_or("none");
        println!("Created: {}", path);
        println!("  Type:    {}", spec_type);
        println!("  Status:  draft");
        println!("  Session: {}", session);
        println!("\nEdit: $EDITOR {}", path);
    }

    Ok(())
}
```

### Wire into internal/mod.rs

Add to `internal/mod.rs`:

```rust
mod create;

// Add to re-exports:
pub(super) use create::{create_spec, create_spec_value};
```

## Step 5: Register MCP Tool

In `src/mcp/server.rs`, add to the tools array (after `spec.split`):

```json
{
    "name": "spec.create",
    "description": "Create a new spec draft — scaffold directory, write frontmatter, commit.",
    "inputSchema": {
        "type": "object",
        "properties": {
            "spec_type": {
                "type": "string",
                "description": "Spec type: feat, fix, refactor, explore"
            },
            "id": {
                "type": "string",
                "description": "Spec identifier (kebab-case)"
            },
            "title": {
                "type": "string",
                "description": "Human title"
            },
            "description": {
                "type": "string",
                "description": "One-line problem statement"
            },
            "blocked_by": {
                "type": "array",
                "items": { "type": "string" },
                "description": "Spec IDs this is blocked by"
            }
        },
        "required": ["spec_type", "id"]
    }
}
```

Add the handler in the match block (after `spec.split`).

**Pattern:** Match the existing handler style — `args.get()` for
parameter extraction, `Response::error` for validation failures,
`Response::success` / `Response::error` for results.

```rust
"spec.create" => {
    let spec_type = args.get("spec_type").and_then(|v| v.as_str()).unwrap_or("");
    let id = args.get("id").and_then(|v| v.as_str()).unwrap_or("");
    if spec_type.is_empty() {
        return Response::error(
            req.id.clone(),
            -32602,
            "spec.create requires 'spec_type' parameter",
        );
    }
    if id.is_empty() {
        return Response::error(
            req.id.clone(),
            -32602,
            "spec.create requires 'id' parameter",
        );
    }
    let title = args.get("title").and_then(|v| v.as_str());
    let description = args.get("description").and_then(|v| v.as_str());
    let blocked_by: Vec<String> = args
        .get("blocked_by")
        .and_then(|v| v.as_array())
        .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
        .unwrap_or_default();

    match crate::commands::spec::create_spec_value(
        spec_type, id, title, description, blocked_by, vec![],
    ) {
        Ok(result) => {
            let text = serde_json::to_string_pretty(&result).unwrap_or_default();
            Response::success(
                req.id.clone(),
                serde_json::json!({
                    "content": [{ "type": "text", "text": text }]
                }),
            )
        }
        Err(e) => Response::error(req.id.clone(), -32603, &e.to_string()),
    }
}
```

This requires adding `create_spec_value` to the parent mod.rs re-exports:

```rust
pub(crate) use internal::create_spec_value;
```

## Step 6: Update /spec Skill

In `resources/claude/spec.md`, add to the MUTATIONS section (before
`spec.promote`):

```markdown
- `spec.create` — Scaffold a new spec. Use when the user says "let's
  spec this out" or when pausing current work to address a discovered
  issue. Infer type from context (bug → fix, new capability → feat).
  Parameters: spec_type (required), id (required), title, description,
  blocked_by.
```

The skill file is compile-time embedded via `include_str!` in
`src/adapters/templates.rs`. A `cargo build` picks up the change.

## Step 7: Add Clap Tests

In `src/commands/spec/mod.rs`, add to the test module:

```rust
#[test]
fn create_basic() {
    let cmd = parse(&["create", "feat", "my-feature"]).unwrap();
    match cmd {
        SpecCommands::Create { r#type, id, title, json, .. } => {
            assert_eq!(r#type, "feat");
            assert_eq!(id, "my-feature");
            assert!(title.is_none());
            assert!(!json);
        }
        _ => panic!("expected Create"),
    }
}

#[test]
fn create_with_options() {
    let cmd = parse(&[
        "create", "fix", "my-bug",
        "--title", "Fix the bug",
        "--blocked-by", "other-spec",
        "--json",
    ]).unwrap();
    match cmd {
        SpecCommands::Create { r#type, id, title, blocked_by, json, .. } => {
            assert_eq!(r#type, "fix");
            assert_eq!(id, "my-bug");
            assert_eq!(title.as_deref(), Some("Fix the bug"));
            assert_eq!(blocked_by, vec!["other-spec"]);
            assert!(json);
        }
        _ => panic!("expected Create"),
    }
}
```

## Commit Strategy

```
1. spec: add SpecCommands::Create + create.rs implementation
   (CLI command, internal function, mod.rs wiring)

2. spec: register spec.create MCP tool
   (server.rs schema + handler, mod.rs re-export)

3. spec: add create to /spec skill
   (resources/claude/spec.md update)
```

## Verification Checklist

```bash
# Unit tests
cargo test -p patina

# Clippy
cargo clippy

# Build + install
cargo build --release && cargo install --path .

# Live test — basic create
patina spec create feat test-spec-create
# Verify: directory exists, SPEC.md has correct frontmatter, git log shows commit

# Live test — with options
patina spec create fix test-fix --title "Test Fix" --description "A test fix"
# Verify: title in heading, description in blockquote, fix template sections

# Live test — duplicate rejection
patina spec create feat test-spec-create
# Should fail: directory already exists

# Live test — invalid type
patina spec create unknown bad-spec
# Should fail: lists valid types

# Live test — JSON output
patina spec create explore test-explore --json
# Verify: JSON with path, type, id, status fields

# MCP test (via patina mcp server or direct)
# Verify spec.create appears in tools/list response

# Cleanup
rm -rf layer/surface/build/feat/test-spec-create
rm -rf layer/surface/build/fix/test-fix
rm -rf layer/surface/build/explore/test-explore
git add -A && git commit -m "test: clean up spec-create test artifacts"
```

## Design Decisions

### D1: Why not use `mutate_spec()` for create?

`mutate_spec()` reads an existing file, applies a closure, writes it
back. Create has no existing file — it builds from scratch. Different
code path, same contract (SpecFrontmatter + serialize_spec_file).

### D2: Session detection — parse YAML, not shell out

Reading `.patina/local/active-session.md` and parsing its YAML
frontmatter is cheaper and more reliable than shelling out to
`patina session` commands. Same approach the session module itself uses.

### D3: DB INSERT — INSERT OR REPLACE, not INSERT OR IGNORE

A failed-then-retried create should update the DB row, not silently
skip it. `INSERT OR REPLACE` handles this. The scrape module uses
plain INSERT with conflict handling — we follow the same pattern but
with OR REPLACE since we know the data is fresh.

### D4: No --beliefs flag

Beliefs are intellectual provenance added during spec writing, not at
scaffold time. The frontmatter field exists (from SpecFrontmatter) but
we don't expose it as a CLI flag. Users add beliefs by editing SPEC.md.

### D5: Body template comes from registry, title/description from CLI

The registry provides section headings (Problem, Solution, etc.). The
CLI provides the `# type: title` heading and `> description` blockquote.
These are concatenated: heading + blockquote + template. This keeps the
template reusable and the per-spec content parameterized.
