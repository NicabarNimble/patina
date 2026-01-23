# Versioning Policy

> How Patina versions releases, designed around milestone-driven development.

**Adopted:** 2026-01-23
**Model:** Milestone-Based Semantic Versioning

---

## The Rule

```
MAJOR.PHASE.MILESTONE

MAJOR (0.x → 1.x): Production-ready declaration (1.0 = public with contributors)
PHASE (0.x.0 → 0.y.0): Major development era begins
MILESTONE (0.x.y → 0.x.z): Significant completion within the phase
```

**Example progression:**
```
0.3.0 = Language support phase started
0.3.1 = C/C++ complete
0.3.2 = Go complete
0.3.3 = Python complete
...
0.3.9 = 9/9 languages done
0.4.0 = New phase (cleanup) begins
```

---

## What Is a Phase?

A phase is a major development era with a coherent theme. Starting a new phase bumps the PHASE number.

**Examples of phases (PHASE bump → 0.x.0):**
- Bootstrap (getting the project working)
- Architecture (establishing patterns)
- Language Support (adding capabilities)
- Semantic Search (new major feature area)
- Server/MCP (production infrastructure)
- Go Public (open source readiness)

## What Is a Milestone?

A milestone is a significant completion within a phase. It's something you'd write a session-end summary about.

**Examples of milestones (MILESTONE bump → 0.x.y):**
- "C/C++ language complete" within Language phase
- "Dagger removed" within Cleanup phase
- "Scry MVP working" within Semantic phase
- "Beliefs in scry" within Epistemic phase

**Not milestones (no bump):**
- Individual bug fixes (batch into next milestone)
- Documentation updates
- Session archives, belief captures

---

## Pre-1.0 Semantics

While in 0.x.y:
- MAJOR stays at 0 (not production-ready)
- PHASE increments when starting a new development era
- MILESTONE increments for completions within the phase

**1.0.0 criteria (future):**
- Public repo with contributors
- Stable API (commands don't change unexpectedly)
- Documentation complete
- Used in production by someone other than the author

---

## How This Interacts with Git

**Commits:** Use conventional commits (`feat:`, `fix:`, `refactor:`, etc.)
- These document what changed, not when to release

**Tags:** Created at milestone completion
- Format: `v0.x.y`
- Annotated tags with milestone description

**Sessions:** Track the work that leads to milestones
- Sessions are the "why", versions are the "what shipped"

---

## Versioning Cadence

No fixed schedule. Versions happen when milestones complete.

**Typical pattern:**
```
Work in bursts (days/weeks)
    ↓
Hit milestone (capability complete)
    ↓
Tag version
    ↓
Move to next milestone
```

This aligns with how Patina is actually developed - intense sessions leading to coherent completions.

---

## Retroactive Application

This policy is applied retroactively to git history in `version-history.md`. That document identifies milestones from project start and assigns version numbers as if this policy had been in place from the beginning.

The current version reflects this retroactive analysis, not the broken release-plz automation.

---

## Future Automation

When/if release automation works:
- Milestones are still manually identified
- Automation handles the tag/release mechanics
- Policy doesn't change, just the tooling

---

## See Also

- `version-history.md` - Retroactive version assignments
- `git-history-audit.md` - Raw git history analysis
- `SPEC.md` - Go-public specification
