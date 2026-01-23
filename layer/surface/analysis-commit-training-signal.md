---
id: analysis-commit-training-signal
status: active
created: 2026-01-06
oxidizer: nicabar
tags: [analysis, commits, training, semantic, ref-repos]
references: [concept-repo-patina, spec/mothership-graph]
---

# Commit Messages as Training Signal for Ref Repo Semantic Projection

## Problem Statement

Ref repos lack user sessions, which patina uses to train semantic projections ("same session = similar"). We need an alternative training signal for ref repos.

**Hypothesis:** Commit messages are natural language descriptions of code changes, providing (NL, code) pairs for free.

---

## Training Strategy

```
Anchor:   commit message (natural language)
Positive: content from files touched by commit
Negative: content from files NOT touched by commit
```

This trains the projection to bring natural language queries close to relevant code.

---

## Patina vs Dojo: Baseline Comparison

### Basic Stats

| Metric | Patina | Dojo |
|--------|--------|------|
| **Total commits** | 1,089 | 2,702 |
| **Timespan** | 6 months (Jul 2025 - Jan 2026) | 2 years (Jan 2023 - Dec 2025) |
| **Contributors** | 1 (human + LLM) | 10+ (team) |
| **Avg message length** | 49-67 chars | 22-70 chars |
| **Conventional commits** | 91% | 62% |

### Commit Type Distribution

```
PATINA                          DOJO
──────────────────────────────────────────────────
docs:     317 (29%)             other:  1036 (38%)
feat:     294 (27%)             fix:     576 (21%)
refactor: 167 (15%)             feat:    486 (18%)
fix:      128 (12%)             refactor: 293 (11%)
other:     97 (9%)              chore:   222 (8%)
```

**Observation:** Patina is docs-heavy (29%), dojo is fix-heavy (21%). Patina documents as it goes; dojo fixes as it grows.

### Scope Patterns

**Patina scopes (conceptual):**
```
build(45), scry(27), spec(25), secrets(23), scrape(21),
sessions(18), oxidize(13), assay(13), retrieval(12)
```

**Dojo scopes (architectural):**
```
katana(246), torii(111), sozo(111), ci(50), dojo-core(21),
dojo-lang(20), torii-indexer(17), katana-core(16)
```

### Moments (High-Signal Commits)

**Dojo moments:**
```
genesis:   1   (dojo init)
breaking:  3   (starknet-rs changes, blockifier, CLI redesign)
big_bang: 12   (executor rewrite, Cairo bumps, v1.0, namespaces)
migration: 37
rewrite:  300
```

---

## Training Signal Quality Factors

| Factor | Description | Impact |
|--------|-------------|--------|
| **Format consistency** | Conventional commits (feat, fix, refactor) | Easier parsing |
| **Message informativeness** | Descriptive vs "fix bug" | Better NL signal |
| **Code relevance** | feat/fix touch code; docs may not | Training pair quality |
| **Architectural signal** | Scopes indicate subsystems | Domain clustering |
| **Noise level** | "wip", "Update README" | Filter needed |

---

## Filtering Recommendations

### Include (high signal):
```sql
WHERE message LIKE 'feat%'
   OR message LIKE 'fix%'
   OR message LIKE 'refactor%'
   OR message LIKE 'perf%'
```

### Exclude (noise):
```sql
WHERE message NOT LIKE 'wip%'
  AND message NOT LIKE 'Update %'
  AND message NOT LIKE 'Merge %'
  AND length(message) > 30
```

### Boost (high signal):
```sql
-- Commits identified as moments
WHERE sha IN (
  SELECT sha FROM moments
  WHERE moment_type IN ('breaking', 'big_bang', 'migration')
)
```

---

## Ref Repo Analysis

### Analysis Metrics

For each ref repo, we measure:
1. **Total commits** - raw volume
2. **Conventional %** - format consistency
3. **Type distribution** - feat/fix/docs/other balance
4. **Top scopes** - architectural signal
5. **Moments** - high-signal events
6. **Message quality** - length distribution, noise level

### Results by Repo

#### Summary Table

