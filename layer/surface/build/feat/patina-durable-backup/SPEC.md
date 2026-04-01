---
type: feat
id: patina-durable-backup
status: draft
created: 2026-03-31
sessions:
  origin: 20260331-072235-030494000
related:
  - src/main.rs
  - src/commands/setup/
  - src/commands/mother/
  - src/paths.rs
  - layer/core/session-capture.md
  - layer/core/spec-driven-design.md
exit_criteria:
  - id: pdb1-backup-surface-exists
    text: "`patina backup` command surface exists with `init`, `run`, `status`, `verify`, `restore`, and `prune` subcommands."
    checked: false
  - id: pdb2-durable-policy-defined
    text: "A backup policy classifies paths as durable, rebuildable, and optional, with durable selected by default."
    checked: false
  - id: pdb3-default-durable-set-backed-up
    text: "Default backup run includes non-rebuildable state: persona events, secrets identity/vault metadata, connection records, global registry, and Mother project events."
    checked: false
  - id: pdb4-rebuildable-state-excluded-by-default
    text: "Default backup excludes rebuildable caches: model cache, repo cache, grammar pipeline downloads, and transient runtime logs."
    checked: false
  - id: pdb5-scheduler-installed-on-init
    text: "`patina backup init` installs platform scheduler job (launchd on macOS, systemd user timer or cron fallback on Linux) and can be disabled explicitly."
    checked: false
  - id: pdb6-manifest-and-checksum-recorded
    text: "Each backup point records manifest metadata (timestamp, policy version, included paths, file counts, bytes) and checksums for restore verification."
    checked: false
  - id: pdb7-restore-flow-is-safe
    text: "`patina backup restore` performs safe restore with Mother lifecycle coordination (stop/restore/permissions/verify/start) and supports point selection."
    checked: false
  - id: pdb8-verify-detects-corruption
    text: "`patina backup verify` detects missing/corrupt snapshot files and reports actionable remediation."
    checked: false
  - id: pdb9-recovery-drill-documented
    text: "CLI docs include a no-panic recovery drill for accidental `rm -rf ~/.patina` and expected post-restore checks."
    checked: false
  - id: pdb10-final-proof
    text: "End-to-end proof: create durable sample state, run backup, delete state in a sandbox, restore, and verify `patina mother status`, `patina repo list`, and `patina connect list` reflect restored state."
    checked: false
---
# feat: Patina Durable Backup System

> Ship a built-in backup system that protects non-rebuildable Patina state by default and keeps rebuildable caches out of snapshots.

## Problem

Accidental deletion of `~/.patina` can erase high-value local state that is not fully recoverable without backups.

Recent incident outcomes showed clear categories:

- **Durable, hard-to-rebuild state** was lost: global connection records, persona event history, secrets identity/vault metadata, and registry coordination state.
- **Rebuildable state** was also lost: models, repo caches, grammar downloads, and Mother runtime data that can be reconstructed.

Patina currently has no first-class, protocol-aware backup workflow. Recovery is manual and easy to get wrong under stress.

## Goal

Make backup and restore a first-class Patina capability with opinionated defaults:

1. Back up only what cannot be rebuilt.
2. Exclude caches by default.
3. Install scheduled backups during setup/init path.
4. Provide one-command restore with verification.

## Core Policy

### Durable (default include)

- `~/.patina/personas/**/events/*`
- `~/.patina/connections/**`
- `~/.patina/registry.yaml`
- `~/.patina/vault.age` (if present)
- `~/.patina/recipients.txt` (if present)
- `~/.patina/identity.enc` (if present)
- `~/.patina/mother/projects/*/events.db`

### Rebuildable (default exclude)

- `~/.patina/cache/models/**`
- `~/.patina/cache/repos/**`
- `~/.patina/pipeline/grammar-*/**`
- `~/.patina/mother/logs/**`
- transient runtime sockets/pids under `~/.patina/run/**`

### Optional (opt-in include)

- `~/.patina/mother/projects/*/patina.db`
- `~/.patina/mother/projects/*/runtime.db`
- selected Mother metadata DBs when operators request full-fidelity runtime replay

## Non-Goals

- Replacing Time Machine or enterprise backup products.
- Shipping cloud backup infrastructure.
- Backing up all of `~/.patina` indiscriminately.
- Mutating protocol semantics to fit backup internals.

## CLI Surface

- `patina backup init` - configure destination, schedule, retention, and policy mode.
- `patina backup run` - take a snapshot now.
- `patina backup status` - show last success/failure, retention state, and covered durable sets.
- `patina backup verify` - integrity/manifest validation.
- `patina backup restore [--point <id>]` - restore selected snapshot safely.
- `patina backup prune` - apply retention policy now.

## Solution Shape

1. Add backup command family and config model.
2. Implement durable-policy selector and path resolver.
3. Implement snapshot writer (manifest + checksums + archive payload).
4. Implement scheduler integration through setup/init workflow.
5. Implement restore with Mother-safe lifecycle choreography.
6. Add verification and corruption reporting.
7. Add docs/recovery drill and end-to-end proof test.

## Verification

```bash
cargo check --workspace -q
cargo test -q --lib

# Functional proof in temp HOME sandbox
patina backup init --dest <tmp> --schedule off
patina backup run
patina backup verify
patina backup restore --point <latest>
```

## Build Readiness

- [ ] Durable/rebuildable policy codified in one module with tests.
- [ ] Backup manifest schema versioned.
- [ ] Restore is idempotent and safe on partial failures.
- [ ] Setup flow can install scheduler non-interactively.
- [ ] Recovery drill documented in user-facing help/docs.
