---
type: feat
id: spec-create
status: active
created: 2026-02-24
blocked_by:
- spec-module-split
sessions:
  origin: 20260224-053924
related:
- src/commands/spec/mod.rs
- src/commands/spec/internal/
- src/spec.rs
- src/mcp/server.rs
- resources/claude/spec.md
- src/adapters/templates.rs
beliefs:
- spec-first
- unix-philosophy
- plugins-are-three-prong-bundles
- mutation-completes-query
---

# feat: Spec Create — Scaffold Specs from the CLI

> Today specs are hand-created: `mkdir`, write frontmatter, add body,
> commit. Every time. `spec create` makes it one command — scaffold the
> directory, populate frontmatter, commit, print path. Completes the
> mutation side of the spec lifecycle that spec-workflow-rigor left as
> Phase 0.

## Problem

The spec lifecycle has 7 mutation commands: promote, pause, resume,
block, complete, abandon, split. All were built in spec-workflow-rigor.
But there's no `create`. The entry point is missing.

**What creation looks like today:**

```bash
mkdir -p layer/surface/build/feat/my-feature
cat > layer/surface/build/feat/my-feature/SPEC.md << 'EOF'
---
type: feat
id: my-feature
status: draft
created: 2026-02-24
sessions:
  origin: 20260224-053924
related: []
beliefs: []
---

# feat: My Feature

> Problem statement here.
EOF
git add layer/surface/build/feat/my-feature/SPEC.md
git commit -m "spec: draft my-feature"
```

That's 5 steps and a lot of boilerplate that varies only by type, id,
date, and session. Every field except the title and body is mechanical.

**What it should look like:**

```bash
patina spec create feat my-feature
# → creates directory, writes frontmatter, commits
# → prints path for editing
```

**Impact on other tools:**

- MCP registered 11 of 12 planned tools. `spec.create` is the missing
  12th. Without it, LLMs can manage specs but can't start them.
- The `/spec` skill describes `create` but it doesn't work — calling
  it falls through to "command not found."
- `spec pause → spec create fix-the-bug` is the natural flow for
  mid-work diversions. Today you have to hand-scaffold between pause
  and the new work.

## Solution

**Prerequisite:** spec-module-split must be complete first (done: v0.30.1).
This spec assumes `internal/` directory already exists. `create.rs` lands
as a new file in the split structure.

**Type system decision (session 20260224-195035):** Instead of a `types.rs`
registry in the bin crate, add a thin `SpecType` enum to `src/spec.rs`
(lib crate). Rationale: registry can't centralize BumpType (lib/bin
boundary), identity mapping doesn't need indirection, and the
[[boundary-string-internal-enum]] pattern keeps `SpecFrontmatter.r#type`
as String while parsing to enum at the boundary. See also
[[adding-type-is-not-migrating-model]].

### `patina spec create <type> <id> [options]`

Single command that scaffolds a spec directory, writes SPEC.md with
populated frontmatter, commits, and prints the path.

```
$ patina spec create feat my-feature --title "My Feature Title"

Created: layer/surface/build/feat/my-feature/SPEC.md
  Type:    feat
  Status:  draft
  Session: 20260224-053924

Edit: $EDITOR layer/surface/build/feat/my-feature/SPEC.md
```

**Parameters:**

| Parameter | Required | Description |
|-----------|----------|-------------|
| `type` | yes | Spec type: feat, fix, refactor, explore |
| `id` | yes | Spec identifier (kebab-case) |
| `--title` | no | Human title (defaults to `<type>: <id>`) |
| `--description` | no | One-line description for the blockquote |
| `--blocked-by` | no | Spec IDs this is blocked by |
| `--related` | no | Related file paths |
| `--json` | no | Structured output |

### Behavior

1. **Validate inputs:**
   - `type` must parse to `SpecType` enum (from `src/spec.rs`).
     Unknown types rejected with `SPEC_TYPES` list in the error message.
   - `id` must be kebab-case: `^[a-z][a-z0-9-]*$`
   - Directory must not already exist on disk
   - `spec/<id>` tag must not exist (prevents collision with archived specs)

2. **Resolve directory path:**
   - Uses `spec_type.as_str()` to build:
     `layer/surface/build/<type>/<id>/SPEC.md`
   - Create the directory with `std::fs::create_dir_all`

3. **Populate frontmatter via `SpecFrontmatter`:**
   - Build a `SpecFrontmatter` struct (from `src/spec.rs`)
   - Set: `type`, `id`, `status: "draft"`, `created: <today>`
   - Set `sessions` to `Sessions::Structured { origin: <active-session-id> }`
     if a session is active (read from `.patina/local/active-session.md`)
   - Set `blocked_by`, `related` from flags (empty if not given)
   - Serialize with `serialize_spec_file()` — same contract as every
     other spec mutation

