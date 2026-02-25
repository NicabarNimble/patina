---
type: feat
id: spec-knowledge-evolution
status: draft
created: 2026-02-22
blocked_by:
- test-blocker
sessions:
- 20260222-054702
- 20260223-120524
- 20260223-132543
related:
- src/mother/graph.rs
- src/commands/belief/mod.rs
- src/commands/scrape/beliefs/mod.rs
- src/commands/scrape/beliefs/verification/mod.rs
- src/retrieval/oracle.rs
- src/mcp/server.rs
beliefs:
- mutation-completes-query
- plugins-are-three-prong-bundles
- specs-orthogonal-to-sessions
- refutation-is-discovery
- build-correct-not-temporary
- knowledge-evolution-first-class
exit_criteria: []
---

# feat: Belief Lifecycle — Hypothesis, Validation, Refutation

> Beliefs need lifecycle states. Today all beliefs are treated as validated
> truth. A hypothesis captured during a session looks identical to a principle
> proven across 50 sessions. This spec adds lifecycle to beliefs so the
> knowledge graph distinguishes theory from fact, preserves refutations as
> learning, and tracks evolution chains.

**Scope: beliefs only.** Spec lifecycle is owned by [[spec-workflow-rigor]].
This spec does not modify spec statuses, spec commands, or spec schemas.

## Problem

### The Knowledge Pollution Cycle

**What happened in session 20260222-054702:**

1. **Hypothesis formed**: "Keychain works over SSH with AlwaysThisDeviceOnly"
2. **Belief captured**: `keychain-always-this-device-only`
3. **Never actually tested**: Assumed working based on theory
4. **Months later**: Empirical testing proves it never worked
5. **Pollution**: False beliefs in knowledge graph, LLMs read them as truth

**The pattern:**
```
Theory → Capture as Truth → Discover Wrong → Pollution
   ↓          ↓                  ↓              ↓
"Should    Belief with no   -25308 error   Knowledge
 work"     evidence marker  in SSH          graph lies
```

**Current problems:**

1. **No distinction between hypothesis and fact**
   - All beliefs treated as validated truth
   - No confidence levels, no evidence tracking
   - `BeliefEntry` already has `evidence_count` and `evidence_verified`
     fields (src/mother/graph.rs) but no lifecycle status

2. **Refutation is exceptional (should be normal)**
   - When proven wrong, we "delete" or "archive"
   - Lose the learning: "Why it failed" disappears
   - Future LLMs retry the same failed approaches

3. **Evolution chains lost**
   - Can't see: "We thought X, tested, discovered Y"
   - No trace of reasoning journey
   - Knowledge appears to come from nowhere

### What Exists Today (Infrastructure Is Partially Built)

**Already in `src/mother/graph.rs` (`BeliefEntry`):**
- `evidence_count: i64` — how many evidence items
- `evidence_verified: i64` — how many verified
- `health_score: f64` — composite health metric
- `contested_by: Option<String>` — contradicting beliefs
- `status: String` — currently just "active"

**Already in `src/commands/scrape/beliefs/verification/mod.rs`:**
- `VerificationQuery` — parse, validate, execute verification queries
- `VerificationResult` — Pass/Contested/Error status per query
- Structural verification already runs during scrape

**Already in `src/commands/belief/mod.rs`:**
- `belief audit` — full health/use/truth metrics per belief
- Sorting by health, staleness detection, grounding analysis
- `--stale`, `--warnings-only`, `--sort health` filters

**Already in belief YAML frontmatter:**
- `supports: [belief-id]` / `attacks: [belief-id]` — relationship graph
- `evidence:` section with session/commit references
- `entrenchment: low | medium | high`

**What's missing:**
- No `hypothesis` / `validated` / `refuted` / `superseded` status values
- No mutation commands — query side exists (audit) but mutation side
  doesn't (hypothesis, validate, refute, supersede, history)
  ([[mutation-completes-query]])
- No lifecycle-aware filtering in context/scry/MCP queries
- No evolution chain tracking (superseded_by, evolved_from)
- No 3-prong architecture for belief commands (no MCP tools, no skill)

### Root Cause

> **"Short-term fixes lead to never fixes. We live in hack-land forever
> unless we build it right the first time."**
> — User insight, session 20260222-054702

