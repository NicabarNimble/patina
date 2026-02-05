---
type: explore
id: cli-commands
status: active
created: 2026-02-05
sessions:
  origin: 20260205-084522
related:
  - layer/surface/build/feat/cli-reorganization/SPEC.md
  - layer/surface/build/feat/system-introspection/SPEC.md
---

# explore: CLI Commands

> Understand what every command does before reorganizing.

## Purpose

Walk through all Patina commands to:
1. Document what each command actually does
2. Identify gaps (missing functionality)
3. Identify overlap (redundant commands)
4. Validate the namespace groupings (core, science, dev, infra)
5. Inform the cli-reorganization spec

## Role in Spec Ecosystem

**This is an `explore` document, not a `feat` spec.** It has no exit criteria or version targets.

| Concern | Role |
|---------|------|
| cli-reorganization | Owns command groupings (core, science, dev, infra) |
| system-introspection | Owns DataContract schema |
| **this doc** | Working notes that inform those specs |

**After alignment:** This document graduates to `layer/surface/reference/cli-commands.md` as living documentation, maintained alongside code.

## Mental Model

```
CORE (top-level)     The capture → index → query → learn loop
                     scrape, oxidize, scry, context, assay, session, belief, persona

patina science       Is it working? How well? Compare alternatives.
                     eval, bench, compare, feedback, config

patina dev           How is Patina built? (for contributors)
                     introspect, doctor, report, contracts

patina infra         Infrastructure setup and management
                     init, adapter, model, mother, repo, secrets, rebuild, upgrade, version, spec, yolo
```

## Exploration Files

| File | Commands | Status |
|------|----------|--------|
| [core.md](core.md) | scrape, oxidize, scry, context, assay, session, belief, persona | TODO |
| [science.md](science.md) | eval, bench, compare, feedback, config | TODO |
| [dev.md](dev.md) | introspect, doctor, report, contracts | TODO |
| [infra.md](infra.md) | init, adapter, model, mother, repo, secrets, rebuild, upgrade, version, spec, yolo | TODO |

## Questions to Answer Per Command

For each command, document:

1. **What does it do?** (one sentence)
2. **What does it read?** (sources)
3. **What does it write?** (sinks)
4. **Who uses it?** (user, dev, both)
5. **When is it used?** (frequency: every session, periodic, rare)
6. **What's missing?** (gaps)
7. **What overlaps?** (with other commands)

## Findings

<!-- Populate as we explore -->

### Gaps Identified
- (to be filled)

### Overlaps Identified
- (to be filled)

### Namespace Validation
- (to be filled)

---

## Status Log

| Date | Status | Note |
|------|--------|------|
| 2026-02-05 | active | Created structure for CLI command exploration |
| 2026-02-05 | active | **Spec alignment:** Clarified role as exploration doc, not feat spec. Feeds into cli-reorganization and system-introspection. Graduates to reference doc after alignment. |
