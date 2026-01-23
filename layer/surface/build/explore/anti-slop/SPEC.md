---
type: explore
id: anti-slop
status: design
created: 2026-01-23
updated: 2026-01-23
sessions:
  origin: 20260123-050814
  work: []
related:
  - layer/surface/build/spec-epistemic-layer.md
  - layer/surface/epistemic/beliefs/
---

# explore: Signal Over Noise

> Patina: Local-first quality layer that surfaces signal over noise. Captures understanding alongside implementation. Completes git, doesn't compete with it.

**Problem:** Open source faces increasing noise across all contribution surfaces - not just code PRs, but issues, discussions, and docs. AI amplifies this by generating content that's syntactically correct but context-free. Git tracks *what* changed but not *why* or *under what understanding*.

**Thesis:** Spec captures understanding. Git captures implementation. Integrating these creates a fuller picture - and a potential trust signal for quality contributions.

---

## Exit Criteria

- [x] Problem space documented (noise types across surfaces)
- [x] Patina's existing capabilities mapped to signal/noise filtering
- [x] Core mechanism identified (linkage as signal, not new tools)
- [x] Honest limitations documented
- [ ] Linkage discipline documented (commit conventions, spec references)
- [ ] Demonstrated on Patina repo (this spec → this session → commits)

---

## The Core Insight

**The signal IS the linkage.**

A quality contribution can be traced:

```
Spec (why this exists)
    ↓ links to
Session (work record)
    ↓ links to
Commit (implementation)
    ↓ links to
Diff/Code (grounded change)
```

If these connections exist and are coherent, that's signal. If they don't, that's noise.

**No new tools needed.** Just discipline in linking existing artifacts:
- Specs link to sessions (`sessions: origin: YYYYMMDD`)
- Sessions link to commits (activity logs, tags)
- Commits reference specs (`implements explore/anti-slop`)
- Code changes are grounded in diffs

The question becomes: **Can you trace this code change back to a spec that explains why?**

---

## The Integration Model

Git solved **what changed**. Patina solves **why, under what understanding, aligned with what beliefs**.

```
┌─────────────────────────────────────┐
│  Frontend: Claude Code, editors     │  ← Where work happens
└─────────────────────────────────────┘
                ↓
┌─────────────────────────────────────┐
│  Patina: Local quality layer        │  ← Understanding capture
│  - Specs (problem/solution)         │
│  - Beliefs (alignment)              │
│  - Sessions (provenance)            │
└─────────────────────────────────────┘
                ↓
┌─────────────────────────────────────┐
│  Git: Version control               │  ← Implementation capture
└─────────────────────────────────────┘
                ↓
┌─────────────────────────────────────┐
│  Backend: GitHub/GitLab/Gitea       │  ← Distribution
└─────────────────────────────────────┘
```

**Key properties:**
- **Branch/fork compatible** - Patina is just files in a repo. No protocol changes.
- **Local-first** - No server dependency, works offline, your knowledge stays yours.
- **Backend agnostic** - Works with any git-compatible backend.
- **Mac-centric (for now)** - Worth the tradeoff. Ship excellent on one platform, expand later.

---

## The Trust Layer Thesis

If Patina proves itself as a quality signal on its own repo, "made with Patina" could become a trust marker:

1. **Dogfood** - Patina uses Patina, builds track record
2. **Empirical evidence** - Patina-assisted contributions have better outcomes (fewer reverts, faster reviews, less churn)
3. **Reputation transfer** - "Made with Patina" becomes a signal maintainers can weight
4. **Adoption flywheel** - More repos use Patina → more data → stronger signal

Not cryptographic proof. **Empirical track record.**

### What "Proving Itself" Looks Like

Measurable outcomes for Patina's own repo:
- PR quality (review cycles needed)
- Revert rate (contributions that stick)
- Time to merge (faster with context?)
- Duplicate issue rate (scry catching repeats)

Build the case study on ourselves first.

---

## Noise Across Surfaces

| Surface | Noise Forms | Signal Characteristics |
|---------|-------------|----------------------|
| **Code PRs** | Generic changes, context-free | Aligns with patterns, references beliefs |
| **PR descriptions** | Vague "improved X", AI boilerplate | Explains why, references project context |
| **Issues** | Generic bug reports, AI feature requests | Specific, relates to existing work |
| **Discussions** | Drive-by opinions, repeated questions | Builds on captured knowledge |
| **Docs PRs** | Surface-level rewording | Addresses actual gaps, matches voice |

---

## Two Perspectives

### 1. Patina as a Public Repo
How does Patina protect itself from noise?

### 2. Patina as a Tool
How does Patina help other repos filter signal from noise?

---

## How Patina Helps (Existing Capabilities)

| Capability | Signal/Noise Value |
|------------|-------------------|
| **Scry** | Duplicate detection - "similar to #142, related to belief X" |
| **Beliefs** | Alignment filter - "conflicts with `sync-first`" |
| **Sessions** | "Already explored" detector - "covered in session 20250815" |
| **Context** | Triage assist - surface relevant patterns for review |
| **Issues index** | Semantic search across existing issues |

### Scry as Duplicate/Relevance Detector

