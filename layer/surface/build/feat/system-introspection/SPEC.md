---
type: feat
id: system-introspection
status: design
created: 2026-02-05
updated: 2026-02-05
sessions:
  origin: 20260205-064001
  updated: 20260205-084522
blocked_by: []
blocks:
  - cli-reorganization
related:
  - layer/surface/reports/data-flow-cheatsheet.md
  - layer/surface/build/feat/mother-v2/SPEC.md
beliefs:
  - measure-first
  - measure-the-measurement
  - simplicity-is-architecture
references:
  - "Jerry Nixon - Modern Architecture 101 (NDC Copenhagen 2025)"
---

# feat: System Introspection

> Know what you're building. Trace any data from source to reader. Understand before changing.

## Philosophy: Argue Every Box

> "Once you understand it enough to understand why you DON'T want it, then you finally have enough why you would actually want it." — Jerry Nixon

**Simplicity is the best architecture.** The architect's job is to be the guardian of what stays OUT of the solution. Every component should be arguable — you should be able to make a case FOR and AGAINST it.

**Defer decisions.** The longer you can put things off, the cheaper they are to change when you realize you made a mistake. Less is always more.

**But some things can't wait.** In the AI/agentic landscape, the ground is shifting:
- Embedding models change (e5-base-v2 → nomic-embed → ???)
- LLM adapters change (Claude Code → OpenCode → Gemini CLI)
- Interface behavior differs across tools
- Local vs cloud models have different characteristics

A/B testing isn't a luxury — it's survival infrastructure when your entire stack can change overnight.

---

## Problem

Patina has grown to:
- 25+ commands
- 5 oracles
- 35,000+ eventlog entries
- 50+ database tables
- Multiple storage layers (project, user, mother)
- Multiple LLM adapters (Claude, Gemini, OpenCode)

**We are losing the mental ability to understand what we're building.**

Symptoms:
- "Where does this data come from?" requires code archaeology
- "What will break if I change X?" is guesswork
- "Why isn't Y showing up in scry?" means tracing through 5 files
- New features get "hacked in" because the flow isn't clear
- The cheatsheet exists but it's static — the code can drift
- "Did the new model improve retrieval?" — no way to compare
- "How does Claude Code behave vs OpenCode?" — no visibility

**Two distinct problems:**

1. **Blueprint** — understanding what exists and how it connects (static)
2. **Experiments** — comparing alternatives in a rapidly changing landscape (comparative)

**The meta-problem:** We measure retrieval quality (MRR, precision) but not system coherence. Andrew Ng's principle applies to architecture too: if you can't measure it, you can't improve it.

---

## Vision

A command that answers:

