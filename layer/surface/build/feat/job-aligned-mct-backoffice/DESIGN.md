# Design: feat: Job-aligned MCT backoffice

## Why This Design

The goal is not generic job-search throughput. The goal is deterministic selection
of roles that align with Patina/MCT direction (Rust + Wasm + component model +
Mother-governed composition).

Prompt-only workflows are useful for summarization, but they are weak as final
policy/decision systems. We need a reproducible, typed, auditable pipeline where:

- ingest and normalization are explicit,
- alignment gates are fail-closed,
- scoring is deterministic,
- Mother capability boundaries stay authoritative.

This turns job search from ad-hoc chat judgments into a governed backoffice flow.

## Build Target

Deliver a first MCT-native job-alignment path:

1. `patina job ingest <url|file>` produces a canonical job record.
2. Hard gates + weighted dimensions are read from a user-owned alignment profile.
3. Typed scorer child returns decision + explanation payload.
4. Repeated ingest reuses canonicalized identity, emits change signal, and appends history.
5. Mother grant/audit behavior is fail-closed for all children in this path.

This feature should provide a concrete implementation candidate for
`sdk-vision-lock` proof criteria (`svl3`, `svl5`, `svl6`, `svl10`).

## Architecture Sketch

### A) Policy + schema canon (deterministic)

Two explicit contracts:

1. **Job record schema** (normalized facts)
   - source URL/file
   - canonical URL key
   - company/title/location/comp range (when known)
   - evidence links
   - confidence + provenance timestamps
   - content hash

2. **Alignment profile** (user policy)
   - hard gates (must-have / must-not-have)
   - weighted dimensions (0..N, explicit weights)
   - decision thresholds (recommend/consider/reject)

Hard gates fail closed if fields required by policy are missing.

### B) Typed child lane (SDK-first)

Pipeline composes typed children; no new prompt-only decision path:

- existing typed record children remain available for normalization/transform stages,
- new typed child `job-fit-scorer` computes deterministic fit + gate outcomes,
- host stores score card + reasons as immutable decision artifacts.

### C) Deterministic cache/replay (borrowed from Defuddle pattern)

For repeat ingest of same canonical URL:

- canonicalize identity key,
- hash normalized content,
- if unchanged: reuse prior score explanation,
- if changed: recompute score + append compact change signal.

This keeps context and compute lean while preserving auditability.

### D) Mother authority + audit boundary

All participating children declare `[needs].toys` (+ optional scopes).
Mother enforces grants and logs GRANT/DENY decisions.
Unknown/missing grants fail closed.

### E) HITL surface (thin)

Minimal command surface:

- `patina job init` (scaffold alignment profile + schema fixtures)
- `patina job ingest <url|file>`
- `patina job show <id>`
- `patina job decide <id> --status <reject|hold|pursue>`

Output style stays compact with artifact pointers (no prompt flooding).

## Resolved Decisions

- Final recommendation path is deterministic; LLM output is advisory-only.
- Job alignment policy is user-owned and explicit, not hidden in prompts.
- Canonical URL + hash replay is first-class (change tracking built in).
- Mother grant/audit behavior remains fail-closed and mandatory.
- This feature is an execution spec under `sdk-vision-lock`, not parallel architecture.

## Commits

1. `feat(job): add deterministic job policy + record canon`  
   Scaffold alignment policy file + canonical record fixtures + loader.
2. `feat(job): add typed job-fit-scorer child`  
   SDK-first child with typed WIT contract for scoring + gate output.
3. `feat(job): add job command surface`  
   `job init/ingest/show/decide` host orchestration and artifact writing.
4. `feat(job): add canonical replay and change signal`  
   Canonical URL/hash cache + deterministic change cues.
5. `feat(job): enforce mother grant/audit path for job pipeline`  
   Fail-closed grant checks and audit visibility for participating children.
6. `test(job): add fixture-driven e2e + replay determinism`  
   Deterministic output and grant-deny behavior coverage.

## Direct Code Targets

### Command + orchestration

- `src/main.rs`
  - add `job` command routing
- `src/commands/mod.rs`
  - register `job` module
- `src/commands/job/mod.rs` (new)
  - clap command surface + public handlers
- `src/commands/job/internal.rs` (new)
  - ingest/normalize/cache/replay orchestration

### Paths + local storage

- `src/paths.rs`
  - add project helpers for job artifacts (policy, records, decisions, cache index)

### Typed contracts

- `wit/toys/patina/job.wit` (new)
  - job types + scorer interface contract
- `wit/toys/deps/toys-registry.toml`
  - register `patina-job` contract

### Child implementation

- `children/job-fit-scorer/Cargo.toml` (new)
- `children/job-fit-scorer/child.toml` (new)
- `children/job-fit-scorer/wit/world.wit` (new)
- `children/job-fit-scorer/src/lib.rs` (new)

### Templates/artifacts

- `resources/templates/job/alignment.toml.tmpl` (new)
  - scaffolded user policy
- `resources/templates/job/job-record.fixture.json` (new)
  - deterministic fixture for tests

### Grant/audit integration

- `src/child/internal/mod.rs`
  - ensure manifest + grant parsing covers new job contract path
- `src/child/internal/child.rs`
  - enforce fail-closed toy access path where applicable
- `src/commands/mother/daemon.rs`
  - wire/verify typed composition behavior for job scorer lane

### Tests

- `tests/job_alignment_pipeline.rs` (new)
  - ingest → score → record deterministic path
- `tests/job_alignment_replay.rs` (new)
  - unchanged replay reuse + changed content signal behavior
- `tests/mother_job_grants.rs` (new)
  - unauthorized grant path denies with audit evidence
- `tests/manifest_authoring_canon.rs`
  - ensure new child manifest vocabulary stays canonical

## Verification Plan

1. `patina spec check job-aligned-mct-backoffice --json`
2. `cargo check --workspace -q`
3. `cargo test --test job_alignment_pipeline`
4. `cargo test --test job_alignment_replay`
5. `cargo test --test mother_job_grants`
6. Manual smoke:
   - `patina job init`
   - `patina job ingest <fixture-url-or-file>`
   - re-run same input to confirm replay reuse
   - mutate fixture and re-run to confirm change signal

## Build Readiness

Medium.

- Governance constraints are already defined in existing umbrella specs.
- Main implementation risk is selecting the minimal first typed contract that
  gives useful scoring without introducing mode/prompt bloat.

## Open Questions

1. Should initial ingest support URL + file equally, or file-first with URL as
   best-effort normalization in v1?
2. Should job policy live under committed project files (`jobs/`) or local-only
   (`.patina/local/jobs/`) by default?
3. Do we represent job records as dedicated `patina:job` types only, or also
   mirror into existing `patina:records` envelope for reuse across transforms?
4. Which minimal toy set should `job-fit-scorer` require in v1 (likely logging +
   measure only) to keep grants tight?
