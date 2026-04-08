# Spec Index

All specs — active on disk, archived in git tags. Agents: read this before
making claims about how something was designed.

## On Disk

### Active
- [child-construction-canon](feat/child-construction-canon/SPEC.md) — registry of reusable children, composition model, SDK extraction (ccc1-ccc2 checked, ccc3-ccc7 pending)

### Draft
- [persona-lake-mvp1](feat/persona-lake-mvp1/SPEC.md) — persona-scoped knowledge lake (blocked by mother-duckdb-ducklake-federation)
- [multiproject-belief-share](feat/multiproject-belief-share/SPEC.md) — child-construction-canon MVP 2 (blocked by mother-duckdb-ducklake-federation)
- [e2ee-multimother-chat](feat/e2ee-multimother-chat/SPEC.md) — child-construction-canon MVP 3
- [slate-pando-migration](feat/slate-pando-migration/SPEC.md) — migrate spec-manager to slate pando via pando platform
- [belief-system-hardening](feat/belief-system-hardening/SPEC.md) — staleness, verification, health scoring
- [cloudflare-worker-child](feat/cloudflare-worker-child/SPEC.md) — Patina child as Cloudflare Worker
- [patina-durable-backup](feat/patina-durable-backup/SPEC.md) — durable backup system
- [engine-consolidate](refactor/engine-consolidate/SPEC.md) — merge PipelineEngine + KnowledgeChildEngine
- [interface-redesign](refactor/interface-redesign/SPEC.md) — Mother-managed interface registry, ephemeral projection, skill system (16 criteria)
- [duckdb-durable-execution](explore/duckdb-durable-execution/SPEC.md) — DuckDB + Mother for Absurd-style durable execution (explore)
- [mother-child-artifact-registry](explore/mother-child-artifact-registry/SPEC.md) — Mother-managed child artifact distribution (explore)
- [mother-password-unlock](explore/mother-password-unlock/SPEC.md) — password-based vault unlock (explore)
- [monty-patina-sandbox-alignment](explore/monty-patina-sandbox-alignment/SPEC.md) — monty/patina goal alignment (explore)
- [spec-manager-wasm-child](explore/spec-manager-wasm-child/SPEC.md) — convert spec-manager to WASM child (explore, deferred)

### Abandoned
- [pando-vocabulary-alignment](refactor/pando-vocabulary-alignment/SPEC.md) — split into child-rename + engine-consolidate; pando vocabulary locked
- [scrape-strategy-seam-exploration](refactor/scrape-strategy-seam-exploration/SPEC.md) — explore scrape strategy extraction
- [spec-prompt-handoff](feat/spec-prompt-handoff/SPEC.md) — deferred in favor of core architecture
- [sdk-toy-conformance-harness](fix/sdk-toy-conformance-harness/SPEC.md) — superseded by sdk-wasi-trait-alignment

## Archived (git tags)

Read any archived spec: `git show spec/<name>:layer/surface/build/<type>/<name>/SPEC.md`

### Recently Archived
- `spec/mother-startup-observability` — Mother startup diagnostics, per-child load telemetry, failure surface (6/6)
- `spec/ducklake-retirement` — DuckLake runtime coupling removed (7/7)
- `spec/sdk-upstream-toy-sync` — pull upstream WASI WIT files with release-based pin model (10/10)
- `spec/sdk-wasi-trait-alignment` — align all toy traits to WASI shape (13/13)
- `spec/sdk-mother-child-retirement` — removed legacy MotherChild API (4/4)
- `spec/mother-duckdb-ducklake-federation` — Mother DuckDB + DuckLake federation substrate (11 criteria)
- `spec/mother-pando-bindings-runtime` — two-phase startup, MotherRuntime trait, lifecycle, manifest integrity (7/7)
- `spec/pando-platform` — composed children as user-facing products, pando.toml, Mother registry, slate pando (Phase A complete)
- `spec/duckdb-version-pin` — aligned DuckDB prebuilt (v1.5.1) and Rust crate (1.10501.0). See Cargo.toml for version encoding notes.
- `spec/spec-archive-db-path` — fix mutation commands reading stale DB instead of frontmatter

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
- `spec/greenfield-mother-patina-data-platform` — Mother owns per-project databases, project sovereignty, DuckDB federation queued as future work
- `spec/move-vault-to-mother` — vault crypto moved to Mother, CLI thinned to IPC-only (v0.45.8)
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
