# GitHub Integration Architecture

Visual overview of how GitHub data flows through Patina.

---

## Current Architecture (Code Only)

```
┌─────────────────────────────────────────────────────────────────┐
│                    patina repo add <url>                         │
└────────────────────────────┬────────────────────────────────────┘
                             │
                             ▼
                    ┌────────────────┐
                    │  Git Clone     │
                    │  to ~/.patina/ │
                    │     repos/     │
                    └────────┬───────┘
                             │
                             ▼
                    ┌────────────────┐
                    │  Code Scrape   │
                    │  - Git history │
                    │  - AST parse   │
                    │  - Symbols     │
                    └────────┬───────┘
                             │
                             ▼
                    ┌────────────────────┐
                    │   patina.db        │
                    │   ├─ eventlog      │
                    │   ├─ code.function │
                    │   ├─ git.commit    │
                    │   └─ fts_search    │
                    └────────┬───────────┘
                             │
                             ▼
                    ┌────────────────┐
                    │  oxidize       │
                    │  (embeddings)  │
                    └────────┬───────┘
                             │
                             ▼
                    ┌────────────────┐
                    │  scry          │
                    │  (query code)  │
                    └────────────────┘
```

---

## Proposed Architecture (Code + GitHub)

```
┌─────────────────────────────────────────────────────────────────┐
│           patina repo add <url> --with-issues                    │
└────────────────────┬────────────────────────┬───────────────────┘
                     │                        │
                     ▼                        ▼
            ┌────────────────┐      ┌──────────────────┐
            │  Git Clone     │      │  GitHub Fetch    │
            │  to ~/.patina/ │      │  - Issues        │
            │     repos/     │      │  - PRs (future)  │
            └────────┬───────┘      │  - Discussions   │
                     │              └────────┬─────────┘
                     │                       │
                     ▼                       ▼
            ┌────────────────┐      ┌──────────────────┐
            │  Code Scrape   │      │  Parse GitHub    │
            │  - Git history │      │  - Detect bounty │
            │  - AST parse   │      │  - Extract meta  │
            │  - Symbols     │      │  - Label tags    │
            └────────┬───────┘      └────────┬─────────┘
                     │                       │
                     └───────────┬───────────┘
                                 │
                                 ▼
                    ┌─────────────────────────┐
                    │      patina.db          │
                    │  ┌───────────────────┐  │
                    │  │ Code Events       │  │
                    │  ├─ eventlog         │  │
                    │  ├─ code.function    │  │
                    │  ├─ git.commit       │  │
                    │  └─────────┬─────────┘  │
                    │            │             │
                    │  ┌─────────▼─────────┐  │
                    │  │ GitHub Events NEW │  │
                    │  ├─ github.issue     │  │
                    │  ├─ github.pr        │  │
                    │  └─ github.comment   │  │
                    │            │             │
                    │  ┌─────────▼─────────┐  │
                    │  │ Unified FTS5      │  │
                    │  │ - Code content    │  │
                    │  │ - Issue bodies    │  │
                    │  │ - PR descriptions │  │
                    │  └─────────┬─────────┘  │
                    └────────────┼─────────────┘
                                 │
                                 ▼
                    ┌─────────────────────────┐
                    │      oxidize            │
                    │  ┌───────────────────┐  │
                    │  │ Code embeddings   │  │
                    │  │ E5 → Semantic MLP │  │
                    │  └───────────────────┘  │
                    │  ┌───────────────────┐  │
                    │  │ Issue embeddings  │  │
                    │  │ E5 → Semantic MLP │  │
                    │  │ (same space!)     │  │
                    │  └───────────────────┘  │
                    └─────────┬───────────────┘
                              │
                              ▼
                    ┌──────────────────────┐
                    │       scry           │
                    │  --include-issues    │
                    │  --label bounty      │
                    │  --state open        │
                    └──────────┬───────────┘
                               │
                               ▼
                    ┌──────────────────────┐
                    │   Combined Results   │
                    │  [CODE] file.rs:123  │
                    │  [ISSUE:dojo#234]    │
                    │  [CODE] other.rs:45  │
                    │  [ISSUE:cairo#89]    │
                    └──────────────────────┘
```

---

## Data Flow: Adding a Repo with Issues

### Step 1: Clone Repository

```bash
patina repo add dojoengine/dojo --with-issues
```

**Actions:**
1. Clone `dojoengine/dojo` to `~/.patina/repos/dojo/`
2. Create patina branch
3. Initialize `.patina/` directory structure

### Step 2: Scrape Code (Existing)

**Sources:**
- Git commits → `git.commit`, `git.commit_file`
- Code files → `code.function`, `code.call`
- Sessions → (if any exist)

