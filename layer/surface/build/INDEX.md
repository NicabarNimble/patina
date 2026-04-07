# Spec Index

All specs — active on disk, archived in git tags. Agents: read this before
making claims about how something was designed.

## On Disk

### Active
- [child-construction-canon](feat/child-construction-canon/SPEC.md) — registry of reusable children, composition model, SDK extraction (ccc1-ccc2 checked, ccc3-ccc7 pending)
- [spec-archive-db-path](fix/spec-archive-db-path/SPEC.md) — find_spec reads stale DB instead of frontmatter on disk

### Draft
- [interface-redesign](refactor/interface-redesign/SPEC.md) — Mother-managed interface registry, ephemeral projection, skill system (16 criteria)
- [engine-consolidate](refactor/engine-consolidate/SPEC.md) — merge PipelineEngine + KnowledgeChildEngine
- [duckdb-version-pin](fix/duckdb-version-pin/SPEC.md) — bump DuckDB prebuilt, align Cargo pin (blocked: crate versioning scheme changed)
- [e2ee-multimother-chat](feat/e2ee-multimother-chat/SPEC.md) — child-construction-canon MVP 3
- [multiproject-belief-share](feat/multiproject-belief-share/SPEC.md) — child-construction-canon MVP 2
- [belief-system-hardening](feat/belief-system-hardening/SPEC.md) — staleness, verification, health scoring
- [persona-lake-mvp1](feat/persona-lake-mvp1/SPEC.md) — persona-scoped knowledge lake
- [mother-duckdb-ducklake-federation](feat/mother-duckdb-ducklake-federation/SPEC.md) — Mother DuckDB + DuckLake federation
- [cloudflare-worker-child](feat/cloudflare-worker-child/SPEC.md) — Patina child as Cloudflare Worker
- [patina-durable-backup](feat/patina-durable-backup/SPEC.md) — durable backup system
- [mother-password-unlock](explore/mother-password-unlock/SPEC.md) — password-based vault unlock
- [monty-patina-sandbox-alignment](explore/monty-patina-sandbox-alignment/SPEC.md) — monty/patina goal alignment
- [spec-manager-wasm-child](explore/spec-manager-wasm-child/SPEC.md) — convert spec-manager from builtin to WASM child (explore: gaps identified, deferred)

### Complete (pending archive)
- [ducklake-retirement](refactor/ducklake-retirement/SPEC.md) — DuckLake retired, lake queries via Mother
- [sdk-mother-child-retirement](refactor/sdk-mother-child-retirement/SPEC.md) — removed legacy MotherChild API

### Abandoned
- [pando-vocabulary-alignment](refactor/pando-vocabulary-alignment/SPEC.md) — split into child-rename + engine-consolidate; pando vocabulary locked
- [scrape-strategy-seam-exploration](refactor/scrape-strategy-seam-exploration/SPEC.md) — explore scrape strategy extraction
- [spec-prompt-handoff](feat/spec-prompt-handoff/SPEC.md) — deferred in favor of core architecture

## Archived (git tags)

469 archived specs total. Browse all: `git tag -l "spec/*" | grep -v "\-start$" | sort`

Read any archived spec: `git show spec/<name>^:layer/surface/build/<type>/<name>/SPEC.md`

### SDK and Toybox (how the SDK was designed)
- `spec/sdk-contract-stabilization` — SDK stability tiers, shim removal gates, child-first API. Defines stable/experimental/internal classification.
- `spec/sdk-toybox-definition` — formal toy definition and litmus test, canonical toybox lock, WASI alignment. Enumerates all host resources and consolidation decisions.
- `spec/composable-toy-sdk` — composable toy model replacing tiered SDK crates
- `spec/single-patina-sdk-consolidation` — collapsed tiered crates into single patina-sdk
- `spec/patina-sdk` — original SDK consolidation (v0.21.0)
- `spec/wit-contract-single-source` — WIT as single source of truth for toy contracts
- `spec/wit-interfaces` — WIT interface definitions exploration
- `spec/pipe-contract-safety` — pipe contract safety and type integrity

### Children and Composition (how children were built)
- `spec/folder-text-to-parquet` — MVP 1: 6 core reusable children built and composed into pipeline. Proves child-construction-canon ccc2.
- `spec/child-rename` — knowledge-child to child rename, wit world merge
- `spec/vocabulary-alignment-child-manifest` — child/toy manifest vocabulary alignment
- `spec/knowledge-child-platform` — original WASM-first children for knowledge system
- `spec/native-child-removal` — removed dead native child infrastructure
- `spec/ducklake-knowledge-child-cutover` — DuckLake cutover to knowledge-child model
- `spec/github-child-owns-forge` — GitHub interaction owned by child, not Mother

### Interface System (how interfaces evolved)
- `spec/interface-vocabulary-hard-cut` — adapter to interface vocabulary, 9/9 criteria, migration confirmed
- `spec/adapter-to-interface-rename` — initial adapter to interface rename (PR #111)
- `spec/interface-surface-reconciliation` — backup, managed takeover, safe rewrites for interface setup
- `spec/ai-interface-seat-separation` — interface-scoped tmux and safe session attach
- `spec/init-interface-projection-separation` — separated init from interface projection
- `spec/interface-tmux-launcher-restoration` — restored tmux launch lanes
- `spec/ai-launcher-surface-consolidation` — launcher surface consolidation

### Mother Architecture (how Mother was designed)
- `spec/knowledge-system-architecture` — domain-agnostic knowledge system with persona federation. Split into forge-plugin, core-plugin, persona-federation.
- `spec/interface-session-model` — per-interface session identity, auto-start, ghost elimination
- `spec/session-handoff-enrichment` — LLM-generated handoff, parent linking
- `spec/session-hardening` — session system hardening
- `spec/mcp-typed-handlers` — eliminate Value soup at protocol boundary

### Data and Retrieval (how the knowledge layer was built)
- `spec/ducklake` — original DuckLake vision (superseded by ducklake-enterprise)
- `spec/ducklake-enterprise` — GitHub lakehouse on composable toy SDK
- `spec/ducklake-retirement` — DuckLake retired
- `spec/retrieval-optimization` — 6.8x retrieval improvement
- `spec/d0-unified-search` — unified search pipeline
- `spec/epistemic-layer` — epistemic markdown layer

### Spec System (how specs themselves work)
- `spec/spec-system` — original spec system design
- `spec/cli-first-spec-workflow` — CLI-first spec workflow
- `spec/spec-structured-exit-criteria` — structured exit criteria in frontmatter
- `spec/spec-complete-archives` — collapse complete + archive into one command
- `spec/spec-query-filesystem-truth` — spec queries use filesystem as truth
- `spec/deterministic-spec-scaffolds` — deterministic scaffolds for agents
