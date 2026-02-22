---
type: feat
id: spec-knowledge-evolution
status: draft
created: 2026-02-22
priority: critical
scope: core-architecture
related:
- layer/surface/build/feat/spec-secrets-dual-storage/SPEC.md
beliefs: []
sessions:
- 20260222-054702
---

# feat: Knowledge Evolution as First-Class System

> Redesign beliefs and specs with lifecycle states built-in from creation.
> Evolution through hypothesis → validation → refutation is normal, not exceptional.
> Prevent knowledge pollution by distinguishing theory from proven fact.

## Problem

### The Knowledge Pollution Cycle

**What happened in session 20260222-054702:**

1. **Hypothesis formed**: "Keychain works over SSH with AlwaysThisDeviceOnly"
2. **Spec created**: `spec-secrets-keychain-ssh` (marked complete)
3. **Belief captured**: `keychain-always-this-device-only`
4. **Never actually tested**: Assumed working based on theory
5. **Months later**: Empirical testing proves it never worked
6. **Pollution**: False beliefs/specs in knowledge graph, LLMs read them as truth

**The pattern:**
```
Theory → Capture as Truth → Discover Wrong → Pollution
   ↓          ↓                  ↓              ↓
"Should    Spec marked      -25308 error   Knowledge
 work"     "complete"       in SSH          graph lies
```

**Current problems:**

1. **No distinction between hypothesis and fact**
   - All beliefs treated as validated truth
   - Specs marked "complete" without empirical tests
   - No confidence levels, no evidence tracking

2. **Refutation is exceptional (should be normal)**
   - When proven wrong, we "delete" or "archive"
   - Lose the learning: "Why it failed" disappears
   - Future LLMs retry the same failed approaches

3. **Short-term fixes become permanent**
   - "We'll add lifecycle later" → never happens
   - Band-aids pile up instead of correct design
   - Technical debt in knowledge layer itself

4. **Evolution chains lost**
   - Can't see: "We thought X, tested, discovered Y"
   - No trace of reasoning journey
   - Knowledge appears to come from nowhere

### Real Impact

**From session 20260222-054702:**
- 3 specs trying to "fix" keychain SSH (all impossible)
- Multiple beliefs about keychain accessibility (all wrong)
- Hours wasted on solutions to unsolvable problems
- Had to build test infrastructure to empirically disprove theories

**Root cause:** No mechanism to say "This is a hypothesis, test before believing"

### The Principle

> **"Short-term fixes lead to never fixes. We live in hack-land forever unless we build it right the first time."**
> — User insight, session 20260222-054702

Don't bolt on lifecycle as a "short-term fix". Redesign the knowledge system correctly.

## Solution

### Core Design: Knowledge Has States

**Every belief and spec has a lifecycle built-in from creation.**

```
Knowledge Lifecycle:

hypothesis → validated → superseded
    ↓           ↓            ↓
(Theory)   (Proven)    (Replaced)
    ↓
refuted
    ↓
(Disproven - keep for learning)
```

**Key insight:** Refutation is discovery, not failure. Failed hypotheses are valuable knowledge.

### Knowledge States

#### Beliefs

| State | Meaning | Query Default | When |
|-------|---------|---------------|------|
| `hypothesis` | Untested theory, may be wrong | Hidden | Initial capture, before testing |
| `validated` | Empirically proven | Shown | Evidence supports, tests pass |
| `refuted` | Proven wrong, keep for learning | Hidden* | Evidence contradicts |
| `superseded` | Replaced by better belief | Hidden | New belief improves on this |

*Available via `--include-refuted` flag to learn from failures

#### Specs

| State | Meaning | When |
|-------|---------|------|
| `draft` | Initial idea, not ready | Brainstorming |
| `hypothesis` | Theory to test | Before implementation |
| `ready` | Ready to implement | After review |
| `active` | Work in progress | Implementation started |
| `validated` | Exit criteria passing | Tests passing, not shipped |
| `complete` | Shipped to production | Released |
| `refuted` | Impossible/wrong approach | Proven unworkable |
| `superseded` | Replaced by better spec | New spec replaces this |

**Critical change:** Can't transition to `validated` or `complete` without evidence.

### Schema Design

#### Belief Schema v2

