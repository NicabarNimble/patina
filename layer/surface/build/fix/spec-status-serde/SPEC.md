---
type: fix
id: spec-status-serde
status: complete
created: 2026-02-05
target: v0.12.0
beliefs:
- system-owns-format
- milestones-in-specs
---

# fix: Canonical SpecFrontmatter in src/spec.rs

## Problem

Spec frontmatter is fragmented across three locations:
- `session/internal.rs` → `SessionFrontmatter`
- `scrape/layer/mod.rs` → `Frontmatter` (partial, read-only)
- `version/internal.rs` → `SpecFrontmatter`

Additionally, `patina spec status` uses regex instead of serde, violating [[system-owns-format]].

## Solution

Create `src/spec.rs` as the canonical source for spec format:
1. Define `SpecFrontmatter` struct with ALL fields (including spec-as-work-item fields)
2. Provide `parse_spec_file()` and `serialize_spec_file()` functions
3. Commands import from `src/spec.rs`, they don't own the format

Follows existing pattern: `src/session.rs` exists for session format.

## The Contract

```rust
// src/spec.rs
pub struct SpecFrontmatter {
    pub r#type: String,
    pub id: String,
    pub status: Option<String>,
    pub created: Option<String>,
    pub updated: Option<String>,
    pub target: Option<String>,           // spec-as-work-item
    pub blocked_by: Vec<String>,          // spec-as-work-item
    pub blocks: Vec<String>,              // spec-as-work-item
    pub sessions: Option<Sessions>,
    pub related: Vec<String>,
    pub beliefs: Vec<String>,
    pub references: Vec<String>,
    pub milestones: Vec<SpecMilestoneEntry>,
    pub current_milestone: Option<String>,
}

pub fn parse_spec_file(content: &str) -> Result<(SpecFrontmatter, String)>;
pub fn serialize_spec_file(frontmatter: &SpecFrontmatter, body: &str) -> Result<String>;
```

## Files to Change

- `src/spec.rs` — NEW: canonical SpecFrontmatter + parse/serialize
- `src/lib.rs` — Add `pub mod spec;`
- `src/commands/spec/internal.rs` — Import from `crate::spec`, remove regex
- `src/commands/version/internal.rs` — Import from `crate::spec`, remove local struct
- `src/commands/scrape/layer/mod.rs` — Consider importing (or keep separate for partial parse)

## Exit Criteria

- [x] `src/spec.rs` exists with canonical `SpecFrontmatter`
- [x] `spec status` uses serde via `patina::spec`
- [x] `version` commands use `patina::spec::SpecFrontmatter`
- [x] All spec-as-work-item fields in the struct
- [x] Field ordering is deterministic