4. **Write body from type template:**
   - Match on `SpecType` enum for body template selection
   - Prepend `# <type>: <title>` heading and blockquote description
   - Body templates have section headings only — no content

5. **Git commit:**
   - `git add <path>`
   - `git commit -m "spec: draft <id>"`

6. **Update database:**
   - Insert into patterns table: id, file_path, status, title, type
   - Same INSERT pattern as scrape uses
   - Avoids requiring a `patina scrape` after creation

### Three-Prong Bundle: CLI + MCP + Skill

Following the [[plugins-are-three-prong-bundles]] pattern established
by spec-workflow-rigor Phase 6:

**CLI command** (`src/commands/spec/mod.rs`):
- New `SpecCommands::Create` variant
- `pub fn create()` delegates to `internal::create_spec()`

**Implementation** (`src/commands/spec/internal/create.rs`):
- `create_spec(type, id, title, description, blocked_by, related, json)`
  — human output
- `create_spec_value(type, id, title, description, blocked_by, related)`
  — returns typed `CreateResult` for MCP (follows MutationResult pattern)

**MCP tool** (`src/mcp/server.rs`):
- Register `spec.create` as the 12th spec tool
- Schema:
  ```json
  {
    "name": "spec.create",
    "description": "Create a new spec draft — scaffold directory, write frontmatter, commit.",
    "inputSchema": {
      "type": "object",
      "properties": {
        "spec_type": { "type": "string", "description": "Spec type: feat, fix, refactor, explore" },
        "id": { "type": "string", "description": "Spec identifier (kebab-case)" },
        "title": { "type": "string", "description": "Human title" },
        "description": { "type": "string", "description": "One-line problem statement" },
        "blocked_by": { "type": "array", "items": { "type": "string" } }
      },
      "required": ["spec_type", "id"]
    }
  }
  ```
- Handler calls `create_spec_value()` and returns result

**Skill update** (`resources/claude/spec.md`):
- Add `spec.create` to the mutations section:
  ```
  - `spec.create` — Scaffold a new spec. Use when the user says "let's
    spec this out" or when pausing current work to address a discovered
    issue. Infer type from context (bug → fix, new capability → feat).
    Parameters: spec_type (required), id (required), title, description,
    blocked_by.
  ```
- Skill file is compile-time embedded via `include_str!` in
  `templates.rs` — rebuild deploys the update

### Session Integration

If an active session exists (`.patina/local/active-session.md`),
automatically set `sessions.origin` in the new spec's frontmatter.
This links the spec to the conversation that spawned it — useful for
provenance when reviewing specs later.

Detection: parse the active-session file's YAML frontmatter for the
`id` field. Same approach used by `patina session` commands.

## Key Files

```
src/spec.rs                           — add SpecType enum, SPEC_TYPES, FromStr, SpecTypeError
src/commands/spec/mod.rs              — add SpecCommands::Create, pub fn create()
src/commands/spec/internal/create.rs  — create_spec(), create_spec_value(), CreateResult [NEW]
src/commands/spec/internal/mod.rs     — add mod create, re-export
src/main.rs                           — dispatch SpecCommands::Create
src/mcp/server.rs                     — register spec.create tool + handler
resources/claude/spec.md              — add create to /spec skill mutations
src/adapters/templates.rs             — spec.md re-embedded on build
```

## Exit Criteria

- [ ] `patina spec create feat my-feature` scaffolds directory + SPEC.md + commits
- [ ] `patina spec create fix my-bug --title "Fix the bug"` uses custom title
- [ ] `patina spec create feat duplicate` fails if directory or tag exists
- [ ] `patina spec create unknown my-spec` fails with valid type list
- [ ] Body uses type-appropriate template from registry
- [ ] `--json` output includes path, type, id, status
- [ ] `--blocked-by other-spec` sets frontmatter field
- [ ] MCP tool `spec.create` registered and functional (12th tool)
- [ ] `/spec create` works via skill (LLM can discover and invoke)
- [ ] Session origin auto-detected from active session
- [ ] Database updated without requiring `patina scrape`

## Non-Goals

- No interactive prompts — all params via flags (composable CLI)
- No template customization — body templates are system data in `types.rs`
- No `--editor` flag to open in $EDITOR — print path, user opens
- No spec type creation (adding new types beyond the 4 in the registry)
- No LLM-generated body content — scaffold is mechanical, writing is human+LLM

## Provenance

Carved out as Phase 0 of spec-workflow-rigor (Feb 2026). The spec
said: "spec create is the entry point to the entire lifecycle and
needs its own spec." 52+ commits implemented Phases 1-7 without it.
The absence was felt every time a new spec was hand-created during
that work — three specs were hand-created in a single session
(adapter-refresh-preserves-user-state, spec-create, spec-module-split).
Blocked by spec-module-split so that `create.rs` lands in the clean
`internal/` directory structure alongside its siblings.