```yaml
---
# Required fields (enforced by tools)
id: <belief-id>
type: belief
status: hypothesis | validated | refuted | superseded  # REQUIRED
confidence: untested | low | medium | high | disproven  # REQUIRED
created: YYYY-MM-DD  # REQUIRED

# Lifecycle tracking
validated: YYYY-MM-DD     # When proven true (if status=validated)
refuted: YYYY-MM-DD       # When proven false (if status=refuted)
superseded: YYYY-MM-DD    # When replaced (if status=superseded)

# Evidence (what supports or refutes this)
evidence:
  - type: test | session | commit | measurement
    description: <what was tested>
    result: PASSED | FAILED | INCONCLUSIVE
    ref: <session-id | commit-sha | test-path>
    date: YYYY-MM-DD

# Evolution chains
superseded_by: <belief-id>   # What replaced this
refutes: [<belief-id>]       # What this disproves
supports: [<belief-id>]      # What this validates
evolved_from: [<belief-id>]  # What this evolved from

# Optional metadata
tags: [<tag>]
sessions: [<session-id>]
---

# Belief Title

## Statement
<One-sentence claim that can be validated or refuted>

## Theory (if status=hypothesis)
<Why we think this might be true>

## Evidence (if status=validated)
<What proves this is true>

## Refutation (if status=refuted)
<What proved this wrong - critical for learning>

## Superseded By (if status=superseded)
<What better belief replaced this>
```

#### Spec Schema v2

```yaml
---
# Required fields
id: <spec-id>
type: feat | fix | refactor
status: draft | hypothesis | ready | active | validated | complete | refuted | superseded
created: YYYY-MM-DD

# Lifecycle tracking
hypothesis_tested: YYYY-MM-DD    # When hypothesis validated
activated: YYYY-MM-DD            # When work started
validated: YYYY-MM-DD            # When tests passed
completed: YYYY-MM-DD            # When shipped
refuted: YYYY-MM-DD              # When proven impossible
superseded: YYYY-MM-DD           # When replaced

# Exit criteria (REQUIRED for validated/complete)
exit_criteria:
  - description: <testable criterion>
    test: <path-to-test-script>
    status: PENDING | PASSED | FAILED
    last_run: YYYY-MM-DD

# Evolution
superseded_by: <spec-id>
refutes: [<spec-id>]
replaces: [<spec-id>]
---
```

### Tool Support

#### New Commands

```bash
# Create hypothesis (not belief yet)
patina belief hypothesis <id> \
  --statement "Raw SecItemCopyMatching bypasses SSH restrictions" \
  --theory "Lower-level API might avoid wrapper restrictions" \
  --test test-ssh-localhost.sh

# Test hypothesis → auto-transition based on evidence
patina belief test <id>
# Runs test, records evidence, updates status:
#   test passes → hypothesis → validated
#   test fails → hypothesis → refuted

# Manually validate with evidence
patina belief validate <id> \
  --evidence session-20260222-054702 \
  --result PASSED

# Manually refute with evidence
patina belief refute <id> \
  --evidence session-20260222-054702 \
  --result "All approaches fail with -25308" \
  --learned "macOS blocks SSH at session level, not API level"

# Mark superseded
patina belief supersede <old-id> <new-id>

# Create spec with exit criteria
patina spec create <id> \
  --type feat \
  --status hypothesis \
  --exit-criteria exit-criteria.yaml

# Validate spec (runs exit criteria tests)
patina spec validate <id>
# Can't mark complete without passing tests

# Refute spec (impossible to implement)
patina spec refute <id> \
  --reason "macOS Security policy prevents this approach" \
  --evidence session-20260222-054702 \
  --superseded-by spec-secrets-dual-storage
```

#### Updated Query Behavior

```bash
# Default: only validated knowledge (clean graph)
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

# Full history (evolution chain)
patina belief history keychain-ssh
→ Timeline:
  2026-02-18: keychain-ssh-accessibility (hypothesis)
  2026-02-20: keychain-ssh-raw-api (hypothesis)
  2026-02-22: keychain-ssh-accessibility (REFUTED - doesn't work)
  2026-02-22: keychain-ssh-raw-api (REFUTED - same error)
  2026-02-22: dual-storage-strategy (VALIDATED - works)

# Show evidence chain
patina belief show <id> --with-evidence
→ Lists all tests, sessions, commits that support/refute
```

