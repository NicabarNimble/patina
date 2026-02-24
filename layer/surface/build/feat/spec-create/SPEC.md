---
type: feat
id: spec-create
status: draft
created: 2026-02-24
sessions:
  origin: 20260224-053924
related:
  - src/commands/spec/mod.rs
  - src/commands/spec/internal.rs
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
> directory, populate frontmatter, open for editing. Completes the
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
| `--description` | no | One-line description for the body |
| `--blocked-by` | no | Spec IDs this is blocked by |
| `--related` | no | Related file paths |
| `--json` | no | Structured output |

### Behavior

1. **Validate inputs:**
   - `type` must be one of: `feat`, `fix`, `refactor`, `explore`
   - `id` must be kebab-case (lowercase, hyphens, no spaces)
   - Directory must not already exist
   - Id must not conflict with an archived spec tag

2. **Resolve directory path:**
   - `layer/surface/build/<type>/<id>/SPEC.md`
   - Create the directory

3. **Populate frontmatter:**
   - `type`, `id`, `status: draft`, `created: <today>`
   - `sessions.origin: <active-session-id>` if a session is active
   - `blocked_by`, `related`, `beliefs` from flags (empty if not given)

4. **Write body template:**
   ```markdown
   # <type>: <title>

   > <description or "TODO: problem statement">

   ## Problem

   ## Solution

   ## Exit Criteria

   - [ ]

   ## Key Files

   ```
   ```

5. **Git commit:**
   - `git add <path>`
   - `git commit -m "spec: draft <id>"`

6. **Update database:**
   - Insert into patterns table (same as scrape would)
   - Avoids requiring a scrape after creation

### Type Validation

Spec types map to release bumps via `BumpType::from_spec_type()`:

| Type | Bump | Directory |
|------|------|-----------|
| `feat` | minor | `layer/surface/build/feat/<id>/` |
| `fix` | patch | `layer/surface/build/fix/<id>/` |
| `refactor` | patch | `layer/surface/build/refactor/<id>/` |
| `explore` | none | `layer/surface/build/explore/<id>/` |

Unknown types are rejected. This keeps the directory structure
predictable and the release system reliable.

### MCP Tool: `spec.create`

Register as the 12th spec MCP tool in `src/mcp/server.rs`.

```json
{
  "name": "spec.create",
  "description": "Create a new spec draft",
  "inputSchema": {
    "type": "object",
    "properties": {
      "spec_type": { "type": "string", "description": "feat, fix, refactor, explore" },
      "id": { "type": "string", "description": "Spec identifier (kebab-case)" },
      "title": { "type": "string", "description": "Human title" },
      "description": { "type": "string", "description": "One-line problem statement" },
      "blocked_by": { "type": "array", "items": { "type": "string" } }
    },
    "required": ["spec_type", "id"]
  }
}
```

Follows the `_value()` pattern established in spec-workflow-rigor
Phase 6 — `create_spec_value()` returns `serde_json::Value`, the
CLI function delegates to it for both JSON and human output.

### `/spec` Skill Update

The skill at `resources/claude/spec.md` already mentions `create` in
the judgment guidance section. Add it to the mutations list:

```
- `spec.create` — Scaffold a new spec. Use when the user says "let's
  spec this out" or when pausing current work to address a discovered
  issue. Infer type from context (bug → fix, new capability → feat).
  Parameters: spec_type (required), id (required), title, description,
  blocked_by.
```

### Session Integration

If an active session exists (`.patina/local/active-session.md`),
automatically set `sessions.origin` in the new spec's frontmatter.
This links the spec to the conversation that spawned it — useful for
provenance when reviewing specs later.

## Key Files

```
src/commands/spec/mod.rs              — add SpecCommands::Create, pub fn create()
src/commands/spec/internal.rs         — create_spec(), create_spec_value()
src/spec.rs                           — SpecFrontmatter (no changes needed)
src/main.rs                           — dispatch SpecCommands::Create
src/mcp/server.rs                     — register spec.create tool
resources/claude/spec.md              — update /spec skill
src/adapters/templates.rs             — spec.md re-embedded on build
```

## Exit Criteria

- [ ] `patina spec create feat my-feature` scaffolds directory + SPEC.md + commits
- [ ] `patina spec create fix my-bug --title "Fix the bug"` uses custom title
- [ ] `patina spec create feat duplicate` fails if directory or tag exists
- [ ] `patina spec create unknown my-spec` fails with valid type list
- [ ] `--json` output includes path, type, id, status
- [ ] `--blocked-by other-spec` sets frontmatter field
- [ ] MCP tool `spec.create` registered and functional
- [ ] `/spec create` works via skill (LLM can discover and invoke)
- [ ] Session origin auto-detected from active session
- [ ] Database updated without requiring `patina scrape`

## Non-Goals

- No interactive prompts — all params via flags (composable CLI)
- No template customization — single template per type
- No `--editor` flag to open in $EDITOR — print path, user opens
- No spec type creation (adding new types beyond feat/fix/refactor/explore)
- No LLM-generated body content — scaffold is mechanical, writing is human+LLM

## Provenance

Carved out as Phase 0 of spec-workflow-rigor (Feb 2026). The spec
said: "spec create is the entry point to the entire lifecycle and
needs its own spec." 52+ commits implemented Phases 1-7 without it.
The absence was felt every time a new spec was hand-created during
that work — this session alone created two specs by hand
(adapter-refresh-preserves-user-state and this one).
