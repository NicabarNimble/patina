---
type: audit
id: durable-rust-unix-realignment-2026-04-14
scope: src/, mother/src/, tests/, sdk/
created: 2026-04-14
status: complete
findings:
  s0_correctness_safety: 0
  s1_architecture_drift: 5
  s2_maintainability_operability: 5
verdict: >
  Core boundaries are still present, but implementation concentration has grown
  in several high-traffic files. The repo is in a "works but drifting" state:
  reliable behavior, increasing change friction. Realignment should prioritize
  decomposition of command/runtime monoliths and repair one concrete spec
  lifecycle regression.
---

# Durable Rust + Unix Realignment Audit (Top-10 Inventory)

## Method

- File-size and function-span scan across `src/`, `mother/src/`, and tests.
- Structural read of high-concentration files.
- Value rubric from:
  - `layer/core/values/dependable-rust.md`
  - `layer/core/values/unix-philosophy.md`
  - `layer/core/values/spec-driven-design.md`
  - `layer/core/values/adapter-pattern.md`
  - `layer/core/values/safety-boundaries.md`
- Live behavior verification for one spec-lifecycle edge case.

## Concentration Snapshot

Largest Rust files (selected):

- `src/commands/measure/internal.rs` — 3140 LOC
- `src/commands/mother/daemon.rs` — 3088 LOC
- `src/main.rs` — 1987 LOC
- `mother/src/http_api.rs` — 1797 LOC
- `src/spec.rs` — 1225 LOC
- `src/child/internal/child.rs` — 2140 LOC

Largest function spans (selected):

- `src/main.rs:1153` `fn main` — ~835 lines
- `src/commands/mother/daemon.rs:584` `fn compose_typed_component` — ~333 lines
- `src/spec.rs:385` `fn execute_command_value` — ~211 lines
- `src/child/internal/child.rs:1366` `fn json_to_component_val` — ~327 lines

---

## Findings (Top 10)

## F-001 — `main` has become a system, not a tool
- **Severity:** S1 (architecture drift)
- **Where:** `src/main.rs:1153` (`fn main`), plus large command dispatch chain below
- **Values impacted:** `unix-philosophy`, `dependable-rust`
- **Why this matters:** Single entrypoint now owns migration/preflight/registry init and a very large command router. Change blast radius is high; review difficulty is high.
- **Smallest acceptable fix:** Move command-family dispatch into separate modules (`main_dispatch/{core,dev,mother,spec,...}.rs`) and keep `main` as orchestration shell only.
- **Proof needed:** Unit tests for each dispatch module + existing CLI parsing tests remain green.

## F-002 — Mother daemon file is overloaded across multiple responsibilities
- **Severity:** S1
- **Where:** `src/commands/mother/daemon.rs` (3088 LOC)
  - `compose_typed_component` at `~L584`
  - `run_server` at `~L2163`
  - `rivet_dispatch` at `~L1314`
  - tests embedded from `~L2274`
- **Values impacted:** `unix-philosophy`, `dependable-rust`
- **Why this matters:** Transport boot, lifecycle warmup, policy mapping, typed composition wiring, and extensive tests are co-located. This is a maintenance bottleneck.
- **Smallest acceptable fix:** Split into:
  - `daemon/startup.rs`
  - `daemon/dispatch.rs`
  - `daemon/composition.rs`
  - `daemon/health.rs`
  - `daemon/tests/*`
- **Proof needed:** Existing daemon tests preserved; no behavior regressions in warmup/rivet dispatch.

## F-003 — Spec execution router is too broad in one function
- **Severity:** S1
- **Where:** `src/spec.rs:385` (`execute_command_value` ~211 lines)
- **Values impacted:** `dependable-rust`, `unix-philosophy`
- **Why this matters:** Project routing, cross-project safety checks, command execution, and response formatting all live in one flow.
- **Smallest acceptable fix:** Separate into `resolve_route`, `authorize_cross_project`, `execute_spec_command`, `render_spec_response` helpers/modules.
- **Proof needed:** Snapshot tests for representative command payloads (create/show/check/complete).

## F-004 — Spec create path mixes validation + file I/O + git + DB in one transactionless flow
- **Severity:** S1
- **Where:** `src/commands/spec/internal/create.rs:87` (`create_spec_value_for_project`)
- **Values impacted:** `unix-philosophy`, `dependable-rust`, `safety-boundaries`
- **Why this matters:** A partial failure can leave inconsistent state between filesystem, git history, and sqlite patterns table.
- **Smallest acceptable fix:** Two-phase flow:
  1. Materialize + validate files
  2. Commit
  3. DB record update
  With explicit rollback/repair notes on failure boundaries.
- **Proof needed:** Failure-path tests for commit failure and DB failure.