#### Session Integration

```bash
# During session: capture hypotheses (not beliefs yet)
/session-note "Hypothesis: Raw API bypasses SSH restrictions"
→ Adds to session hypotheses section (not belief yet)

# Test hypotheses
./test-ssh-localhost.sh
→ Evidence: FAILED with -25308

# Session end: review hypotheses
/session-end

# Before archiving, prompt:
Hypotheses to resolve:
  1. "Raw API bypasses SSH" → REFUTED (test failed)
     Action: Create refuted belief? [y/N]

  2. "Dual storage works" → VALIDATED (tests passed)
     Action: Create validated belief? [Y/n]

Specs to update:
  - spec-keychain-macos26-regression → REFUTE (impossible)
  - spec-secrets-dual-storage → PROMOTE to active

Continue? [Y/n]
```

### Migration Strategy

#### Existing Beliefs/Specs

**Grandfather existing knowledge:**
```bash
# Auto-migration on first run
patina knowledge migrate

Migrating existing beliefs:
  • No status field → assume `validated` (trust historical work)
  • Add confidence: medium (not empirically re-tested)
  • Add created: <git log date>

Migrating existing specs:
  • complete/active/draft → keep status
  • Add exit_criteria: [] (empty, needs filling)
  • Add evidence: [] (empty, needs filling)

Migration complete.
Going forward: ALL new knowledge requires lifecycle fields.
```

**Enforcement:**
```bash
# New beliefs must have status
patina belief create <id>
Error: Missing required field: status
Hint: Use `patina belief hypothesis` for untested theories

# Can't mark validated without evidence
patina belief validate <id>
Error: No evidence recorded
Required: Run tests or add evidence manually
```

## Implementation

### Phase 1: Schema & Core (Week 1)

**Files to create:**
- `src/models/knowledge_lifecycle.rs` - Lifecycle state machine
- `src/models/evidence.rs` - Evidence tracking
- `layer/core/knowledge-evolution.md` - Core belief documenting this

**Files to modify:**
- `src/models/belief.rs` - Add lifecycle fields
- `src/models/spec.rs` - Add lifecycle fields, exit criteria
- `src/db/schema.sql` - Add lifecycle columns

**Migration:**
- `src/commands/knowledge/migrate.rs` - One-time migration

### Phase 2: Commands (Week 2)

**New commands:**
- `patina belief hypothesis` - Create hypothesis
- `patina belief test` - Test hypothesis with evidence
- `patina belief validate` - Manually validate
- `patina belief refute` - Manually refute
- `patina belief supersede` - Mark superseded
- `patina belief history` - Show evolution
- `patina spec validate` - Run exit criteria tests
- `patina spec refute` - Mark impossible

**Modified commands:**
- `patina context` - Add filters: `--include-hypothesis`, `--include-refuted`
- `patina scry` - Default hide refuted/hypothesis
- `patina spec status` - Enforce evidence for validated/complete

### Phase 3: Session Integration (Week 3)

**Session workflow:**
- Capture hypotheses during session (not beliefs yet)
- Test hypotheses → record evidence
- Session end → review → promote or refute

**Files to modify:**
- `src/commands/session/update.rs` - Add hypotheses section
- `src/commands/session/end.rs` - Hypothesis review prompt

### Phase 4: Query Filters (Week 4)

**Implement filtering:**
- Default: hide hypothesis/refuted/superseded
- Flags to include them
- MCP tools respect filters

**Files to modify:**
- `src/retrieval/oracle.rs` - Add lifecycle filters
- `src/mcp/handlers/context.rs` - Add filter params
- `src/mcp/handlers/scry.rs` - Add filter params

## Testing

### Exit Criteria

**1. Hypothesis → Validated flow works**
```bash
# Create hypothesis
patina belief hypothesis test-belief \
  --statement "ChaCha20 is faster than AES" \
  --test bench-crypto.sh

# Test it
patina belief test test-belief
# bench-crypto.sh runs, records evidence

# Check status
patina belief show test-belief
# status: validated (if test passed)
# evidence: bench-crypto.sh (PASSED, 2026-02-22)
```