```bash
# What does scry touch?
patina introspect scry
  Reads:
    - .patina/local/data/patina.db (beliefs, commits_fts, co_changes, ...)
    - .patina/local/data/embeddings/.../semantic.usearch
    - ~/.patina/cache/personas/default/persona.db
  Writes:
    - .patina/local/data/patina.db (eventlog: scry.query, scry.use, scry.feedback)
  Write Path: action-time
  Oracles: semantic, lexical, temporal, persona, belief

# What writes to the beliefs table?
patina introspect --table beliefs
  Writers:
    - patina scrape layer (eventlog: belief.surface)
  Readers:
    - patina scry (BeliefOracle, belief_fts)
    - patina context (aggregate stats, topic-ranked via BeliefOracle)
    - patina belief audit
  Source: layer/surface/epistemic/beliefs/*.md

# What's the full flow from belief files to scry results?
patina introspect --trace "layer/surface/epistemic/beliefs/*.md"
  1. SOURCE: layer/surface/epistemic/beliefs/*.md (59 files)
  2. SCRAPE: patina scrape layer
     → eventlog: belief.surface (47 events)
     → materialized: beliefs table (47 rows)
     → materialized: belief_fts (FTS5 index)
  3. OXIDIZE: patina oxidize
     → embedded: semantic.usearch (IDs 4B-5B range)
  4. QUERY: patina scry "topic"
     → BeliefOracle reads: semantic.usearch + beliefs table
     → RRF fusion with other oracles
     → result returned

# AGGREGATE VIEWS: Answer "what are ALL the X?"

# What are ALL the raw data sources?
patina introspect --sources
  Files:
    - src/**/* (scrape code)
    - .git/ (scrape git)
    - layer/sessions/*.md (scrape sessions)
    - layer/core/*.md (scrape layer, context)
    - layer/surface/*.md (scrape layer, context)
    - layer/surface/epistemic/beliefs/*.md (scrape layer)
  APIs:
    - GitHub API (scrape forge)
  User Input:
    - stdin/args (persona note)
    - git status/log (session start/update/end)

# Where do we store ALL of this?
patina introspect --sinks
  Project (.patina/local/):
    - data/patina.db (eventlog, tables, FTS)
    - data/embeddings/.../semantic.usearch (vectors)
    - active-session.md (current session)
  User (~/.patina/):
    - personas/default/events/*.jsonl (persona source)
    - cache/personas/default/persona.db (persona materialized)
    - mother/graph.db (cross-project routing)
  Layer (git-tracked):
    - layer/sessions/*.md (session archives)
    - layer/surface/epistemic/beliefs/*.md (belief source)

# What are the different write paths?
patina introspect --write-paths
  scrape (batch import):
    - scrape code → eventlog → tables
    - scrape git → eventlog → tables
    - scrape sessions → eventlog → tables
    - scrape layer → eventlog → tables
    - scrape forge → eventlog → tables
  action-time (user acts):
    - session start/update/end → eventlog + markdown
    - scry → eventlog (query, use, feedback)
    - persona note → jsonl
  dual-write (eventlog + source file):
    - session end → eventlog + layer/sessions/*.md
    - belief create → markdown (source of truth)
```

---

## Design Principles

### 1. Introspection from Code, Not Docs

The cheatsheet is useful but static. The introspection command should:
- Read actual database schemas
- Check actual file existence
- Count actual rows/events
- Be generated, not written

### 2. Declare Data Contracts in Code

Each command should declare what it reads/writes:

```rust
// In src/commands/scrape/layer/mod.rs
pub const DATA_CONTRACT: DataContract = DataContract {
    reads: &[
        Source::Files("layer/surface/epistemic/beliefs/*.md"),
        Source::Files("layer/core/*.md"),
        Source::Files("layer/surface/*.md"),
    ],
    writes: &[
        Sink::Table("beliefs"),
        Sink::Table("patterns"),
        Sink::Eventlog("belief.surface"),
        Sink::Eventlog("pattern.surface"),
        Sink::Fts("belief_fts"),
        Sink::Fts("pattern_fts"),
    ],
};
```

Benefits:
- Introspection reads contracts, not heuristics
- Contracts are checked against reality (doctor)
- Refactoring updates contracts explicitly
- New commands must declare their footprint

### 3. Layered Understanding

```
Level 0: FILE EXISTENCE
  "Does patina.db exist? Does semantic.usearch exist?"
  → patina doctor (existing)

Level 1: SCHEMA AWARENESS
  "What tables exist? What columns? What indexes?"
  → patina introspect --schema

Level 2: DATA FLOW
  "What command writes to this table? What reads it?"
  → patina introspect --flow <table|file>

Level 3: TRACE
  "Follow data from source to final reader"
  → patina introspect --trace <path>

Level 4: IMPACT
  "If I change X, what might break?"
  → patina introspect --impact <path|table>
```

### 4. Write Path Taxonomy

Not all writes are equal. Understanding HOW data enters the system is as important as WHERE it goes:

```
SCRAPE (batch import)
  Pattern: external source → eventlog → materialized tables
  Commands: scrape code, scrape git, scrape sessions, scrape layer, scrape forge
  Characteristic: Can re-run, idempotent, incremental via scrape_meta

ACTION-TIME (user acts)
  Pattern: user action → eventlog (+ optional side effect)
  Commands: session start/update/end, scry, persona note
  Characteristic: Happens during use, not batch

DUAL-WRITE (eventlog + source file)
  Pattern: action → both eventlog AND markdown/yaml file
  Commands: session end (archive), belief create
  Characteristic: Source file is truth, eventlog is index

READ-ONLY
  Pattern: query existing data, no writes
  Commands: scry (reads), assay, context
  Note: scry writes to eventlog for tracking, but that's metadata not data
```

