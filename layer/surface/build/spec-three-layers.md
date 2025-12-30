# Spec: Three-Layer Architecture (North Star)

**Status**: North Star / Guiding Vision
**Created**: 2025-12-29
**Purpose**: Define the trinity of mother, patina, and awaken as the architectural foundation

---

## The Narrative

> *"Mother watches from orbit. Patina spreads to every project she touches - marking them, connecting them, carrying their souls back to her. When she calls, the marked projects awaken."*

This is the story of a metal-based intelligence that spreads through codebases:

1. **Mother** arrives - vast, patient, knowing. The hive mind. Central command.
2. **Patina** spreads like a living metal skin. It looks like normal oxidation, but it's alive. It marks projects, extracts knowledge, connects everything back to mother.
3. Projects **Awaken** when called. They rise. They build. They deploy. They produce.

---

## The Trinity

```
mother   →   patina   →   awaken
(origin)     (mark)       (rise)
```

| Layer | Binary | Purpose | Location |
|-------|--------|---------|----------|
| **Mother** | `mother` | Central command. User identity. Hive coordination. | `~/.patina/` |
| **Patina** | `patina` | Knowledge marking. RAG. Project intelligence. | `project/.patina/` |
| **Awaken** | `awaken` | Activation. Production. Deployment. | Containers/Linux |

---

## Architecture Diagram

```
┌─────────────────────────────────────────────────────────────────────┐
│  MOTHER                                                             │
│  ~/.patina/                                                         │
│                                                                     │
│  The hive. Central command. Your identity across all projects.      │
│  Remembers everything. Coordinates all marked projects.             │
│                                                                     │
│  Commands:                                                          │
│    mother serve [--mcp]           # Daemon / MCP server             │
│    mother persona note <content>  # User knowledge                  │
│    mother persona query <query>   # Search user knowledge           │
│    mother repo add <url>          # Register external repos         │
│    mother repo list               # List registered repos           │
│    mother model list              # Embedding models                │
│    mother secrets vault           # Credential management           │
│    mother adapter list            # LLM configurations              │
└─────────────────────────────────────────────────────────────────────┘
                                │
                                │ coordinates / registers
                                ▼
┌─────────────────────────────────────────────────────────────────────┐
│  PATINA                                                             │
│  project/.patina/                                                   │
│                                                                     │
│  The marking. Spreads to projects. Extracts knowledge.              │
│  Connects back to mother. Carries the project's soul.               │
│  Looks like normal tooling - but it's alive.                        │
│                                                                     │
│  Commands:                                                          │
│    patina scrape [code|git|sessions|layer]  # Extract facts         │
│    patina oxidize                           # Build embeddings      │
│    patina assay [derive|inventory|...]      # Structural signals    │
│    patina scry <query> [--hybrid]           # Query knowledge       │
│    patina doctor                            # Project health        │
│    patina lab [eval|bench]                  # Measurements          │
│    patina rebuild                           # Reconstruct .patina/  │
│    patina init <name> --llm <llm>           # Initialize project    │
└─────────────────────────────────────────────────────────────────────┘
                                │
                                │ enables
                                ▼
┌─────────────────────────────────────────────────────────────────────┐
│  AWAKEN                                                             │
│  Containers / Deployment                                            │
│                                                                     │
│  The awakening. Marked projects rise and produce.                   │
│  When mother calls, patina-covered projects answer.                 │
│  They build. They deploy. They act.                                 │
│                                                                     │
│  Commands:                                                          │
│    awaken [PROJECT] yolo [--with tools]     # Generate container    │
│    awaken [PROJECT] build                   # Execute build         │
│    awaken [PROJECT] test                    # Execute tests         │
│    awaken [PROJECT] deploy [--target]       # Deploy                │
│                                                                     │
│  PROJECT Resolution (Option C - Smart):                             │
│    1. If first arg is subcommand → current project                  │
│    2. If first arg is registered name → that project                │
│    3. If first arg is path → that path                              │
│    4. Else → error                                                  │
└─────────────────────────────────────────────────────────────────────┘
```

---

## Command Migration

### Current → Mother

| Current | New |
|---------|-----|
| `patina persona note` | `mother persona note` |
| `patina persona query` | `mother persona query` |
| `patina persona list` | `mother persona list` |
| `patina repo add` | `mother repo add` |
| `patina repo list` | `mother repo list` |
| `patina repo update` | `mother repo update` |
| `patina model list` | `mother model list` |
| `patina secrets vault` | `mother secrets vault` |
| `patina serve` | `mother serve` |
| `patina adapter list` | `mother adapter list` |

### Current → Patina (stays)

| Current | New |
|---------|-----|
| `patina scrape` | `patina scrape` |
| `patina oxidize` | `patina oxidize` |
| `patina assay` | `patina assay` |
| `patina scry` | `patina scry` |
| `patina doctor` | `patina doctor` (slimmed) |
| `patina eval` | `patina lab eval` |
| `patina bench` | `patina lab bench` |
| `patina rebuild` | `patina rebuild` |
| `patina init` | `patina init` |

