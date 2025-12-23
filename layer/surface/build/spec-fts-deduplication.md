# Spec: FTS5 Deduplication

**Status:** In progress - fix applied, decision pending on benchmark update
**Parent:** [build.md](../../core/build.md)
**Origin:** Session 20251222-191614 (git archaeology of duplication bug)

## Problem

Layer documentation ranks #1 in both semantic AND lexical oracles individually, but ranks #5 in final fused output. Code symbols dominate despite losing both individual oracles.

**Root cause:** Functions are written to eventlog twice:
- `code.function` event (with rich metadata)
- `code.symbol` event (lightweight, for abandoned `code_search` table)

`populate_fts5()` pulls ALL `code.%` events, so the same function appears twice in FTS5. RRF fusion sums reciprocal ranks:
- spec-pipeline.md: ranks #1, #1 → score = 1/61 + 1/61 = 0.033
- code symbol with 2 entries: ranks #1, #2 → score = 1/61 + 1/62 = 0.032

With 8x duplication (4 symbol + 4 function across multiple scrape runs), code artificially dominates.

## Git Archaeology Summary

| Date | Commit | What Happened |
|------|--------|---------------|
| Aug 2025 | `b7c1ddf2` | `code_search` created for text search alongside `code_fingerprints` for fingerprint similarity |
| Sep 2025 | `5bce30c1` | ExtractedData refactor preserved dual-write: "don't break existing behavior" |
| Nov 2025 | `73f7a227` | Eventlog added with dual-write to BOTH tables AND eventlog |
| Nov 2025 | `44a43c5c` | FTS5 added, pulls from eventlog with `WHERE event_type LIKE 'code.%'` |

**Verdict:** `code_search` was designed for a fingerprint-based architecture that no longer exists. The symbol writes are vestigial. FTS5 inherited the duplication without anyone realizing the old architecture was gone.

## Key Finding

**`code_search` table is NEVER QUERIED by anything.** Grep confirms:
- `function_facts` → used by assay, oxidize, scry, MCP
- `type_vocabulary` → used by assay
- `code_search` → zero queries

The symbol events exist only to populate an orphaned table.

## Solution

### Phase 1: Filter at FTS Population (Immediate Fix)

Change `populate_fts5()` query to exclude `code.symbol` events:

```sql
-- Before (duplicates)
WHERE event_type LIKE 'code.%'
  AND json_extract(data, '$.name') IS NOT NULL

-- After (no duplicates)
WHERE event_type LIKE 'code.%'
  AND event_type != 'code.symbol'
  AND json_extract(data, '$.name') IS NOT NULL
```

**Rationale:**
- Surgical fix - one line change
- Low risk - doesn't touch extraction or language processors
- Immediately verifiable with benchmark
- Preserves symbol events in eventlog for future use if needed

### Phase 2: Clean Up Extraction (Future)

Remove `add_symbol()` calls for things that have richer fact types:
- Functions → have `FunctionFact`
- Types (struct/enum/trait) → have `TypeFact`
- Constants → have `ConstantFact`

Keep `add_symbol()` for things without richer types:
- Imports (though these also have `ImportFact` - review needed)
- Language-specific constructs without dedicated fact types

**Scope:** ~50 call sites across 10 language files. Defer until Phase 1 is validated.

### Phase 3: Remove Dead Code (Future)

- Remove `code_search` table from schema
- Remove `insert_symbols()` writes to `code_search`
- Consider removing `code.symbol` event type entirely

## Tasks

### Phase 1 (This Session)

| Task | File | Scope |
|------|------|-------|
| Add `event_type != 'code.symbol'` filter to populate_fts5() | `src/commands/scrape/database.rs` | ~1 line |
| Run fresh scrape | CLI | Verify |
| Run benchmark | `patina bench` | Validate MRR improvement |
| Test sharp queries from spec-observable-scry.md | Manual | Confirm layer docs surface |

### Phase 2 (Deferred)

| Task | Scope |
|------|-------|
| Audit all `add_symbol()` calls in language processors | 10 files |
| Remove redundant symbol writes for functions | ~20 sites |
| Remove redundant symbol writes for types | ~20 sites |
| Update tests | As needed |

### Phase 3 (Deferred)

| Task | Scope |
|------|-------|
| Remove `code_search` table from schema | `database.rs` |
| Remove `code_search` insert logic | `database.rs` |
| Consider deprecating `code.symbol` event type | Eventlog design |

## Validation

### Phase 1 Exit Criteria

| Criterion | How to Test |
|-----------|-------------|
| No duplicate entries in code_fts | `SELECT symbol_name, COUNT(*) FROM code_fts GROUP BY symbol_name, file_path HAVING COUNT(*) > 1` |
| Layer docs surface for relevant queries | `patina scry "pipeline architecture"` returns spec-pipeline.md in top 3 |
| MRR maintained or improved | `patina bench` shows MRR >= 0.624 |
| No regression in lexical search | Sharp test: `patina scry "RRF fusion"` returns engine.rs |

### Sharp Tests

From spec-observable-scry.md:

**Test 1: Orientation Query**
```
patina scry "What should I know about the retrieval module?"
```
Should include layer docs alongside code.

**Test 2: Targeted Query**
```
patina scry "Where is RRF fusion implemented?"
```
Should return implementation locations, not be dominated by duplicates.

## Design Alignment

| Principle | How This Honors It |
|-----------|-------------------|
| **unix-philosophy** | Fix does one thing: remove duplicates from FTS |
| **dependable-rust** | Internal change, no API change |
| **data quality > algorithm** | Fixes data, doesn't add fusion complexity |
| **git as memory** | Eventlog preserved, just filtered at query time |

## Test Results (Session 20251222-191614)

### Fix Applied
```rust
// src/commands/scrape/database.rs:80-100
WHERE event_type LIKE 'code.%'
  AND event_type != 'code.symbol'  // Filter duplicates
  AND json_extract(data, '$.name') IS NOT NULL
GROUP BY source_id, event_type     // Dedupe across scrape runs
```

### Metrics
| Metric | Before | After | Delta |
|--------|--------|-------|-------|
| FTS5 entries | 20,059 | 2,770 | -86% |
| MRR | 0.604 | 0.338 | -44% |
| Recall@5 | 42.7% | 17.7% | -59% |

### Sharp Test: "pipeline architecture" --hybrid
- **BEFORE:** `analyze_architecture` #1 (code beats doc)
- **AFTER:** `spec-pipeline` #1 (doc wins!) ✓

### Why Benchmark Regressed
Benchmark ground truth uses file-level doc_ids (`./src/foo.rs`), but FTS5 now returns symbol-level (`./src/foo.rs::bar`). Content IS found, just different format.

### Decision Required
1. **Keep fix + update benchmark** - Recommended
2. **Revert** - Unacceptable (layer docs don't surface)
3. **Hybrid** - Complex, unclear benefit

## References

- Session 20251222-191614 - Git archaeology and root cause analysis
- Session 20251222-141635 - Original discovery of duplication
- Session 20251214-175410 - BM25 scale mismatch (related but different issue)
- [spec-observable-scry.md](spec-observable-scry.md) - Sharp tests for validation
- [spec-pipeline.md](spec-pipeline.md) - Pipeline architecture context