## Solution

### Core Design: Beliefs Have Lifecycle States

```
Belief Lifecycle:

hypothesis → validated → superseded
    ↓           ↓            ↓
(Theory)   (Proven)    (Replaced)
    ↓
refuted
    ↓
(Disproven - keep for learning)
```

**Key insight:** Refutation is discovery, not failure. Failed hypotheses are
valuable knowledge ([[refutation-is-discovery]]).

### Belief States

| State | Meaning | Query Default | When |
|-------|---------|---------------|------|
| `hypothesis` | Untested theory, may be wrong | Hidden | Initial capture, before testing |
| `validated` | Empirically proven | Shown | Evidence supports, tests pass |
| `refuted` | Proven wrong, keep for learning | Hidden* | Evidence contradicts |
| `superseded` | Replaced by better belief | Hidden | New belief improves on this |

*Available via `--include-refuted` flag to learn from failures

### Architecture: Three-Layer Capability

Every belief operation exposed through three layers
([[plugins-are-three-prong-bundles]]):

```
┌─────────────────────────────────────────────┐
│  Adapter Skill (/belief)                    │  ← WHEN to act (LLM judgment)
│  Single skill describes full capability.    │
│  LLM reads once, knows what's available.    │
├─────────────────────────────────────────────┤
│  MCP Tools (JSON-RPC typed parameters)      │  ← HOW to call (interface)
│  Same operations as CLI, structured I/O.    │
├─────────────────────────────────────────────┤
│  CLI Commands (Rust, deterministic)         │  ← WHAT happens (execution)
│  Explicit params, --json output.            │
└─────────────────────────────────────────────┘
```

### Command Decomposition

Following the same [[unix-philosophy]] pattern as spec-workflow-rigor.
Each command does one thing.

**Query commands** (read-only, already partially exist):

| Command | Do X |
|---|---|
| `belief audit` | Show health/use/truth metrics (exists) |
| `belief list` | Show all beliefs with filters (new) |
| `belief show <id>` | Show single belief with evidence (new) |
| `belief history <id>` | Show evolution chain (new) |

**Mutation commands** (each does exactly one thing):

| Command | Transition | Side effects |
|---|---|---|
| `belief hypothesis <id>` | → hypothesis | Create with required statement + theory |
| `belief validate <id>` | hypothesis→validated | Evidence required + git commit |
| `belief refute <id>` | hypothesis→refuted | Evidence + "what we learned" required |
| `belief supersede <old> <new>` | any→superseded | Link to replacement + git commit |

**All mutation commands support `--json`** for structured output.

### Belief Schema v2

Extend existing YAML frontmatter. All new fields optional with
`skip_serializing_if` — zero breakage for existing beliefs.

```yaml
---
id: <belief-id>
type: belief
status: hypothesis | validated | refuted | superseded  # NEW (default: validated for migration)
confidence: untested | low | medium | high | disproven  # NEW
created: YYYY-MM-DD

# Lifecycle tracking (NEW)
validated_date: YYYY-MM-DD     # When proven true
refuted_date: YYYY-MM-DD       # When proven false
superseded_date: YYYY-MM-DD    # When replaced

# Evidence (what supports or refutes this)
evidence:
  - type: test | session | commit | measurement
    description: <what was tested>
    result: PASSED | FAILED | INCONCLUSIVE
    ref: <session-id | commit-sha | test-path>
    date: YYYY-MM-DD

# Evolution chains (NEW)
superseded_by: <belief-id>   # What replaced this
evolved_from: [<belief-id>]  # What this evolved from

# Existing fields (unchanged)
supports: [<belief-id>]
attacks: [<belief-id>]
entrenchment: low | medium | high
tags: [<tag>]
sessions: [<session-id>]
---
```

### Updated Query Behavior

```bash
# Default: only validated + active knowledge (clean graph)
patina context keychain
→ Hides: hypothesis, refuted, superseded
→ Shows: Only validated beliefs

# Include hypotheses (experimental knowledge)
patina context keychain --include-hypothesis
→ Shows: validated + hypothesis (marked as unproven)

# Learn from failures (what NOT to try)
patina context keychain --include-refuted
→ Shows: refuted beliefs with "why it failed"
→ Prevents re-attempting failed approaches

# Evolution chain for a belief
patina belief history keychain-ssh
→ Timeline:
  2026-02-18: keychain-ssh-accessibility (hypothesis)
  2026-02-20: keychain-ssh-raw-api (hypothesis)
  2026-02-22: keychain-ssh-accessibility (REFUTED - doesn't work)
  2026-02-22: keychain-ssh-raw-api (REFUTED - same error)
  2026-02-22: dual-storage-strategy (VALIDATED - works)
```

