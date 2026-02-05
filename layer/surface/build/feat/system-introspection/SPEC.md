---
type: feat
id: system-introspection
status: design
created: 2026-02-05
updated: 2026-02-05
sessions:
  origin: 20260205-064001
related:
  - layer/surface/reports/data-flow-cheatsheet.md
  - layer/surface/build/feat/mother-v2/SPEC.md
beliefs:
  - measure-first
  - measure-the-measurement
---

# feat: System Introspection

> Know what you're building. Trace any data from source to reader. Understand before changing.

## Problem

Patina has grown to:
- 25+ commands
- 5 oracles
- 35,000+ eventlog entries
- 50+ database tables
- Multiple storage layers (project, user, mother)

**We are losing the mental ability to understand what we're building.**

Symptoms:
- "Where does this data come from?" requires code archaeology
- "What will break if I change X?" is guesswork
- "Why isn't Y showing up in scry?" means tracing through 5 files
- New features get "hacked in" because the flow isn't clear
- The cheatsheet exists but it's static — the code can drift

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

### 4. Measure System Coherence

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

### Phase 1: Data Contracts

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
}

pub enum Source {
    Files(&'static str),      // glob pattern
    Table(&'static str),      // SQLite table
    Fts(&'static str),        // FTS5 virtual table
    Eventlog(&'static str),   // event_type prefix
    Usearch(&'static str),    // usearch index
    ExternalDb(&'static str), // e.g., persona.db
}

pub enum Sink {
    Table(&'static str),
    Fts(&'static str),
    Eventlog(&'static str),
    Usearch(&'static str),
    ExternalDb(&'static str),
    Files(&'static str),
}
```

**Exit:** Every command in `src/commands/` has a `DATA_CONTRACT` constant.

### Phase 2: Introspect Command

Add `patina introspect` command.

**Subcommands:**
```bash
patina introspect <command>           # Show what a command reads/writes
patina introspect --table <name>      # Show readers/writers for a table
patina introspect --event <type>      # Show writers/consumers for event type
patina introspect --schema            # Dump full schema awareness
patina introspect --trace <path>      # Trace data flow from source
patina introspect --orphans           # Find orphan tables/events
```

**Exit:** `patina introspect scry` shows accurate reads/writes/oracles.

### Phase 3: Contract Verification

Extend `patina doctor` to verify contracts against reality.

**Checks:**
- Declared tables exist in schema
- Declared event types appear in eventlog
- Declared file patterns match actual files
- No writes to undeclared sinks

**Exit:** `patina doctor` reports contract violations.

### Phase 4: Impact Analysis

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

### Phase 5: Coherence Metrics

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

## Non-Goals

- **Visual diagrams** — text output is sufficient, diagrams can be generated externally
- **Runtime tracing** — this is static analysis, not distributed tracing
- **Automatic contract generation** — contracts are declared, not inferred

---

## Exit Criteria

### v0.13.0 or v0.14.0 (after mother-v2 foundations)

- [ ] `DataContract` type exists in `src/introspection/`
- [ ] 80%+ of commands have declared contracts
- [ ] `patina introspect <command>` works for declared commands
- [ ] `patina introspect --table <name>` shows readers/writers
- [ ] `patina introspect --orphans` finds unused tables/events
- [ ] `patina doctor` checks contract accuracy
- [ ] Cheatsheet can be regenerated from contracts

### Stretch

- [ ] `patina introspect --trace <path>` follows full data flow
- [ ] `patina introspect --impact <X>` suggests tests
- [ ] Coherence metrics in `patina doctor --coherence`

---

## Open Questions

1. **Where does this command live?** Options:
   - `patina introspect` (new top-level)
   - `patina doctor introspect` (under doctor)
   - `patina report introspect` (under report)

2. **How to handle oracles?** Oracles are internal to scry but have their own read patterns. Should they have contracts?

3. **Contract drift detection?** Can we detect when code changes but contracts don't?

4. **Integration with tests?** Should contract violations fail CI?

---

## Relationship to Other Specs

- **mother-v2**: Mother needs introspection to know what artifacts exist across projects
- **data-flow-cheatsheet**: The cheatsheet becomes a generated output, not a maintained doc
- **belief system**: Beliefs about architecture (like `measure-first`) apply to introspection itself

---

## Status Log

| Date | Status | Note |
|------|--------|------|
| 2026-02-05 | design | Created from state-of-union session — recognized need for system observability |