New issue comes in. Run:
```bash
patina scry "user can't login after password reset"
```
→ "Similar to #142, #89. Related belief: `session-management`"

Surfaces: is this noise (duplicate) or signal (new information)?

### Beliefs as Alignment Filter

Issue requests "add async everywhere". Check against beliefs:
```
⚠️ May conflict with:
- sync-first (confidence: 0.88)
See layer/surface/epistemic/beliefs/sync-first.md
```

Not blocking - surfacing context. Maybe the request is valid and belief should revise. Or maybe it's noise.

### Sessions as "Already Explored" Signal

Proposal for approach X. Scry reveals:
```
Session 20250815-103422: "Explored X, rejected due to Y"
Belief: `not-x` (confidence: 0.75)
```

Saves time - we already thought about this.

---

## Signal Detection Questions

| Question | Patina Capability |
|----------|------------------|
| Have we seen this before? | Scry for similar content |
| Does this align with our direction? | Check against beliefs |
| Did we already decide against this? | Search sessions/defeated beliefs |
| Who has context on this? | Session provenance |
| Is this generic or specific? | Pattern match against project vocabulary |

---

## Linkage As Quality Measure

### What We Already Have

| Artifact | Links To | How |
|----------|----------|-----|
| **Spec** | Sessions | `sessions: origin:` field |
| **Session** | Commits | Activity log, git tags |
| **Belief** | Sessions | `## Evidence` section |
| **Commit** | ??? | Message only (weak) |
| **Code** | ??? | No linkage |

### What's Weak

- **Commits don't reference specs** - "Fixed bug" vs "implements explore/anti-slop"
- **Code doesn't link to justification** - No way to ask "why does this line exist?"
- **Reverse traversal missing** - Can go spec→session→commit, hard to go commit→spec

### Strengthening Linkage (No New Tools)

1. **Commit message discipline** - Reference spec in commits
   ```
   feat: add signal detection

   Implements: explore/anti-slop
   Session: 20260123-050814
   ```

2. **Spec references in code comments** (sparingly)
   ```rust
   // See: layer/surface/build/explore/anti-slop/SPEC.md
   fn evaluate_signal() { ... }
   ```

3. **Session activity logs** - Already capture commits, keep doing it

4. **Scry for traversal** - Use existing semantic search to find related specs/sessions

### The Quality Question

For any contribution, ask:

> Can I trace this change back to a spec that explains why it exists?

- **Yes, full chain** → Signal (understanding demonstrated)
- **Partial chain** → Review needed (some context, gaps remain)
- **No chain** → Noise (context-free, generic)

---

## Asymmetric Friction Model

Noise economics require: generate quickly, submit to many projects, hope some stick.

**Anything requiring project-specific engagement breaks this model** - not cryptographic security, just doesn't scale for noisy actors.

Goal: Friction **low for signal** (Patina makes it easy to learn project context) and **high for noise** (requires engagement noisy actors won't do).

---

## What Can't Be Faked (Easily)

| Signal | Fakeable? | Notes |
|--------|-----------|-------|
| Process followed | Yes | Tools don't know who uses them |
| Intent stated | Yes | Words are cheap |
| Project-specific knowledge | Harder | Requires actual engagement |
| Outcomes over time | Hardest | Requires track record |

**Limitation:** Bad actor using Patina correctly produces same signals as good actor. Process verification necessary but not sufficient.

---

## Non-Goals (For Now)

- **New detection tools** - Bots, CI integrations, automated scoring
- **ZK proofs of understanding** - Cryptographic verification
- **On-chain reputation** - Outcome tracking, slashable stake
- **Proof of personhood** - Anti-sybil mechanisms
- **Git blame for intent** - Structured intent metadata in commits

The answer is **linkage discipline**, not new infrastructure. Use existing systems (specs, sessions, commits, scry) and connect them well.

See [[design.md]] for extended exploration of deferred ideas.

---

## Honest Limitations

This filters **lazy noise**, not **determined bad actors**.

Someone willing to engage with Patina could still submit garbage with plausible context. But that's a smaller threat surface.

**Goal isn't perfect filtering. It's raising signal-to-noise ratio.**

---

## Open Questions

1. **Adoption** - How do contributors learn Patina before it's widespread?
2. **False positives** - What if genuine contributors don't use Patina?
3. **Gaming** - Once patterns known, can noise generators fake engagement?
4. **Measurement** - How do we know this improves signal/noise ratio?
5. **Barrier height** - Is "use Patina" too high for casual contributors?
6. **Cross-project** - Can noise patterns from one repo inform another?

---

## Status Log

| Date | Status | Note |
|------|--------|------|
| 2026-01-23 | design | Initial exploration from session discussion |
| 2026-01-23 | design | Expanded from "anti-slop" to "signal over noise" across all surfaces |
| 2026-01-23 | design | Added trust layer thesis and integration model |
| 2026-01-23 | design | Reframed: linkage as signal, not new tools |

---

## See Also

- [[design.md]] - Extended thinking on git blame for intent, ZK ideas
- [[spec-epistemic-layer.md]] - Beliefs/patterns foundation
- [[session-20260123-050814]] - Origin discussion
