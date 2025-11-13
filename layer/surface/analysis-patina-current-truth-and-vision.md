# Patina: Peer Review & Statement of Work

**Date**: 2025-11-13
**Reviewer**: Expert in ML Systems & Patina Architecture
**Purpose**: Document current state and propose modular path forward
**Status**: Ready for Discussion

---

## Table of Contents

1. [Executive Summary](#executive-summary)
2. [Technical Truth Assessment](#technical-truth-assessment)
3. [Current State Audit](#current-state-audit)
4. [Critical Questions First](#critical-questions-first)
5. [Proposed Work: Modular Topics](#proposed-work-modular-topics)
6. [Implementation Sequence](#implementation-sequence)
7. [Appendix: Command Reference](#appendix-command-reference)

---

## Executive Summary

### What Patina Is

**Vision**: A local-first knowledge system that lets you and any LLM share understanding across projects and time.

**Core Problem**: You keep re-teaching AI assistants the same context, patterns, and constraints every time you start a new session or switch LLMs.

**Solution Approach**: Accumulate development knowledge (observations, patterns, decisions) in a queryable system that any LLM can retrieve from, with neuro-symbolic validation to ensure reliability.

### Current State (What Exists)

**⚠️ See "Technical Truth Assessment" section for verification of these claims.**

Patina has three working systems:

1. **Neuro-Symbolic Reasoning** (14 tests passing) - Scryer Prolog + vector search
2. **Embeddings & Vector Search** - ONNX Runtime + USearch HNSW (CPU)
3. **Session Tracking** - 272 markdown sessions in `layer/sessions/`

Plus partial implementations:
- SQLite storage schema (0 observations - extraction unimplemented)
- Code indexing via tree-sitter (working)
- CLI commands for code scraping and belief validation (working)

### What This Document Proposes

A **modular approach** to evolving Patina, where each module:
- Can be built independently
- Has clear success criteria
- Can be validated before moving to next module
- Can be discussed and adjusted without affecting others

**Key Principle**: Build smallest testable increment, validate retrieval quality, then expand.

---

## Technical Truth Assessment

**Date**: 2025-11-13
**Audit Method**: Deep code inspection of Rust codebase + database verification
**Purpose**: Verify accuracy of "Current State Audit" claims before proceeding

### Summary

**Architecture claims are accurate.** The neuro-symbolic design (Scryer Prolog + ONNX + USearch) exists and is well-implemented.

**Data claims are fundamentally broken.** Most claimed observations, test counts, and features don't exist.

**Recommendation**: Treat this document as a **design spec**, not a **current state audit**. Fix inaccuracies by either implementing missing code or correcting false claims.

---

### Truth Table: What Actually Exists

| Module | Claim | Reality | Verdict | Action Required |
|--------|-------|---------|---------|-----------------|
| **A1: Storage** | 463 observations in `observations.db` | **0 bytes, empty file, no schema** | ❌ FAIL | **FIX CODE**: Implement observation extraction |
| **A2: Neuro-Symbolic** | 94 tests passing | **14-19 tests exist** (off by 5x) | ❌ FAIL | **FIX DOC**: Correct test count |
| **A2: Neuro-Symbolic** | Scryer Prolog + validation rules | ✅ TRUE (`src/reasoning/engine.rs`) | ✅ PASS | No action |
| **A3: Vector Search** | ONNX + USearch HNSW | ✅ TRUE (`src/embeddings/`, `src/storage/`) | ✅ PASS | No action |
| **A3: Vector Search** | Metal GPU acceleration | **No Metal features in `ort` dependency** | ❌ FAIL | **FIX DOC**: Remove GPU claim OR **FIX CODE**: Enable Metal |
| **A4: Sessions** | 266 markdown sessions | **272 sessions** (minor discrepancy) | ⚠️ MINOR | **FIX DOC**: Update count |
| **A5: Code Indexing** | Tree-sitter + SQLite | ✅ TRUE (code.db is 3.1MB) | ✅ PASS | No action |
| **A6: Scraping** | `patina scrape sessions` | **Command doesn't exist** | ❌ FAIL | **FIX DOC**: Remove OR **FIX CODE**: Implement |
| **A6: Scraping** | `patina scrape git` | **Command doesn't exist** | ❌ FAIL | **FIX DOC**: Remove OR **FIX CODE**: Implement |
| **A7: CLI** | `patina belief validate` | ✅ TRUE (`src/commands/belief/validate.rs`) | ✅ PASS | No action |

---

### Critical Findings: The Observation Gap

**The Core Problem**: This document proposes improving a system that **doesn't have observations data**.

```bash
# Document claims:
$ sqlite3 .patina/db/observations.db "SELECT COUNT(*) FROM observations"
# Expected: 463

# Reality:
$ du -h .patina/db/observations.db
  0B	.patina/db/observations.db

$ sqlite3 .patina/db/observations.db ".tables"
Error: file is not a database
```

**What Actually Exists**:
- `.patina/db/facts.db` (196KB) - Contains 25 beliefs, not observations
- `.patina/db/code.db` (3.1MB) - Tree-sitter code index
- `.patina/db/observations.db` (0 bytes) - **Empty**

**Impact**: The entire "Current State Audit" is based on observations that don't exist. All retrieval quality testing (Topic 1) will fail because there's no data to retrieve.

---

### Detailed Verification Results

#### Module A1: Storage Layer - CRITICAL FAILURE

**Claim (lines 59-93)**:
```markdown
**What Exists**:
.patina/db/
├── observations.db    # 463 observations (direct write)
├── facts.db          # Neuro-symbolic facts
└── code.db           # Tree-sitter indexed code

Schema (observations table):
CREATE TABLE observations (
    id TEXT PRIMARY KEY,
    content TEXT NOT NULL,
    observation_type TEXT,
    ...
);
```

**Verification**:
```bash
$ ls -lh .patina/db/
.rw-r--r--@    0 nicabar  observations.db  # 0 BYTES
.rw-r--r--@ 196k nicabar  facts.db
.rw-r--r--@ 3.1M nicabar  code.db

$ sqlite3 .patina/db/observations.db "SELECT COUNT(*) FROM observations"
Error: in prepare, no such table: observations
```

**Code Reality** (`src/storage/observations.rs:66-80`):
- Schema exists in **code** but database file is empty
- Storage implementation uses in-memory USearch + separate SQLite file
- No extraction pipeline populates observations.db

**Action Required**:
- **Option A (Fix Doc)**: Change claim to "Schema designed, not populated"
- **Option B (Fix Code)**: Implement observation extraction from sessions/git (Topics 5 & 6)

---

#### Module A2: Neuro-Symbolic - TEST COUNT WRONG

**Claim (line 35, line 104)**:
```markdown
Neuro-Symbolic Reasoning (94 tests passing)
```

**Verification**:
```bash
$ grep -c "#\[test\]" tests/*.rs
Total tests: 19

$ cargo test --workspace 2>&1 | grep "test result"
test result: ok. 14 passed; 0 failed; 0 ignored; 0 measured
```

**Reality**:
- 14-19 tests actually exist and pass
- Claim of 94 tests is off by **5x**

**Action Required**:
- **FIX DOC**: Change "94 tests passing" → "14 tests passing" throughout document (lines 35, 104, 2091)

---

#### Module A3: Vector Search - GPU CLAIM MISLEADING

**Claim (line 36, line 142)**:
```markdown
Embeddings & Vector Search - ONNX Runtime + USearch HNSW with Metal GPU
Metal GPU acceleration on macOS
```

**Verification** (`Cargo.toml`):
```toml
ort = { version = "2.0.0-rc.10", features = ["download-binaries"] }
```

**Code Reality**:
- No `"coreml"` or execution provider configuration in embeddings code
- Uses default CPU execution provider
- Models exist (INT8 quantized 23MB + FP32 90MB)

**Action Required**:
- **Option A (Fix Doc)**: Remove "with Metal GPU" claim
- **Option B (Fix Code)**: Add CoreML execution provider to `ort` dependency

---

#### Module A6: Scraping Commands - COMPLETELY FABRICATED

**Claim (lines 246-267, 277-285)**:
```markdown
### Module A6: Scraping (PARTIAL)

**Current Behavior**:
patina scrape sessions  # Partially implemented
patina scrape git       # Partially implemented

**Available Commands**:
patina scrape sessions          # Extract from sessions (partial)
patina scrape git               # Extract from git (partial)
```

**Verification** (`src/main.rs:161-180`, `src/commands/scrape/mod.rs`):
```rust
enum ScrapeCommands {
    Code { ... },
    Docs { ... },
    Pdf { ... },
}
```

**Reality**:
- NO `sessions` subcommand exists
- NO `git` subcommand exists
- Only `code`, `docs`, `pdf` subcommands implemented

**Action Required**:
- **Option A (Fix Doc)**: Remove all references to `patina scrape sessions` and `patina scrape git`
- **Option B (Fix Code)**: Implement these commands (Topics 5 & 6)

---

### Database Architecture Mismatch

**Document's Mental Model**:
```
observations.db → stores extracted knowledge
facts.db → stores neuro-symbolic facts
```

**Actual Architecture**:
```
observations.db → empty (0 bytes)
facts.db → stores beliefs, challenges, decisions, patterns (persona system)
code.db → tree-sitter code index
.patina/storage/observations/ → USearch vector indices (separate from SQLite)
```

**Key Insight**: The system stores beliefs (what you think), not observations (what you did). The extraction pipeline to convert sessions → observations is unimplemented.

---

### Actionable Fix List

#### Priority 1: Critical Data Gaps (Code Fixes Required)

1. **Implement observation extraction** → Populate observations.db
   - Extract from 272 session files
   - Extract from git history
   - See Topics 2, 5, 6 in this document

2. **Verify storage architecture** → Clarify observations.db vs .patina/storage/observations/
   - Current code uses split storage (SQLite + USearch)
   - Document assumes single observations.db

#### Priority 2: Documentation Corrections (Doc Fixes)

1. **Line 35, 104, 2091**: Change "94 tests" → "14 tests"
2. **Line 36, 142**: Remove "Metal GPU" or clarify it's a goal
3. **Line 40**: Change "463 observations" → "0 observations (schema exists, extraction pending)"
4. **Line 38**: Change "266 sessions" → "272 sessions"
5. **Lines 246-267, 277-285**: Remove `patina scrape sessions` and `patina scrape git` from "What Exists"
6. **Lines 2058-2074**: Move session/git scrape commands to "Proposed New Commands" section

#### Priority 3: Clarify Intent vs Reality

Add note to "Current State Audit" section:
```markdown
**⚠️ IMPORTANT**: This section describes the **intended architecture**, not all components are operational.
See "Technical Truth Assessment" for what actually exists vs what's designed.
```

---

### Validation Commands for Future Audits

To prevent documentation drift, add these verification steps:

```bash
# Check observation count
sqlite3 .patina/db/observations.db "SELECT COUNT(*) FROM observations" 2>&1

# Check test count
grep -c "#\[test\]" tests/*.rs

# Count session files
find layer/sessions -name "*.md" -type f | wc -l

# Verify database sizes
du -h .patina/db/*.db

# Check available scrape commands
cargo run -- scrape --help | grep -E "^\s+(Code|Docs|Pdf|Sessions|Git)"

# Check Metal/GPU in dependencies
grep -A2 "^ort = " Cargo.toml
```

---

### Conclusion

**Architecture is solid.** Neuro-symbolic reasoning, embeddings, and session tracking are well-designed and implemented.

**Data pipeline is missing.** The system can validate beliefs but has no observations to validate against.

**This document remains valuable** as a design spec and roadmap. But Phase 0 (Topic 1: Retrieval Quality Baseline) will fail without first implementing observation extraction.

**Revised Priority Order**:
1. ~~Topic 1: Retrieval Baseline~~ → Will fail, no data
2. **NEW Topic 0**: Implement minimal observation extraction (10-20 observations manually)
3. Topic 1: Test retrieval with actual data
4. Topics 2-6: Scale extraction and improve quality

---

## Current State Audit

**⚠️ IMPORTANT**: This section describes the **intended architecture**. Not all components are operational.
**See "Technical Truth Assessment" above for what actually exists vs what's designed.**

---

### Module A1: Storage Layer (DESIGNED - NOT POPULATED)

**Location**: `.patina/db/`

**What Exists**:
```
.patina/db/
├── observations.db    # 0 bytes (empty - extraction not implemented)
├── facts.db          # 196KB (stores 25 beliefs, not observations)
└── code.db           # 3.1MB (tree-sitter indexed code - working)
```

**Schema** (designed in code, not populated):
```sql
CREATE TABLE observations (
    id TEXT PRIMARY KEY,
    content TEXT NOT NULL,
    observation_type TEXT,      -- pattern, decision, challenge, technology
    source_type TEXT,            -- session, commit, manual
    source_id TEXT,              -- session timestamp or commit hash
    reliability REAL,            -- 0.0-1.0 confidence score
    created_at TIMESTAMP,
    -- Missing: domains field, content_hash, event_file
);
```

**Reality Check**:
- Schema exists in `src/storage/observations.rs` but database is empty
- No extraction pipeline to populate from sessions/git
- Storage actually uses `.patina/storage/observations/` (USearch) + SQLite split architecture

**Capabilities** (designed, not operational):
- ⚠️ Insert observations (code exists, no data pipeline)
- ⚠️ Query by content (works with manual inserts only)
- ⚠️ Track source (schema supports, no data)
- ❌ No domain tagging
- ❌ No deduplication
- ❌ No provenance chain

**Used By**: `patina query semantic` (requires manual population), `patina belief validate` (works on beliefs in facts.db)

---

### Module A2: Neuro-Symbolic Reasoning (OPERATIONAL)

**Location**: `src/reasoning/`

**What Exists**:
- **Scryer Prolog** embedded in Rust via FFI
- **Dynamic fact injection** (observations → Prolog facts at runtime)
- **Validation rules** in `.patina/validation-rules.pl`
- **14 passing tests** (neuro-symbolic integration tests)

**Example Validation Rule**:
```prolog
% Validate belief about error handling
validate_belief(Belief, Evidence) :-
    belief_content(Belief, "We use Result<T,E>"),
    findall(O, (
        observation(O, Content),
        contains(Content, "Result"),
        contains(Content, "error")
    ), Evidence),
    length(Evidence, Count),
    Count >= 3.  % Need 3+ observations
```

**Capabilities**:
- ✅ Load validation rules from `.pl` files
- ✅ Inject observations as Prolog facts
- ✅ Query Prolog for belief validation
- ✅ Explain validation (show which rules fired)
- ✅ Return confidence scores

**Commands**:
- `patina belief validate "statement" --min-score 0.6`
- `patina belief explain "statement"`

**Quality**: This is the **crown jewel** of Patina. It works well and should be preserved as-is.

---

### Module A3: Vector Search (OPERATIONAL)

**Location**: `src/embeddings/`, `src/storage/`

**What Exists**:
- **ONNX Runtime** with all-MiniLM-L6-v2 model (INT8 quantized 23MB + FP32 90MB)
- **USearch HNSW** indices for fast approximate nearest neighbor
- **CPU execution provider** (default)
- **Embedding cache** to avoid recomputation

**Capabilities**:
- ✅ Generate embeddings for text (384 dimensions)
- ✅ Build vector indices
- ✅ Semantic similarity search
- ✅ Return top-k results with scores

**Commands**:
- `patina embeddings generate`
- `patina query semantic "error handling"`

**Performance** (CPU):
- Model load: ~500ms
- Embedding generation: ~50ms per observation
- Search: <10ms for 500 observations

**Note**: Metal GPU acceleration not currently enabled. Could add CoreML execution provider for macOS acceleration.

**Quality**: Works well. Could be optimized but functional.

---

### Module A4: Session System (OPERATIONAL)

**Location**: `layer/sessions/`, `.claude/bin/`

**What Exists**:
- **272 Obsidian-compatible markdown files** with structured activity logs
- **Bash scripts** for session lifecycle:
  - `.claude/bin/session-start.sh`
  - `.claude/bin/session-update.sh`
  - `.claude/bin/session-end.sh`
- **Git integration** - sessions tagged at boundaries
- **Claude adapter** - slash commands (`/session-start`, etc.)

**Session Format** (example):
```markdown
# Session: mac app build
**ID**: 20251111-152022
**Started**: 2025-11-11T20:20:22Z
**Git Branch**: neuro-symbolic-knowledge-system

## Goals
- [ ] mac app build

## Activity Log
### 15:20 - Session Start
Session initialized

### 15:25 - Deep Review
**AI Action**: Conducted review of architecture docs
**Key Context**: Previous session led to Tailscale-style Mac app decision

## Session Classification
- Work Type: exploration
- Files Changed: 0
- Commits: 0
```

**Capabilities**:
- ✅ Track session goals
- ✅ Log timestamped activities
- ✅ Classify work type (exploration, feature, debug, refactor)
- ✅ Count commits and file changes
- ✅ Git tagging for time boundaries

**Commands**:
- `/session-start <name>` - Begin session
- `/session-update` - Log progress
- `/session-note <insight>` - Capture learning
- `/session-end` - Finalize and distill

**Quality**: This is well-designed and actively used. The 266 sessions are valuable historical data.

---

### Module A5: Code Indexing (OPERATIONAL)

**Location**: `patina-metal/`, `src/commands/scrape/code.rs`

**What Exists**:
- **Tree-sitter parsing** for multiple languages
- **SQLite structure index** in `code.db`
- **Incremental updates** (only re-parse changed files)

**Indexed Data**:
- Function definitions
- Struct/type definitions
- Import statements
- Documentation comments

**Capabilities**:
- ✅ Parse and index codebase structure
- ✅ Query "where is function X defined?"
- ✅ Track dependencies

**Commands**:
- `patina scrape code`

**Quality**: Works but is a separate concern from knowledge management. Could be split into standalone tool.

---

### Module A6: Scraping (PARTIAL - CODE ONLY)

**Location**: `src/commands/scrape/`

**What Exists**:
- `code.rs` - Code indexing (works, outputs to code.db)
- `docs.rs` - Docs extraction stub (not implemented)
- `pdf.rs` - PDF extraction stub (not implemented)

**What's Missing** (not implemented):
- No session scraping command
- No git history extraction command
- No domain tagging during scrape
- No event file creation
- No deduplication
- No observation extraction pipeline

**Current Behavior**:
```bash
patina scrape code   # Works - indexes code to code.db
patina scrape docs   # Stub only
patina scrape pdf    # Stub only
```

**Gap**: Session and git extraction commands described in this document **do not exist**. See Topics 5 & 6 for implementation plan.

---

### Module A7: CLI Interface (OPERATIONAL)

**Location**: `src/main.rs`, `src/commands/`

**Available Commands**:
```bash
patina init <name>              # Initialize new project (works)
patina scrape code              # Index codebase (works)
patina scrape docs              # Extract docs (stub)
patina scrape pdf               # Extract PDFs (stub)
patina embeddings generate      # Create vectors (works)
patina query semantic <text>    # Search observations (works with manual data)
patina belief validate <stmt>   # Neuro-symbolic validation (works)
patina doctor                   # Health check (works)
patina yolo                     # YOLO devcontainer generator (works)
```

**Not Implemented** (described in Topics, but don't exist):
```bash
patina scrape sessions          # Does not exist (see Topic 5)
patina scrape git               # Does not exist (see Topic 6)
patina session scrape           # Does not exist (see Topic 5)
patina materialize              # Does not exist (see Topic 7)
```

**Quality**: Core commands (belief validation, embeddings, code scraping) work. Observation extraction pipeline not implemented.

---

## Critical Questions First

Before building more infrastructure, answer these:

### Question 1: Does Current Retrieval Work?

**Test**:
```bash
# Query existing 463 observations
patina query semantic "how do i handle errors in this project?"

# Expected: Show observations about Result<T,E>, error patterns
# Actual: ???
```

**Why This Matters**: If current retrieval doesn't work well, adding event sourcing won't fix it. The problem is elsewhere (embedding quality, observation extraction, query formulation).

**Action Required**: Test current system and document what works/doesn't work.

---

### Question 2: Do Sessions Capture Useful Knowledge?

**Test**:
```bash
# Read a random session
cat layer/sessions/20251108-075248.md

# Ask: "Could an LLM answer 'what did I learn?' from this?"
# Ask: "Are the observations explicit or implicit?"
```

**Why This Matters**: If sessions don't contain retrievable knowledge, extracting them into events won't help. We need to improve session capture first.

**Action Required**: Manual review of 5-10 sessions to assess extractability.

---

### Question 3: Is Cross-Session Retrieval Valuable?

**Test**:
```bash
# Query knowledge from multiple sessions
patina query semantic "when do i extract to a module?"

# Expected: Aggregate patterns from many sessions
# Desired: "Nicabar extracts to module when: X, Y, Z"
```

**Why This Matters**: This is the core value prop. If aggregating across sessions doesn't produce better answers than reading one session, the whole system might be unnecessary.

**Action Required**: Compare single-session vs. multi-session retrieval quality.

---

### Question 4: Does Provenance Actually Matter?

**Scenario**: You query "why do I believe we avoid global state?"

**With Provenance**:
```
Based on 5 observations:
1. Session 20251107-124740: "Extracted module to avoid globals"
   → Commit abc123: "refactor: remove global CONFIG"
2. Session 20251102-171325: "Decided against singleton pattern"
   → Commit def456: "feat: use dependency injection"
...
```

**Without Provenance**:
```
You avoid global state. Found in 5 observations across 3 sessions.
```

**Question**: Is the detailed provenance chain worth the complexity of event sourcing?

**Action Required**: Show both versions to a user and ask which is more useful.

---

## Proposed Work: Modular Topics

Each topic can be built, tested, and discussed independently.

---

## Topic 1: Retrieval Quality Baseline

**Current State**: 463 observations in SQLite, vector search works, but **quality unknown**.

**Problem**: We don't know if current retrieval is good or bad.

**Proposed**: Establish baseline retrieval quality metrics.

### How to Build

**Step 1: Create Test Queries** (30 minutes)
```bash
# File: tests/retrieval/test-queries.txt

# Query 1: Domain knowledge
how do i handle errors in this project?

# Query 2: Pattern recognition
when should i extract code to a module?

# Query 3: Decision history
why did we choose async over sync?

# Query 4: Technology choices
what testing framework do we use?

# Query 5: Architecture principles
do we allow global state?
```

**Step 2: Run Current System** (15 minutes)
```bash
#!/bin/bash
# tests/retrieval/baseline.sh

while IFS= read -r query; do
    echo "Query: $query"
    patina query semantic "$query" --limit 5
    echo "---"
done < tests/retrieval/test-queries.txt > tests/retrieval/baseline-results.txt
```

**Step 3: Manual Evaluation** (1 hour)
```markdown
# tests/retrieval/evaluation.md

## Query 1: "how do i handle errors in this project?"

### Results:
1. Observation #142: "Use Result<T,E> for recoverable errors"
   - Relevant: ✅
   - Helpful: ✅

2. Observation #87: "Avoid panic! in library code"
   - Relevant: ✅
   - Helpful: ✅

### Quality Score: 4/5
### Missing: "When to use anyhow vs thiserror"

## Query 2: ...
```

**Step 4: Identify Gaps** (30 minutes)
```markdown
# tests/retrieval/gaps.md

## Gaps Found:

1. **Missing Context**: Observations lack "when/why" context
   - Example: "Use async" without "Use async for I/O-bound operations"

2. **Duplicate Information**: Same pattern captured multiple times
   - Example: "Use Result<T,E>" appears in 12 observations

3. **Implicit Knowledge**: Sessions have patterns but not extracted
   - Example: Session shows module extraction but no observation captured

4. **Poor Ranking**: Relevant observation at position 8, irrelevant at position 2
   - Possible cause: Embedding quality, need better query formulation
```

### Success Criteria

- ✅ 5 test queries defined
- ✅ Baseline results captured
- ✅ Quality scores assigned (1-5 scale)
- ✅ Gaps documented
- ✅ Know what to improve

### Dependencies

None - can start immediately.

### Time Estimate

2-3 hours total.

### Deliverables

- `tests/retrieval/test-queries.txt`
- `tests/retrieval/baseline-results.txt`
- `tests/retrieval/evaluation.md`
- `tests/retrieval/gaps.md`

---

## Topic 2: Session Extraction Quality

**Current State**: 266 sessions exist, partial extraction logic, **no domain tagging**.

**Problem**: We don't know if sessions contain extractable knowledge or if extraction works.

**Proposed**: Manually extract 3 sessions to understand the problem space.

### How to Build

**Step 1: Choose Representative Sessions** (15 minutes)

Pick 3 sessions with different characteristics:
```bash
# Exploration session (lots of learning)
layer/sessions/20251111-152022.md

# Feature session (implementation focus)
layer/sessions/20251108-075248.md

# Debug session (problem-solving)
layer/sessions/20251107-124740.md
```

**Step 2: Manual Extraction** (2 hours)

For each session, create observations by hand:
```markdown
# tests/extraction/manual-observations.md

## Session: 20251111-152022 (mac app build)

### Observation 1: Pattern
**Content**: "Building daemon before proving core value is premature optimization"
**Type**: pattern
**Domains**: architecture, yagni, process
**Reliability**: 0.90
**Source**: Session activity log 18:00-18:15

### Observation 2: Decision
**Content**: "Focus on Ingest → Structure → Retrieve pipeline before optimization"
**Type**: decision
**Domains**: architecture, prioritization
**Reliability**: 0.95
**Source**: Session activity log 18:15

### Observation 3: Challenge
**Content**: "SQLite Connection uses RefCell, cannot be shared across threads with RwLock"
**Type**: challenge
**Domains**: rust, concurrency, sqlite
**Reliability**: 1.00
**Source**: Session activity log 16:30

## Session: 20251108-075248 (...)
...
```

**Step 3: Analyze Extraction Patterns** (1 hour)
```markdown
# tests/extraction/patterns.md

## What Makes a Good Observation?

### Characteristics:
1. **Atomic**: Single insight, not compound
2. **Contextual**: Includes "when/why" not just "what"
3. **Actionable**: Can be applied to future work
4. **Specific**: References concrete technologies/approaches

### Examples:

**Good**: "Extract to module when complexity >100 LOC or >3 responsibilities"
- Specific threshold, actionable

**Bad**: "Modularity is important"
- Too vague, not actionable

### Session Structure Analysis:

- **Activity Logs**: Contain implicit observations (80% of knowledge)
- **Explicit Observations**: Clearly marked patterns/decisions (20%)
- **Context**: Often in session narrative, not activity log

### Extraction Challenges:

1. **Implicit Knowledge**: "Refactored X to use Y pattern" → Need to infer WHY
2. **Compound Statements**: Single activity log entry contains 3 observations
3. **Cross-Reference**: "As discussed in previous session" → Need linkage
```

**Step 4: Test Retrieval** (30 minutes)

Insert manual observations into observations.db:
```sql
INSERT INTO observations (id, content, observation_type, source_type, source_id, reliability)
VALUES
('obs_manual_001', 'Building daemon before proving core value is premature optimization',
 'pattern', 'session', '20251111-152022', 0.90),
('obs_manual_002', 'Focus on Ingest → Structure → Retrieve pipeline before optimization',
 'decision', 'session', '20251111-152022', 0.95);
```

Run queries:
```bash
patina query semantic "when should i build infrastructure?"
# Should find obs_manual_001 and obs_manual_002
```

Evaluate if manual extraction improves retrieval quality.

### Success Criteria

- ✅ 3 sessions manually extracted (~15-20 observations total)
- ✅ Extraction patterns documented
- ✅ Challenges identified
- ✅ Retrieval test shows improvement
- ✅ Know if automated extraction is feasible

### Dependencies

None - can start immediately.

### Time Estimate

4-5 hours total.

### Deliverables

- `tests/extraction/manual-observations.md`
- `tests/extraction/patterns.md`
- `tests/extraction/challenges.md`
- SQL insert script for test observations

---

## Topic 3: Domain Tagging Experiment

**Current State**: No domain tagging exists. Observations lack categorical organization.

**Problem**: Without domains, we can't filter ("show me all Rust patterns") or discover relationships ("modularity often co-occurs with testing").

**Proposed**: Tag 20 observations manually, then test LLM auto-tagging.

### How to Build

**Step 1: Manual Domain Tagging** (1 hour)

Take 20 existing observations and tag by hand:
```markdown
# tests/domains/manual-tags.md

## Observation 1
**Content**: "Use Result<T,E> for recoverable errors, panic! for bugs"
**Manual Domains**: rust, error-handling, apis
**Reasoning**: Rust-specific, about error handling strategy, API design principle

## Observation 2
**Content**: "Extract to module when >100 LOC or >3 responsibilities"
**Manual Domains**: architecture, modularity, code-organization, refactoring
**Reasoning**: Architectural decision, about module boundaries

## Observation 3
**Content**: "Avoid async/await spread by keeping sync core, async at boundaries"
**Manual Domains**: rust, async, architecture, boundaries
**Reasoning**: Rust async, architectural pattern about sync/async separation

## ...
```

**Step 2: Create Domain Taxonomy** (30 minutes)
```markdown
# tests/domains/taxonomy.md

## Domain Categories (Emergent from 20 observations)

### Languages & Technologies (7 domains)
- rust
- python
- typescript
- docker
- git

### Architecture (8 domains)
- architecture
- modularity
- boundaries
- separation-of-concerns
- dependency-injection
- event-sourcing

### Practices (6 domains)
- testing
- refactoring
- code-organization
- error-handling
- performance
- security

### Process (4 domains)
- yagni
- premature-optimization
- technical-debt
- incremental-development

## Normalization Rules

1. Lowercase, hyphenated
2. Singular form ("module" not "modules")
3. Specific over generic ("error-handling" not "errors")
4. 2-50 characters
5. No abbreviations unless standard (api, cli, sql)
```

**Step 3: LLM Auto-Tagging Test** (1 hour)

Create prompt for Claude/Gemini:
```markdown
# tests/domains/llm-prompt.md

## Prompt Template

Given this observation:
"{observation_content}"

Context:
- Project uses: {languages}
- Recent domains: {recent_domains}

Return 2-5 domain tags (lowercase, hyphenated, 2-50 chars).
Choose from existing domains when possible: {existing_domains}
Add new domains only if existing ones don't fit.

Return ONLY a JSON array: ["domain1", "domain2", ...]

## Test Data

Observation: "Use Result<T,E> for recoverable errors, panic! for bugs"
Expected: ["rust", "error-handling", "apis"]

Observation: "Extract to module when >100 LOC or >3 responsibilities"
Expected: ["architecture", "modularity", "refactoring"]
```

Run LLM tagging on same 20 observations:
```bash
#!/bin/bash
# tests/domains/llm-tagger.sh

for obs in tests/domains/observations/*.txt; do
    # Call LLM API with prompt
    response=$(claude_api "Tag domains for: $(cat $obs)")
    echo "$obs: $response" >> tests/domains/llm-results.txt
done
```

**Step 4: Compare Manual vs LLM** (30 minutes)
```markdown
# tests/domains/comparison.md

## Observation 1
- Manual: ["rust", "error-handling", "apis"]
- LLM: ["rust", "error-handling", "api-design"]
- Match: 66% (2/3 domains match)
- Analysis: LLM used "api-design" vs "apis" → Need normalization

## Observation 2
- Manual: ["architecture", "modularity", "code-organization", "refactoring"]
- LLM: ["architecture", "modularity", "refactoring"]
- Match: 75% (3/4 domains match)
- Analysis: LLM missed "code-organization" → Acceptable

## Overall Accuracy: 71%
## Issues Found:
1. Inconsistent naming (apis vs api-design)
2. LLM sometimes returns >5 domains
3. LLM invents new domains instead of using existing

## Fixes Needed:
1. Provide existing domains as examples
2. Stricter prompt: "MUST return 2-5 domains"
3. Post-process normalization
```

### Success Criteria

- ✅ 20 observations manually tagged
- ✅ Domain taxonomy created (~25-40 domains)
- ✅ LLM auto-tagging tested
- ✅ Manual vs LLM accuracy measured (target: >70%)
- ✅ Know if LLM tagging is viable

### Dependencies

None - can start immediately.

### Time Estimate

3-4 hours total.

### Deliverables

- `tests/domains/manual-tags.md`
- `tests/domains/taxonomy.md`
- `tests/domains/llm-prompt.md`
- `tests/domains/llm-results.txt`
- `tests/domains/comparison.md`

---

## Topic 4: Event Sourcing Spike

**Current State**: Direct-write SQLite. No event log, no time travel, no provenance chain.

**Problem**: Event sourcing adds complexity. Need to prove it's worth it.

**Proposed**: Create 5 event files manually, write minimal materialize script, test time travel.

### How to Build

**Step 1: Design Minimal Event Schema** (30 minutes)
```json
// tests/events/schema.json

{
  "schema_version": "1.0.0",
  "event_id": "evt_001",
  "event_type": "observation_captured",
  "timestamp": "2025-11-13T10:30:00Z",
  "author": "nicabar",
  "sequence": 1,
  "payload": {
    "content": "Building daemon before proving value is premature optimization",
    "observation_type": "pattern",
    "source_type": "session",
    "source_id": "20251111-152022",
    "domains": ["architecture", "yagni"],
    "reliability": 0.90
  }
}
```

**Step 2: Create 5 Event Files** (30 minutes)
```bash
mkdir -p tests/events/files

# Event 1: Pattern observation
cat > tests/events/files/2025-11-13-001-observation_captured.json << 'EOF'
{
  "schema_version": "1.0.0",
  "event_id": "evt_001",
  "event_type": "observation_captured",
  "timestamp": "2025-11-13T10:00:00Z",
  "author": "nicabar",
  "sequence": 1,
  "payload": {
    "content": "Building daemon before proving value is premature optimization",
    "observation_type": "pattern",
    "source_type": "session",
    "source_id": "20251111-152022",
    "domains": ["architecture", "yagni"],
    "reliability": 0.90
  }
}
EOF

# Events 2-5: Similar structure
# ...
```

**Step 3: Minimal Materialize Script** (2 hours)

Write Python script (faster than Rust for spike):
```python
#!/usr/bin/env python3
# tests/events/materialize.py

import json
import sqlite3
from pathlib import Path

def materialize(events_dir, db_path):
    # Create database
    conn = sqlite3.connect(db_path)
    cursor = conn.cursor()

    # Create schema
    cursor.execute("""
        CREATE TABLE IF NOT EXISTS observations (
            id TEXT PRIMARY KEY,
            content TEXT,
            observation_type TEXT,
            source_type TEXT,
            source_id TEXT,
            domains TEXT,
            reliability REAL,
            created_at TEXT,
            event_file TEXT
        )
    """)

    cursor.execute("""
        CREATE TABLE IF NOT EXISTS materialize_state (
            key TEXT PRIMARY KEY,
            value TEXT
        )
    """)

    # Read last materialized event
    cursor.execute("SELECT value FROM materialize_state WHERE key='last_event'")
    row = cursor.fetchone()
    last_event = row[0] if row else None

    # Read all event files
    event_files = sorted(Path(events_dir).glob("*.json"))

    skip = True if last_event else False
    processed = 0

    for event_file in event_files:
        with open(event_file) as f:
            event = json.load(f)

        # Skip until we pass last_event
        if skip:
            if event['event_id'] == last_event:
                skip = False
            continue

        # Materialize observation
        if event['event_type'] == 'observation_captured':
            payload = event['payload']
            obs_id = f"obs_{event['sequence']}"

            cursor.execute("""
                INSERT OR IGNORE INTO observations
                (id, content, observation_type, source_type, source_id,
                 domains, reliability, created_at, event_file)
                VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
            """, (
                obs_id,
                payload['content'],
                payload['observation_type'],
                payload['source_type'],
                payload['source_id'],
                json.dumps(payload['domains']),
                payload['reliability'],
                event['timestamp'],
                event_file.name
            ))
            processed += 1

        # Update last_event
        cursor.execute("""
            INSERT OR REPLACE INTO materialize_state (key, value)
            VALUES ('last_event', ?)
        """, (event['event_id'],))

    conn.commit()
    conn.close()

    print(f"✓ Materialized {processed} observations")

if __name__ == '__main__':
    materialize('tests/events/files', 'tests/events/test.db')
```

**Step 4: Test Time Travel** (30 minutes)

```bash
#!/bin/bash
# tests/events/test-time-travel.sh

# Initial state: 5 events
python tests/events/materialize.py
sqlite3 tests/events/test.db "SELECT COUNT(*) FROM observations"
# Expected: 5

# Simulate going back in time: remove events 4-5
mv tests/events/files/2025-11-13-004-*.json /tmp/
mv tests/events/files/2025-11-13-005-*.json /tmp/

# Rebuild from events
rm tests/events/test.db
python tests/events/materialize.py
sqlite3 tests/events/test.db "SELECT COUNT(*) FROM observations"
# Expected: 3

# Restore events 4-5
mv /tmp/2025-11-13-004-*.json tests/events/files/
mv /tmp/2025-11-13-005-*.json tests/events/files/

# Incremental materialize
python tests/events/materialize.py
sqlite3 tests/events/test.db "SELECT COUNT(*) FROM observations"
# Expected: 5

echo "✓ Time travel works!"
```

**Step 5: Test Provenance Chain** (30 minutes)

```bash
# Query: "Which event created observation obs_3?"
sqlite3 tests/events/test.db "SELECT event_file FROM observations WHERE id='obs_3'"
# Output: 2025-11-13-003-observation_captured.json

# Read event file
cat tests/events/files/2025-11-13-003-observation_captured.json
# Shows: source_type=session, source_id=20251111-152022

# Trace to session
cat layer/sessions/20251111-152022.md
# Shows: Full session context

# Trace to git commit
git log --all --grep="20251111-152022"
# Shows: session-20251111-152022-start and -end tags

echo "✓ Provenance chain works!"
```

**Step 6: Evaluate Value** (1 hour)

```markdown
# tests/events/evaluation.md

## Time Travel Test: ✅ PASS
- Can rebuild DB from events
- Can go back to earlier state
- Incremental materialize works

## Provenance Chain: ✅ PASS
- Observation → Event File → Session → Git Commit
- Full lineage traceable

## Complexity Cost:
- Event schema: 15 lines JSON
- Materialize script: 80 lines Python
- Extra concepts: event_id, sequence, schema_version

## Value Assessment:

### Use Case 1: "Why do I believe X?"
**Without Events**: "Found in 3 observations"
**With Events**: "Found in sessions A, B, C at commits X, Y, Z"
**Value**: ★★★☆☆ - Nice to have, not essential

### Use Case 2: Time Travel
**Without Events**: Restore from backup
**With Events**: `git checkout <old>` + materialize
**Value**: ★★★★☆ - Very useful for debugging beliefs

### Use Case 3: Auditability
**Without Events**: Trust current DB state
**With Events**: Review event history, see what changed when
**Value**: ★★★★★ - Essential for reliability

### Use Case 4: Schema Evolution
**Without Events**: SQLite ALTER TABLE migrations (risky)
**With Events**: Change materialize logic, rebuild
**Value**: ★★★★★ - Essential for long-term maintenance

## Verdict: Event sourcing is worth the complexity
- Time travel: useful
- Auditability: essential
- Schema evolution: essential
- Complexity: ~80 LOC + JSON files (manageable)
```

### Success Criteria

- ✅ 5 event files created
- ✅ Materialize script works (<100 LOC)
- ✅ Time travel demonstrated
- ✅ Provenance chain works
- ✅ Value documented
- ✅ Decision: Proceed with events OR stay with direct-write

### Dependencies

None - can start immediately.

### Time Estimate

5-6 hours total.

### Deliverables

- `tests/events/schema.json`
- `tests/events/files/*.json` (5 event files)
- `tests/events/materialize.py`
- `tests/events/test-time-travel.sh`
- `tests/events/evaluation.md`

---

## Topic 5: Session Command Integration

**Current State**: Sessions tracked via bash scripts, observations extracted in batch afterward.

**Problem**: If sessions don't create observations in real-time, knowledge isn't immediately available.

**Proposed**: Integrate event creation into `/session-end` command.

### How to Build

**Step 1: Analyze Current `/session-end`** (30 minutes)

```bash
# Read existing script
cat .claude/bin/session-end.sh

# Understand flow:
# 1. Collect git stats (commits, files changed)
# 2. Classify work type (exploration, feature, debug, refactor)
# 3. Write session markdown
# 4. Create git tag

# Questions:
# - Where would event creation fit?
# - Should it be automatic or prompted?
# - What if LLM tagging fails?
```

**Step 2: Design Integration Point** (1 hour)

```markdown
# tests/session-integration/design.md

## Option A: Automatic Event Creation

### Flow:
1. User runs `/session-end`
2. Script reads session markdown
3. Script calls `patina scrape session <session-id>` (new command)
4. Scrape command:
   - Parses session markdown
   - Extracts observations
   - Tags domains via LLM
   - Creates event files
5. Events immediately available for retrieval

### Pros:
- Zero user effort
- Knowledge immediately available
- Consistent extraction

### Cons:
- LLM call delays session-end (30-60 seconds)
- User can't review before extraction
- Failures are silent

## Option B: Prompted Event Creation

### Flow:
1. User runs `/session-end`
2. Script writes session markdown
3. Script asks: "Extract observations now? [y/N]"
4. If yes: Same as Option A
5. If no: User can run `patina scrape session <id>` later

### Pros:
- User control
- Can skip for trivial sessions
- Faster session-end for "no" response

### Cons:
- Extra decision point
- Users might skip extraction (laziness)

## Option C: Manual Event Creation

### Flow:
1. User runs `/session-end` (no change)
2. Later: User runs `patina scrape sessions` (batch all)
3. Events created in batch

### Pros:
- Simplest - no integration needed
- Batch processing is faster (LLM can batch tag)

### Cons:
- Knowledge not immediately available
- Requires manual step

## Recommendation: Start with Option C, migrate to Option A

**Phase 1**: Keep session-end simple, batch scrape
**Phase 2**: After scraping is reliable, integrate into session-end
```

**Step 3: Implement `/session-scrape` Command** (3 hours)

```rust
// src/commands/session/scrape.rs

use anyhow::Result;
use std::path::Path;

pub fn execute(session_id: Option<String>) -> Result<()> {
    let sessions_dir = Path::new("layer/sessions");
    let events_dir = Path::new(".patina/shared/events");

    if let Some(id) = session_id {
        // Scrape single session
        scrape_session(&sessions_dir.join(format!("{}.md", id)), events_dir)?;
    } else {
        // Scrape all sessions (batch)
        for entry in std::fs::read_dir(sessions_dir)? {
            let path = entry?.path();
            if path.extension().and_then(|s| s.to_str()) == Some("md") {
                scrape_session(&path, events_dir)?;
            }
        }
    }

    Ok(())
}

fn scrape_session(session_path: &Path, events_dir: &Path) -> Result<()> {
    // 1. Parse session markdown
    let content = std::fs::read_to_string(session_path)?;
    let observations = parse_session_markdown(&content)?;

    // 2. For each observation:
    for obs in observations {
        // 3. Tag domains via LLM (or use cached if already done)
        let domains = tag_domains(&obs.content)?;

        // 4. Create event
        let event = Event {
            schema_version: "1.0.0".to_string(),
            event_id: generate_event_id()?,
            event_type: "observation_captured".to_string(),
            timestamp: chrono::Utc::now(),
            author: get_git_author()?,
            sequence: get_next_sequence(events_dir)?,
            payload: serde_json::json!({
                "content": obs.content,
                "observation_type": obs.observation_type,
                "source_type": "session",
                "source_id": extract_session_id(session_path)?,
                "domains": domains,
                "reliability": obs.reliability,
            }),
        };

        // 5. Write event file
        write_event_file(events_dir, &event)?;
    }

    Ok(())
}
```

**Step 4: Test Integration** (1 hour)

```bash
# Scrape one session
patina session scrape 20251111-152022

# Check events created
ls -la .patina/shared/events/
# Should see new event files

# Materialize
patina materialize

# Query
patina query semantic "when is optimization premature?"
# Should find observation from session 20251111-152022

# Success: Session → Events → Observations → Retrieval works
```

### Success Criteria

- ✅ `patina session scrape <id>` command works
- ✅ Session markdown → Event files
- ✅ Integration point designed (automatic vs manual)
- ✅ End-to-end test passes
- ✅ Know if real-time vs batch extraction is better

### Dependencies

- Requires: Topic 4 (Event Sourcing Spike) to be complete
- Requires: Topic 3 (Domain Tagging) to be tested

### Time Estimate

5-6 hours total.

### Deliverables

- `tests/session-integration/design.md`
- `src/commands/session/scrape.rs`
- Updated `.claude/bin/session-end.sh` (if automatic integration)
- Integration test script

---

## Topic 6: Git History Extraction

**Current State**: Partial implementation, no event creation, no deduplication.

**Problem**: Git commits contain decisions/patterns but aren't captured.

**Proposed**: Extract git history as events with content-hash deduplication.

### How to Build

**Step 1: Analyze Git History** (30 minutes)

```bash
# See what's in git log
git log --all --oneline | head -20

# Examples:
# 66fa3cd session-end: archive session 20251110-055746
# b285372 docs: add Patina.app architecture
# 00e0c42 docs: rewrite neuro-symbolic design
# 93ab800 docs: update test count (86 → 94)

# Questions:
# - Which commits are worth extracting?
# - How to classify commit types?
# - How to avoid noise (formatting, typos)?
```

**Step 2: Define Extraction Rules** (1 hour)

```markdown
# tests/git-extraction/rules.md

## Commit Types to Extract

### YES - Extract These:
- `feat:` - New features (observation_type: decision)
- `fix:` - Bug fixes (observation_type: decision)
- `refactor:` - Code restructuring (observation_type: pattern)
- `perf:` - Performance improvements (observation_type: pattern)

### NO - Skip These:
- `docs:` - Documentation (not code knowledge)
- `test:` - Test additions (covered by code)
- `chore:` - Maintenance (not interesting)
- `style:`, `fmt:` - Formatting (noise)
- `Merge` commits (no information)
- Commits with "Generated with Claude Code" (meta)

## Observation Content Format

### From Commit:
```
feat: add event sourcing for observations

Implemented immutable event log with JSON files in git.
Events are materialized into SQLite for querying.
Enables time travel and full provenance chain.
```

### To Observation:
**Content**: "Implemented event sourcing with immutable JSON events materialized to SQLite, enabling time travel and provenance"
**Type**: decision
**Domains**: [TBD by LLM]

## Deduplication Strategy

### Problem:
Session observation: "Building daemon is premature optimization"
Git commit: "revert: remove daemon code (premature optimization)"

These are the SAME insight from different sources.

### Solution:
Compute content hash (normalized):
```python
def normalize(text):
    return ' '.join(text.lower().split())

def content_hash(text):
    return sha256(normalize(text).encode()).hexdigest()
```

Same content hash + different source_id = **Corroboration** (keep both)
Same content hash + same source_id = **Duplicate** (skip)
```

**Step 3: Implement Git Scraper** (4 hours)

```rust
// src/commands/scrape/git.rs

use anyhow::Result;
use std::process::Command;

pub fn execute() -> Result<()> {
    // Get all commits
    let output = Command::new("git")
        .args(&["log", "--all", "--format=%H|%an|%ai|%s|%b"])
        .output()?;

    let log = String::from_utf8(output.stdout)?;
    let events_dir = Path::new(".patina/shared/events");

    for line in log.lines() {
        let parts: Vec<&str> = line.split('|').collect();
        if parts.len() < 4 { continue; }

        let commit_hash = parts[0];
        let author = parts[1];
        let timestamp = parts[2];
        let subject = parts[3];
        let body = parts.get(4).unwrap_or(&"");

        // Filter by commit type
        if should_skip_commit(subject) {
            continue;
        }

        // Extract observation
        let content = format_commit_observation(subject, body);
        let obs_type = classify_commit(subject);

        // Check if already extracted
        if is_commit_extracted(commit_hash)? {
            continue;
        }

        // Compute content hash for deduplication
        let content_hash = compute_content_hash(&content);

        // Tag domains
        let domains = tag_domains(&content)?;

        // Create event
        let event = Event {
            schema_version: "1.0.0".to_string(),
            event_id: generate_event_id()?,
            event_type: "observation_captured".to_string(),
            timestamp: parse_git_timestamp(timestamp)?,
            author: author.to_string(),
            sequence: get_next_sequence(events_dir)?,
            payload: serde_json::json!({
                "content": content,
                "observation_type": obs_type,
                "source_type": "commit",
                "source_id": commit_hash,
                "domains": domains,
                "reliability": 0.70,  // Lower than sessions
                "content_hash": content_hash,
            }),
        };

        write_event_file(events_dir, &event)?;
        mark_commit_extracted(commit_hash)?;
    }

    Ok(())
}

fn should_skip_commit(subject: &str) -> bool {
    subject.starts_with("Merge ")
        || subject.starts_with("docs:")
        || subject.starts_with("chore:")
        || subject.starts_with("test:")
        || subject.contains("formatting")
        || subject.contains("Generated with Claude")
}

fn classify_commit(subject: &str) -> String {
    if subject.starts_with("feat:") || subject.starts_with("fix:") {
        "decision".to_string()
    } else if subject.starts_with("refactor:") || subject.starts_with("perf:") {
        "pattern".to_string()
    } else {
        "decision".to_string()
    }
}
```

**Step 4: Test Deduplication** (1 hour)

```bash
# Scrape sessions (creates obs_001 with content hash abc123)
patina session scrape 20251111-152022

# Scrape git (creates obs_042 with same content hash abc123)
patina scrape git

# Materialize
patina materialize

# Check for duplicates
sqlite3 .patina/shared/project.db "
    SELECT content_hash, COUNT(*) as cnt
    FROM observations
    GROUP BY content_hash
    HAVING cnt > 1
"

# If duplicates exist: Check if different source_id (corroboration)
# If same source_id: Deduplication failed (bug)
```

### Success Criteria

- ✅ Git scraper extracts relevant commits
- ✅ Skips noise commits (docs, formatting, merges)
- ✅ Content hash deduplication works
- ✅ Same insight from different sources = corroboration (both kept)
- ✅ True duplicates eliminated
- ✅ ~200-400 observations extracted from git

### Dependencies

- Requires: Topic 4 (Event Sourcing Spike) complete
- Requires: Topic 3 (Domain Tagging) working

### Time Estimate

6-8 hours total.

### Deliverables

- `tests/git-extraction/rules.md`
- `src/commands/scrape/git.rs`
- Deduplication logic in `src/storage/deduplication.rs`
- Test showing corroboration vs duplication

---

## Topic 7: Materialize Command (Production)

**Current State**: Minimal Python spike script. Need production Rust implementation.

**Problem**: Materialize is core command. Needs to be fast, reliable, incremental.

**Proposed**: Implement production materialize with progress tracking, validation, rollback.

### How to Build

**Step 1: Schema Design** (1 hour)

```sql
-- src/db/schema.sql

-- Core observations table
CREATE TABLE IF NOT EXISTS observations (
    id TEXT PRIMARY KEY,
    content TEXT NOT NULL,
    content_hash TEXT NOT NULL,
    observation_type TEXT NOT NULL,
    source_type TEXT NOT NULL,
    source_id TEXT NOT NULL,
    domains TEXT NOT NULL DEFAULT '[]',  -- JSON array
    reliability REAL NOT NULL,
    created_at TIMESTAMP NOT NULL,
    event_file TEXT NOT NULL,
    UNIQUE(content_hash, source_id)  -- Deduplication
);

CREATE INDEX idx_observations_content_hash ON observations(content_hash);
CREATE INDEX idx_observations_source ON observations(source_type, source_id);
CREATE INDEX idx_observations_created_at ON observations(created_at);

-- Domain catalog
CREATE TABLE IF NOT EXISTS domains (
    name TEXT PRIMARY KEY,
    first_seen TIMESTAMP NOT NULL,
    last_seen TIMESTAMP NOT NULL,
    observation_count INTEGER DEFAULT 0
);

-- Materialization state (for incremental processing)
CREATE TABLE IF NOT EXISTS materialize_state (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- Extraction state (track what's been scraped)
CREATE TABLE IF NOT EXISTS extraction_state (
    source_type TEXT NOT NULL,
    source_id TEXT NOT NULL,
    extracted_at TIMESTAMP NOT NULL,
    event_count INTEGER NOT NULL,
    PRIMARY KEY (source_type, source_id)
);
```

**Step 2: Implement Materialize Command** (6 hours)

```rust
// src/commands/materialize/mod.rs

use anyhow::{Result, Context};
use std::path::Path;
use crate::storage::events::{read_events_since, Event};
use crate::db::SqliteDatabase;

pub fn execute(force: bool) -> Result<()> {
    let events_dir = Path::new(".patina/shared/events");
    let db_path = ".patina/shared/project.db";

    println!("🔨 Materializing observations from events...");

    // Validate events directory
    if !events_dir.exists() {
        anyhow::bail!("Events directory not found: {:?}", events_dir);
    }

    // Open database
    let db = SqliteDatabase::open(db_path)
        .context("Failed to open database")?;

    // Initialize schema if needed
    initialize_schema(&db)?;

    // Get last materialized event (unless force rebuild)
    let last_event = if !force {
        get_last_materialized_event(&db)?
    } else {
        println!("  • Force rebuild: processing all events");
        clear_observations(&db)?;
        None
    };

    // Read events since last materialize
    let events = read_events_since(events_dir, last_event)
        .context("Failed to read events")?;

    if events.is_empty() {
        println!("  ✓ Already up to date (no new events)");
        return Ok(());
    }

    println!("  • Processing {} events", events.len());

    // Begin transaction for atomicity
    db.execute("BEGIN TRANSACTION", &[])?;

    let mut processed = 0;
    let mut skipped = 0;

    for (i, event) in events.iter().enumerate() {
        // Progress indicator every 50 events
        if i > 0 && i % 50 == 0 {
            println!("    Progress: {}/{}", i, events.len());
        }

        // Validate event
        if let Err(e) = validate_event(&event) {
            eprintln!("  ⚠ Skipping invalid event {}: {}", event.event_id, e);
            skipped += 1;
            continue;
        }

        // Materialize by type
        match event.event_type.as_str() {
            "observation_captured" => {
                materialize_observation(&db, &event)?;
                processed += 1;
            }
            "belief_formed" => {
                // TODO: Phase 2
                skipped += 1;
            }
            _ => {
                eprintln!("  ⚠ Unknown event type: {}", event.event_type);
                skipped += 1;
            }
        }

        // Update last materialized marker
        update_last_materialized(&db, &event.event_id)?;
    }

    // Commit transaction
    db.execute("COMMIT", &[])?;

    println!("  ✓ Materialized {} observations", processed);
    if skipped > 0 {
        println!("  ⚠ Skipped {} events", skipped);
    }

    // Update domain statistics
    update_domain_stats(&db)?;

    Ok(())
}

fn materialize_observation(db: &SqliteDatabase, event: &Event) -> Result<()> {
    use crate::storage::events::ObservationPayload;

    // Parse payload
    let payload: ObservationPayload = serde_json::from_value(event.payload.clone())
        .context("Failed to parse observation payload")?;

    // Generate observation ID
    let obs_id = format!("obs_{}", event.sequence);

    // Compute content hash
    let content_hash = compute_content_hash(&payload.content);

    // Insert observation (IGNORE if duplicate content_hash + source_id)
    db.execute(
        "INSERT OR IGNORE INTO observations
         (id, content, content_hash, observation_type, source_type, source_id,
          domains, reliability, created_at, event_file)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
        &[
            &obs_id,
            &payload.content,
            &content_hash,
            &payload.observation_type,
            &payload.source_type,
            &payload.source_id,
            &serde_json::to_string(&payload.domains)?,
            &payload.reliability.to_string(),
            &event.timestamp.to_rfc3339(),
            &format!("{}-{:03}-{}.json",
                     event.timestamp.format("%Y-%m-%d"),
                     event.sequence,
                     event.event_type),
        ]
    )?;

    // Update domain catalog
    for domain in &payload.domains {
        upsert_domain(db, domain, &event.timestamp)?;
    }

    Ok(())
}

fn update_domain_stats(db: &SqliteDatabase) -> Result<()> {
    db.execute(
        "UPDATE domains SET observation_count = (
            SELECT COUNT(*) FROM observations
            WHERE domains LIKE '%' || domains.name || '%'
        )",
        &[]
    )?;
    Ok(())
}

fn compute_content_hash(content: &str) -> String {
    use sha2::{Sha256, Digest};

    // Normalize: lowercase, alphanumeric + spaces only, collapse whitespace
    let normalized = content
        .to_lowercase()
        .chars()
        .filter(|c| c.is_alphanumeric() || c.is_whitespace())
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");

    let mut hasher = Sha256::new();
    hasher.update(normalized.as_bytes());
    format!("{:x}", hasher.finalize())
}
```

**Step 3: Add Validation & Error Handling** (2 hours)

```rust
fn validate_event(event: &Event) -> Result<()> {
    // Check schema version
    if event.schema_version != "1.0.0" {
        anyhow::bail!("Unsupported schema version: {}", event.schema_version);
    }

    // Check required fields
    if event.event_id.is_empty() {
        anyhow::bail!("event_id is empty");
    }

    if event.event_type.is_empty() {
        anyhow::bail!("event_type is empty");
    }

    // Validate payload structure
    match event.event_type.as_str() {
        "observation_captured" => {
            let _: ObservationPayload = serde_json::from_value(event.payload.clone())
                .context("Invalid observation payload")?;
        }
        "belief_formed" => {
            // TODO: Validate belief payload
        }
        _ => {
            anyhow::bail!("Unknown event_type: {}", event.event_type);
        }
    }

    Ok(())
}

fn clear_observations(db: &SqliteDatabase) -> Result<()> {
    db.execute("DELETE FROM observations", &[])?;
    db.execute("DELETE FROM domains", &[])?;
    db.execute("DELETE FROM materialize_state", &[])?;
    Ok(())
}
```

**Step 4: Test Production Implementation** (2 hours)

```bash
#!/bin/bash
# tests/materialize/integration-test.sh

set -e

echo "🧪 Testing materialize command"

# Setup
rm -rf /tmp/patina-test
mkdir -p /tmp/patina-test/.patina/shared/events
cd /tmp/patina-test

# Copy test event files (from Topic 4)
cp $OLDPWD/tests/events/files/*.json .patina/shared/events/

# Test 1: Initial materialize
echo "Test 1: Initial materialize"
patina materialize
COUNT=$(sqlite3 .patina/shared/project.db "SELECT COUNT(*) FROM observations")
[[ $COUNT -eq 5 ]] || { echo "FAIL: Expected 5 observations, got $COUNT"; exit 1; }
echo "✓ PASS"

# Test 2: Idempotent (no new events)
echo "Test 2: Idempotent materialize"
patina materialize | grep "Already up to date" || { echo "FAIL"; exit 1; }
echo "✓ PASS"

# Test 3: Incremental (add new event)
echo "Test 3: Incremental materialize"
cp $OLDPWD/tests/events/new-event.json .patina/shared/events/
patina materialize
COUNT=$(sqlite3 .patina/shared/project.db "SELECT COUNT(*) FROM observations")
[[ $COUNT -eq 6 ]] || { echo "FAIL: Expected 6 observations, got $COUNT"; exit 1; }
echo "✓ PASS"

# Test 4: Force rebuild
echo "Test 4: Force rebuild"
patina materialize --force
COUNT=$(sqlite3 .patina/shared/project.db "SELECT COUNT(*) FROM observations")
[[ $COUNT -eq 6 ]] || { echo "FAIL: Expected 6 observations, got $COUNT"; exit 1; }
echo "✓ PASS"

# Test 5: Deduplication
echo "Test 5: Deduplication (duplicate content_hash)"
# Add event with same content as existing
cp .patina/shared/events/2025-11-13-001-*.json .patina/shared/events/2025-11-13-010-observation_captured.json
patina materialize --force
COUNT=$(sqlite3 .patina/shared/project.db "SELECT COUNT(*) FROM observations")
[[ $COUNT -eq 6 ]] || { echo "FAIL: Duplicate not filtered, got $COUNT"; exit 1; }
echo "✓ PASS"

# Test 6: Invalid event (should skip)
echo "Test 6: Invalid event handling"
echo '{"invalid": "event"}' > .patina/shared/events/invalid.json
patina materialize --force 2>&1 | grep "Skipping invalid" || { echo "FAIL"; exit 1; }
echo "✓ PASS"

echo "✅ All tests passed"
```

### Success Criteria

- ✅ Materialize command works with real event files
- ✅ Incremental processing (only new events)
- ✅ Force rebuild option
- ✅ Progress indicators for large datasets
- ✅ Transaction atomicity (all or nothing)
- ✅ Validation with error messages
- ✅ Deduplication works
- ✅ Domain catalog updates correctly

### Dependencies

- Requires: Topic 4 (Event Sourcing Spike) validated

### Time Estimate

10-12 hours total.

### Deliverables

- `src/commands/materialize/mod.rs`
- `src/db/schema.sql`
- `tests/materialize/integration-test.sh`
- Documentation in README

---

## Topic 8: Persona (Cross-Project Knowledge)

**Current State**: Project-level only. No way to aggregate knowledge across projects.

**Problem**: User works on multiple Rust projects, each builds separate knowledge base. Can't query "how do I handle errors across all my Rust work?"

**Proposed**: Persona layer that aggregates observations from multiple projects.

### Status

**DEFERRED TO PHASE 2**

This is the ultimate goal but requires project-level system to work first.

### Design Notes

When ready to implement:

```
~/.patina/persona/
├── observations.db     # Aggregated from all projects
├── projects/
│   ├── patina/        # Symlink to ~/Projects/patina/.patina/shared
│   ├── other-project/ # Symlink to ~/Projects/other/.patina/shared
```

**Aggregation strategy**:
1. Each project has `.patina/shared/events/`
2. Persona scans all registered projects
3. Reads events, deduplicates, builds unified observations.db
4. Query persona: "How do I handle errors?" → Answers from ALL projects

---

## Topic 9: Domain Relationships (Optional)

**Current State**: Domains are tags. No relationships between them.

**Problem**: Can't discover "modularity often appears with testing" or "async usually needs error-handling".

**Proposed**: Discover domain co-occurrence relationships during oxidize.

### Status

**OPTIONAL - DEFER UNTIL DOMAINS PROVEN USEFUL**

If domain tagging works well and users query by domain, then relationships add value. But test domains first.

### Design Notes

```rust
// When ready:
// 1. Cluster observations by semantic similarity
// 2. Count domain co-occurrence within clusters
// 3. Store relationships

CREATE TABLE domain_relationships (
    domain_a TEXT NOT NULL,
    domain_b TEXT NOT NULL,
    relationship_type TEXT NOT NULL,  -- "co_occurs_with"
    strength REAL NOT NULL,           -- 0.0-1.0
    discovered_at TIMESTAMP NOT NULL,
    PRIMARY KEY (domain_a, domain_b, relationship_type)
);
```

---

## Implementation Sequence

### Phase 0: Validation (Week 1)

**Goal**: Prove current system works and identify gaps.

**Tasks**:
1. Topic 1: Retrieval Quality Baseline (3 hours)
2. Topic 2: Session Extraction Quality (5 hours)

**Outcome**: Know if current retrieval works, understand extraction challenges.

---

### Phase 1: Event Foundation (Week 2)

**Goal**: Event sourcing proven and working.

**Tasks**:
1. Topic 4: Event Sourcing Spike (6 hours)
2. Topic 7: Materialize Command (12 hours)

**Outcome**: Can create events manually, materialize to DB, query.

---

### Phase 2: Domain Tagging (Week 3)

**Goal**: Domains work and improve retrieval.

**Tasks**:
1. Topic 3: Domain Tagging Experiment (4 hours)
2. Integrate domains into materialize (2 hours)
3. Test domain filtering: `patina query semantic "error handling" --domain rust` (2 hours)

**Outcome**: Domains add value, LLM tagging is reliable.

---

### Phase 3: Session Integration (Week 4)

**Goal**: Sessions automatically become observations.

**Tasks**:
1. Topic 5: Session Command Integration (6 hours)
2. Batch scrape all 266 sessions (2 hours)
3. Test retrieval quality improvement (2 hours)

**Outcome**: 266 sessions → ~500 observations, better retrieval than baseline.

---

### Phase 4: Git Extraction (Week 5)

**Goal**: Git commits add corroboration and coverage.

**Tasks**:
1. Topic 6: Git History Extraction (8 hours)
2. Test deduplication vs corroboration (2 hours)
3. Validate total observations ~800 (1 hour)

**Outcome**: Complete knowledge base with sessions + git.

---

### Phase 5: Polish & Use (Week 6)

**Goal**: Production-ready system in daily use.

**Tasks**:
1. Documentation (README, command help) (4 hours)
2. Performance tuning (embeddings, queries) (4 hours)
3. Use system for 1 week in real development (track issues)

**Outcome**: Patina is useful, gaps documented for next phase.

---

## Appendix: Command Reference

### Current Commands (Working)

```bash
# Project lifecycle
patina init <name>              # Initialize new project
patina doctor                   # Health check

# Code indexing
patina scrape code              # Index codebase structure

# Embeddings
patina embeddings generate      # Create vectors

# Querying
patina query semantic <text>    # Semantic search
patina belief validate <stmt>   # Neuro-symbolic validation
patina belief explain <stmt>    # Show validation reasoning

# Session management
/session-start <name>           # Begin session (Claude adapter)
/session-update                 # Log progress
/session-note <insight>         # Capture insight
/session-end                    # Finalize session
```

### Proposed New Commands

```bash
# Event sourcing
patina materialize              # Rebuild DB from events
patina materialize --force      # Full rebuild

# Scraping
patina session scrape <id>      # Extract single session
patina session scrape           # Extract all sessions
patina scrape git               # Extract git history

# Domain filtering (after Topic 3)
patina query semantic <text> --domain <name>   # Filter by domain
patina domains list                            # Show all domains
patina domains stats                           # Domain statistics
```

---

## Success Criteria: Full System

### Data Quality
- ✅ 800+ observations (sessions + git)
- ✅ <5% duplicates (deduplication working)
- ✅ 50-100 domains (emergent taxonomy)
- ✅ 80%+ observations have 2-5 domains

### Retrieval Quality
- ✅ Test queries score 4+/5 (manual evaluation)
- ✅ Better than baseline (Topic 1)
- ✅ Provenance chain works (observation → event → source)

### System Reliability
- ✅ 94 tests passing (preserve neuro-symbolic)
- ✅ Materialize handles 1000+ events
- ✅ Commands have `--help` text
- ✅ Errors are clear and actionable

### Usability
- ✅ `/session-end` creates events automatically
- ✅ Query response time <2 seconds
- ✅ Can rebuild DB from events (<5 min for 800 events)

---

## Key Principles

### Build, Measure, Learn
- Every topic has success criteria
- Test retrieval quality before adding complexity
- If something doesn't improve retrieval, stop

### Modularity
- Each topic is independent
- Can discuss and adjust without breaking others
- Can defer optional topics (relationships, persona)

### User Value First
- Focus on "does this help answer questions?"
- Not "is this architecturally elegant?"
- Spike before committing to large builds

### Preserve Working Systems
- Neuro-symbolic reasoning (94 tests) stays as-is
- Embeddings & vector search stays as-is
- Session system stays as-is (extend, don't replace)

---

## Review Process

### For Each Topic:

1. **Review Current State**: Is description accurate?
2. **Review Proposed Work**: Is approach sound?
3. **Review Success Criteria**: Are they measurable?
4. **Review Dependencies**: Anything missing?
5. **Review Time Estimate**: Realistic?

### Overall Questions:

1. Is implementation sequence logical?
2. Are any topics missing?
3. Should any topics be combined/split?
4. What's the highest risk? How to mitigate?

---

**Status**: Ready for Discussion
**Next Action**: Review Topic 1 (Retrieval Baseline) and decide whether to proceed

---

*This document captures current state honestly and proposes a modular path forward. Each topic can be built, validated, and discussed independently. The focus is user value (retrieval quality) not architectural purity.*