**Why this matters:** When debugging "where did X come from?", knowing the write path tells you:
- Scrape → check source files + scrape_meta checkpoint
- Action-time → check eventlog for the action
- Dual-write → check both markdown AND eventlog

### 5. Measure System Coherence

Like retrieval quality metrics, we need architecture metrics:

| Metric | Meaning | Target |
|--------|---------|--------|
| **Contract coverage** | % of commands with declared contracts | 100% |
| **Contract accuracy** | % of declared writes that actually happen | 100% |
| **Orphan tables** | Tables written but never read | 0 |
| **Orphan events** | Event types written but never consumed | 0 |
| **Flow completeness** | % of sources that reach at least one reader | 100% |

---

## Implementation

### The Argue-Every-Box Test

Before adding any component, we must be able to argue FOR and AGAINST it:

| Component | FOR | AGAINST | Verdict |
|-----------|-----|---------|---------|
| **Data Contracts** | Know what you're changing before you change it | Maintenance overhead | **Essential** — foundation |
| **Introspect Command** | Query the blueprint, generate docs | Static, not runtime | **Essential** — developer UX |
| **Experiment Infrastructure** | Compare models, adapters in volatile landscape | Another system | **Essential** — survival |
| **Contract Verification** | Catch drift automatically | CI complexity | **Nice to have** |
| **OTEL Tracing** | Debug production, timing analysis | No prod users yet, complexity | **Defer** |

---

### Phase 1: Data Contracts (Blueprint Foundation)

Add `DataContract` struct and declarations to each command.

**Files:**
- `src/introspection/mod.rs` — contract types
- `src/commands/*/mod.rs` — contract declarations

**Contract schema:**
```rust
pub struct DataContract {
    pub command: &'static str,
    pub description: &'static str,
    pub reads: &'static [Source],
    pub writes: &'static [Sink],
    pub write_path: WritePath,  // Categorize how this command writes
}

pub enum Source {
    Files(&'static str),      // glob pattern
    Table(&'static str),      // SQLite table
    Fts(&'static str),        // FTS5 virtual table
    Eventlog(&'static str),   // event_type prefix
    Usearch(&'static str),    // usearch index
    ExternalDb(&'static str), // e.g., persona.db
    Api(&'static str),        // external API (e.g., GitHub)
    UserInput,                // stdin, args, interactive
}

pub enum Sink {
    Table(&'static str),
    Fts(&'static str),
    Eventlog(&'static str),
    Usearch(&'static str),
    ExternalDb(&'static str),
    Files(&'static str),
    Jsonl(&'static str),      // append-only jsonl (persona)
}

/// Write path taxonomy — how does this command capture data?
pub enum WritePath {
    /// Batch import of external data (scrape commands)
    /// Pattern: read source → eventlog → materialized tables
    Scrape,

    /// Events written as user acts (session, scry, persona note)
    /// Pattern: action → eventlog (+ optional side effect)
    ActionTime,

    /// Write to both eventlog AND source file
    /// Pattern: action → eventlog + markdown file
    DualWrite,

    /// Read-only command (scry, assay, context)
    ReadOnly,
}
```

**Exit:** Every command in `src/commands/` has a `DATA_CONTRACT` constant.

### Phase 2: Introspect Command (Query the Blueprint)

Add `patina introspect` command.

**Subcommands:**
```bash
# Per-command queries
patina introspect <command>           # Show what a command reads/writes
patina introspect --table <name>      # Show readers/writers for a table
patina introspect --event <type>      # Show writers/consumers for event type

# Aggregate views (answer "what are ALL the X?")
patina introspect --sources           # ALL raw data sources across all commands
patina introspect --sinks             # ALL storage locations across all commands
patina introspect --write-paths       # Categorize: scrape | action-time | dual-write

# Schema and analysis
patina introspect --schema            # Dump full schema awareness
patina introspect --trace <path>      # Trace data flow from source
patina introspect --orphans           # Find orphan tables/events
```

