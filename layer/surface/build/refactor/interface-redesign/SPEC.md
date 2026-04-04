---
type: refactor
id: interface-redesign
status: draft
created: 2026-04-04
sessions:
  origin: 20260403-070944-045859000
beliefs:
  - "[[vocabulary-drift-compounds]]"
  - "[[adapter-is-dependable-rust-at-external-edges]]"
  - "[[core-principles-contain-blast-radius]]"
  - "[[core-verbs-standalone-mother-additive]]"
  - "[[children-have-agency-toys-are-capabilities]]"
  - "[[stale-context-is-hostile-context]]"
related:
  - src/interface/
  - src/commands/ai/
  - src/interface/runtime/templates.rs
  - src/interface/internal/bundle.rs
  - src/interface/internal/bootstrap.rs
  - src/interface/launch.rs
blocked_by:
  - adapter-to-interface-rename
exit_criteria:

  - id: ir1-mother-interface-registry
    text: "Mother discovers interfaces from ~/.patina/interfaces/{name}/interface.toml. patina ai list reads from this registry, not from hardcoded Rust array. Three built-in interfaces (claude, gemini, opencode) seeded on first run from binary-embedded defaults."
    checked: false

  - id: ir2-interface-manifest
    text: "Each interface has interface.toml with: name, display, kind (interactive|headless|rpc), detect command, vendor_bootstrap flag, version, sessions.max_concurrent, sessions.allow_attach, and skills list referencing Mother-approved skills by name."
    checked: false

  - id: ir3-mother-skill-repo
    text: "~/.patina/skills/ exists as Mother's approved skill repository. Skills are directories with SKILL.md following the Agent Skills standard. The 'patina' base skill always exists and is always projected."
    checked: false

  - id: ir4-base-patina-skill
    text: "~/.patina/skills/patina/SKILL.md teaches any interface how to use Patina: scry, context, assay, repo, belief, spec commands. When to use each. How to ask Mother. How sessions and specs work. This skill is always projected into every interface — not optional, not listed in manifest."
    checked: false

  - id: ir5-ephemeral-projection
    text: "On session start, Mother projects interface files into the project (.{name}/, AGENTS.md, vendor shims). On session end, Mother removes the projection. Project directory is clean between sessions — only layer/ and .patina/ remain."
    checked: false

  - id: ir6-projection-from-registry
    text: "Projection reads templates from ~/.patina/interfaces/{name}/templates/ and skills from ~/.patina/skills/ (base patina skill + manifest-referenced skills). No include_str!() at runtime — binary-embedded templates are seed data extracted once, Mother's on-disk copy is the runtime source."
    checked: false

  - id: ir7-gitignore-interface-files
    text: ".gitignore includes .claude/, .gemini/, .opencode/, AGENTS.md, CLAUDE.md, GEMINI.md. Interface files never enter git history. patina init sets this up. Existing projects get migration guidance."
    checked: false

  - id: ir8-session-cleanup
    text: "patina ai end (and session-end.sh wrapper) removes projected interface files after archiving session artifact and committing session changes. Orphan detection: Mother detects projections with no active session and offers cleanup."
    checked: false

  - id: ir9-multi-session-control
    text: "interface.toml sessions.max_concurrent controls how many simultaneous sessions of that interface a project can have. Default 1. sessions.allow_attach enables tmux reconnection to existing sessions. Mother enforces limits and tracks active sessions per interface per project."
    checked: false

  - id: ir10-durable-object-state
    text: "Each interface instance in a project has durable state in mother_interface_instances table (project_uid, interface_name, lifecycle, active_session_id, active_pid, last_session_id, last_handoff). Mother creates row on first use, sets active on session start, dormant on session end. Orphan detection via heartbeat: lifecycle=active but pid dead. Project-specific overrides live in project/.patina/interfaces/{name}/overrides/."
    checked: false

  - id: ir11-detect-and-version
    text: "Mother runs interface detect command to check availability and cache installed version. patina ai list shows both Mother's bundle version and the detected tool version. Detect is lazy — cached in state/, refreshed on explicit check."
    checked: false

  - id: ir12-skill-projection-merge
    text: "During projection, Mother merges skills from three sources in order: (1) base patina skill (always), (2) manifest-referenced skills from ~/.patina/skills/, (3) interface-specific skills from templates/skills/. All land in .{name}/skills/ (or equivalent for the interface format). Projection is self-contained."
    checked: false

  - id: ir13-no-hardcoded-interfaces
    text: "InterfaceBundle, INTERFACE_BUNDLES, claude_templates/gemini_templates/opencode_templates modules removed. interface_bundle() reads from Mother's registry on disk. Binary retains embedded seed data for first-run extraction only."
    checked: false

  - id: ir14-interface-lifecycle-events
    text: "Interface lifecycle emits events to project events.db via existing insert_event(): interface.projected (files created), interface.cleaned (files removed), interface.orphaned (stale projection detected), interface.recovered (orphan cleaned). Payloads include interface name, projected paths, bundle version, session ref. Events export to layer/events.jsonl via patina events export."
    checked: false

  - id: ir15-bundle-update-flow
    text: "On patina binary upgrade, Mother detects binary version > bundle version and re-extracts seed data for built-in interfaces. Re-extract uses a seed manifest to only touch seed-origin files — user-added files survive. interface.toml pinned=true skips auto-update entirely. Tool version detection (claude --version) is independent — cached lazily, refreshed on patina ai list or session start."
    checked: false

  - id: ir16-compile-proof
    text: "cargo check --workspace -q passes. cargo test -q --lib passes. patina ai list shows all registered interfaces. patina ai claude round-trip works (project → launch → session → end → clean)."
    checked: false