**Output:**
- `~/.patina/repos/dojo/.patina/data/patina.db`
- Events stored in `eventlog` table
- FTS5 index populated with code content

### Step 3: Scrape GitHub (NEW)

**Command:**
```bash
gh issue list --repo dojoengine/dojo \
  --limit 1000 --state all \
  --json number,title,body,state,labels,author,createdAt,updatedAt,url
```

**Processing:**
1. Parse JSON response
2. Detect bounties (label + body analysis)
3. Extract metadata (labels, state, timestamps)
4. Insert into `github_issues` table
5. Insert into `fts_search` table

**Output:**
- 200-500 issues stored
- Bounties flagged
- Full-text indexed

### Step 4: Generate Embeddings (Existing + Enhanced)

**For Code:**
```
code.function → extract text → E5 embed → Semantic MLP → 256-dim
```

**For Issues (NEW):**
```
github.issue → title + body → E5 embed → Semantic MLP → 256-dim
```

**Key insight:** Same semantic space! Issues and code are comparable.

### Step 5: Register in Mothership

**Update `~/.patina/registry.yaml`:**
```yaml
repos:
  dojo:
    path: ~/.patina/repos/dojo
    github: dojoengine/dojo
    contrib: false
    registered: 2025-11-28T10:00:00Z
    domains: [cairo, starknet, ecs]
    github_data:                        # NEW
      issues: true
      issue_count: 347
      open_bounties: 12
      last_github_scrape: 2025-11-28T10:00:00Z
```

---

## Query Flow: Finding Bounties

### User Query

```bash
patina scry "cairo testing patterns" \
  --repo dojo \
  --include-issues \
  --label bounty \
  --state open
```

### Step 1: Parse Query

**Flags:**
- `--repo dojo` → Query `~/.patina/repos/dojo/.patina/data/patina.db`
- `--include-issues` → Include `github.issue` event types
- `--label bounty` → Filter `labels LIKE '%bounty%'`
- `--state open` → Filter `state = 'open'`

**Query text:** "cairo testing patterns"

### Step 2: Dual Search Strategy

**A. Lexical Search (FTS5)**

```sql
SELECT content_type, title, content, path, rank
FROM fts_search
WHERE fts_search MATCH 'cairo AND testing AND patterns'
  AND content_type IN ('code', 'issue')  -- include issues
ORDER BY rank
LIMIT 50;
```

**Returns:**
- `[CODE] src/tests/cairo_test.rs` - Contains "cairo testing"
- `[ISSUE:dojo#156] Bounty: Add cairo integration tests` - Title match
- `[CODE] src/cairo/patterns.rs` - Contains "patterns"

**B. Semantic Search (Vector)**

```sql
-- Embed query: "cairo testing patterns" → E5 → 768-dim → Semantic MLP → 256-dim

SELECT et.event_type, et.event_id, e.similarity
FROM embeddings e
JOIN eventlog et ON e.event_id = et.event_id
WHERE et.event_type IN ('code.function', 'github.issue')  -- include issues
  AND e.dimension = 'semantic'
ORDER BY usearch_similarity(e.vector, :query_vector) DESC
LIMIT 50;
```

**Returns (ordered by semantic similarity):**
- `[ISSUE:dojo#234] Implement cairo test framework` - High semantic match
- `[CODE] src/tests/integration.rs:45` - Test patterns code
- `[ISSUE:dojo#189] Testing best practices for cairo` - Conceptually similar

### Step 3: Filter by GitHub Metadata

**Apply `--label bounty --state open`:**

```sql
SELECT *
FROM github_issues
WHERE number IN (:issue_numbers_from_search)
  AND labels LIKE '%bounty%'
  AND state = 'open';
```

**Filters results to only:**
- Open issues
- Tagged as bounty
- Matching search query

### Step 4: Merge and Rank

**Combine FTS5 + Semantic + Filters:**
1. Union lexical and semantic results
2. Apply GitHub filters (label, state)
3. Re-rank by combined score
4. Group by result type (code vs issue)

### Step 5: Display Results

```
🔍 Results for "cairo testing patterns" in dojo (4 found)

💰 Bounties (2)
  #234 [OPEN] Implement cairo test framework (500 USDC)
       Labels: bounty, cairo, testing
       https://github.com/dojoengine/dojo/issues/234

  #189 [OPEN] Testing best practices for cairo (200 USDC)
       Labels: bounty, documentation, cairo
       https://github.com/dojoengine/dojo/issues/189

📄 Code (2)
  src/tests/cairo_test.rs:45
    Test runner implementation for cairo contracts

  src/cairo/patterns.rs:123
    Common testing patterns for cairo development
```

---

## Semantic Space Unification

**Critical insight:** Issues and code share the same semantic dimension.

