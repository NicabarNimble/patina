# Spec: Quality Gates (Measurement-First)

**Status:** Active

**Philosophy:** Andrew Ng's practical ML approach applied to dev tools - measure before optimizing, clean before extending, ship with confidence.

**Scope note:** Through this quality gates work, we'll refine what Patina IS - an evolving layer of knowledge that covers projects and connects to a unifying system. Features like `yolo` and `hosts-deploy` may move to a future orchestration layer. Measurement first, then scope clarity.

---

## The Problem

Patina has grown to ~41k lines across 20 commands. Before adding more features:
1. What do we actually have? (inventory)
2. Does it work? (quality)
3. Is it used? (value)
4. What's dead weight? (cleanup)

---

## Current State Audit

### Test Coverage

| Metric | Value |
|--------|-------|
| Test files | 56 |
| Test functions | 172 |
| Tests passing | 207 |
| Tests failing | 0 |
| Tests ignored | 4 |

**Gap:** No coverage % tracking. Unknown which modules lack tests.

### Retrieval Quality

| Metric | Baseline | Before Fix | After Fix | Notes |
|--------|----------|------------|-----------|-------|
| MRR | 0.624 | 0.448 | **0.588** | ✅ Restored (exceeds target 0.55) |
| Recall@5 | - | 20.8% | **31.2%** | ✅ Improved +50% |
| Recall@10 | 67.5% | 24.0% | **40.6%** | ⚠️ Still below baseline |
| Latency | ~135ms | 177ms | **168ms** | ✅ Improved |

**Root cause (fixed 2025-12-27):** Stale database entries from deleted commands + outdated ground truth paths.

**Gap:** No CI tracking. Need to add quality gate to prevent future regressions.

### Measurement Tools (Already Built)

| Tool | Purpose | Status |
|------|---------|--------|
| `patina eval` | Retrieval quality by dimension | ✅ Working |
| `patina eval --feedback` | Real-world precision from sessions | ✅ Working |
| `patina bench retrieval` | Ground truth benchmark with MRR/Recall | ✅ Working |
| `eval/retrieval-queryset.json` | 8 ground truth queries | ✅ Exists |

We have the infrastructure. We need to use it.

### Command Inventory

**Core Pipeline:**
- `scrape` → `oxidize` → `scry` → `serve`

**Measurement:**
- `eval` - retrieval quality evaluation
- `bench` - ground truth benchmarking

**Supporting:**
- `init`, `doctor`, `rebuild`, `model`, `repo`, `adapter`, `secrets`, `persona`, `assay`, `upgrade`, `version`, `build`, `test`

**Niche:**
- `yolo` - devcontainer generation

**Archived (removed 2025-12-27):**
- `query` (140 lines) - superseded by `scry`
- `ask` (350 lines) - superseded by `scry`
- `embeddings` (160 lines) - superseded by `oxidize`
- `belief` (165 lines) - experimental, unused

**Total archived:** ~815 lines

---

## Quality Gates

### Gate 1: Tests Pass

```bash
cargo test --workspace
```

**CI:** Already enforced.

### Gate 2: No Clippy Warnings

```bash
cargo clippy --workspace -- -D warnings
```

**CI:** Already enforced.

### Gate 3: Retrieval Quality

```bash
patina bench retrieval --query-set eval/retrieval-queryset.json
# MRR >= 0.55 (within 10% of target)
```

**CI:** Not yet enforced. Need to add.

### Gate 4: Format Check

```bash
cargo fmt --all -- --check
```

**CI:** Already enforced.

---

## Cleanup Plan

### Phase 1: Investigate Retrieval Regression ✅ COMPLETED

Root causes identified and fixed (2025-12-27):

**Cause 1: Stale database entries**
- 154 entries from deleted commands (query, ask, embeddings, belief) polluting index
- Queries returning deleted files as top results

**Cause 2: Outdated ground truth paths**
- Scrape refactored from flat files to modules (git.rs → git/mod.rs, sessions.rs → sessions/mod.rs)
- Ground truth paths in queryset didn't match actual file structure

**Actions taken**:
- ✅ Rebuilt database: `patina scrape code --force` (removed stale entries)
- ✅ Rebuilt indices: `patina oxidize` (regenerated all projections)
- ✅ Updated queryset: Fixed ground truth paths (commit 98057220)

**Results**:
- MRR: 0.427 → **0.588** (+37.7%, exceeds target of 0.55)
- Recall@5: 20.8% → 31.2% (+50.0%)
- Recall@10: 24.0% → 40.6% (+69.2%)
- Latency: 177ms → 168ms (-5.1%)

### Phase 2: Archive Legacy Commands ✅ COMPLETED

Removed from CLI, code preserved via git tag `archive/legacy-commands-20251227`:

Commands removed (2025-12-27):
- ✅ `query` (superseded by `scry`)
- ✅ `ask` (superseded by `scry`)
- ✅ `embeddings` (superseded by `oxidize`)
- ✅ `belief` (experimental, unused)

Kept as lightweight wrappers:
- `build` - docker/dagger build wrapper (32 lines)
- `test` - test runner wrapper (31 lines)

### Phase 3: Add CI Quality Gate

```yaml
# .github/workflows/ci.yml
- name: Retrieval Quality
  run: |
    patina bench retrieval --query-set eval/retrieval-queryset.json --json
```

---

## Exit Criteria

| Criteria | Status |
|----------|--------|
| Retrieval regression investigated | [x] |
| MRR restored to >= 0.55 | [x] |
| Legacy commands archived | [x] |
| CI gate for retrieval quality | [ ] |
| README command list accurate | [x] |

---

## Next Session Start

1. Add CI quality gate (Phase 3)
2. Consider improving Recall@10 (currently 40.6%, baseline was 67.5%)
3. Investigate q4-assay query (still at RR=0.00)

---

## References

- Andrew Ng: "Don't add features without data"
- [spec-lab-automation.md](./spec-lab-automation.md) - Extended benchmarking vision
- [spec-work-deferred.md](./spec-work-deferred.md) - Parking lot for future ideas
