---
type: feat
id: report
status: design
created: 2026-01-13
updated: 2026-02-06
blocked_by:
  - eval-repair
related:
  - layer/surface/build/fix/eval-repair/SPEC.md
  - layer/surface/build/feat/doctor-dev/SPEC.md
  - layer/surface/build/explore/lab-automation/SPEC.md
---

# feat: Project Report

> Report quality = tool quality. If scry can't answer "what are the main modules," that's a scry bug.

**Blocked by:** eval-repair (report uses scry/assay output — need working quality measurement first)

---

## Philosophy

`patina report` generates a project state snapshot using patina's own tools. The report is a self-hosted eval: every section tests whether scry, assay, and the knowledge base actually work.

```
Generate Report → Review Quality → Improve Tools → Better Reports
```

Reports stored in `layer/` become part of the searchable knowledge base.

---

## Command Interface

```bash
patina report                        # Project state report
patina report --output ./report.md   # Custom output
patina report --repo gemini-cli      # Ref repo report
patina report history                # List past reports
patina report diff 2026-01-01       # Diff current vs past
```

---

## Report Sections

| Section | Source | Tests |
|---------|--------|-------|
| Summary (lines, files, modules) | assay, filesystem | Basic indexing |
| Architecture | scry queries | Does scry understand structure? |
| Largest Modules | assay modules | Structural analysis |
| Recent Churn (30d) | git log | Temporal awareness |
| RAG Index Health | knowledge DB | Freshness, coverage |
| Tool Performance | scry timings | Latency, empty results |

---

## Phases

### Phase 1: Basic Report
- [ ] Add `patina report` command
- [ ] Summary metrics (lines, files, modules via assay)
- [ ] Scry integration for architecture section
- [ ] Save to `layer/surface/reports/`

### Phase 2: Full Metrics
- [ ] Assay integration (modules, complexity hotspots)
- [ ] Git churn analysis (most changed files, 30d)
- [ ] RAG health stats (last scrape, vector count, stale files)
- [ ] Tool performance tracking (scry latency, empty results)

### Phase 3: Ref Repos
- [ ] `--repo` flag for ref repos
- [ ] Store in `~/.patina/mother/reports/`
- [ ] `--repo all` for batch generation

### Phase 4: History & Diff
- [ ] `report history` subcommand
- [ ] `report diff` for comparisons over time
- [ ] Track metrics in JSON + markdown

---

## Exit Criteria

- [ ] `patina report` produces useful markdown answering "what's the state of this repo?"
- [ ] Empty scry results in report indicate tool gaps (self-hosting test)
- [ ] Report stored in `layer/` and searchable via scry
- [ ] Historical tracking shows trends over time
- [ ] Patina generates a meaningful report about itself

---

## References

- build.md: `patina report` listed as completed infrastructure
- doctor-dev: health checks at session boundaries (complementary, not overlapping)
- lab-automation: retrieval benchmarks over time (different concern — quality vs state)