### Migration Strategy

**Grandfather existing beliefs:**
```bash
patina belief migrate

Migrating existing beliefs:
  • No status field → assume `validated` (trust historical work)
  • Add confidence: medium (not empirically re-tested)
  • Add created: <git log date>

Migration complete.
Going forward: ALL new beliefs require lifecycle fields.
```

## Implementation

### Phase 1: Schema & Lifecycle States

**Files to modify:**
- `src/mother/graph.rs` — Add lifecycle fields to `BeliefEntry`
- `src/commands/scrape/beliefs/mod.rs` — Parse new frontmatter fields during scrape
- Belief YAML files — new optional fields (serde defaults, zero breakage)

**New:**
- Lifecycle state validation in belief scraper
- Migration command: `patina belief migrate`

**Exit criteria:**
- [ ] `BeliefEntry` has `status`, `confidence`, lifecycle date fields
- [ ] Scrape parses new fields from YAML frontmatter
- [ ] Existing beliefs parse without error (all new fields optional)
- [ ] `belief migrate` sets status=validated, confidence=medium on legacy beliefs

### Phase 2: Mutation Commands

**Files to modify:**
- `src/commands/belief/mod.rs` — Add subcommands: hypothesis, validate, refute, supersede

**New commands:**
- `patina belief hypothesis <id>` — Create hypothesis belief
- `patina belief validate <id>` — Validate with evidence
- `patina belief refute <id>` — Refute with evidence + learning
- `patina belief supersede <old> <new>` — Mark superseded
- `patina belief history <id>` — Show evolution chain
- `patina belief list` — Show all beliefs with filters

**Exit criteria:**
- [ ] `belief hypothesis` creates belief with status=hypothesis
- [ ] `belief validate` requires evidence, transitions to validated
- [ ] `belief refute` requires evidence + "learned", transitions to refuted
- [ ] `belief supersede` links beliefs and transitions to superseded
- [ ] `belief history` shows evolution chain
- [ ] All mutation commands support `--json`
- [ ] All mutation commands git commit their changes

### Phase 3: Query Filters

**Files to modify:**
- `src/retrieval/oracle.rs` — Add lifecycle status filters
- `src/mcp/server.rs` — Add filter params to context/scry tools
- `src/commands/scrape/beliefs/mod.rs` — Lifecycle-aware materialized views

**Exit criteria:**
- [ ] `patina context` hides hypothesis/refuted/superseded by default
- [ ] `patina scry` respects lifecycle filters
- [ ] `--include-hypothesis` and `--include-refuted` flags work
- [ ] MCP tools respect filters

### Phase 4: MCP Tools + `/belief` Skill

**Files to modify:**
- `src/mcp/server.rs` — Register belief mutation/query tools
- `resources/claude/belief.md` — `/belief` skill definition

**Exit criteria:**
- [ ] All belief commands available as MCP tools
- [ ] `/belief` skill describes full capability
- [ ] LLM can discover, select, and invoke belief tools from conversation

## Testing

### Exit Criteria

**1. Hypothesis → Validated flow works**
```bash
patina belief hypothesis test-belief \
  --statement "ChaCha20 is faster than AES"
patina belief validate test-belief \
  --evidence session-20260222-054702 --result PASSED
patina belief show test-belief
# status: validated, evidence: session-20260222-054702 (PASSED)
```

**2. Hypothesis → Refuted flow works**
```bash
patina belief hypothesis keychain-ssh \
  --statement "Keychain works over SSH"
patina belief refute keychain-ssh \
  --evidence session-20260222-054702 \
  --learned "macOS blocks SSH at session level, not API level"
patina belief show keychain-ssh
# status: refuted, evidence present, learning preserved
```

**3. Query filters work**
```bash
patina context keychain
# Shows: Only validated beliefs (hypothesis/refuted hidden)

patina context keychain --include-refuted
# Shows: validated + refuted (with failure reasons)
```

