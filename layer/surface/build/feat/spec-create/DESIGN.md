# Design Walkthrough: spec-create

Step-by-step implementation guide. Read SPEC.md first for the full
rationale — this doc is the mechanical "how."

**Prerequisite:** spec-module-split is complete (v0.30.1). The
`src/commands/spec/internal/` directory exists. `create.rs` lands as
a new file in the split structure.

**Type system decision:** No `types.rs` registry. Instead, a thin
`SpecType` enum lives in `src/spec.rs` (lib crate) with `FromStr` +
`as_str()`. See [[boundary-string-internal-enum]] and
[[adding-type-is-not-migrating-model]] for rationale.

## Before You Start

```bash
# Verify prerequisite
ls src/commands/spec/internal/mod.rs     # must exist (from spec-module-split)
cargo test -p patina                     # all green

# Understand the pattern — read one existing mutation
cat src/commands/spec/internal/mutations.rs | head -80
```

Read these files in order:
1. `src/spec.rs` — `SpecFrontmatter`, `serialize_spec_file()`, and new `SpecType` enum
2. `src/commands/spec/mod.rs` — where Create variant goes
3. `src/commands/spec/internal/mutations.rs` — `MutationResult`/`_value()` pattern to follow
4. `src/mcp/server.rs` — where MCP tool gets registered
5. `resources/claude/spec.md` — skill definition to update

## Step 1: Add SpecType enum to src/spec.rs

Add to `src/spec.rs` (lib crate), after the existing types but before
Parse/Serialize section:

```rust
/// Canonical list of valid spec types (for error messages, help text, tests).
pub const SPEC_TYPES: &[&str] = &["feat", "fix", "refactor", "explore"];

/// Typed spec type — parse from string at boundaries, match internally.
///
/// Follows [[boundary-string-internal-enum]]: SpecFrontmatter.r#type stays
/// String for serde compatibility; this enum is used for validation and
/// exhaustive matching in new code.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpecType {
    Feat,
    Fix,
    Refactor,
    Explore,
}

/// Error when parsing an invalid spec type string.
#[derive(Debug)]
pub struct SpecTypeError {
    pub got: String,
}

impl std::fmt::Display for SpecTypeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "invalid spec type \"{}\" (expected one of: {})",
            self.got,
            SPEC_TYPES.join(", ")
        )
    }
}

impl std::error::Error for SpecTypeError {}

impl std::str::FromStr for SpecType {
    type Err = SpecTypeError;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s {
            "feat" => Ok(SpecType::Feat),
            "fix" => Ok(SpecType::Fix),
            "refactor" => Ok(SpecType::Refactor),
            "explore" => Ok(SpecType::Explore),
            _ => Err(SpecTypeError { got: s.to_string() }),
        }
    }
}

impl SpecType {
    /// Canonical string form (matches YAML frontmatter values).
    pub fn as_str(&self) -> &'static str {
        match self {
            SpecType::Feat => "feat",
            SpecType::Fix => "fix",
            SpecType::Refactor => "refactor",
            SpecType::Explore => "explore",
        }
    }
}
```

Add tests in the existing `#[cfg(test)]` block:

```rust
#[test]
fn test_spec_type_roundtrip() {
    for &name in SPEC_TYPES {
        let t: SpecType = name.parse().expect(name);
        assert_eq!(t.as_str(), name);
    }
}

#[test]
fn test_spec_type_invalid() {
    let err = "unknown".parse::<SpecType>().unwrap_err();
    assert!(err.to_string().contains("unknown"));
    assert!(err.to_string().contains("feat"));
}
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

Add re-exports in `spec/mod.rs`:

```rust
pub(crate) use internal::create_spec_value;
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
} => {
    commands::spec::create(
        &r#type,
        &id,
        title.as_deref(),
        description.as_deref(),
        blocked_by,
        related,
        json,
    )?;
}
```

## Step 4: Create internal/create.rs

New file: `src/commands/spec/internal/create.rs`

Key differences from DESIGN v1:
- **No `types::lookup()`** — parse `SpecType` directly from string
- **No `serde_json::Value` return** — typed `CreateResult` struct
- **No `std::process::Command` for git** — use `patina::git::*` helpers
- **Body templates** — match on `SpecType` enum locally

### CreateResult (typed return, follows MutationResult pattern)

```rust
#[derive(Debug, Serialize)]
pub struct CreateResult {
    pub command: &'static str,
    pub spec_id: String,
    pub spec_type: String,
    pub status: &'static str,
    pub path: String,
    pub directory: String,
    pub session_origin: Option<String>,
}
```

### Body templates (match on SpecType)

```rust
fn body_template(spec_type: SpecType) -> &'static str {
    match spec_type {
        SpecType::Feat => "## Problem\n\n## Solution\n\n## Exit Criteria\n\n## Non-Goals\n",
        SpecType::Fix => "## Problem\n\n## Root Cause\n\n## Fix\n\n## Exit Criteria\n",
        SpecType::Refactor => "## Current State\n\n## Target State\n\n## Steps\n\n## Exit Criteria\n",
        SpecType::Explore => "## Question\n\n## Findings\n\n## Conclusions\n",
    }
}
```

### Core flow

1. Parse type: `let spec_type: SpecType = type_str.parse()?`
2. Validate id: regex `^[a-z][a-z0-9-]*$`
3. Check directory doesn't exist: `layer/surface/build/{spec_type.as_str()}/{id}/`
4. Check archive tag doesn't exist: `spec/{id}`
5. Create directory: `std::fs::create_dir_all`
6. Build `SpecFrontmatter` struct with `serialize_spec_file()`
7. Build body: `# {type}: {title}\n\n> {description}\n\n{template}`
8. Write SPEC.md
9. Git commit: `patina::git::add_paths()` + `patina::git::commit()`
   (uses `git_stage_and_commit` helper from mutations.rs)
10. Update DB: INSERT OR REPLACE into patterns table
11. Return `CreateResult`

### Session detection

```rust
fn active_session_id() -> Option<String> {
    let content = std::fs::read_to_string(".patina/local/active-session.md").ok()?;
    let content = content.strip_prefix("---")?;
    let end = content.find("\n---")?;
    let frontmatter: serde_yaml::Value = serde_yaml::from_str(&content[..end]).ok()?;
    frontmatter.get("id")?.as_str().map(|s| s.to_string())
}
```

### Wire into internal/mod.rs

```rust
mod create;

// Re-exports:
pub(crate) use create::create_spec_value;
pub(super) use create::create_spec;
```

## Step 5: Register MCP Tool

In `src/mcp/server.rs`, add tool schema (after `spec.split`) and handler.
Handler calls `crate::commands::spec::create_spec_value()`.

## Step 6: Update /spec Skill

In `resources/claude/spec.md`, add to the MUTATIONS section:

```markdown
- `spec.create` — Scaffold a new spec. Use when the user says "let's
  spec this out" or when pausing current work to address a discovered
  issue. Infer type from context (bug → fix, new capability → feat).
  Parameters: spec_type (required), id (required), title, description,
  blocked_by.
```

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
1. feat: add SpecType enum to src/spec.rs
   (enum, FromStr, SpecTypeError, SPEC_TYPES constant, tests)

2. feat: add spec create command
   (create.rs, SpecCommands::Create, main.rs dispatch, clap tests)

3. feat: register spec.create MCP tool
   (server.rs schema + handler, mod.rs re-export)

4. docs: add create to /spec skill
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

# Pre-push checks
./resources/git/pre-push-checks.sh

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

### D5: Body template via SpecType match, not registry

Body templates are matched on the `SpecType` enum directly in create.rs.
Only one consumer (create) uses templates — a shared registry would be
premature abstraction. The match is exhaustive, so adding a new type
forces a compiler error.

### D6: SpecType in lib, not bin

`SpecType` lives in `src/spec.rs` (lib crate) alongside `SpecFrontmatter`.
This allows `BumpType` (also in lib) to adopt it later via
`impl From<SpecType> for Option<BumpType>`. `SpecFrontmatter.r#type`
stays `String` — no serde migration needed.

### D7: CreateResult over serde_json::Value

Follows the `MutationResult`/`SplitResult` pattern from
spec-mutation-cleanup (v0.30.2). Typed struct with `#[derive(Serialize)]`
gives compile-time field guarantees while still serializing to JSON
for MCP via `serde_json::to_string_pretty`.
