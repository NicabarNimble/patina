---
type: refactor
id: version-semver-alignment
status: complete
created: 2026-01-31
sessions:
  origin: 20260131-093100
related:
  - feat/v1-release
  - fix/session-092-hardening
---

# refactor: Align Version Model with Semver Convention

## Problem

Patina's current version model maps `Phase.Milestone` to `MAJOR.MINOR.PATCH`:

```
0.9.0  phase 9, milestone 0
0.9.1  phase 9, milestone 1 (version system)
0.9.2  phase 9, milestone 2 (sessions)
0.9.3  phase 9, milestone 3 (epistemic)
```

This consumes the PATCH position for planned milestones, leaving no room for
bugfix releases between milestones. After releasing 0.9.2, we discovered 6
bugs that needed fixing — but had no version slot to release them independently
of the next feature milestone.

Every major open source project (Rust, Go, Kubernetes, Django, Linux) reserves
PATCH for bugfixes:

```
MAJOR.MINOR.PATCH
  x  .  y  .  z

MAJOR (x.0.0) — stability commitment or breaking change
MINOR (0.x.0) — new functionality (milestones)
PATCH (0.0.x) — bugfixes only
```

## Solution

Shift milestones to MINOR version bumps. PATCH is reserved for fixes.

### Migration

No history renumbering. Accept that 0.9.0-0.9.2 used the old convention.
Start the new model from the next milestone forward:

```
Old (already released):
0.9.0  ✓ Public release
0.9.1  ✓ Version system alignment
0.9.2  ✓ Session system & adapter parity

New (going forward):
0.9.3       Fix: session 0.9.2 hardening (patches from this session)
0.10.0      Epistemic E4 (belief automation)
0.10.1      (reserved for fixes discovered after 0.10.0)
0.11.0      Mother federated query
0.12.0      Dynamic ONNX loading
0.13.0      WASM grammars
0.14.0      GitHub releases + Homebrew
1.0.0       All pillars complete — stability commitment
```

After 1.0.0, the same model continues: 1.1.0 = feature, 1.1.1 = fix, 2.0.0 = breaking.

### Fix Spec Metadata

Fix specs gain an `affects_since` field linking to the version that introduced
the bug. The fix ships on the current version (trunk model — no backports):

```yaml
---
type: fix
id: session-092-hardening
affects_since: 0.9.2
related:
  - feat/v1-release
---
```

### Commands

- `patina version milestone` — bumps MINOR (0.9.x → 0.10.0), requires a spec
- `patina version patch` — NEW: bumps PATCH (0.10.0 → 0.10.1), requires a fix spec
- `patina version phase` — bumps MAJOR (0.x.y → 1.0.0), stability commitment

### v1-release SPEC Update

The milestone list in `feat/v1-release/SPEC.md` needs to be updated to reflect
the new numbering. Milestone names stay the same, version numbers shift.

## Deliverables

1. Update `patina version milestone` to bump MINOR instead of PATCH
2. Add `patina version patch` command for fix releases
3. Update `feat/v1-release/SPEC.md` milestone table
4. Update fix spec template to include `affects_since` field
5. Update Cargo.toml version to 0.9.3 for current fix batch

## Exit Criteria

- [x] `patina version milestone` bumps 0.10.0 → 0.11.0 (not 0.10.0 → 0.10.1)
- [x] `patina version patch` bumps 0.10.0 → 0.10.1
- [x] Fix specs have `affects_since` field
- [x] v1-release SPEC reflects new numbering
- [x] Existing 0.9.0-0.9.2 tags untouched