```
Query: "entity spawning patterns"
  │
  ├─> E5-base-v2 (768-dim)
  │
  └─> Semantic MLP (768 → 1024 → 256)
      │
      ├─> Compare with CODE embeddings
      │   ├─ src/ecs/spawn.rs (similarity: 0.87)
      │   └─ src/world/entity.rs (similarity: 0.76)
      │
      └─> Compare with ISSUE embeddings
          ├─ Issue #234: "Entity spawning optimization" (similarity: 0.91)
          └─ Issue #156: "Spawn patterns documentation" (similarity: 0.83)
```

**Why this works:**
- E5 model trained on code AND natural language
- Same embedding model for both code and issues
- Same MLP projection maintains semantic relationships
- Issues often describe code concepts → high overlap

**Training signal for semantic dimension:**
- **Code**: Same session = related (existing)
- **Issues**: Same labels = related (NEW)
- **Cross-type**: Issue mentioning file = related (future)

---

## Storage Schema

### New Tables

```sql
-- GitHub issues
CREATE TABLE github_issues (
    id INTEGER PRIMARY KEY,
    number INTEGER NOT NULL,
    title TEXT NOT NULL,
    body TEXT,
    state TEXT NOT NULL,        -- open, closed
    labels TEXT,                -- JSON array
    author TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    closed_at TEXT,
    url TEXT NOT NULL,
    is_bounty BOOLEAN DEFAULT 0,
    bounty_amount TEXT,

    -- Link to eventlog
    event_id INTEGER UNIQUE,
    FOREIGN KEY (event_id) REFERENCES eventlog(id)
);

-- Extend eventlog
INSERT INTO eventlog (event_type, timestamp, metadata)
VALUES (
    'github.issue',
    '2025-11-28T10:00:00Z',
    json_object(
        'number', 234,
        'title', 'Implement storage optimization',
        'is_bounty', true
    )
);

-- Extend embeddings (reuse existing table!)
INSERT INTO embeddings (event_id, dimension, vector)
VALUES (
    :event_id,
    'semantic',
    :issue_embedding  -- Same 256-dim as code
);

-- Extend FTS5 (reuse existing table!)
INSERT INTO fts_search (content_type, title, content, path)
VALUES (
    'issue',
    'Implement storage optimization',
    'Currently, large worlds with 10k+ entities...',
    'https://github.com/dojoengine/dojo/issues/234'
);
```

**Key insight:** Minimal schema changes! Reuse existing infrastructure.

---

## Incremental Updates

### First Scrape

```bash
patina repo add dojoengine/dojo --with-issues
# Fetches all 500 issues (~15 seconds)
```

### Daily Update

```bash
patina repo update dojo --with-issues
# Only fetches issues updated since last scrape (~1 second)
```

**Implementation:**
```bash
# Track last scrape time in registry
LAST_SCRAPE=$(yq eval '.repos.dojo.github_data.last_github_scrape' registry.yaml)

# Only fetch updated issues
gh issue list --repo dojoengine/dojo \
  --search "updated:>=$LAST_SCRAPE" \
  --json ...

# Update only changed issues (INSERT OR REPLACE)
```

**Efficiency:**
- Initial: 500 issues = 2.5MB, 15 seconds
- Daily: 5-10 issues = 50KB, 1 second
- Weekly: 20-30 issues = 150KB, 2 seconds

---

## Cross-Project Queries (Future)

### Query All Repos for Bounties

```bash
patina scry "bounty cairo" --all-repos --include-issues --label bounty --state open
```

**Process:**
1. Load registry: `~/.patina/registry.yaml`
2. Find all repos with `github_data.issues: true`
3. Query each repo's database
4. Merge results
5. Rank by semantic similarity

**Output:**
```
💰 Bounties matching "cairo" (5 found across 3 repos)

[dojo#234] Implement cairo test framework (500 USDC)
[cairo#891] Add felt252 serialization (200 USDC)
[madara#45] Fix cairo compiler warnings (100 USDC)
[dojo#189] Cairo testing best practices (200 USDC)
[cairo#776] Optimize cairo compilation (300 USDC)
```

### Persona-Aware Matching (Phase 4)

```bash
patina scry "bounty" --match-my-skills
```

**Process:**
1. Load persona domains from `~/.patina/persona/domains/`
2. Your domains: `[cairo, rust, ecs, testing]`
3. Filter bounties by domain overlap
4. Rank by skill match + bounty amount

**Output:**
```
💰 Bounties matching your skills (3 found)

[dojo#234] cairo + ecs: Implement entity caching (500 USDC)
  Your skills: cairo ✓, ecs ✓, testing ✓
  Match: 100%

[cairo#891] cairo + testing: Add serialization tests (200 USDC)
  Your skills: cairo ✓, testing ✓
  Match: 66%

[madara#102] rust + cairo: Optimize RPC layer (400 USDC)
  Your skills: rust ✓, cairo ✓
  Match: 66%
```

