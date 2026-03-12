---
type: refactor
id: cli-mcp-skill-unification
status: draft
created: 2026-03-11
sessions:
  origin: 20260311-112321-EF79
related:
  - /Users/nicabar/Projects/Sandbox/AI/RUST/patina/layer/surface/build/refactor/agentic-surface-architecture/SPEC.md
  - /Users/nicabar/Projects/Sandbox/AI/RUST/patina/layer/surface/build/feat/patina-ai-interface-layer/SPEC.md
  - /Users/nicabar/Projects/Sandbox/AI/RUST/patina/layer/surface/build/feat/session-narrative-system/SPEC.md
  - /Users/nicabar/Projects/Sandbox/AI/RUST/patina/layer/surface/build/feat/persona-federation/SPEC.md
  - /Users/nicabar/Projects/Sandbox/AI/RUST/patina/layer/surface/build/refactor/continuous-operation/SPEC.md
beliefs:
  - patina-is-beliefs-plus-action
  - mother-is-connection-and-continuity
  - interfaces-are-not-core
  - dependable-rust
  - unix-philosophy
  - context-files-are-rules-not-docs
exit_criteria:
  - id: mother-owned-capabilities
    text: Mother owns a canonical capability registry for interface-facing tools and skill bundles, instead of adapter-local command inventories
    checked: false
  - id: cli-and-mcp-projections
    text: Patina capabilities project into both CLI commands and MCP tool definitions from the same source of truth
    checked: false
  - id: json-projection-required
    text: Agent-relevant CLI capabilities provide stable machine-readable `--json` output from the same typed data used by MCP, so interfaces and scripts do not need to scrape prose
    checked: false
  - id: interface-discovery-via-mcp
    text: Interfaces can discover available Patina tools through MCP and use that to learn what actions are available without hardcoded adapter-specific lists
    checked: false
  - id: universal-skill-registry
    text: Skills are modeled as canonical Patina artifacts with adapter-specific projections rather than copied one-off prompt files
    checked: false
  - id: adapter-projection-thin
    text: Claude/OpenCode/Gemini adapter layers are reduced to projection and launch concerns rather than owning behavioral truth
    checked: false
  - id: compatibility-preserved
    text: Existing CLI, MCP, and adapter workflows remain available during migration without breaking the current trusted path
    checked: false
---
# refactor: CLI, MCP, and Skill Unification — Mother-Owned Capability Projections

> Unify Patina capabilities under Mother-owned definitions that project into CLI, MCP, and interface skill injection, so interfaces can discover and use the same tools truthfully without adapter-specific hardcoding.

## Current State

Patina has three partially overlapping surfaces today:

- the native CLI command tree
- the MCP server and its tool schemas
- adapter-injected skills/commands for Claude, OpenCode, and Gemini

All three are useful, but they do not yet share one clean source of
truth.

What exists now:

- a real MCP server under `src/mcp/`
- real CLI commands under `src/commands/`
- adapter template injection under `src/adapters/` and `src/interface/`

But the architecture is still mixed:

- many MCP tools wrap existing CLI/domain functions after the fact
- adapter templates still describe commands and skills locally
- interfaces do not reliably learn available tools from Mother
- skill injection and MCP injection are still mostly adapter-era
  mechanics

This is workable, but it does not match the new Mother-first interface
architecture.

## Target State

The target is a Mother-owned capability model with three projections:

- **CLI projection** — `patina ...`
- **MCP projection** — tool discovery and invocation for interface
  actors
- **skill projection** — interface-specific instruction bundles and
  command surfaces

Core rule:

- capability truth lives with Mother/core
- CLI and MCP both project from that truth
- interfaces use MCP to discover tools
- skill bundles teach behavior and workflow, not duplicate the tool
  inventory by hand

This means MCP becomes a first-class way to teach interfaces what tools
exist in the system, but not by scraping the CLI help text or copying
adapter-local markdown. Interfaces should be able to ask Mother, through
MCP:

- what tools exist
- what each tool does
- what schema each tool accepts
- what scope/persona/project restrictions apply

The CLI remains a premier operator surface, but it is no longer the only
place where capability truth lives.

## Steps

### 1. Define interface-facing capabilities explicitly

Introduce a canonical capability registry for interface-facing Patina
operations. This is separate from Child toys.

Examples:

- retrieval/search capabilities
- spec governance capabilities
- measure/health capabilities
- future session/spec/skill management capabilities

Each capability definition should include:

- stable name
- description
- typed input/output shape
- scope/grant metadata
- projection hints for CLI, MCP, and skills

### 2. Make CLI and MCP projections of the same capabilities

Refactor new or migrated areas so that:

- CLI handlers call shared capability functions
- CLI provides stable machine-readable `--json` output for
  agent-relevant commands
- MCP tool schemas and handlers are generated from or tightly derived
  from the same capability definitions

The intent is not to autogenerate everything blindly. The intent is to
eliminate drift between:

- what the CLI can do
- what the CLI says in `--json`
- what MCP says exists
- what interfaces are taught to call

### 3. Use MCP as interface discovery

Design `patina ai` and future interfaces so they can use MCP
`tools/list` plus capability metadata as the way they learn what Patina
tools are available.

This should support:

- OpenCode/Gemini/Claude learning the available Patina tool surface
- future web/headless agents doing the same
- less custom adapter documentation for tool availability

### 4. Create a universal skill registry

Define canonical Patina skills as structured artifacts with optional
supporting scripts/assets.

A skill should represent:

- behavior/workflow guidance
- patterns and guardrails
- when to use certain capabilities
- optional references/scripts

It should not be the only place where tool truth lives.

Canonical skills should then project into adapter-specific shapes for:

- Claude
- OpenCode
- Gemini

### 5. Thin the adapters

Move adapter layers toward:

- launch
- attach
- render projections
- adapter-specific file/config formats

and away from:

- hand-maintained tool inventories
- unique skill semantics
- hardcoded capability descriptions that drift from Mother/core

### 6. Migrate incrementally

Do not rewrite the whole command tree in one pass.

Start with the surfaces most important to interfaces:

- search/retrieval (`scry`, `context`, `assay`, `mother`)
- spec lifecycle
- session skills/commands
- measurement/health

For those surfaces, `--json` should be treated as a required projection,
not an optional afterthought.

Preserve the trusted existing path while proving the shared capability
model on those areas first.

## Exit Criteria

1. Mother owns a canonical capability registry for interface-facing
   tools and skill bundles, instead of adapter-local command
   inventories.
2. Patina capabilities project into both CLI commands and MCP tool
   definitions from the same source of truth.
3. Agent-relevant CLI capabilities provide stable machine-readable
   `--json` output from the same typed data used by MCP, so interfaces
   and scripts do not need to scrape prose.
4. Interfaces can discover available Patina tools through MCP and use
   that to learn what actions are available without hardcoded
   adapter-specific lists.
5. Skills are modeled as canonical Patina artifacts with adapter-
   specific projections rather than copied one-off prompt files.
6. Claude/OpenCode/Gemini adapter layers are reduced to projection and
   launch concerns rather than owning behavioral truth.
7. Existing CLI, MCP, and adapter workflows remain available during
   migration without breaking the current trusted path.
