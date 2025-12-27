# Spec: Quality Gates (Measurement-First)

**Status:** Active

**Philosophy:** Andrew Ng's practical ML approach applied to dev tools - measure before optimizing, clean before extending, ship with confidence.

**Scope note:** Through this quality gates work, we'll refine what Patina IS - an evolving layer of knowledge that covers projects and connects to a unifying system. Features like `yolo` and `hosts-deploy` may move to a future orchestration layer. Measurement first, then scope clarity.

---

## The Problem

Patina has grown to ~42k lines across 24 commands. Before adding more features:
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

| Metric | Baseline | Current | Notes |
|--------|----------|---------|-------|
| MRR | 0.624 | 0.448 | Regression detected |
| Recall@10 | 67.5% | 24.0% | Regression detected |
| Latency | ~135ms | ~161ms | Slight increase |

**Gap:** No CI tracking. Quality regressed without notice.

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
- `init`, `doctor`, `rebuild`, `model`, `repo`, `adapter`, `secrets`, `persona`, `assay`, `upgrade`, `version`

**Niche:**
- `yolo` - devcontainer generation

**Legacy (candidates for removal):**

| Command | Lines | Reason |
|---------|------:|--------|
| `query` | 140 | Superseded by `scry` |
| `ask` | 350 | Superseded by `scry` |
| `embeddings` | 160 | Superseded by `oxidize` |
| `belief` | 165 | Experimental, unused |
| `build` | 32 | Docker stub, rarely used |
| `test` | 31 | Docker stub, rarely used |

**Total legacy:** ~880 lines (candidates for archival)

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

### Phase 1: Investigate Retrieval Regression

Before archiving anything, understand why MRR dropped:
- [ ] Verify ground truth paths are still valid
- [ ] Compare current vs baseline scry behavior
- [ ] Identify root cause

### Phase 2: Archive Legacy Commands

Remove from CLI, preserve code via git tag:

```bash
git tag archive/legacy-commands
```

Commands to remove:
- `query` (use `scry`)
- `ask` (use `scry`)
- `embeddings` (use `oxidize`)
- `belief` (experimental)
- `build` (docker stub)
- `test` (docker stub)

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
| Retrieval regression investigated | [ ] |
| MRR restored to >= 0.55 | [ ] |
| Legacy commands archived | [ ] |
| CI gate for retrieval quality | [ ] |
| README command list accurate | [x] |

---

## Next Session Start

1. Run `patina bench retrieval --query-set eval/retrieval-queryset.json`
2. Check if ground truth paths in queryset are still valid
3. Pick up from Phase 1 of Cleanup Plan

---

## References

- Andrew Ng: "Don't add features without data"
- [spec-lab-automation.md](./spec-lab-automation.md) - Extended benchmarking vision
- [spec-work-deferred.md](./spec-work-deferred.md) - Parking lot for future ideas