**2. Hypothesis → Refuted flow works**
```bash
# Create hypothesis
patina belief hypothesis keychain-ssh \
  --statement "Keychain works over SSH" \
  --test test-ssh-localhost.sh

# Test it
patina belief test keychain-ssh
# test-ssh-localhost.sh runs, fails with -25308

# Check status
patina belief show keychain-ssh
# status: refuted
# evidence: test-ssh-localhost.sh (FAILED, -25308)
```

**3. Query filters work**
```bash
# Default: clean knowledge graph
patina context keychain
# Shows: Only validated beliefs

# Include refuted to learn from failures
patina context keychain --include-refuted
# Shows: validated + refuted (with failure reasons)
```

**4. Spec validation enforced**
```bash
# Can't mark complete without tests
patina spec status test-spec complete
# Error: Exit criteria not met (0/3 passed)

# Run tests first
patina spec validate test-spec
# Runs exit_criteria tests, updates status

# Now can complete
patina spec status test-spec complete
# Success (all tests passed)
```

**5. Evolution chains visible**
```bash
patina belief history keychain-ssh
# Timeline:
#   hypothesis: keychain-ssh-accessibility (REFUTED 2026-02-22)
#   hypothesis: keychain-ssh-raw-api (REFUTED 2026-02-22)
#   validated: dual-storage-strategy (CURRENT)
```

**6. Session integration works**
```bash
# During session
/session-note "Hypothesis: X bypasses Y"

# Session end
/session-end
# Prompts: Resolve hypothesis → create belief? [y/N]
```

**7. Migration preserves existing knowledge**
```bash
patina knowledge migrate

# Existing beliefs still queryable
patina context keychain
# Shows: Migrated beliefs (status=validated, confidence=medium)
```

**8. Evidence requirements enforced**
```bash
# New belief without evidence
patina belief create test --status validated
# Error: Can't mark validated without evidence

# Must provide evidence
patina belief create test \
  --status hypothesis \
  --evidence session-12345
# Success
```

## Success Metrics

**1. Knowledge pollution prevented**
- Zero "complete" specs without passing exit criteria
- Zero "validated" beliefs without evidence
- All new knowledge has lifecycle tracking

**2. Learning from failures preserved**
- Refuted beliefs kept with "why it failed"
- Future LLMs can query: "What approaches were tried and failed?"
- No re-attempting proven impossible approaches

**3. Clean default queries**
- `patina context` shows only validated knowledge (no noise)
- Hypotheses hidden unless explicitly requested
- Refuted knowledge available but not default

**4. Evolution chains visible**
- Can trace: theory → test → discovery → refined theory
- "We thought X, discovered Y" is documented
- Reasoning journey preserved, not just conclusions

**5. Tooling enforces discipline**
- Can't skip lifecycle (required fields)
- Can't claim validation without evidence
- Can't mark specs complete without tests passing

## Related Work

**Enables:**
- spec-secrets-dual-storage (proper lifecycle from start)
- All future specs (built on correct foundation)

**Replaces:**
- All "short-term fix" approaches to knowledge management
- Manual spec/belief archival (now automatic with lifecycle)

**Beliefs to capture after implementation:**
- `knowledge-evolution-first-class`: "Knowledge has states. Evolution through refutation is normal."
- `build-correct-not-temporary`: "Short-term fixes become permanent. Build it right the first time."
- `evidence-driven-validation`: "Beliefs require evidence. Tests, not opinions, determine truth."
- `refutation-is-discovery`: "Failed hypotheses are valuable. They prevent re-learning."

## Notes

**This is NOT a "nice to have" feature.**

This is a **fundamental redesign of how Patina manages knowledge**. Without this:
- Knowledge graph pollutes over time
- LLMs read false beliefs as truth
- Same failed approaches retried repeatedly
- "We'll fix it later" technical debt accumulates

**With this:**
- Knowledge graph stays clean (validated facts only by default)
- Learning from failures preserved (refuted beliefs kept)
- Evolution chains visible (theory → discovery documented)
- Discipline enforced by tools (can't skip evidence)

**The choice:** Live in hack-land forever, or build it right now.

**Priority: CRITICAL** - Blocks clean knowledge management for all future work.