**Key aggregate questions this answers:**
- "What are ALL the methods we read raw data?" → `--sources`
- "Where do we store ALL of this?" → `--sinks`
- "Is scrape our only way to capture data?" → `--write-paths`

**Exit:** `patina introspect scry` shows accurate reads/writes/oracles.

### Phase 3: Experiment Infrastructure (A/B Testing)

**Why this can't be deferred:** The AI/agentic landscape is in rapid flux. We need to compare:
- **Embedding models:** e5-base-v2 vs nomic-embed vs future models
- **LLM adapters:** Claude Code vs OpenCode vs Gemini CLI behavior
- **Oracle configurations:** belief weight 0.3 vs 0.4, different fusion strategies
- **Local vs cloud:** same adapter, different model providers

**What we need:**
```bash
# Define a configuration
patina config create my-experiment \
  --model e5-base-v2 \
  --belief-weight 0.4 \
  --adapter claude

# Run eval against a config
patina eval --config my-experiment

# Compare configs
patina eval --compare baseline my-experiment
  baseline:     MRR 0.72, P@5 0.65
  my-experiment: MRR 0.74, P@5 0.68
  delta:        +0.02 MRR, +0.03 P@5

# Track which config produced which session
patina session start "test feature" --config my-experiment
```

**Configuration dimensions:**
```rust
pub struct ExperimentConfig {
    pub name: String,
    pub embedding_model: String,      // e5-base-v2, nomic-embed-text
    pub oracle_weights: OracleWeights,
    pub adapter: String,              // claude, gemini, opencode
    pub model_provider: Option<String>, // local, anthropic, openai, google
}
```

**Key insight:** The adapter (Claude Code, OpenCode) often brings its own model. We need to track:
1. Which adapter is being used (the UI/interface)
2. Which model is actually responding (may be adapter-determined or user-configured)

**Exit:**
- `patina config list` shows available configurations
- `patina eval --config X` runs eval with specific config
- `patina eval --compare A B` shows delta between configs
- Sessions record which config was active

### Phase 4: Contract Verification

Extend `patina doctor` to verify contracts against reality.

**Checks:**
- Declared tables exist in schema
- Declared event types appear in eventlog
- Declared file patterns match actual files
- No writes to undeclared sinks

**Exit:** `patina doctor` reports contract violations.

### Phase 5: Impact Analysis

Add `--impact` mode for change planning.

```bash
patina introspect --impact beliefs
  Direct readers:
    - scry (BeliefOracle)
    - context (aggregate stats)
    - belief audit
  Transitive impact:
    - MCP scry tool (calls scry)
    - MCP context tool (calls context)
  Suggested tests:
    - patina scry "test query" --explain
    - patina context --topic "test"
    - patina belief audit
```

**Exit:** `patina introspect --impact X` shows what to test after changing X.

### Phase 6: Coherence Metrics

Add metrics to `patina doctor` or `patina eval`.

```bash
patina doctor --coherence
  Contract coverage: 23/25 commands (92%)
  Contract accuracy: 100%
  Orphan tables: 1 (navigation — legacy)
  Orphan events: 0
  Flow completeness: 100%
```

**Exit:** Coherence metrics visible and tracked over time.

---

## Deferred: Runtime Observability (OTEL)

**Why defer:** "Not going to help during development. Only going to help after you've pushed out and trying to debug while your boss is breathing down your neck." — Jerry Nixon

**When to revisit:** When Patina has production users and we need to debug:
- Why is scry slow for user X?
- Which oracle is the bottleneck?
- What's the P99 latency for context?

**What it would add:**
- Structured spans (start/end timing per operation)
- Metrics (latency percentiles, throughput)
- Distributed traces (causality across commands)
- Export to Jaeger/Honeycomb/etc.

**Current alternative:** Eventlog already captures business events (`scry.query`, `scry.use`). For timing, we can add simple duration fields to eventlog events without full OTEL complexity.

**The defer test:** Can we argue AGAINST adding OTEL now? Yes — no prod users, adds complexity, existing eventlog covers most needs. Revisit when production debugging becomes the bottleneck.

---

## Non-Goals (For Now)

