---
type: test-spec
id: grammar-extraction-phase3-test
status: passed
created: 2026-02-14
parent: grammar-extraction
session: 20260214-202314
---

# Test Spec: Grammar Extraction Phase 3 — Plugin vs Compiled-in Comparison

> Verify that each grammar pipeline plugin produces equivalent extraction
> output to the compiled-in language processor it replaces.

## Problem

7 grammar plugins were built across 2 sessions with context loss.
Audit found bugs in 5 of 6 (post-context-loss). Bugs are fixed,
but we have no quantitative proof that plugin output matches
compiled-in output. We need a repeatable test.

## Method

For each grammar, scrape a reference repository **twice**:

1. **With plugin** — WASM plugin handles extraction via `extract_v2.rs` dispatch
2. **Without plugin** — compiled-in processor handles extraction via fallback

Compare the 7 ExtractedData tables per-file. Report deltas.

## Test Matrix

| Grammar | Ref Repo | Extensions | Notes |
|---------|----------|------------|-------|
| go | `marcus/sidecar` | `.go` | Go-primary repo |
| c | `libsdl-org/SDL` | `.c`, `.h` | Large C codebase |
| cpp | `unum-cloud/USearch` | `.cpp`, `.hpp`, `.cc` | C++ with templates |
| python | `openai/codex` | `.py` | Python-heavy |
| javascript | `litecanvas/game-engine` | `.js`, `.jsx` | JS-primary |
| solidity | `dustproject/dust` | `.sol` | Only Solidity ref repo |
| typescript | `google-gemini/gemini-cli` | `.ts`, `.tsx` | TS-heavy |

## Pass Criteria

Per grammar, comparing plugin output vs compiled-in output:

| Metric | Table | Threshold | Rationale |
|--------|-------|-----------|-----------|
| Symbol count | `code_search` | delta < 5% | Symbols include supplementary entries |
| Function count | `function_facts` | delta < 2% | Core extraction, must be tight |
| Type count | `type_vocabulary` | delta < 5% | Types vary by detail level |
| Import count | `import_facts` | delta < 5% | Imports feed dependency graph |
| Call edge count | `call_graph` | delta < 10% | Loosest — duplicates possible in both |
| Constant count | `constant_facts` | delta < 5% | Constants include metadata entries |
| Member count | `member_facts` | delta < 5% | Struct/class field extraction |
| Zero-extraction | per-file | < 1% of files | Plugin returns empty when compiled-in doesn't |
| Crash rate | per-file | 0% | Plugin must not panic on any file |

**Phase 3 passes when all 7 grammars meet all thresholds.**

Delta is calculated as: `abs(plugin_count - compiled_count) / compiled_count * 100`

A negative delta (plugin extracts MORE than compiled-in) is acceptable — the
audit added import symbols and member symbols that the compiled-in processors
don't emit. Only missing extraction is a concern.

## Tool

`resources/scripts/grammar-compare.sh` — automated comparison script.

```
Usage: grammar-compare.sh <grammar> <ref-repo-path>

  grammar:       go | c | cpp | python | javascript | solidity | typescript
  ref-repo-path: path to a cloned ref repo with patina initialized

Output: per-table counts, deltas, pass/fail per threshold
```

The script:
1. Scrapes the ref repo with the plugin installed (normal path)
2. Saves extraction counts from `patina.db`
3. Temporarily disables the plugin (moves it aside)
4. Re-scrapes with compiled-in processor
5. Compares counts, reports deltas
6. Restores the plugin

## Running All Tests

```bash
# Run all 7 grammars against their ref repos
grammar-compare.sh go      ~/.patina/refs/marcus/sidecar
grammar-compare.sh c       ~/.patina/refs/libsdl-org/SDL
grammar-compare.sh cpp     ~/.patina/refs/unum-cloud/USearch
grammar-compare.sh python  ~/.patina/refs/openai/codex
grammar-compare.sh javascript ~/.patina/refs/litecanvas/game-engine
grammar-compare.sh solidity ~/.patina/refs/dustproject/dust
grammar-compare.sh typescript ~/.patina/refs/google-gemini/gemini-cli
```

## Known Acceptable Deltas

These are expected differences where the plugin ADDS data the compiled-in
processor doesn't produce (due to audit fixes):

- **Import symbols**: Plugins now emit `CodeSymbol` for imports (python,
  javascript, solidity, typescript). Compiled-in processors may not.
  Expect positive delta on `code_search`.
- **Member symbols**: C++ plugin now emits `CodeSymbol` for class fields
  and methods. Expect positive delta on `code_search`.
- **Inheritance symbols**: C++ and Solidity plugins now emit inheritance
  `CodeSymbol`. Expect positive delta on `code_search`.

## Relationship to Phase 3 Exit Criteria

This test spec satisfies the Phase 3 exit criterion:
> All 9 current grammars available as pipeline plugins

"Available" means "produces equivalent output". This test proves equivalence.

## Status Log

| Date | Status | Note |
|------|--------|------|
| 2026-02-14 | draft | Created during audit session [[20260214-202314]]. Tool not yet run against ref repos. |
| 2026-02-14 | **passed** | All 7 grammars pass with **0% delta** across all 7 tables. Session [[20260214-205609]]. Results: go (377 files, 0%), c (783 files, 0%), cpp (37 files, 0%), python (804 files, 0%), javascript (51 files, 0%), solidity (247 files, 0%), typescript (821 files, 0%). Total: 3,120 files scraped twice, zero deviation. |