| Repo | Commits | Timespan | Contributors | Conv% | Avg Len | Moments | Quality |
|------|---------|----------|--------------|-------|---------|---------|---------|
| **SDL** | 20,844 | 1970-2026 | 848 | 23% | 57 | 293 | ⭐⭐⭐ |
| **opencode** | 6,539 | Mar-Jan 2026 | 500 | 50% | 32 | 171 | ⭐⭐⭐⭐ |
| **gemini-cli** | 3,742 | Apr-Jan 2026 | 428 | 55% | 58 | 277 | ⭐⭐⭐⭐⭐ |
| **scryer-prolog** | 3,624 | 2016-2025 | 59 | 13% | 45 | 37 | ⭐⭐ |
| **dojo** | 2,702 | Jan 2023-Dec 2025 | 125 | 62% | 46 | 380 | ⭐⭐⭐⭐ |
| **codex** | 2,600 | Apr-Jan 2026 | 293 | 54% | 53 | 75 | ⭐⭐⭐⭐ |
| **livestore** | 2,225 | Sep 2023-Dec 2025 | 44 | 57% | 41 | 387 | ⭐⭐⭐⭐ |
| **USearch** | 1,798 | Feb 2023-Dec 2025 | 88 | 52% | 36 | 139 | ⭐⭐⭐ |
| **starknet-foundry** | 1,665 | Jul 2023-Dec 2025 | 91 | 17% | 42 | 95 | ⭐⭐ |
| **dust** | 963 | Feb 2024-Oct 2025 | 10 | 17% | 27 | 55 | ⭐⭐ |
| **daydreams** | 960 | Dec 2024-Oct 2025 | 29 | 27% | 18 | 34 | ⭐⭐ |
| **game-engine** | 536 | Feb 2024-Nov 2025 | 4 | 13% | 16 | 5 | ⭐ |
| **Personal_AI_Infrastructure** | 362 | Sep 2025-Jan 2026 | 9 | 62% | 54 | 30 | ⭐⭐⭐⭐ |

#### Commit Type Distribution

```
REPO                 feat   fix   refactor  docs  chore  other   CONV%
─────────────────────────────────────────────────────────────────────────
SDL               20,844    1  3942      24    92      1  15976    23%
opencode           6,539  389  1344      22   468    644   3246    50%
gemini-cli         3,742  577   934     188   127    195   1677    55%
scryer-prolog      3,624    -   432      11     2      -   3156    13%
dojo               2,702  486   576     293    23    222    995    62%
codex              2,600  356   637      21    54    321   1197    54%
livestore          2,225  113   374     274   191    251    953    57%
USearch            1,798    2   446     131   149      6    870    52%
starknet-foundry   1,665   13   203      36    19      4   1377    17%
dust                 963   17   122       9     2      4    803    17%
daydreams            960   15   120      10    47     30    705    27%
game-engine          536    -    70       1     -      -    464    13%
PAI                  362   63    96       6    47      7    139    62%
```

#### Moments Distribution

```
REPO              genesis  breaking  big_bang  migration  rewrite  major  TOTAL
──────────────────────────────────────────────────────────────────────────────────
livestore              1         3        21         20      302     40    387
dojo                   1         3        12         37      300     27    380
SDL                    1         5        62         87       74     64    293
gemini-cli             1         4         3         34      220     15    277
opencode               1         7        20          7      114     22    171
USearch                1         1         -          -      136      1    139
starknet-foundry       1         1        12         14       40     27     95
codex                  1         1         5         18       41      9     75
dust                   1         2         5         14        7     26     55
scryer-prolog          1         4         -          5       19      8     37
daydreams              1         -         7          -       13     13     34
PAI                    1         -         9          3       10      7     30
game-engine            1         -         -          -        2      2      5
```

#### Top Scopes by Repo

**gemini-cli** (best scope coverage):
```
core(206), cli(143), release(50), ui(49), ci(43), telemetry(39), infra(37)
```

**opencode**:
```
desktop(287), tui(201), app(24), share(17), cli(9), core(8), lsp(6)
```

**dojo**:
```
katana(246), torii(111), sozo(111), ci(50), dojo-core(21), dojo-lang(20)
```

**Most repos**: Minimal scope usage (no-scope dominant)

#### Sample Non-Conventional Messages

**SDL** (descriptive but not conventional):
```
"Added SDL_isinf(), SDL_isinff(), SDL_isnan(), and SDL_isnanf()"
"Document the range of trigger axes for virtual joysticks"
"cmake: cache cmake config installation folder in SDL_INSTALL_CMAKEDIR_ROOT"
```

**opencode** (mixed):
```
"release: v0.3.128"
"core: fix additions and deletions counting in edit tool filediff"
"wip: zen"
```

**game-engine** (very terse):
```
"update", "fix", "ajustes", "limpar"
```

---

### Repo-Specific Notes

#### Tier 1: High Signal (⭐⭐⭐⭐⭐)

**gemini-cli** - Best training candidate
- 55% conventional, 58 char avg
- Excellent scope coverage (core, cli, ui, telemetry)
- 277 moments for boosting
- Rich commit messages

#### Tier 2: Good Signal (⭐⭐⭐⭐)

**dojo** - Strong architectural signal
- 62% conventional, strong scopes (katana/torii/sozo)
- 380 moments (most rewrites)
- Team project with varied perspectives

**opencode** - Modern AI CLI patterns
- 50% conventional, good scopes (desktop, tui)
- 171 moments, rapid development

**codex** - OpenAI reference
- 54% conventional, 53 char avg
- PR-style commits with context