---

# refactor: Interface Redesign — Mother-Managed, Ephemeral, Skill-Driven

## Problem

Interfaces are hardcoded in Rust (`InterfaceBundle` array, `include_str!()` templates,
per-interface install functions). Adding or modifying an interface requires recompilation.
Interface files are permanently installed in projects and committed to git, causing stale
context (AGENTS.md lies between sessions) and polluting git history with non-knowledge files.

Skills are wired per-interface in Rust code. There's no shared skill repository and no
base skill that teaches interfaces how to use Patina.

## Design

### Mother as Interface Registry

Mother discovers interfaces from `~/.patina/interfaces/{name}/interface.toml`.
The binary seeds three built-in interfaces on first run. After extraction, Mother
owns them — updates don't require recompilation.

```
~/.patina/interfaces/
├── claude/
│   ├── interface.toml
│   └── templates/
│       ├── commands/
│       ├── skills/          # interface-specific skills
│       ├── bin/
│       └── bootstrap/       # AGENTS.md section template, vendor shim template
├── gemini/
├── opencode/
└── pi/                      # user-added, same structure
```

### interface.toml

```toml
name = "opencode"
display = "OpenCode"
kind = "interactive"                    # interactive | headless | rpc
detect = "opencode --version"
vendor_bootstrap = false                # no OPENCODE.md shim
version = "0.46.0"                      # Patina bundle version

[sessions]
max_concurrent = 1
allow_attach = true

[skills]
include = ["epistemic-beliefs", "spec", "patina-review"]
```

### Mother's Approved Skill Repo

```
~/.patina/skills/
├── patina/                             # base skill — ALWAYS projected
│   └── SKILL.md
├── epistemic-beliefs/
│   ├── SKILL.md
│   ├── scripts/create-belief.sh
│   └── references/
├── spec/
│   └── SKILL.md
└── patina-review/
    └── SKILL.md
```

The `patina` base skill is injected into every projection regardless of manifest.
It teaches the interface how to use Patina: scry, context, assay, repo, belief,
spec, sessions. This is the "how to ask Mother" knowledge.

### Durable Object Model — Mother SQLite, Not Project Files

Each interface instance in a project has durable state in Mother's SQLite
(`~/.patina/mother/state.db`), not in project filesystem. This follows the
existing pattern — `mother_sessions` already tracks session lifecycle in
SQLite. Interface instances extend this.

**Global definition** (templates, manifest): `~/.patina/interfaces/{name}/`
**Instance state** (lifecycle, session refs): `mother_interface_instances` table
**Project overrides** (custom instructions): `project/.patina/interfaces/{name}/overrides/`

