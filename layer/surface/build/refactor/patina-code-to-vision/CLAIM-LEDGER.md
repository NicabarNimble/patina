# Patina Code-to-Vision Claim Ledger

Updated: 2026-03-22

Status keys:

- `verified-false` = code disproves the criterion today
- `verified-partial` = some pieces exist, criterion not satisfied
- `verified-true` = criterion appears satisfied now
- `unverified` = needs explicit code/behavior proof

## CV Truth Map

| CV | Status | Evidence |
|---|---|---|
| CV1 | verified-false | Mother runtime split across `mother/src/*`, `src/mother/*`, and `src/commands/mother/*`. |
| CV2 | verified-false | CLI still contains substantial Mother runtime command/server logic in `src/commands/mother/daemon.rs` and `src/commands/mother/mod.rs`. |
| CV3 | verified-false | `try_daemon_*` probe/fallback paths still present in `src/commands/context.rs`, `src/commands/measure/mod.rs`, `src/commands/spec/mod.rs`, `src/commands/lake.rs`, `src/commands/scry/internal/routing.rs`. |
| CV4 | verified-false | Placeholder filter logic still used (`contains("not yet implemented")`) in core command routing files above. |
| CV5 | verified-false | `cargo check` reports 40 warnings (run dated 2026-03-22). |
| CV6 | verified-partial | Child vocabulary bridge exists, but `src/plugin/*` still exists alongside `src/child/*`. |
| CV7 | verified-false | No bundled `measure-health` child found; session-writer availability depends on runtime load path/artifacts, not guaranteed as always-bundled. |
| CV8 | verified-false | No project child-needs manifest + connect-time child resolution flow implemented. |
| CV9 | verified-false | `src/commands/spec/internal/*` still core; no `children/spec-manager/`. |
| CV10 | verified-false | No `wit/toys/layer-fs.wit` or `wit/toys/git.wit` host implementations found. |
| CV11 | verified-partial | Scrape is strategy-structured and grammar-driven in-core, but code strategy is not childized yet. |
| CV12 | verified-false | `patina spec list` still routed through core implementation, not child availability gate. |
| CV13 | verified-false | `rename` and `reopen` not present as shipped spec subcommands. |
| CV14 | verified-false | No required HITL confirmation gate for `spec complete` / `spec abandon`. |
| CV15 | verified-false | `src/commands/doctor.rs` exists as core command surface. |
| CV16 | verified-false | Version command still coupled to spec readiness logic (`src/commands/version/internal.rs`). |
| CV17 | verified-false | `src/commands/session/*` still exists as core command surface. |
| CV18 | verified-false | `src/commands/lake.rs` still core command surface. |

## Notes

- This ledger is the gating artifact for Phase 0. Update this file before/after each phase.
- If any CV text is semantically inaccurate, amend SPEC wording before implementation.