**livestore** - State management patterns
- 57% conventional, 387 moments
- Heavy refactoring signal

**Personal_AI_Infrastructure** - Similar domain
- 62% conventional, 54 char avg
- Small but focused

#### Tier 3: Medium Signal (⭐⭐⭐)

**SDL** - Huge but noisy
- 20K commits, but only 23% conventional
- Descriptive messages but non-standard format
- 293 moments for filtering

**USearch** - Useful domain
- 52% conventional, 139 moments
- Vector search patterns relevant to patina

#### Tier 4: Low Signal (⭐⭐)

**scryer-prolog, starknet-foundry, dust, daydreams**
- Low conventional % (<30%)
- Sparse or terse messages
- Filter heavily or skip

#### Tier 5: Skip (⭐)

**game-engine**
- Only 536 commits, 13% conventional
- Very terse messages (avg 16 chars)
- Minimal moments (5)

---

## Implementation

### Training Pair Generation

```rust
// src/commands/oxidize/commits.rs

pub fn generate_commit_pairs(db_path: &str, num_pairs: usize) -> Result<Vec<TrainingPair>> {
    // 1. Load meaningful commits (filter noise)
    let commits = query_filtered_commits(db)?;

    // 2. For each commit:
    //    - anchor = commit message
    //    - positive = random touched file content
    //    - negative = random untouched file content

    // 3. Optionally weight by moment type
}
```

### Fallback Strategy in oxidize

```rust
"semantic" => {
    if has_sessions(db_path)? {
        // User project: use session observations
        generate_same_session_pairs(db_path, num_pairs)?
    } else {
        // Ref repo: use commit messages
        generate_commit_pairs(db_path, num_pairs)?
    }
}
```

---

## Key Findings

### 1. Commit Messages ARE Viable Training Signal

**Total corpus:** 48,560 commits across 13 ref repos

**Usable after filtering:** ~25,000 commits (conventional + length > 30)

**High-signal moments:** 2,018 moments (breaking, big_bang, migration)

### 2. Quality Varies Dramatically

| Tier | Repos | Conv% | Strategy |
|------|-------|-------|----------|
| Tier 1 | gemini-cli | 55% | Use as-is |
| Tier 2 | dojo, opencode, codex, livestore, PAI | 50-62% | Filter + boost |
| Tier 3 | SDL, USearch | 23-52% | Heavy filter, use moments |
| Tier 4 | scryer-prolog, starknet-foundry, dust, daydreams | <30% | Moments only |
| Tier 5 | game-engine | 13% | Skip |

### 3. Scopes Reveal Architecture

Best scope signals:
- **gemini-cli**: core, cli, ui, telemetry (product architecture)
- **dojo**: katana, torii, sozo (three pillars)
- **opencode**: desktop, tui, app (delivery modes)

These can inform domain-weighted training.

### 4. Moments Are Universal

Every repo has moments derived. These are the highest-signal commits:
- **breaking**: API changes (training on "what changed")
- **big_bang**: Major features (training on "what's important")
- **migration**: Schema/structure changes (training on "how it evolved")

---

## Recommended Training Pipeline

### Phase 1: Filter

```sql
SELECT sha, message FROM commits
WHERE (
    message LIKE 'feat%'
    OR message LIKE 'fix%'
    OR message LIKE 'refactor%'
    OR message LIKE 'perf%'
)
AND length(message) > 30
AND message NOT LIKE '%wip%'
AND message NOT LIKE 'Merge %'
```

### Phase 2: Boost

```sql
-- Weight moments higher
SELECT c.sha, c.message,
  CASE
    WHEN m.moment_type = 'breaking' THEN 3.0
    WHEN m.moment_type = 'big_bang' THEN 2.0
    WHEN m.moment_type = 'migration' THEN 1.5
    ELSE 1.0
  END as weight
FROM commits c
LEFT JOIN moments m ON c.sha = m.sha
```

### Phase 3: Generate Pairs

```
For each commit:
  anchor   = commit.message
  positive = random file content from commit_files WHERE sha = commit.sha
  negative = random file content from code_search WHERE path NOT IN touched_files
```

### Phase 4: Train

Same MLP as current oxidize (768 → 1024 → 256), with weighted sampling based on moment type.

---

## Next Steps

1. **Implement `generate_commit_pairs()`** in `src/commands/oxidize/commits.rs`
2. **Add fallback** in oxidize: use commits when no sessions exist
3. **Run on Tier 1-2 repos** first (gemini-cli, dojo, opencode, codex, livestore)
4. **Measure** semantic search quality before/after
5. **Iterate** on filtering if needed

---

## References

- [concept-repo-patina](./concept-repo-patina.md) - Repo patina extraction vision
- spec/mothership-graph (archived git tag) - Graph routing for ref repos
- Session 20260106-190145 - Origin of this analysis