```sql
CREATE TABLE mother_interface_instances (
    project_uid TEXT NOT NULL,
    interface_name TEXT NOT NULL,
    lifecycle TEXT NOT NULL DEFAULT 'dormant',  -- dormant | active | orphaned
    active_session_id TEXT,            -- FK to mother_sessions.runtime_id
    active_pid INTEGER DEFAULT 0,      -- process ID for orphan detection
    last_session_id TEXT,              -- previous session for handoff context
    last_handoff TEXT,                 -- summary for next session context
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    PRIMARY KEY (project_uid, interface_name)
);
```

Lifecycle: dormant → active (wake/project) → dormant (hibernate/cleanup).
Mother detects orphans: lifecycle=active but pid is dead → lifecycle=orphaned → offer cleanup.
Mother's heartbeat thread (already runs every 60s) checks for orphans.

This reuses Mother's existing infrastructure:
- `mother_sessions` already tracks Active/Completed/Archived per interface
- `mother_child_state` pattern for key-value persistence
- Heartbeat thread for periodic checks
- WAL checkpointing for durability

### Ephemeral Projection Lifecycle

**Session start** (`patina ai opencode`):
1. Mother validates: project exists, on patina branch, interface registered, tool detected
2. Mother checks session limits (max_concurrent, allow_attach)
3. Mother creates session artifact + git tag
4. Mother projects: templates/ + skills → project directory
5. Mother generates AGENTS.md from bootstrap template + environment + project state
6. Mother launches process (interactive: exec/tmux, headless: spawn)

**Session end** (`patina ai end` / `/session-end`):
1. Archive session artifact, create end git tag
2. Commit session changes (layer/ files)
3. Remove projection (.{name}/, AGENTS.md, vendor shims)
4. Update durable state (DO hibernates)

**Between sessions**: project has no interface files. Clean. Only `layer/` and `.patina/`.

### Skill Projection Merge Order

During projection into `.{name}/skills/`:
1. **Base**: `~/.patina/skills/patina/` → always
2. **Manifest**: each name in `[skills].include` → from `~/.patina/skills/{name}/`
3. **Interface-specific**: `templates/skills/*` → interface's own skills

### Bundle Update Flow

Two independent version concerns:

**Tool version** (Claude Code 2.1 → 2.2): Updates independently of Patina. Mother
runs the detect command (`claude --version`), caches result, shows in `patina ai list`.
No Patina changes needed.

**Bundle version** (Patina interface templates, skills, commands): Three update paths:

1. **Binary upgrade** — `cargo install patina-ai` ships new seed data. On next use,
   Mother detects binary version > bundle version, re-extracts seed templates.
   Re-extract uses a seed manifest to only overwrite seed-origin files — user-added
   files in `~/.patina/interfaces/{name}/templates/` survive.

2. **Manual edit** — User modifies templates or adds skills directly in
   `~/.patina/interfaces/{name}/`. Next session projection picks up changes.
   No binary rebuild.

3. **Future: remote pull** — `patina interface update claude` from git/registry.
   Same pattern as `patina repo add` but for interface definitions.

**Pinning** — `interface.toml` `pinned = true` tells Mother: don't auto-update
this bundle on binary upgrade. For users who fork and customize a bundle entirely.

**No active session impact** — updates happen before projection. Since projections
are ephemeral and regenerated fresh each session, there's no migration concern.
The old projection was cleaned up when the last session ended.

### What Gets Gitignored

```gitignore
# Patina interface projections (ephemeral, managed by Mother)
.claude/
.gemini/
.opencode/
AGENTS.md
CLAUDE.md
GEMINI.md
```

## What Does NOT Change

- `layer/` — permanent, committed, patina branch
- `.patina/` — Mother-owned local state, already gitignored
- Session lifecycle commands (session-start, update, note, end)
- The patina branch model
- Core Patina commands (scry, context, assay, etc.)
- Mother daemon architecture

## Migration

Existing projects have interface files committed to git. Migration:
1. Add interface files to .gitignore
2. `git rm --cached .claude/ CLAUDE.md AGENTS.md` etc.
3. Commit removal
4. Next `patina ai claude` projects fresh from Mother

Can be automated via `patina interface migrate` command.
