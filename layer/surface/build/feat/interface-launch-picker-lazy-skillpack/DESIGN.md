# Design: Interface launch picker + lazy skillpack ensure

## Why this design

This slice is about making launcher behavior predictable without changing session internals.

Current session envelope logic is already strong. The inconsistency is in launch entry paths:
- `patina` (interactive UX)
- `patina ai <interface>` (direct UX)
- non-project init (`Are you lost?`)

Design goal: one launch engine, two UX shells.

---

## UX contract

### 1) Existing Patina project, interactive TTY

`patina` always shows interface picker.

- default marker: project last-used interface
- fallback: `.patina/config.toml` `interfaces.default`
- user selection launches through same backend as `patina ai <interface>`

### 2) Existing project, non-interactive

No picker. Launch resolved default directly.

### 3) Non-project directory

Keep current banner, clarify prompt wording:

- banner: `Are you lost?`
- prompt: `Initialize this directory as a Patina project? [y/N]`
- `N`/empty (default): exit
- `Y`: continue to HITL interface selection

Then run init + selected-interface setup only.

### 4) Direct command path

`patina ai <interface>` remains direct and non-interactive.

- selected interface only
- lazy ensure before launch
- then existing session envelope flow

---

## State model (Mother-managed project recency)

Mother owns project recency state for launcher defaults.

Proposed keys (runtime state store):
- namespace: `interface-launch`
- key: `<project_uid>/last_interface`
- key: `<project_uid>/last_launch_at`
- key: `<project_uid>/last_launch_mode` (`picker` / `direct` / `init`)

Write on successful launch/check-in.
Read before picker render.

Fallback chain for picker default:
1. Mother `last_interface`
2. project config `interfaces.default`
3. global detected default

This keeps recency out of tracked config while preserving deterministic policy fallback.

---

## Setup + freshness control

### Setup semantics

- default setup path: selected/default interface only
- explicit all-bundle prewarm: `--all` (or equivalent explicit mode)

### Freshness decision

For selected interface only:
1. missing managed projection -> prepare
2. stale managed metadata/version mismatch -> refresh managed projection
3. compatible/current -> no rewrite

No force rewrite unless user asks.

External tool version (`pi --version`, `claude --version`, etc.) is observed and logged, but independent from Patina bundle freshness.

---

## Observability contract (minimal, stable)

Keep launch observability strict and small.

Emit at most one event per logical launch/setup action with correlation fields:
- `project_uid`
- `interface`
- `runtime_id` / `session_id` when created
- `decision_path` (`picker`, `direct`, `init`)
- `bundle_version_before` / `bundle_version_after`
- `tool_version_observed`
- `action` (`noop`, `prepare`, `refresh`, `unknown`)

Do not emit noisy duplicates for the same decision branch.

This preserves auditability for FNIOS-style operational scrutiny while keeping event volume understandable.

---

## Migration / old-project self-heal

Single-user reset path should be automatic on revisit.

On first launch in older projects:
- detect stale/mismatched managed interface projection
- run selected-interface reconcile/refresh
- continue launch without requiring manual cleanup

Older projects should converge to new behavior simply by being opened and launched.

---

## What stays unchanged

- session artifact structure
- session check-in/out semantics
- start/end tagging behavior
- voice behavior (explicitly out of scope)

---

## Implementation slices

1. Launcher behavior matrix + TTY gating
2. Mother `last_interface` state read/write
3. `Are you lost?` prompt wording update
4. selected-only setup in init path
5. selected-interface freshness/reconcile contract
6. observability schema + dedupe discipline
7. migration/self-heal tests + smoke checks

---

## Verification focus

- interactive picker shown in project mode (`patina`)
- non-interactive mode bypasses picker
- init prompt default is no/exit, yes enters interface selection
- selected-only setup during init
- `patina` selection and `patina ai <interface>` produce same launch/result path
- old project revisit self-heals without manual prep
