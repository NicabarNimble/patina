# Spec Workflow Walkthroughs

> UX scenarios showing how a user and Claude Code interact with the spec
> system through the three-layer architecture: user talks naturally → LLM
> (skill layer) decides what to invoke → CLI (execution layer) does the work.

## 1. The Fresh Idea

> Requires: Phase 0 (spec-create), Phase 3 (spec next), Phase 6 (/spec skill)

User discovers a problem during work and wants to capture it.

```
User: hey so i just realized our error messages are terrible. when scrape
      fails on a file it just says "parse error" with no context. we should
      fix this across the board.

Claude: That sounds like a good candidate for a spec. Want me create one?
        I'd frame it as a fix spec — improving error messages across scrape
        parse failures.

User: yeah do it

Claude: [runs: patina spec create error-message-quality --type fix]

        Created: layer/surface/build/feat/error-message-quality/SPEC.md
        Status: draft

        I've scaffolded the spec with the problem statement from our
        conversation. Want to keep designing it now, or come back to it
        later?

User: later, lets keep working on what we were doing

Claude: Sounds good. It's sitting in draft — `spec next` will surface it
        when you're ready.
```

**What happened:** The `/spec` skill recognized a problem worth capturing.
`spec create` scaffolded a draft from conversation context. No workflow
disruption — user stays on their current work.

## 2. The Morning Startup

> Requires: Phase 1 (pause/block statuses), Phase 3 (spec next), Phase 5 (session integration)

User begins a session and needs to know what to work on.

```
User: /session-start morning work

Claude: Session started on branch: patina

        Spec landscape:
          Active:  spec-workflow-rigor
          Paused:  spec-git-tag-system (5d) — resolve before pausing another
          Blocked: spec-knowledge-evolution (waiting on spec-workflow-rigor)
          Drafts:  2 available (error-message-quality, measurement-coverage)

        Recommended: continue spec-workflow-rigor (active, blocks 2 others)

        Also: spec-git-tag-system has been paused 5 days. Resume, split,
        or abandon it?

User: lets keep going on workflow-rigor. i'll deal with git-tag-system
      after this lands.

Claude: Makes sense — workflow-rigor unblocks both knowledge-evolution and
        git-tag-system. Let me pull up where we left off.

        [runs: patina spec resume context for spec-workflow-rigor]

        Since you started this spec:
          Your work: 12 files changed, 340 insertions
          Codebase changes: 3 commits since last touch
          Current phase: Phase 1 — Command Decomposition

        The `spec promote` command is implemented. Next up is `spec pause`.
        Want to pick up there?
```

**What happened:** `spec next` surfaced the recommended work with reasoning.
Queue pressure showed the paused spec. Context diffs helped the LLM
reconstruct where work left off. No archaeology needed.

## 3. The Wander-and-Return

> Requires: Phase 0 (spec create), Phase 1 (pause/block commands)

User is mid-implementation and discovers a prerequisite is missing.

```
User: ok so im trying to implement spec pause and the WIP commit logic
      needs to check for merge conflicts but we dont have a
      has_merge_conflicts() helper in the git module

Claude: Right — that's a gap we identified in D1. It's a small addition
        to src/git/operations.rs. Want me to add it now? It's part of
        Phase 1 scope.

User: actually wait. looking at this more, the whole git operations module
      is a mess. half the functions take a path, half assume cwd, theres
      no consistent error handling. if i add another helper it makes it
      worse. we should clean that up first.

Claude: That's a different scope than workflow-rigor. Two options:

        1. **Block workflow-rigor** on a git-operations cleanup spec, then
           do the cleanup first
        2. **Add the minimal helper now** (has_merge_conflicts), take a note
           about the cleanup, and come back to it

        The cleanup is real but workflow-rigor doesn't need ALL of git/ops
        fixed — just the one helper. What do you think?

User: no youre right, the cleanup is real but i dont want to get pulled
      off. add the helper, create a draft spec for the cleanup.

Claude: [runs: patina spec create git-operations-cleanup --type refactor]

        Created draft: git-operations-cleanup

        Now adding has_merge_conflicts() to src/git/operations.rs and
        continuing with spec pause.
```

**What happened:** The LLM recognized scope creep and offered options. User
made the judgment call — minimal fix now, captured the real work as a draft.
No workflow disruption. The draft sits in the queue until the user is ready.

## 4. The Block

> Requires: Phase 1 (block command, get_blocked_specs fix)

User hits a real dependency — can't continue without another spec landing.