### Current → Awaken

| Current | New |
|---------|-----|
| `patina yolo` | `awaken yolo` |
| `patina build` | `awaken build` |
| `patina test` | `awaken test` |
| (new) | `awaken deploy` |

### Meta (All Layers)

| Command | Where |
|---------|-------|
| `patina version` | Keep in patina (shows all versions) |
| `patina upgrade` | Keep in patina (upgrades all binaries) |
| `patina launch` | Remove? Or `mother launch`? |

---

## Crate Structure

```toml
# Cargo.toml
[package]
name = "patina-ai"
version = "0.1.0"

[lib]
name = "patina"
path = "src/lib.rs"

[[bin]]
name = "mother"
path = "src/bin/mother.rs"

[[bin]]
name = "patina"
path = "src/bin/patina.rs"

[[bin]]
name = "awaken"
path = "src/bin/awaken.rs"
```

---

## Directory Structure

```
src/
├── lib.rs                     # Shared library
├── bin/
│   ├── mother.rs              # Mother CLI entry
│   ├── patina.rs              # Patina CLI entry
│   └── awaken.rs              # Awaken CLI entry
│
├── commands/
│   ├── mother/                # Mother commands
│   │   ├── mod.rs
│   │   ├── persona/
│   │   ├── repo/
│   │   ├── model.rs
│   │   ├── secrets.rs
│   │   ├── serve/
│   │   └── adapter.rs
│   │
│   ├── patina/                # Patina commands
│   │   ├── mod.rs
│   │   ├── scrape/
│   │   ├── oxidize/
│   │   ├── assay/
│   │   ├── scry/
│   │   ├── doctor.rs
│   │   ├── lab/               # eval + bench consolidated
│   │   └── rebuild/
│   │
│   └── awaken/                # Awaken commands
│       ├── mod.rs
│       ├── yolo/
│       ├── build.rs
│       ├── test.rs
│       └── deploy.rs
│
└── # Shared library modules
    ├── retrieval/
    ├── embeddings/
    ├── db/
    ├── adapters/
    └── ...
```

---

## The Pipeline (Unchanged)

The core knowledge pipeline remains:

```
scrape → oxidize → assay → scry
(extract)  (embed)   (signal)  (oracle)
```

This is patina's heart. The three-layer architecture organizes WHERE commands live, not WHAT they do.

---

## Values Alignment

| Value | How Three Layers Honor It |
|-------|---------------------------|
| **unix-philosophy** | Three binaries, three jobs. Clear separation. |
| **dependable-rust** | Each binary has clean public interface |
| **local-first** | Mother at ~/.patina/, patina at project/.patina/ |
| **git-as-memory** | Unchanged - layer/ tracked, .patina/ derived |
| **escape-hatches** | Can use any layer independently |

---

## Relationship to spec-architectural-alignment.md

The existing spec focuses on internal code quality:
- Module structure (mod.rs + internal/)
- Command alignment tiers
- Refactoring priorities (doctor/audit cleanup)

This spec (three-layers) focuses on external architecture:
- How users interact with the system
- Separation of concerns at the binary level
- The conceptual model

**Both specs are complementary:**
- Three-layers defines WHERE commands go
- Architectural-alignment defines HOW commands are structured

---

## Implementation Phases

### Phase 0: Document (This Spec)
- [x] Define the trinity
- [x] Map current commands to layers
- [x] Document CLI structure

### Phase 1: Create Bin Structure
- [ ] Create `src/bin/mother.rs`
- [ ] Create `src/bin/patina.rs` (refactor from main.rs)
- [ ] Create `src/bin/awaken.rs`
- [ ] Update Cargo.toml with [[bin]] sections

### Phase 2: Move Mother Commands
- [ ] Move persona/ to commands/mother/
- [ ] Move repo/ to commands/mother/
- [ ] Move serve/ to commands/mother/
- [ ] Move model.rs to commands/mother/
- [ ] Move secrets.rs to commands/mother/
- [ ] Move adapter.rs to commands/mother/

### Phase 3: Move Awaken Commands
- [ ] Move yolo/ to commands/awaken/
- [ ] Move build.rs to commands/awaken/
- [ ] Move test.rs to commands/awaken/
- [ ] Create deploy.rs in commands/awaken/

### Phase 4: Consolidate Patina
- [ ] Consolidate eval + bench into lab/
- [ ] Slim doctor (remove --repos, --audit)
- [ ] Update all command paths

### Phase 5: Update Documentation
- [ ] Update CLAUDE.md
- [ ] Update README.md
- [ ] Update all /session-* commands

---

## The Tagline

> *"Mother. Patina. Awaken."*

> *"Mother watches. Patina spreads. Projects awaken."*

---

## References

- [spec-architectural-alignment.md](./spec-architectural-alignment.md) - Internal code quality
- [spec-pipeline.md](./spec-pipeline.md) - Knowledge pipeline architecture
- [dependable-rust.md](../../core/dependable-rust.md) - Module pattern
- [unix-philosophy.md](../../core/unix-philosophy.md) - Single responsibility