- **Visual diagrams** — text output is sufficient, diagrams can be generated externally
- **Runtime tracing (OTEL)** — deferred until production users exist
- **Automatic contract generation** — contracts are declared, not inferred
- **MLflow-style experiment tracking** — lightweight config comparison is enough

---

## Exit Criteria

### v0.12.0: DataContract Foundation

- [ ] `DataContract` type exists in `src/introspection/` with `Source`, `Sink`, `WritePath` enums
- [ ] Schema supports extension (cli-reorganization adds `CommandGroup`)
- [ ] Scrape commands have declared contracts (scrape code, git, layer, forge)

### v0.13.0: Introspect Command

- [ ] `patina introspect <command>` works for declared commands
- [ ] `patina introspect --table <name>` shows readers/writers
- [ ] `patina introspect --sources` shows ALL raw data sources
- [ ] `patina introspect --sinks` shows ALL storage locations
- [ ] `patina introspect --write-paths` categorizes scrape vs action-time vs dual-write
- [ ] 80%+ of commands have declared contracts
- [ ] Cheatsheet can be regenerated from contracts

### v0.14.0: Experiment Infrastructure

- [ ] `ExperimentConfig` type for model/adapter/weights
- [ ] `patina config create/list` commands
- [ ] `patina eval --config X` runs eval with specific config
- [ ] `patina eval --compare A B` shows delta between configs
- [ ] Sessions record which config was active

### Stretch (v0.15.0+)

- [ ] `patina introspect --orphans` finds unused tables/events
- [ ] `patina introspect --trace <path>` follows full data flow
- [ ] `patina introspect --impact <X>` suggests tests
- [ ] `patina doctor` checks contract accuracy
- [ ] Coherence metrics in `patina doctor --coherence`

---

## Open Questions

1. **Where does introspect command live?** Options:
   - `patina introspect` (new top-level) ← leaning this way
   - `patina doctor introspect` (under doctor)
   - `patina report introspect` (under report)

2. **How to handle oracles?** Oracles are internal to scry but have their own read patterns. Should they have contracts?

3. **Contract drift detection?** Can we detect when code changes but contracts don't?

4. **Config storage location?** Options:
   - `.patina/local/configs/` (project-local)
   - `~/.patina/configs/` (user-level, shared across projects)
   - Both (with inheritance)

5. **Adapter detection:** How do we know which adapter is running? Options:
   - Environment variable set by adapter
   - Detect from MCP connection metadata
   - Explicit `--adapter` flag on session start

---

## Relationship to Other Specs

- **mother-v2**: Mother needs introspection to know what artifacts exist across projects
- **data-flow-cheatsheet**: The cheatsheet becomes a generated output, not a maintained doc
- **belief system**: Beliefs about architecture (like `measure-first`) apply to introspection itself

### Ownership Boundaries (aligned 2026-02-05)

| Concern | Owner | This Spec's Role |
|---------|-------|------------------|
| `DataContract` schema | **this spec** | Defines `Source`, `Sink`, `WritePath` |
| `CommandGroup` enum | cli-reorganization | Imports and uses for metadata |
| `introspect` command | **this spec** | Defines behavior, subcommands |
| Code file structure | cli-reorganization | Places introspect in `dev/` |
| Experiment infrastructure | **this spec** | Defines `ExperimentConfig`, `config` command |

**Note:** cli-reorganization may extend `DataContract` with `group` and `related` fields. The core schema lives here.

---

## Status Log

| Date | Status | Note |
|------|--------|------|
| 2026-02-05 | design | Created from state-of-union session — recognized need for system observability |
| 2026-02-05 | design | Added Jerry Nixon framing (argue every box, defer decisions). Elevated A/B testing to essential — AI landscape volatility means experiments are survival infrastructure, not a luxury. OTEL deferred until prod users. |
| 2026-02-05 | design | Added aggregate views (`--sources`, `--sinks`, `--write-paths`) and `WritePath` taxonomy (scrape, action-time, dual-write). Answers: "What are ALL the X?" questions. |
| 2026-02-05 | design | **Spec alignment:** This spec owns DataContract schema and introspect command. cli-reorganization owns CommandGroup and code structure. Version targets aligned: v0.12=DataContract, v0.13=introspect, v0.14=experiments. |
