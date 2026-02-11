---
type: feat
id: cli-reorganization
status: abandoned
created: 2026-02-05
updated: 2026-02-05
blocked_by:
- system-introspection
sessions:
  origin: 20260205-084522
related:
- layer/surface/build/explore/cli-commands/SPEC.md
beliefs:
- simplicity-is-architecture
- unix-philosophy
---

# feat: CLI Reorganization

> Flat CLI. Organized code. Self-documenting for LLMs.

## Problem

Patina has grown to 25+ commands. Two distinct problems:

**1. Mental model is unclear:**
- Hard to see what's "core Patina" vs peripheral
- Related commands scattered in code
- New contributors don't know where things live

**2. LLMs can't understand the system:**
- No structured way to see what a command does
- No way to see what reads/writes what
- Code doesn't self-document its role

**The old approach (CLI namespaces) was wrong.** We don't need `patina science eval` — we need organized code that an LLM can read and understand.

---

## Key Insight

> "Most commands can stay top-level. It's how we organize the CODE that matters. An LLM needs to see the command and understand."

**Three concerns:**
1. **CLI UX** — Keep it flat, simple
2. **Code organization** — Group related code together
3. **LLM comprehension** — Data contracts make code self-documenting

---

## Mental Model

Commands grouped by concern (for code organization, not CLI):

```
┌─────────────────────────────────────────────────────────────┐
│  CORE — The capture → index → query → learn loop            │
│                                                             │
│  scrape, oxidize, scry, context, assay, session,           │
│  belief, persona                                            │
└─────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────┐
│  SCIENCE — Is it working? How well?                         │
│                                                             │
│  eval, bench, compare (NEW), feedback, config (NEW)         │
└─────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────┐
│  DEV — How is Patina built? (for contributors)              │
│                                                             │
│  introspect (NEW), doctor, report, contracts (NEW)          │
└─────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────┐
│  INFRA — Infrastructure setup and management                │
│                                                             │
│  init, adapter, model, mother, repo, secrets,               │
│  rebuild, upgrade, version, spec, yolo                      │
└─────────────────────────────────────────────────────────────┘
```

**All commands stay top-level in CLI.** The grouping is for code organization.

---

## Design

### 1. CLI Stays Flat

**No namespaces.** All commands at top level:

```bash
patina eval           # NOT patina science eval
patina doctor         # NOT patina dev doctor
patina init           # NOT patina infra init
patina introspect     # NEW, top-level
patina compare        # NEW, top-level
```

**Why flat?**
- Simpler to type
- Easier to discover
- No cognitive load of "which namespace?"
- Unix philosophy: simple tools

### 2. Code Organized by Group

```
src/commands/
├── mod.rs                    # Routes all commands
│
├── # CORE (capture → index → query → learn)
├── core/
│   ├── mod.rs               # Group documentation
│   ├── scrape/              # Capture
│   ├── oxidize.rs           # Index
│   ├── scry/                # Query
│   ├── context.rs           # Query
│   ├── assay/               # Query
│   ├── session/             # Learn
│   ├── belief/              # Learn
│   └── persona/             # Learn
│
├── # SCIENCE (measurement, evaluation)
├── science/
│   ├── mod.rs               # Group documentation
│   ├── eval.rs
│   ├── bench.rs
│   ├── compare.rs           # NEW
│   ├── feedback.rs          # Extracted from eval
│   └── config.rs            # NEW
│
├── # DEV (introspection, for contributors)
├── dev/
│   ├── mod.rs               # Group documentation
│   ├── introspect.rs        # NEW
│   ├── doctor.rs
│   ├── report.rs
│   └── contracts.rs         # NEW
│
└── # INFRA (setup, management)
    └── infra/
        ├── mod.rs           # Group documentation
        ├── init.rs
        ├── adapter/
        ├── model/
        ├── mother/
        ├── repo/
        ├── secrets.rs
        ├── rebuild.rs
        ├── upgrade.rs
        ├── version.rs
        ├── spec.rs
        └── yolo.rs
```

**Benefits:**
- Related code lives together
- Easy to navigate for contributors
- Group `mod.rs` documents the category
- LLM sees logical structure

### 3. Data Contracts for LLM Comprehension

Every command has a data contract that an LLM can read:

```rust
//! # eval — Evaluate retrieval quality
//!
//! **Group:** Science (measurement, evaluation, experiments)
//! **Related:** bench, compare, feedback, config
//!
//! ## What it does
//! Runs quality evaluation across retrieval dimensions (semantic, temporal).
//!
//! ## Data Contract
//! - Reads: beliefs table, code_fts, semantic.usearch
//! - Writes: nothing (read-only)
//! - Write Path: ReadOnly
//!
//! ## Usage
//! ```
//! patina eval                       # All dimensions
//! patina eval --dimension semantic  # Specific dimension
//! patina eval --feedback            # Real-world precision
//! ```

use crate::introspection::{DataContract, Source, WritePath, CommandGroup};

pub const DATA_CONTRACT: DataContract = DataContract {
    command: "eval",
    group: CommandGroup::Science,
    description: "Evaluate retrieval quality (MRR, precision)",
    related: &["bench", "compare", "feedback", "config"],
    reads: &[
        Source::Table("beliefs"),
        Source::Table("code_fts"),
        Source::Usearch("semantic.usearch"),
    ],
    writes: &[],
    write_path: WritePath::ReadOnly,
};

pub fn run(args: EvalArgs) -> Result<()> {
    // ...
}
```

**What LLM sees:**
- Group (Science)
- Related commands
- What it reads/writes
- Usage examples

### 4. Group Documentation

Each group's `mod.rs` documents the category:

```rust
//! # Science Commands
//!
//! Measurement, evaluation, and experiments.
//!
//! ## Commands
//! - `eval` — Quality metrics (MRR, precision)
//! - `bench` — Ground truth benchmarking
//! - `compare` — A/B config comparison
//! - `feedback` — Real-world precision from sessions
//! - `config` — Manage experiment configurations
//!
//! ## Mental Model
//! These commands answer: "Is Patina working well?"
//!
//! ## Data Flow
//! ```
//! patina.db + vectors → eval/bench → metrics
//! config A + config B → compare → delta
//! session feedback → feedback → precision
//! ```

pub mod eval;
pub mod bench;
pub mod compare;
pub mod feedback;
pub mod config;
```

---

## Help Screen

With flat CLI but organized code:

```
$ patina --help

Patina - Context management for AI-assisted development

CORE:
  scrape     Capture knowledge from code, git, layer, forge
  oxidize    Build embeddings and projections
  scry       Search codebase knowledge
  context    Get project patterns and conventions
  assay      Query codebase structure
  session    Track development sessions
  belief     Manage epistemic beliefs
  persona    Cross-project user knowledge

SCIENCE:
  eval       Evaluate retrieval quality
  bench      Benchmark with ground truth
  compare    Compare configurations (A/B)
  feedback   Real-world precision from sessions

DEV:
  introspect Query data flows and contracts
  doctor     Health checks
  report     System state dump

INFRA:
  init       Initialize project
  adapter    Manage LLM adapters
  model      Manage embedding models
  mother     Daemon management
  repo       External repositories
  ...

Run 'patina <command> --help' for details.
```

**Note:** Help groups commands by category, but all are invoked directly (`patina eval`, not `patina science eval`).

---

## Migration

### Phase 1: Reorganize Code

Move files into group directories without changing CLI:

```
# Before
src/commands/eval/mod.rs
src/commands/doctor.rs

