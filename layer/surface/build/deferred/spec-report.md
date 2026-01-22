# Spec: Project Report

**Purpose:** Generate comprehensive state-of-repo reports using patina's own tools. Serves dual purpose: useful output + tool quality measurement.

**Status:** Draft

---

## Command Interface

```bash
# Project report (current directory)
patina report                     # Generate report for current project
patina report --output ./report.md  # Custom output location

# Ref repo report
patina report --repo gemini-cli   # Generate report for tracked ref repo
patina report --repo all          # Generate reports for all ref repos

# History
patina report history             # List past reports
patina report diff 2026-01-01    # Diff current vs past report
```

---

## Philosophy

**The report quality = tool quality.**

If `scry` can't answer "what are the main modules", that's a bug in scry, not in the report. This creates a feedback loop:

```
Generate Report → Review Quality → Improve Tools → Better Reports
```

Reports are stored in `layer/` so they become part of the searchable knowledge base.

---

## Report Sections

### 1. Summary (computed)

| Metric | Source | How |
|--------|--------|-----|
| Total lines | `assay` or `tokei` | Count source lines |
| Files | filesystem | Count by extension |
| Modules | `assay modules` | Parse mod.rs files |
| Change since last | diff with previous report | Compare metrics |

### 2. Architecture (via scry)

Ask patina's own RAG system about the codebase:

```rust
let queries = [
    "what are the main architectural components",
    "how is the codebase organized",
    "what are the core abstractions",
];
for q in queries {
    let results = scry(q, limit=5)?;
    report.add_scry_results(q, results);
}
```

**Why:** Tests if scry understands high-level structure.

### 3. Largest Modules (via assay)

```rust
let modules = assay_modules()?;
modules.sort_by(|a, b| b.lines.cmp(&a.lines));
report.add_table("Largest Modules", modules.take(10));
```

**Why:** Identifies candidates for refactoring.

### 4. Complexity Hotspots (via assay)

```rust
let files = assay_complexity()?;  // cyclomatic, nesting depth
report.add_table("Complex Files", files.filter(|f| f.complexity > threshold));
```

**Why:** Identifies maintenance burden.

### 5. Recent Churn (via git)

```rust
let churn = git_log_shortstat(days=30)?;
report.add_table("Most Changed (30d)", churn.take(10));
```

**Why:** Hot files often need attention.

### 6. Test Coverage (via assay or cargo)

```rust
let coverage = assay_test_coverage()?;  // or parse cargo tarpaulin
report.add_metric("Test Coverage", coverage.percentage);
report.add_table("Untested Modules", coverage.uncovered);
```

**Why:** Quality signal.

### 7. RAG Index Health

```rust
let db = open_knowledge_db()?;
report.add_metrics([
    ("Last scrape", db.last_scrape_time()),
    ("Total vectors", db.vector_count()),
    ("Indexed files", db.file_count()),
    ("Stale files", db.stale_file_count()),  // modified since scrape
]);
```

**Why:** Ensures the knowledge base is fresh.

### 8. Dependency Analysis

```rust
let deps = parse_cargo_toml()?;
report.add_table("Dependencies", deps);
report.add_metric("Direct deps", deps.len());
report.add_metric("Outdated", deps.filter(|d| d.outdated).count());
```

**Why:** Supply chain awareness.

### 9. Tool Performance (meta)

```rust
report.add_metrics([
    ("Scry avg latency", scry_timings.avg()),
    ("Scry empty results", scry_timings.empty_count()),
    ("Assay parse errors", assay_errors.count()),
]);
```

**Why:** Measures tool quality directly.

---

## Report Storage

### Project Reports

```
layer/surface/reports/
├── 2025-12-17-state.md
├── 2026-01-08-state.md
└── index.json           # metadata for quick listing
```

Reports in `layer/` are:
- Version controlled (git tracks changes)
- Searchable via scry (part of knowledge base)
- Human readable (markdown)

### Ref Repo Reports

```
~/.patina/mothership/reports/
├── gemini-cli/
│   ├── 2026-01-08-state.md
│   └── index.json
├── opencode/
│   └── ...
└── index.json           # global index
```

---

## Output Format

```markdown
# Project State Report: patina

**Generated:** 2026-01-08T07:30:00Z
**By:** patina report v0.1.0

## Summary

| Metric | Value | Change |
|--------|-------|--------|
| Lines of code | 42,318 | +1,204 |
| Source files | 156 | +3 |
| Modules | 48 | +2 |
| Test coverage | 62% | +4% |

## Architecture

> **Query:** "what are the main architectural components"

Based on scry results:
- **retrieval/** - Multi-oracle RAG engine with RRF fusion
- **embeddings/** - ONNX-based vector generation
- **commands/** - CLI entry points (~50% of codebase)
- **adapters/** - LLM-specific integrations

[Full scry output in appendix]

## Largest Modules

| Module | Lines | % of Total |
|--------|-------|------------|
| commands/scrape/code/languages | 7,223 | 17% |
| commands/scry | 1,358 | 3% |
| commands/init | 1,224 | 3% |

## Recent Churn (30 days)

| File | Changes | Commits |
|------|---------|---------|
| src/retrieval/engine.rs | +342 -89 | 12 |
| src/commands/bench/mod.rs | +201 -45 | 8 |

## RAG Index Health

| Metric | Value |
|--------|-------|
| Last scrape | 2026-01-07 20:21 |
| Total vectors | 12,432 |
| Indexed files | 148/156 (95%) |
| Stale files | 3 |

## Tool Performance

| Tool | Metric | Value |
|------|--------|-------|
| scry | Avg latency | 145ms |
| scry | Empty results | 1/8 queries |
| assay | Parse errors | 0 |

---

## Appendix: Raw Scry Results

[Full outputs for transparency/debugging]
```

---

## Implementation Plan

### Phase 1: Basic Report
- [ ] Add `patina report` command
- [ ] Implement summary metrics (lines, files, modules)
- [ ] Add scry integration for architecture section
- [ ] Save to `layer/surface/reports/`

### Phase 2: Full Metrics
- [ ] Add assay integration (modules, complexity)
- [ ] Add git churn analysis
- [ ] Add RAG health stats
- [ ] Add tool performance tracking

### Phase 3: Ref Repos
- [ ] Add `--repo` flag for ref repos
- [ ] Store in mothership reports folder
- [ ] Add `--repo all` for batch generation

### Phase 4: History & Diff
- [ ] Add `report history` subcommand
- [ ] Add `report diff` for comparisons
- [ ] Track metrics over time (JSON + markdown)

---

## Success Criteria

1. **Useful output:** Report answers "what's the state of this repo?"
2. **Tool validation:** Empty scry results indicate tool gaps
3. **Historical tracking:** Can see trends over time
4. **Self-hosting:** Patina can generate meaningful reports about itself

---

## Open Questions

1. Should reports trigger automatic scrape if index is stale?
2. How to handle ref repos without full scrape (lightweight mode)?
3. Include AI-generated summaries or keep purely data-driven?
4. JSON + Markdown, or Markdown only?