**4. Evolution chains visible**
```bash
patina belief history keychain-ssh
# Timeline showing hypothesis → refuted chain
```

**5. Migration preserves existing knowledge**
```bash
patina belief migrate
patina context keychain
# Shows: Migrated beliefs (status=validated, confidence=medium)
```

## Success Metrics

**1. Knowledge pollution prevented**
- Zero "validated" beliefs without evidence
- All new beliefs have lifecycle tracking

**2. Learning from failures preserved**
- Refuted beliefs kept with "why it failed"
- Future LLMs can query: "What approaches were tried and failed?"

**3. Clean default queries**
- `patina context` shows only validated knowledge (no noise)
- Hypotheses hidden unless explicitly requested

**4. Evolution chains visible**
- Can trace: theory → test → discovery → refined theory
- Reasoning journey preserved, not just conclusions

## Non-Goals

- **No spec lifecycle changes** — spec-workflow-rigor owns spec statuses
  (draft/ready/active/paused/blocked/complete/abandoned). `spec abandon`
  covers what this spec originally called "spec refute". Exit criteria
  checking for `spec complete` belongs in workflow-rigor.
- **No session-end hypothesis prompting** — violates
  [[specs-orthogonal-to-sessions]]. Beliefs are captured by the LLM
  during conversation, not by session lifecycle hooks.
- **No `patina knowledge migrate`** — migration lives under `belief`
  namespace (`patina belief migrate`), not a new command namespace.
- **No automatic test execution** — `belief test <id>` (originally proposed)
  is out of scope. Validation is manual evidence recording, not automatic
  test running. The LLM or user decides what constitutes evidence.

## Related Work

**Builds on:**
- [[spec-workflow-rigor]] — same command decomposition pattern, same
  3-prong architecture, same unix-philosophy alignment
- [[mutation-completes-query]] — belief audit (query) exists, mutation
  commands don't. This spec completes the pair.
- [[refutation-is-discovery]] — core principle driving this design
- [[build-correct-not-temporary]] — don't bolt on lifecycle later

**Distinct from:**
- [[spec-workflow-rigor]] — that spec owns spec lifecycle
- [[measurement-coverage]] — measurement system (observability layer)

## Alignment Audit (2026-02-23, session 20260223-132543)

**Disposition: REWRITE (done)**

Original spec proposed lifecycle changes for both beliefs AND specs.
Spec lifecycle is now fully owned by spec-workflow-rigor. This rewrite
scopes the spec to belief lifecycle only.

**What changed from original:**
- Removed all spec status additions (hypothesis, validated, refuted, superseded for specs)
- Removed `spec validate`, `spec refute`, `spec create` references
- Removed session-end hypothesis prompting (violates specs-orthogonal-to-sessions)
- Fixed all file paths to actual code locations:
  - `src/models/belief.rs` → `src/mother/graph.rs` (where `BeliefEntry` actually lives)
  - `src/models/spec.rs` → removed (spec lifecycle not in scope)
  - `src/db/schema.sql` → removed (schema is programmatic in sqlite.rs)
  - `src/commands/session/update.rs` / `end.rs` → removed (all in internal.rs, not in scope)
  - `src/mcp/handlers/*` → `src/mcp/server.rs` (monolithic, no handlers/ dir)
- Removed time estimates ("Week 1", etc.)
- Added 3-prong architecture (CLI + MCP + Skill)
- Added blocked_by: spec-workflow-rigor
- Downgraded priority: critical → high
- Changed scope: core-architecture → belief-lifecycle
- Added beliefs from session 20260223-120524 decisions

## Key Files

```
# Belief system (where code actually lives)
src/mother/graph.rs                          — BeliefEntry struct (add lifecycle fields)
src/commands/belief/mod.rs                   — belief commands (add mutation subcommands)
src/commands/scrape/beliefs/mod.rs           — belief scraper (parse new frontmatter)
src/commands/scrape/beliefs/verification/    — structural verification (already exists)
src/retrieval/oracle.rs                      — query abstraction (add lifecycle filters)
src/mcp/server.rs                            — MCP tools (add belief tools)
resources/claude/belief.md                   — /belief skill definition (new)
```
