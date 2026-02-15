---
type: explore
id: grammar-measure
status: draft
created: 2026-02-14
sessions:
  origin: 20260214-205609
related:
  - layer/surface/build/feat/grammar-extraction/SPEC.md
  - layer/surface/build/feat/grammar-extraction/TEST-SPEC.md
beliefs:
  - context-loss-audit-required
  - graceful-extraction
  - parser-agnostic-interfaces
---

# explore: Grammar Extraction Quality — Invariants + Coverage

> Absolute quality measurement for grammar extraction output.
> Not "same as before" — "is this extraction actually good?"

## Motivation

`grammar-compare.sh` proves plugin = compiled-in. That's a port fidelity
test. It says nothing about whether the extraction is *correct*. A grammar
could faithfully extract zero functions from a 500-function file and pass
with flying colors.

We need two things:

1. **Invariants**: structural rules that must always hold. Violations are bugs.
2. **Coverage**: per-grammar extraction statistics. Not pass/fail — a
   distribution you can read, track over time, and compare across versions.

## Tool: `grammar-measure.sh <grammar|all> [repo-path]`

Single tool, two output sections per grammar. Runs against any scraped
repo (uses existing `patina.db`, no re-scrape needed).

### Output Format

```
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
  GRAMMAR: python
  REPO:    /Users/x/.patina/cache/repos/openai/codex
  FILES:   804 scraped
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

  INVARIANTS (22 rules)
  ─────────────────────
  ✓ 01  function_facts: all rows have non-empty name
  ✓ 02  function_facts: all rows have non-empty file
  ✗ 03  call_graph: all callers exist in function_facts        [14 orphans]
  ✓ 04  call_graph: all rows have non-empty callee
  ...
  RESULT: 21/22 passed (1 violation, 14 affected rows)

  COVERAGE
  ────────
  TABLE                FILES   ROWS   MIN  MEDIAN   P95    MAX
  function_facts         780    10654    1       8    42    312
  code_search            804    30102    1      22    98    587
  type_vocabulary        312     3962    1       6    38    201
  import_facts           798    14188    1      12    52    178
  call_graph             743   112253    1      88   502   3841
  constant_facts         421     3411    1       4    24    112
  member_facts           389     6921    1       8    51    298

  ZERO-EXTRACTION: 24/804 files (3.0%) — files with parseable code but
  no rows in any table.

  TOP EMPTY FILES (sample):
    src/vendor/six.py           (142 lines, 0 functions extracted)
    tests/fixtures/empty.py     (0 lines — expected)
    ...
```

## Invariants: The Rules

### Row-level (per table, per row)

| # | Table | Rule | What a violation means |
|---|-------|------|----------------------|
| 01 | function_facts | name is non-empty | Parser returned a function with no name |
| 02 | function_facts | file is non-empty | Lost file context during extraction |
| 03 | function_facts | start_line > 0 | Invalid line number |
| 04 | function_facts | end_line >= start_line (where both exist) | Swapped or missing line range |
| 05 | code_search | name is non-empty | Nameless symbol |
| 06 | code_search | kind is non-empty | Unclassified symbol |
| 07 | code_search | path is non-empty | Lost file context |
| 08 | import_facts | import_path is non-empty | Import with no target |
| 09 | import_facts | file is non-empty | Lost file context |
| 10 | call_graph | caller is non-empty | Call edge with no source |
| 11 | call_graph | callee is non-empty | Call edge with no target |
| 12 | call_graph | file is non-empty | Lost file context |
| 13 | type_vocabulary | name is non-empty | Nameless type |
| 14 | constant_facts | name is non-empty | Nameless constant |
| 15 | member_facts | container is non-empty | Member with no parent |
| 16 | member_facts | name is non-empty | Nameless member |

### Cross-table (referential integrity)

| # | Rule | What a violation means |
|---|------|----------------------|
| 17 | Every file in function_facts exists in code_search | Functions found but no symbols emitted for that file |
| 18 | call_graph caller exists in function_facts (same file) | Call edge references a function we didn't extract |
| 19 | member_facts container exists in type_vocabulary or function_facts | Member belongs to unknown parent |

### Aggregate (per-grammar)

