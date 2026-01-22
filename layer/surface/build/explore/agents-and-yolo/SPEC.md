---
type: explore
id: agents-and-yolo
status: open
created: 2026-01-21
session-origin: 20260121-170710
---

# explore: Agents and Autonomous Workspaces

**Problem:** Two concepts need clarity: (1) Should patina have a first-class "agent" concept distinct from adapters? (2) Does `yolo` (1,613 lines of devcontainer generation) belong in patina or should it be extracted/removed?

**Goal:** Decide the fate of yolo and whether to build an agent system.

---

## Quick Reference

| Component | Lines | Question |
|-----------|-------|----------|
| `patina yolo` | 1,613 | Keep / extract / remove? |
| Agent concept | 0 | Build / defer indefinitely? |
| `awaken` binary | 0 | Split shipping tools out? |

---

## Status

- **Phase:** Exploration (not implementation)
- **Blocking:** Nothing
- **Related:** spec/remove-codex (archived), three-layers spec

---

## Open Questions

1. **What is an "agent" in patina's world?**
   - A subprocess spawned by an adapter?
   - A background task (like forge sync)?
   - A separate CLI tool?

2. **Does yolo belong in patina?**
   - Pro: Enables autonomous AI development vision
   - Con: Devcontainer generation isn't RAG/context orchestration

3. **Should there be an "awaken" binary?**
   - three-layers spec suggests: mother (infra), patina (product), awaken (shipping)
   - `build`, `test`, `yolo`, `deploy` would live in awaken
   - Note: build/test being removed (see refactor/remove-dev-env)

4. **What would agents do?**
   - Background indexing?
   - Autonomous code review?
   - Continuous scrape/oxidize?

---

## Exploration Tasks

- [ ] Review session history for agent vision
- [ ] Survey: Is yolo actually used?
- [ ] Decide: Extract yolo vs remove vs keep
- [ ] Decide: Build agent system vs defer indefinitely

---

## Possible Outcomes

- Remove yolo entirely
- Extract yolo to standalone tool
- Build agent system with yolo as first agent
- Keep yolo as-is, freeze development

---

See [[design.md]] for historical context and detailed analysis.
