# Design: Patina Durable Backup System

## Why This Design

Patina separates protocol truth (`layer/`) from local runtime state (`~/.patina`).
That split is good for portability, but it creates an operational risk: a single
`rm -rf ~/.patina` can remove durable local knowledge that is not represented
in git-tracked layer artifacts.

This design adds a native backup plane that is:

- policy-aware (durable vs rebuildable),
- lightweight (local snapshots first),
- recoverable under stress (single restore command),
- aligned with Patina core values (simple interfaces, protocol unchanged).

## Data Classification Model

The backup engine uses explicit classes:

- `durable`: must preserve; default include.
- `rebuildable`: safe to drop; default exclude.
- `optional`: operator chooses by profile.

Classification is encoded in one policy module and emitted in every backup
manifest so operators can audit exactly what was protected.

## Proposed Architecture

```
patina backup *
  |
  +-- policy resolver
  |     +-- durable path set
  |     +-- rebuildable exclusions
  |     +-- optional selectors
  |
  +-- snapshot writer
  |     +-- manifest.json (policy/version/paths/counts/bytes)
  |     +-- checksums.json (sha256 by file)
  |     +-- payload archive (compressed tar)
  |
  +-- scheduler adapter
  |     +-- launchd (macOS)
  |     +-- systemd user timer (linux preferred)
  |     +-- cron fallback
  |
  +-- restore orchestrator
        +-- stop mother (if running)
        +-- restore files + permissions
        +-- verify checksums
        +-- start mother (optional)
        +-- report health checks
```

## Backup Point Format

Directory layout (example):

```
~/.patina/backups/20260331-073000/
  manifest.json
  checksums.json
  payload.tar.zst
```

Manifest includes:

- backup id, created timestamp, policy version,
- included classes and resolved path set,
- excluded class list,
- file count + byte size,
- tool version and host info,
- verify status.

## Scheduler Integration

`patina backup init` installs a job by default (opt-out supported):

- macOS: launchd agent under `~/Library/LaunchAgents/com.patina.backup.plist`
- Linux: systemd user timer when available, cron otherwise

Default cadence: hourly lightweight snapshots with retention (e.g. 24 hourly,
14 daily, 8 weekly).

## Restore Safety Contract

Restore flow is transactional-at-operator-level:

1. Create pre-restore safeguard copy of current `~/.patina` (timestamped).
2. Stop Mother if active.
3. Restore payload.
4. Apply secure permissions (`~/.patina` and `run/` 0700).
5. Verify checksums and required durable files.
6. Start Mother (unless `--no-start`).
7. Print post-restore checks.

If verification fails, the command exits non-zero with exact missing/corrupt
entries and keeps the safeguard copy for rollback.

## Command Behavior Details

- `backup run`: creates one snapshot using current policy profile.
- `backup verify`: checks manifest completeness and checksum integrity.
- `backup status`: shows last success/failure, age, retention health.
- `backup prune`: retention compaction only; never touches latest snapshot.
- `backup restore`: supports selecting point by id or `latest`.

## Security Notes

- Snapshot payload should be encrypted at rest by default when secrets class is
  included (age with local identity, or operator-provided recipient).
- Restore must never print secret values; only file metadata.

## Implementation Order

1. `feat(backup): add backup command scaffold and config model`
2. `feat(backup): implement durable policy resolver + tests`
3. `feat(backup): implement snapshot writer (manifest/checksum/archive)`
4. `feat(backup): add verify and status commands`
5. `feat(backup): add restore orchestrator and safe rollback behavior`
6. `feat(backup): add scheduler installers (launchd/systemd/cron)`
7. `docs(backup): add recovery drill and operator playbook`

## Verification Plan

Functional acceptance:

1. Create sandbox HOME with synthetic durable + rebuildable data.
2. Run `patina backup run`.
3. Delete sandbox `~/.patina`.
4. Run `patina backup restore`.
5. Verify durable files restored and rebuildable exclusions respected.
6. Run `patina mother status`, `patina repo list`, `patina connect list`.

Regression coverage:

- policy resolver tests,
- manifest schema stability tests,
- checksum corruption detection tests,
- restore idempotency tests.

## Open Questions

- Should backup destination default inside `~/.patina/backups` or require
  separate destination on first init for better disk-loss resilience?
- Should secrets-class backups require explicit `--include-secrets` on first
  run, even though they are durable?
- How aggressively should retention prune by default for laptop disk budgets?
