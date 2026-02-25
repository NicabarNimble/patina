---
type: feat
id: spec-blocking-queue
status: draft
created: 2026-02-22
priority: high
related:
- layer/surface/build/feat/spec-agent-checklists/SPEC.md
- layer/surface/build/feat/spec-knowledge-evolution/SPEC.md
- layer/core/spec-driven-design.md
beliefs:
- spec-first
sessions:
- 20260222-054702
---

# feat: Spec Blocking & Queue System

> Add explicit dependency tracking and queue visibility to specs. Solve the "oh shit"
> cascade: when working on spec-X, discover need spec-Y first, pause X, work on Y,
> resume X later. Track blocking relationships, show what's ready to work on, guide
> LLM and human through dependencies without losing context.

## Problem

### The "Oh Shit" Cascade

**Current workflow when work pivots:**

```
Day 1: Start spec-keychain-ssh (status: active)
       LLM begins implementation

Day 2: Test reveals -25308 error over SSH
       "Oh shit, keychain won't work over SSH"
       Need different approach

Day 3: Start spec-secrets-dual-storage (new approach)
       Status: both specs are "active"

Day 4: Which spec to work on?
       spec-keychain-ssh still shows "active" but blocked
       spec-secrets-dual-storage is the real work
       No explicit tracking of dependency
```

**The problem:**
- ❌ No way to mark spec-X as "paused, waiting for spec-Y"
- ❌ Can't query "what specs are ready to work on?"
- ❌ LLM starts session, unclear which spec to continue
- ❌ Multiple incomplete specs pile up, no clear priority
- ❌ Dependency relationships implicit, not explicit

### Real Example (Session 20260222-054702)

**What happened:**
1. spec-secrets-keychain-ssh marked "complete" but never worked
2. spec-keychain-macos26-regression created to "fix" it
3. Empirical testing proved keychain SSH impossible
4. spec-secrets-dual-storage created (real solution)
5. spec-launcher-auth blocked (needs secrets working first)

**Current spec tree chaos:**
```
spec-secrets-keychain-ssh (complete ← wrong!)
spec-keychain-macos26-regression (active ← impossible!)
spec-secrets-dual-storage (draft ← real work)
spec-launcher-auth (active ← actually blocked!)
```

**What's missing:**
- No link: "launcher-auth blocked by dual-storage"
- No query: "what's ready to work on?"
- No history: "when did this become blocked?"
- No guidance: "what should I work on next?"

### Why This Matters