```
User: ok so spec pause works, spec resume works, now im doing spec block
      and the blocked_by field updates the DB inline... but when i look at
      get_blocked_specs() it only uses the spec_deps JOIN. it doesnt check
      the status field at all. so if i set status: blocked but the spec_deps
      entry is missing, the spec falls through the cracks.

Claude: That's the gap we found in the code analysis — `get_blocked_specs()`
        needs to account for `status: blocked` too, not just `spec_deps`.
        That's Phase 1 scope, should we fix it now?

User: yes but actually this connects to something bigger. the whole query
      side (spec ready, spec blocked, spec list) needs to be updated for
      the new statuses. and spec-knowledge-evolution wants to add belief
      lifecycle filters to the same query path. we should do the query
      updates after all the new statuses land.

Claude: So you want to finish the mutation commands first (pause, resume,
        block, complete, abandon), then do a focused pass on the query
        commands. That's natural — mutations define the states, queries
        read them.

        Want me to note this in the spec? I'd add it as a Phase 1b:
        query alignment after mutation commands land.

User: yeah do that. and block the query work on the mutations being done.
      its all phase 1 but theres an ordering.
```

**What happened:** User discovered an ordering dependency within a phase.
The LLM helped frame it (mutations before queries) and updated the spec.
No status change — still active, just refined the implementation plan.

## 5. The Split

> Requires: Phase 2 (spec split)

User realizes a spec is half-done and half-wrong.

```
User: ok look. ive been working on measurement-coverage for two weeks now.
      the event infrastructure works great — all 5 verbs have producers,
      eventlog schema is clean, patina measure shows the summary. but the
      consumer views... the user view vs maintainer view distinction is way
      more complex than i thought. i keep going back and forth on what to
      show. and the plugin measurement API is a whole thing.

Claude: Sounds like Phase 1 (infrastructure) is solid but Phases 2-4
        (views, plugins, regression) need more design time. Want to split?

        That would:
        1. Complete measurement-coverage with what's done (Phase 1 — event
           infrastructure, all verbs have producers, basic `patina measure`)
        2. Create measurement-views-and-plugins as a new draft for the
           consumer views, plugin API, and regression detection

        The done half gets a real release. The undone half goes back to
        draft.

User: yeah thats the right call. ship what works.

Claude: [runs: patina spec split measurement-coverage
               --id measurement-views-and-plugins]

        Split complete:
          measurement-coverage completed (v0.24.0, archived)
            Tag: spec/measurement-coverage-v1-complete
          measurement-views-and-plugins created as draft
            split_from: measurement-coverage
            Body: Phases 2-4 from parent spec

        The infrastructure is shipped. The views and plugin API go back
        through the lifecycle from draft.
```

**What happened:** User recognized the half-done pattern. The LLM suggested
split with a clear framing of what's done vs what needs more design. `spec
split` shipped the working half (real release) and sent the rest back to
draft. No work lost — provenance chain tracks the lineage.

## 6. The Completion

> Requires: Phase 1 (complete command), Phase 3 (spec next), Phase 5 (session integration)

User finishes a spec and sees the cascade.

```
User: ok spec promote and spec complete are both working, tests pass,
      everything is wired up. phase 1 is done. lets ship it.

Claude: Nice. Let me complete the spec.

        [runs: patina spec complete spec-workflow-rigor]

        Completed: spec-workflow-rigor
          Release: v0.25.0 (patch bump)
          Tag: spec/spec-workflow-rigor
          Archived: layer/surface/build/feat/spec-workflow-rigor/SPEC.md

        This unblocks 2 specs:
          spec-knowledge-evolution (was blocked, now draft — ready to promote)
          git-tag-system (was blocked, now draft — ready to promote)

        `spec next` recommends: spec-knowledge-evolution (high priority,
        was blocked longest). Also: spec-git-tag-system has been paused
        5 days — consider resolving it.

User: lets pick up knowledge-evolution next session. end this one.

Claude: [runs: patina session end]

        Session complete. spec-workflow-rigor shipped, 2 specs unblocked.
```

**What happened:** `spec complete` triggered the full pipeline: release,
archive, git tag. The cascade showed what was unblocked. `spec next` gave
a recommendation for what to work on next. Clean session boundary.

---

## What These Walkthroughs Demonstrate

1. **The LLM is the judgment layer** — it recognizes when to create, pause,
   block, split, or complete. The user talks naturally.
2. **Commands are deterministic tools** — explicit params, no inference,
   `--json` output. The LLM calls them after conversation reaches a decision.
3. **Queue pressure works** — paused specs show age, blocked specs show
   blockers, `spec next` recommends with reasoning.
4. **No workflow disruption** — creating drafts, noting cleanup work, and
   capturing ideas all happen without derailing current work.
5. **Git preserves everything** — every transition tagged, WIP commits on
   pause, archives recoverable, split provenance tracked.
6. **The `/spec` skill is the single entry point** — one skill, full menu,
   LLM reads it once and knows what's available.