## F-005 — Archived spec read path regressed (concrete lifecycle bug)
- **Severity:** S1
- **Where:**
  - `src/commands/spec/internal/archive.rs:297-304` returns placeholder `file_path` as `(archived: spec/<id>)`
  - `src/commands/spec/internal/archive.rs:369+` `load_spec` reads `found.file_path` from filesystem
  - `src/commands/spec/internal/queries.rs:450+` `show_spec_value` always uses `load_spec`
- **Values impacted:** `spec-driven-design`, `dependable-rust`
- **Observed behavior:** `patina spec show/check <archived-id>` fails with `No such file or directory` after completion/archive.
- **Smallest acceptable fix:** Add archived-aware load path that reads from `git show spec/<id>:<spec-path>` (or stores archived path metadata).
- **Proof needed:** New tests: show/check on archived spec tags must succeed.

## F-006 — HTTP API module concentration is high
- **Severity:** S2
- **Where:** `mother/src/http_api.rs` (1797 LOC), tests start at `~L977`
- **Values impacted:** `dependable-rust`, `unix-philosophy`
- **Why this matters:** API contracts, handlers, router table wiring, and large test scaffolds are coupled in one file.
- **Smallest acceptable fix:** Split by endpoint domains (`health`, `lifecycle`, `inspector`, `rivet`, `builtin`) and keep trait/contracts in a thin core module.
- **Proof needed:** Existing endpoint tests pass unchanged.

## F-007 — Typed JSON/component conversion is oversized and drift-prone
- **Severity:** S2
- **Where:** `src/child/internal/child.rs`
  - `json_to_component_val` (~327 lines) at `~L1366`
  - `component_val_to_json` at `~L1694`
- **Values impacted:** `dependable-rust`, `spec-driven-design`
- **Why this matters:** High manual branching increases risk when WIT types evolve.
- **Smallest acceptable fix:** Move conversion logic to dedicated module with table-driven conversion tests and fixture conformance locks.
- **Proof needed:** round-trip tests per primitive/composite WIT type family.

## F-008 — Integration tests are monolithic and expensive to iterate
- **Severity:** S2
- **Where:**
  - `tests/wasm_integration.rs` (2273 LOC)
  - `tests/pando_parity.rs` (2064 LOC)
- **Values impacted:** `unix-philosophy`
- **Why this matters:** Broad tests are valuable, but current size reduces locality and slows targeted diagnosis.
- **Smallest acceptable fix:** Split by feature domains and retain one high-level E2E per domain.
- **Proof needed:** Total behavior parity maintained; targeted lanes become faster.

## F-009 — Query/dispatch boundaries still have stringly command handling hot spots
- **Severity:** S2
- **Where:** `src/main.rs:1032` (`make_query_dispatch` ~120 lines)
- **Values impacted:** `dependable-rust`
- **Why this matters:** String matching and ad-hoc field extraction increase drift and reduce compile-time guarantees.
- **Smallest acceptable fix:** Replace ad-hoc map parsing with typed enums/struct payload parsing at boundary.
- **Proof needed:** Parser tests for valid/invalid query payloads.

## F-010 — Push lane cost is high enough to incentivize policy bypasses
- **Severity:** S2 (operability)
- **Where:** pre-push flow (Tier1/Tier2 full escalation behavior)
- **Values impacted:** `safety-boundaries` (process reliability), `spec-driven-design`
- **Why this matters:** Very long full-lane checks are correct but can create pressure to bypass (`--no-verify`) during urgent pushes.
- **Smallest acceptable fix:** Improve path-impact scoping and keep full-lane available as explicit opt-in/CI gate while preserving mandatory critical guards.
- **Proof needed:** median pre-push runtime reduction without regression in guard coverage.

---

## Recommended Execution Order (House-Cleaning Plan)

1. **Fix correctness first**
   - F-005 archived spec read regression.
2. **Reduce command/runtime concentration**
   - F-001, F-002, F-003 (router and daemon decomposition).
3. **Harden transactional boundaries**
   - F-004 create flow decomposition + failure-path tests.
4. **Stabilize conversion/API hot spots**
   - F-006, F-007, F-009.
5. **Improve developer operability**
   - F-008, F-010.

---

## Suggested Spec Slices (small, auditable)

- `refactor/spec-archive-read-path` (F-005)
- `refactor/main-command-router-split` (F-001)
- `refactor/mother-daemon-split` (F-002)
- `refactor/spec-execution-router-split` (F-003)
- `fix/spec-create-transaction-boundary` (F-004)
- `refactor/http-api-endpoint-modules` (F-006)
- `refactor/child-wit-value-conversion-module` (F-007)
- `refactor/integration-test-domain-split` (F-008)
- `refactor/typed-query-dispatch-contract` (F-009)
- `ops/prepush-impact-lane-tuning` (F-010)

---

## Bottom line

The repo is not in failure mode; it is in **concentration drift** mode.

You can restore the tighter Jon-style durable Rust + Unix feel without a rewrite by executing the above as small specs with hard proof gates.
