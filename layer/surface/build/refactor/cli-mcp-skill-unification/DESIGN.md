# Design: CLI, MCP, and Skill Unification

## Why This Exists

Patina now has a much cleaner runtime and interface architecture than it
did when the adapter system was first built.

The remaining messy area is the overlap between:

- CLI commands
- MCP tools
- adapter-injected skills and slash-command files

The old system worked by pushing MCP config and prompt files into
adapter-specific directories. That got useful tools into Claude/OpenCode
when those interfaces were otherwise hard to reach.

But the result is drift risk:

- CLI knows one thing
- MCP exposes another
- adapter skills describe a third

The new architecture should keep the operator quality of the CLI while
making Mother the authority on capability definitions and letting MCP be
the discovery layer for interfaces.

## Design Position

Patina should be:

- **Mother-first in capability truth**
- **CLI-first in operator experience**

That means:

- the CLI stays strong and ergonomic
- but the CLI is not the only source of truth
- MCP is a projection of canonical capabilities
- skills are workflow/instruction projections, not shadow capability
  definitions

## Key Distinctions

### CLI is not the enemy

This refactor does not replace the CLI with MCP.

The CLI should remain:

- the best human/operator shell
- scriptable
- introspectable
- machine-friendly by default for agent-relevant verbs

That means important commands should expose stable `--json` output.
Agents should not need to scrape prose when the CLI already knows the
typed result.

### MCP is not just a wrapper

MCP should become the interface discovery and invocation surface for
interactive/front-end actors.

Interfaces should be able to ask:

- what tools are available
- what each tool does
- what parameters it takes
- what scope or persona constraints apply

without relying on hand-written adapter docs.

### Skills are not tools

Skills should teach:

- behavior
- workflow
- patterns
- guardrails

Skills should not be the sole place where actual tool capability is
defined.

### Toys are not MCP tools

Child toys are Mother-granted capability bundles for Children.

MCP tools are interface-visible operations for interface actors.

They may touch some of the same underlying runtime services, but they
are different layers and must remain separate.

## Proposed Architecture

### 1. Canonical capability registry

Introduce a registry for interface-facing Patina capabilities.

Each entry should define:

- capability id
- description
- input schema
- output shape
- scope/grant requirements
- projection metadata

This can begin as native Rust structures in a private module.

### 2. Shared handlers behind CLI and MCP

For migrated capabilities:

- CLI command implementations should call shared capability functions
- CLI `--json` output should render the same typed result values rather
  than reparsing human text
- MCP handlers should call the same shared capability functions
- tool descriptions and schemas should come from the capability
  registry, not separately maintained prose where possible

This is the main anti-drift mechanism.

### 3. JSON is the first machine-readable CLI projection

MCP is not the only machine-readable surface.

For operator scripts, local automation, and interfaces that can shell
out before or alongside MCP integration, `--json` is the simplest bridge
to the system.

So each migrated capability should ideally have:

- pretty human CLI output
- stable `--json` output
- MCP tool projection

all backed by the same typed result.

### 4. MCP as interface teaching layer

The capability registry should project into MCP `tools/list`.

Then `patina ai` and future interfaces can rely on MCP for truthful tool
discovery rather than static adapter-specific teaching about available
tools.

This does not mean the interface gets no context files. It means context
files stop carrying the whole tool inventory by hand.

### 5. Universal skill registry

Canonical skills should live as Patina-owned artifacts with:

- id
- title
- summary
- content/instructions
- optional assets/scripts
- applicability metadata

Projection rules render them into:

- Claude skill/command directories
- OpenCode equivalents
- Gemini equivalents

### 6. Adapter projection layer

Adapters remain responsible for:

- file layout differences
- config formats
- launch specifics
- tmux/session attachment behavior

Adapters stop being responsible for:

- deciding which Patina tools exist
- inventing divergent descriptions of capabilities
- owning the only copy of session/spec workflow guidance

## Migration Strategy

Start with the most interface-critical surfaces:

1. retrieval/search tools
2. spec tools
3. session workflow commands
4. measurement/health

For each migrated area:

1. define the capability entry
2. route CLI through shared handler
3. add or stabilize `--json` output
4. route MCP through shared handler
5. update skill projection to reference the capability rather than
   duplicate it

## File Targets

- new capability registry module under `src/interface/` or a nearby
  native Mother-facing module
- `src/mcp/server/tools.rs`
- `src/mcp/server/*`
- `src/commands/*` for migrated domains
- `src/adapters/templates.rs`
- `src/interface/internal/bootstrap.rs`
- adapter-specific context/skill projection code under
  `src/adapters/*` and `src/interface/*`

## What This Enables

If this is done correctly:

- interfaces can discover Patina tools via MCP
- scripts and agents can trust `patina ... --json`
- Patina can keep a CLI-first feel without becoming CLI-fragmented
- skills become lighter and more durable
- future web/headless interfaces can learn the same tool surface
- adapter drift decreases sharply

## Explicit Answer To The Core Question

Yes: under this design, MCP should become a primary way to teach
interfaces about the CLI-capable tools available in the system.

But not by treating MCP as a scrape of the CLI.

Instead:

- Mother/core capability definitions are primary
- CLI and MCP are sibling projections
- interfaces learn tool availability through MCP
- skills teach usage patterns around those tools
