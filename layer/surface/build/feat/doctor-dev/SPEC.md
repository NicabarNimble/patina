---
type: feat
id: doctor-dev
status: abandoned
created: 2026-02-06
sessions:
  origin: 20260206-060219
related:
- layer/surface/build/feat/spec-drift-detection/SPEC.md
- layer/surface/build/explore/observability/SPEC.md
- layer/surface/build/fix/spec-tree-and-cycles/SPEC.md
- layer/surface/build/fix/eval-repair/SPEC.md
beliefs:
- stale-context-is-hostile-context
- process-checkpoints-catch-what-tooling-misses
- measure-the-measurement
---

# feat: Doctor Dev — Full State Review at Session End

> The beads "deacon patrol" pattern: periodic sweep catches what hooks and watchers miss.
> Doctor --dev is patina's patrol — a full state review that runs at session boundaries.

## Problem

Patina has one enforcement path: the human runs `patina spec status`. If they forget, the
system drifts. This already happened with auto-bump (code existed, nobody ran the command)
and with spec-as-work-item (8/8 exit criteria met, status still "active").

**Beads' redundant observation pattern:**

| Observer | When | What |
|----------|------|------|
| Daemon hook | On state change | Reacts immediately |
| Witness verification | After work | Verifies correctness |
| **Deacon patrol** | **Periodic sweep** | **Catches everything else** |

Patina needs the patrol. `doctor --dev` is that patrol — a comprehensive state review that
runs at session boundaries and dumps findings into the session record.

---

## Design

### `patina doctor --dev`

A full development state review that checks everything a developer should know before
ending a session:

```
$ patina doctor --dev

🔍 Development State Review

Spec Health:
  ✅ 8 specs with consistent status
  ⚠️  spec-as-work-item: 8/8 exit criteria checked but status=active
     → Run: patina spec status spec-as-work-item complete
  ⚠️  scrape-layer-unify: 2 unchecked items (--only flag, DataContract)
     → 1 deferrable, 1 blocked by system-introspection
  ❌ spec-system: 2/13 criteria met, no commits in 11 days → STALE

Dependency Graph:
  ✅ No circular dependencies
  ⚠️  cli-reorganization blocked by system-introspection (draft)

Uncommitted Work:
  Modified: Cargo.lock
  Untracked: layer/sessions/20260205-213927.md

Retrieval Health:
  Last scrape: 2 hours ago (6220 items)
  Stale patterns: 3 specs with >30 day drift
  Belief coverage: 68 beliefs, 14 with code reach

Session Summary:
  Commits: 5
  Files changed: 10
  Specs touched: 2
  Beliefs captured: 0
```

### Integration with Session End

`patina session end` runs `doctor --dev` and injects the output into the active session
markdown before archiving:

```markdown
## Doctor Review (auto-generated at session end)

Spec Health:
  ⚠️  spec-as-work-item: 8/8 exit criteria checked but status=active

Uncommitted Work:
  Modified: Cargo.lock

Session Summary:
  Commits: 5, Files changed: 10
```

**Order of operations:**
1. `session end` called
2. `doctor --dev` runs, produces review text
3. Review injected into active-session.md under `## Doctor Review`
4. Session archived (markdown finalized, git tagged)
5. User sees review in terminal output

### Frequency: 1/3 Session Ends

To avoid friction, `doctor --dev` runs on ~1/3 of session ends:

```rust
// Simple: hash session ID, run if hash % 3 == 0
fn should_run_doctor(session_id: &str) -> bool {
    let hash = session_id.bytes().fold(0u32, |acc, b| acc.wrapping_add(b as u32));
    hash % 3 == 0
}
```

When skipped, session end works exactly as today. When triggered, adds ~1-2 seconds
for the state review. User can also run `patina doctor --dev` manually at any time.

**Override:** `patina session end --doctor` forces review. `patina session end --no-doctor`
skips it.

---

## What Doctor --dev Checks

### Spec State Contradictions (from spec-drift-detection)

| Check | Signal | Severity |
|-------|--------|----------|
| All checkboxes checked, status != complete | Status contradiction | ⚠️ Warning |
| Status = active/in_progress, no commits in 30+ days | Stale work | ⚠️ Warning |
| Status = complete, unchecked exit criteria remain | Premature closure | ❌ Critical |
| Circular dependencies in spec_deps | Deadlocked queue | ❌ Critical |

### Session Hygiene

| Check | Signal | Severity |
|-------|--------|----------|
| Uncommitted changes | Work may be lost | ⚠️ Warning |
| Untracked files in layer/ | Patterns not committed | ⚠️ Warning |
| 0 beliefs captured in session with 5+ commits | Missed learning | ℹ️ Info |
| Specs touched in commits but not updated | Possible spec drift | ⚠️ Warning |

### Knowledge Health

| Check | Signal | Severity |
|-------|--------|----------|
| Last scrape > 4 hours ago | Stale index | ℹ️ Info |
| Temporal drift on specs (updated vs commits) | Spec drift | ⚠️ Warning |

---

## Relationship to Other Specs

| Spec | Relationship |
|------|--------------|
| **spec-drift-detection** | Doctor --dev USES temporal drift + status contradiction checks |
| **spec-tree-and-cycles** | Doctor --dev USES cycle detection |
| **eval-repair** | Doctor --dev could surface product metrics (Phase 4) |
| **mother-children** | Future: mother could trigger doctor --dev via watcher |

**Implementation dependency:** spec-drift-detection Phase 1+2 should land first to give
doctor --dev the spec health checks. But doctor --dev can ship with just session hygiene
checks initially, adding spec health when drift detection lands.

---

## Exit Criteria

- [ ] `patina doctor --dev` runs full development state review
- [ ] Spec state contradiction detected (all-checked-but-not-complete)
- [ ] Output injected into active-session.md at session end
- [ ] Runs on ~1/3 session ends (hash-based, deterministic per session)
- [ ] `--doctor` / `--no-doctor` flags on `session end`
- [ ] At least 1 real contradiction caught during testing

---

## Open Questions

1. **Should doctor --dev block session end on critical findings?**
   - Option A: Always advisory (show findings, end anyway)
   - Option B: Block on critical, advisory on warning
   - Leaning toward A — advisory only, don't block the human

2. **Should findings be machine-parseable?**
   - `--json` output for agent consumption
   - Useful if mother-children agents consume doctor output

---

## Files to Change

```
src/commands/doctor.rs          # Add --dev flag, state review logic
src/commands/session/
├── mod.rs                      # Wire doctor --dev into end flow
└── internal.rs                 # Inject review into active session markdown
```