---

## Implementation Phases (Detailed)

### Phase 1: Issues MVP (2-3 days)

**Day 1: Schema + Scraping**
- [ ] Add `github_issues` table to `src/schema.sql`
- [ ] Create `src/commands/scrape/github.rs`
- [ ] Implement `scrape_github_issues()` using `gh issue list`
- [ ] Insert issues into `eventlog` and `github_issues`
- [ ] Add to FTS5 index

**Day 2: Query Integration**
- [ ] Add `--include-issues` flag to `scry` command
- [ ] Filter by `--label`, `--state` flags
- [ ] Update result display to show issue metadata
- [ ] Test with dojoengine/dojo

**Day 3: Bounty Detection**
- [ ] Implement `detect_bounty()` logic
- [ ] Extract bounty amount from body
- [ ] Add bounty-specific display in results
- [ ] Test with OnlyDust repos

**Acceptance:**
```bash
patina repo add dojoengine/dojo --with-issues
patina scry "bounty cairo" --repo dojo --include-issues --label bounty
# Returns: 5+ bounties with correct metadata
```

### Phase 2: Semantic Search (1-2 days)

**Day 1: Embeddings**
- [ ] Generate E5 embeddings for issue title + body
- [ ] Store in existing `embeddings` table
- [ ] Use existing semantic MLP (same space as code)

**Day 2: Query**
- [ ] Semantic search includes issue embeddings
- [ ] Test cross-type ranking (code vs issues)
- [ ] Verify relevance

**Acceptance:**
```bash
patina scry "entity component patterns" --repo dojo --include-issues
# Returns: Mix of code files and issues, semantically relevant
```

### Phase 3: PRs and Discussions (2-3 days)

**Similar to Phase 1, but for:**
- [ ] `github_prs` table
- [ ] `github_discussions` table
- [ ] `gh pr list` integration
- [ ] `gh api graphql` for discussions

### Phase 4: Cross-Project (1 day)

- [ ] `--all-repos` flag in scry
- [ ] Query multiple databases
- [ ] Merge and rank results
- [ ] Domain filtering

---

## Success Metrics

### Phase 1 (Issues MVP)

✅ **Complete when:**
- Can find OnlyDust bounties via `scry --include-issues`
- Lexical search works on issue titles and bodies
- Results show issue metadata (number, labels, state, URL)
- Bounty detection >90% accurate (manual validation)
- Can filter by labels and state

### Phase 2 (Semantic)

✅ **Complete when:**
- Semantic search works across code + issues
- Issue relevance comparable to code relevance
- Cross-type results ranked appropriately
- E5 embeddings reuse existing infrastructure

### Hackathon-Ready

✅ **Complete when:**
- Can discover bounties across 3+ repos
- Query time <2 seconds for cross-repo search
- Semantic + lexical combined relevance validated
- OnlyDust workflow tested end-to-end

---

## Risk Mitigation

### 1. GitHub Rate Limits

**Risk:** 5000 req/hour limit
**Mitigation:**
- Batch requests (1000 issues per call)
- Incremental updates (only fetch changed issues)
- Cache responses locally
- Monitor via `gh api rate_limit`

### 2. Storage Growth

**Risk:** 500 repos × 200 issues × 5KB = 500MB
**Mitigation:**
- Reasonable for modern systems
- Cleanup old closed issues (configurable)
- Compress old data

### 3. Relevance Quality

**Risk:** Issue results not relevant to query
**Mitigation:**
- Start with lexical (FTS5) - proven to work
- Add semantic incrementally
- Measure precision@10 like code search
- User feedback loop

### 4. Authentication

**Risk:** `gh` CLI not authenticated
**Mitigation:**
- Check auth before scraping: `gh auth status`
- Clear error message: "Run `gh auth login`"
- Document in README

---

## Conclusion

GitHub integration is a **natural extension** of Patina's existing architecture:

✅ **Reuses existing infrastructure:**
- Same eventlog schema (just new event types)
- Same FTS5 index (just new content types)
- Same embeddings table (same semantic space)
- Same query interface (just new flags)

✅ **High value for low cost:**
- 2-3 days for issues MVP
- Unlocks OnlyDust bounty discovery
- Enables hackathon workflows
- Minimal storage overhead

✅ **Clear upgrade path:**
- Phase 1: Issues (MVP)
- Phase 2: Semantic search
- Phase 3: PRs + discussions
- Phase 4: Cross-project + persona

**Next step:** Implement Phase 1 Issues MVP