**Specs exist to keep LLM focused** (don't wander off-task). But when work pivots:
- LLM loses context: "which spec am I working on?"
- Human loses context: "what's blocked on what?"
- Sessions don't guide: "continue where you left off"
- Knowledge scattered: dependency info in session notes, not spec metadata

**We have durable state (git, sessions, beliefs) but lack tracking rigor:**
- State exists but not queryable
- Dependencies exist but not explicit
- Queue exists (in our heads) but not visible

## Current State

### Spec Statuses (Exist)

```yaml
status: draft | ready | active | complete
```

**Commands:**
- `patina spec list` - show all specs
- `patina spec ready` - show active + drafts
- `patina spec blocked` - exists but shows nothing (no blocking tracked)
- `patina spec status <id> <status>` - change status

**What works:**
- Can see what specs exist
- Can change status manually
- Can archive completed specs

**What doesn't work:**
- Can't mark "spec-X needs spec-Y"
- Can't query "what's not blocked?"
- Can't see dependency chains
- Can't track why/when blocked

### What We Already Have (Durable State)

✅ **Git commits** - permanent history
✅ **Git tags** - session boundaries, spec archives
✅ **Sessions** - work logs, decisions
✅ **Database** - scraped specs metadata
✅ **Beliefs** - captured learnings

**Not missing state. Missing tracking rigor.**

## Solution

### Core Design: Explicit Blocking in YAML

**Add blocking metadata to spec frontmatter:**

```yaml
---
id: spec-launcher-auth
status: blocked  # ← New status value
blocked_by:
  - spec-secrets-dual-storage  # List of blocking spec IDs
blocked_date: 2026-02-22
blocked_reason: |
  Started implementation but discovered keychain SSH fails with -25308.
  Need encrypted file secrets infrastructure first.

# Optional: soft dependencies (nice-to-have, not blocking)
depends_on:
  - spec-git-tag-system

# Track all state changes
state_history:
  - date: 2026-02-20
    from: draft
    to: active
    session: 20260220-120045
  - date: 2026-02-22
    from: active
    to: blocked
    session: 20260222-054702
    reason: "Discovered keychain SSH impossible"
---
```

**Key principles:**
1. **Explicit over implicit** - blocking is metadata, not just notes
2. **Queryable** - can find all blocked specs
3. **Traceable** - state_history shows when/why blocked
4. **Durable** - survives in git, sessions, database

### Commands

#### New: `patina spec block`

```bash
patina spec block <id> --by <blocker-id> --reason "..."

# Example:
patina spec block spec-launcher-auth \
  --by spec-secrets-dual-storage \
  --reason "Need secrets working over SSH first"

# What it does:
1. Update spec YAML:
   - status: active → blocked
   - Add blocked_by: [spec-secrets-dual-storage]
   - Add blocked_date: 2026-02-22
   - Add blocked_reason: "..."
   - Append to state_history
2. Git commit: "spec: block launcher-auth (waiting on dual-storage)"
3. Log in active session (if exists)
4. Update database (if scrape runs)
```

#### New: `patina spec unblock`

```bash
patina spec unblock <id>

# What it does:
1. Check if all blockers are complete
   - If not: Error with blocker status
   - If yes: Proceed
2. Update spec YAML:
   - status: blocked → active
   - Remove blocked_by field
   - Append to state_history
3. Git commit: "spec: unblock launcher-auth (dual-storage complete)"
4. Log in active session

# Smart unblock (checks completion):
patina spec unblock spec-launcher-auth
# → Error: Still blocked by spec-secrets-dual-storage (status: active)

# Force unblock (skip check):
patina spec unblock spec-launcher-auth --force
# → Unblocks even if blocker not complete
```

#### Enhanced: `patina spec blocked`

```bash
patina spec blocked

# Output:
BLOCKED SPECS (2):

  spec-launcher-auth (blocked 2 days ago)
    Status: blocked
    Blocked by: spec-secrets-dual-storage (active)
    Reason: Need secrets working over SSH first
    Action: Complete spec-secrets-dual-storage to unblock

  spec-feature-x (blocked 5 days ago)
    Status: blocked
    Blocked by: spec-dependency (complete) ← Ready to unblock!
    Hint: Run `patina spec unblock spec-feature-x`

READY TO UNBLOCK (1):
  spec-feature-x - all blockers complete
```

#### Enhanced: `patina spec ready`

```bash
patina spec ready

# Output:
READY TO WORK (not blocked, 3 specs):

  spec-secrets-dual-storage (active)
    Priority: HIGH - blocks 2 other specs
    ↓ Unblocks: spec-launcher-auth, spec-feature-y
    Last worked: 2 days ago (session-20260220-054702)

  spec-git-tag-system (active)
    Priority: MEDIUM - blocks 0 specs
    Last worked: 7 days ago

  spec-knowledge-evolution (draft)
    Priority: MEDIUM - blocks 0 specs
    Ready to promote: patina spec status spec-knowledge-evolution ready

BLOCKED (waiting, 2 specs):

  spec-launcher-auth (waiting on spec-secrets-dual-storage)
  spec-feature-x (ready to unblock - run `patina spec unblock spec-feature-x`)
```

#### New: `patina spec next`

```bash
patina spec next

# Output:
RECOMMENDED: spec-secrets-dual-storage

Why:
  • Status: active (already in progress)
  • Impact: HIGH (blocks 2 other specs)
  • Dependencies: none (not blocked)
  • Last worked: 2 days ago (session-20260220-054702)
  • Momentum: 3 sessions, 47 commits

Other options:
  1. spec-git-tag-system (active, medium priority)
  2. spec-knowledge-evolution (draft, ready to start)

Blocked (not ready):
  • spec-launcher-auth (waiting on this spec!)
```

### Database Schema

**Extend `specs` table:**

```sql
-- Add columns to existing specs table
ALTER TABLE specs ADD COLUMN blocked_date TEXT;
ALTER TABLE specs ADD COLUMN blocked_reason TEXT;

-- New table for blocking relationships
CREATE TABLE spec_blocks (
    blocked_spec TEXT NOT NULL,
    blocker_spec TEXT NOT NULL,
    since TEXT NOT NULL,
    reason TEXT,
    PRIMARY KEY (blocked_spec, blocker_spec),
    FOREIGN KEY (blocked_spec) REFERENCES specs(id),
    FOREIGN KEY (blocker_spec) REFERENCES specs(id)
);

CREATE INDEX idx_spec_blocks_blocked ON spec_blocks(blocked_spec);
CREATE INDEX idx_spec_blocks_blocker ON spec_blocks(blocker_spec);

-- Query: What blocks this spec?
SELECT blocker_spec, reason, since
FROM spec_blocks
WHERE blocked_spec = 'spec-launcher-auth';

-- Query: What does this spec block?
SELECT blocked_spec, reason, since
FROM spec_blocks
WHERE blocker_spec = 'spec-secrets-dual-storage';

-- Query: Specs ready to work (not blocked)
SELECT s.id, s.status, s.created
FROM specs s
LEFT JOIN spec_blocks sb ON s.id = sb.blocked_spec
WHERE s.status IN ('active', 'ready')
  AND sb.blocked_spec IS NULL
ORDER BY s.created;
```

### Session Integration

**Auto-track blocking changes:**

```bash
/session-update

# Output includes:
**Spec changes:**
- spec-launcher-auth: active → blocked
  Blocked by: spec-secrets-dual-storage
  Reason: Need secrets infrastructure first
```

**Auto-suggest at session start:**

```bash
/session-start "Continue secrets work"

# Output:
Session: Continue secrets work
Branch: patina
Tag: session-20260222-120000-claude-start

Spec landscape:
  • Recommended: spec-secrets-dual-storage (active, blocks 2 specs)
  • Ready: spec-git-tag-system (active)
  • Blocked: spec-launcher-auth (waiting on dual-storage)

Continue with spec-secrets-dual-storage? [Y/n]
```

**Auto-notify on unblock:**

```bash
/session-update

# If completed a blocker:
**Specs unblocked:**
✓ Completed spec-secrets-dual-storage
  → Unblocks: spec-launcher-auth (ready to resume)

Hint: Run `patina spec unblock spec-launcher-auth`
```

## Implementation

### Phase 1: Minimal (YAML + Query)

**Goal:** Start tracking blocking manually, query it.

**Changes:**
1. Document new YAML fields (no code changes):
   ```yaml
   blocked_by: [spec-id]
   blocked_date: YYYY-MM-DD
   blocked_reason: "..."
   ```

2. Update `src/commands/spec/blocked.rs`:
   - Parse YAML files
   - Show specs with `blocked_by` field
   - Display blocker status

**Deliverable:**
- Manually add `blocked_by:` to spec YAML
- `patina spec blocked` shows them

**Exit criteria:**
- [ ] Can manually add blocking metadata to spec YAML
- [ ] `patina spec blocked` parses and displays blocked specs

### Phase 2: Commands (Automate)

**Goal:** Automate blocking/unblocking with commands.

**New files:**
- `src/commands/spec/block.rs`
- `src/commands/spec/unblock.rs`

**Changes:**
1. Implement `patina spec block`:
   - Update YAML (status, blocked_by, blocked_date, blocked_reason)
   - Append to state_history
   - Git commit change
   - Log in session if active

2. Implement `patina spec unblock`:
   - Check blocker status
   - Update YAML (remove blocking, change status)
   - Git commit
   - Log in session

**Exit criteria:**
- [ ] `patina spec block <id> --by <blocker> --reason "..."` updates YAML
- [ ] `patina spec unblock <id>` removes blocking if blocker complete
- [ ] Changes committed to git with descriptive message
- [ ] Session log records blocking changes

### Phase 3: Database & Queries

**Goal:** Fast queries, better priority recommendations.

**Changes:**
1. Add database schema (spec_blocks table)
2. Update `src/commands/scrape/specs/mod.rs`:
   - Parse blocked_by from YAML
   - Insert into spec_blocks table
3. Enhance `patina spec ready`:
   - Query database for unblocked specs
   - Show priority based on blocker count
4. Implement `patina spec next`:
   - Recommend spec based on:
     - Status (active > ready > draft)
     - Impact (blocks N other specs)
     - Momentum (recent sessions)

**Exit criteria:**
- [ ] Database stores blocking relationships
- [ ] `patina spec ready` shows priority (high if blocks others)
- [ ] `patina spec next` recommends what to work on
- [ ] Queries are fast (<100ms for 100 specs)

### Phase 4: Session Integration

**Goal:** Seamless workflow with session commands.

**Changes:**
1. Update `/session-start`:
   - Query `patina spec next`
   - Suggest recommended spec
   - Offer to set as active

2. Update `/session-update`:
   - Detect spec status changes
   - Show blocking/unblocking events
   - Suggest unblock if blocker complete

3. Update `/session-end`:
   - If spec completed, check what it unblocks
   - Suggest next spec to work on

**Exit criteria:**
- [ ] `/session-start` suggests spec to work on
- [ ] `/session-update` tracks blocking changes
- [ ] `/session-end` suggests unblocked specs

## Testing

### Manual Test Cases

**Test 1: Block a spec**
```bash
# Setup
patina spec status spec-A active
patina spec status spec-B draft

# Block A on B
patina spec block spec-A --by spec-B --reason "Need B first"

# Verify
grep "blocked_by" layer/surface/build/.../spec-A/SPEC.md
# → blocked_by: [spec-B]

patina spec blocked
# → spec-A listed
```

**Test 2: Unblock when blocker complete**
```bash
# Setup (from Test 1)

# Try unblock while B still draft
patina spec unblock spec-A
# → Error: Still blocked by spec-B (draft)

# Complete B
patina spec status spec-B complete

# Unblock A
patina spec unblock spec-A
# → Success: spec-A active

# Verify
patina spec blocked
# → Empty (A no longer blocked)
```

**Test 3: Queue query**
```bash
# Setup
patina spec block spec-A --by spec-B
patina spec block spec-C --by spec-B
patina spec status spec-D active

# Query ready specs
patina spec ready
# → Shows: spec-B (blocks 2), spec-D
# → Hides: spec-A, spec-C (blocked)
```

**Test 4: Dependency chain**
```bash
# Setup
patina spec block spec-A --by spec-B
patina spec block spec-B --by spec-C

# Complete C
patina spec status spec-C complete

# Unblock B
patina spec unblock spec-B  # → Success

# Try unblock A (B still active)
patina spec unblock spec-A
# → Error: Still blocked by spec-B (active)

# Complete B
patina spec status spec-B complete

# Unblock A
patina spec unblock spec-A  # → Success
```

### Exit Criteria

**Phase 1 (Minimal):**
- [ ] Blocked specs show in `patina spec blocked`
- [ ] Blocking metadata in YAML
- [ ] Manual workflow works

**Phase 2 (Commands):**
- [ ] `patina spec block` updates YAML + commits
- [ ] `patina spec unblock` checks blocker status
- [ ] State history tracks changes
- [ ] Session logs record blocking

**Phase 3 (Database):**
- [ ] Blocking relationships in database
- [ ] `patina spec ready` shows priority
- [ ] `patina spec next` recommends work
- [ ] Fast queries (<100ms)

**Phase 4 (Integration):**
- [ ] Session commands suggest specs
- [ ] Workflow: block → work → unblock → resume
- [ ] No manual YAML editing needed

## Non-Goals

**Not building:**
- ❌ Automatic dependency resolution (Temporal-style orchestration)
- ❌ Workflow language (LangGraph-style state machines)
- ❌ Complex priority algorithms
- ❌ Multi-project coordination (that's Mother's job)
- ❌ Automatic unblocking (human decides when to unblock)

**Why:**
- We have durable state (git, sessions, database)
- We just need explicit tracking + queries
- Keep it simple: mark blocked, query queue, resume work
- Over-engineering adds complexity without value

## Success Metrics

**Problem solved:**
- ✅ "Which spec should I work on?" → `patina spec next`
- ✅ "What's blocked on what?" → `patina spec blocked`
- ✅ "Can I resume spec-X?" → Check blocker status
- ✅ State survives sessions (durable)
- ✅ LLM guided to correct spec (session integration)

**Workflow improved:**
```
Before:
- Start spec-X (active)
- Discover need spec-Y
- Both show "active"
- Unclear which to work on
- Manual coordination

After:
- Start spec-X (active)
- Block: `patina spec block X --by Y`
- Query: `patina spec ready` → shows Y
- Work on Y until complete
- Unblock: `patina spec unblock X`
- Resume X
- Tracked, queryable, durable
```

## Related Work

**Builds on:**
- [[spec-driven-design]]: Specs guide LLM work
- [[git-tags-as-knowledge-refs]]: Git tags preserve state
- [[spec-first]]: Design before implement

**Enables:**
- [[spec-agent-checklists]]: Blocking is first step toward checklists
- [[spec-knowledge-evolution]]: Specs can evolve (blocked → active → complete)

**Informed by:**
- Session 20260222-054702: Discovered blocking need during SSH keychain work
- Temporal/LangGraph patterns: Borrowed ideas, kept simple
- Current pain: Multiple specs in-flight, unclear priority