# After
src/commands/science/eval.rs
src/commands/dev/doctor.rs
```

**CLI unchanged.** `patina eval` still works.

### Phase 2: Add Data Contracts

Add `DATA_CONTRACT` to each command. Enables:
- `patina introspect <command>` works
- LLMs can read contracts

### Phase 3: Add New Commands

- `patina compare` (A/B testing)
- `patina introspect` (data flows)
- `patina feedback` (extracted from eval)

### Phase 4: Update Help

Group commands in help output by category.

---

## Command Groups

### Core (8 commands)

| Command | Phase | Purpose |
|---------|-------|---------|
| `scrape` | CAPTURE | Gather knowledge from sources |
| `oxidize` | INDEX | Build embeddings |
| `scry` | QUERY | Search codebase |
| `context` | QUERY | Get patterns |
| `assay` | QUERY | Query structure |
| `session` | LEARN | Track work |
| `belief` | LEARN | Manage beliefs |
| `persona` | LEARN | Cross-project knowledge |

### Science (5 commands)

| Command | Status | Purpose |
|---------|--------|---------|
| `eval` | EXISTS | Quality metrics |
| `bench` | EXISTS | Ground truth testing |
| `compare` | **NEW** | A/B comparison |
| `feedback` | EXTRACT | Real-world precision |
| `config` | **NEW** | Experiment configs |

### Dev (4 commands)

| Command | Status | Purpose |
|---------|--------|---------|
| `introspect` | **NEW** | Data flows, contracts |
| `doctor` | EXISTS | Health checks |
| `report` | EXISTS | State dump |
| `contracts` | **NEW** | List all contracts |

### Infra (11 commands)

| Command | Purpose |
|---------|---------|
| `init` | Initialize project |
| `adapter` | LLM adapters |
| `model` | Embedding models |
| `mother` | Daemon |
| `repo` | External repos |
| `secrets` | Secret management |
| `rebuild` | Rebuild from sources |
| `upgrade` | CLI updates |
| `version` | Semver |
| `spec` | Spec lifecycle |
| `yolo` | Devcontainer |

---

## Exit Criteria

### v0.12.0: Code Structure + CommandGroup

- [ ] `CommandGroup` enum defined (`Core`, `Science`, `Dev`, `Infra`)
- [ ] Commands reorganized into `core/`, `science/`, `dev/`, `infra/`
- [ ] Each group has `mod.rs` with documentation
- [ ] CLI unchanged (all commands still top-level)

### v0.13.0: Contract Integration + Help

- [ ] `DataContract` extended with `group: CommandGroup` and `related: &[&str]`
- [ ] 80%+ of commands have declared contracts (parallel with system-introspection)
- [ ] `patina introspect <command>` works (delivered by system-introspection)
- [ ] Help screen groups commands by category
- [ ] Group `mod.rs` files document data flows

### v0.14.0: New Commands

- [ ] `patina compare` — A/B config comparison (uses ExperimentConfig from system-introspection)
- [ ] `patina feedback` — extracted from eval
- [ ] `patina contracts` — list all contracts (or `patina introspect --contracts`)

---

## Open Questions

1. **Rename `science`?**
   - Alternatives: `measure`, `quality`, `eval` (as group name)
   - `science` implies experimentation, rigor

2. **Top-level `help` grouping?**
   - Show groups in help (current design)
   - Or flat alphabetical list?

3. **Should `contracts` be a subcommand of `introspect`?**
   - `patina introspect contracts` vs `patina contracts`

---

## Relationship to Other Specs

- **system-introspection**: Defines `DataContract` schema, `patina introspect`
- **scrape-layer-unify**: Affects `scrape` (core command)
- **mother-v2**: Affects `mother` (infra command)
- **explore/cli-commands**: Documents what each command does (graduates to reference after alignment)

### Ownership Boundaries (aligned 2026-02-05)

| Concern | Owner | This Spec's Role |
|---------|-------|------------------|
| `DataContract` schema | system-introspection | Extends with `CommandGroup`, `related` |
| `CommandGroup` enum | **this spec** | Defines core, science, dev, infra |
| Code file structure | **this spec** | `src/commands/{core,science,dev,infra}/` |
| `introspect` command design | system-introspection | Places in `dev/` group |
| `config`, `compare`, `feedback` commands | **this spec** | Defines as new commands, uses experiment infra from system-introspection |
| Help screen grouping | **this spec** | Groups commands by category in help output |

**Note:** `DataContract` core schema (Source, Sink, WritePath) defined by system-introspection. This spec adds `group: CommandGroup` and `related: &[&str]` fields.

---

## Status Log

| Date | Status | Note |
|------|--------|------|
| 2026-02-05 | design | Created with CLI namespaces (science, dev, infra) |
| 2026-02-05 | design | **Revised:** Flat CLI, organized code. Namespaces become code organization, not CLI. Key insight: LLMs need to see command + code and understand. Data contracts are the bridge. |
| 2026-02-05 | design | **Spec alignment:** This spec owns CommandGroup and code structure. system-introspection owns DataContract schema. Version targets aligned: v0.12=structure+CommandGroup, v0.13=contract integration, v0.14=new commands. |