| # | Rule | Threshold | What a violation means |
|---|------|-----------|----------------------|
| 20 | Zero-extraction rate | < 5% of files with >10 lines | Grammar can't parse real files |
| 21 | No table is completely empty | > 0 rows | Grammar produces no output for a table |
| 22 | Duplicate row rate | < 1% per table | Extraction walks the same node twice |

## Coverage: The Statistics

Per table, per grammar, against a ref repo:

| Metric | What it tells you |
|--------|------------------|
| **files** | How many files have at least 1 row in this table |
| **rows** | Total row count |
| **min** | Minimum rows per file (usually 1) |
| **median** | Typical file — is extraction finding a reasonable amount? |
| **p95** | Large files — is extraction scaling or hitting limits? |
| **max** | Largest extraction — sanity check for runaway duplication |
| **zero-extraction files** | Files with parseable code but 0 rows across all tables |

### Why percentiles, not averages

Extraction distributions are heavy-tailed. A 500-function file next to
fifty 3-function files makes the average useless. Median tells you the
typical experience. P95 tells you if large files work. Max catches
degenerate cases (one file producing 50k call edges = bug).

### Tracking over time

Output a single JSON line per grammar per run:

```json
{
  "grammar": "python",
  "repo": "openai/codex",
  "timestamp": "2026-02-14T21:00:00Z",
  "files_scraped": 804,
  "invariant_violations": 1,
  "zero_extraction_rate": 0.03,
  "tables": {
    "function_facts": { "files": 780, "rows": 10654, "min": 1, "median": 8, "p95": 42, "max": 312 },
    "code_search": { "files": 804, "rows": 30102, "min": 1, "median": 22, "p95": 98, "max": 587 }
  }
}
```

Append to `~/.patina/local/data/grammar-measure.jsonl`. Now you have a
time series. Diff two runs to see if a grammar version improved or regressed.

## Implementation Notes

### All queries are pure SQLite

No re-scrape needed. The tool reads `patina.db` directly. This means:

- Fast (~1s per grammar, not minutes)
- Can run on any repo that's been scraped
- Can run against historical databases

### Percentiles via SQLite

```sql
-- Median (p50)
SELECT rows_per_file FROM (
  SELECT COUNT(*) as rows_per_file
  FROM function_facts GROUP BY file
  ORDER BY rows_per_file
  LIMIT 1 OFFSET (SELECT COUNT(DISTINCT file) FROM function_facts) / 2
);

-- P95
LIMIT 1 OFFSET (SELECT COUNT(DISTINCT file) FROM function_facts) * 95 / 100
```

### Duplicate detection

```sql
SELECT file, name, params, return_type, start_line, COUNT(*) as n
FROM function_facts
GROUP BY file, name, params, return_type, start_line
HAVING n > 1;
```

Same pattern per table with appropriate columns.

### Zero-extraction files

Requires knowing which files SHOULD have been parsed. Join against
the file list from `code_search` or query the filesystem for matching
extensions.

```sql
-- Files in code_search with no function_facts
SELECT DISTINCT cs.path
FROM code_search cs
LEFT JOIN function_facts ff ON cs.path = ff.file
WHERE ff.file IS NULL
AND cs.path LIKE '%.py';
```

## Relationship to grammar-compare.sh

| Tool | Question | When to use |
|------|----------|-------------|
| `grammar-compare.sh` | Does the plugin match compiled-in? | After porting a grammar |
| `grammar-measure.sh` | Is the extraction actually good? | After any grammar change, before release |

They compose. Compare first (port fidelity), then measure (absolute quality).

## Future: Ground Truth Golden Files

Invariants catch structural bugs. Coverage catches degenerate extraction.
Neither catches *wrong* extraction (function exists but wrong name, or
import path is truncated). For that you need golden files:

- Hand-curate expected output for ~5 files per language
- Store in `resources/test/golden/<grammar>/`
- Exact diff against extraction output

Separate tool, separate spec. Invariants + coverage come first because
they're fully automatable and catch the most common problems.

## Status Log

| Date | Status | Note |
|------|--------|------|
| 2026-02-14 | draft | Sketched during session [[20260214-205609]] after all 7 grammars passed comparison test. Motivated by: "where is this a lie?" |
