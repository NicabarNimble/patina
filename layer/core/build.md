# Build Recipe

**Version:** 0.45.1 — Three pillars: epistemic (complete), mother (architecture shipped), distribution (grammar plugins + SDK shipped).

---

## What Patina IS

A local-first RAG network: portable project knowledge + personal mother.

- **Patina Projects:** `patina init .` - full RAG (semantic, temporal, dependency)
- **Reference Repos:** `patina repo add <url>` - lightweight index in `~/.patina/cache/repos/`
- **Mother:** `~/.patina/` - registry, personas, `patina mother start` daemon

**Completed infrastructure:** Scrape pipeline, oxidize embeddings, query/scry, Mother daemon, persona, rebuild command, MCP server, hybrid retrieval (MRR 0.624), model management, feedback loop, assay structural queries, spec work-item system (ready queue, auto-release), WASM plugin system (WIT component model, 4 worlds: command/task/pipeline/mother-child), patina-sdk on crates.io, 9 grammar pipeline plugins, doctor/models/repos children, version consolidation, security hardening, workspace cleanup (10 root dirs). All working.

---

## The Architecture

**Spec:** [reference/spec-pipeline.md](../surface/build/reference/spec-pipeline.md)

```
                            GIT (source of truth)
                                    │
                   ┌────────────────┼────────────────┐
                   ▼                ▼                ▼
             scrape git      scrape code      github-connector
           (commits+parsed)   (symbols)      (issues, PRs via broker)
                   │                │                │
                   └────────────────┴────────────────┘
                                    │
                                    ▼
                               SQLite DB
                                    │
                   ┌────────────────┴────────────────┐
                   ▼                                 ▼
               oxidize                            assay
           (→ embeddings)                      (→ signals)
                   │                                 │
                   └────────────┬────────────────────┘
                                ▼
                              scry
                       (unified oracle)
                                │
                                ▼
                          LLM Frontend
```

**Core insight:** scry (semantic) and assay (factual) are the two query layers between LLM and codebase knowledge. Everything else prepares for that moment.

| Command | Role | "Do X" |
|---------|------|--------|
| scrape git | Extract | Capture commits, co-changes, parsed conventional commits |
| scrape code | Extract | Capture symbols, functions, types |
| github-connector | Extract | Capture issues, PRs from GitHub (via mother/broker) |
| oxidize | Prepare (semantic) | Build embeddings from facts |
| assay | Query (factual) | Structural signals, FTS5 search, temporal, belief grounding |
| scry | Query (semantic) | Multi-domain vector similarity — meaning, not keywords |

**Next:** Mother v2 — cross-project belief index ([[cross-project-beliefs]], Phase 2), belief truthfulness/staleness ([[belief-truthfulness]]), git tag-aware knowledge diff ([[git-tag-system]]). Plugin ecosystem complete: 4 worlds, SDK published, 9 grammar plugins, 3 mother children. Next: federated belief search, environment ownership.

**Values alignment:**
- [unix-philosophy](unix-philosophy.md): One tool, one job
- [dependable-rust](dependable-rust.md): Black box interfaces
- [adapter-pattern](adapter-pattern.md): Trait-based external system integration
- local-first: No cloud, rebuild from git
- git as memory: layer/ tracked, .patina/ derived

---

## v1.0 Roadmap

**Spec:** [feat/v1-release/SPEC.md](../surface/build/feat/v1-release/SPEC.md)

| Pillar | Current | Target |
|--------|---------|--------|
| **Epistemic** | **COMPLETE** (v0.10.0) — 128 beliefs, verification, grounding, forge | Truthfulness/staleness ([[belief-truthfulness]]), federated search |
| **Mother** | v0.16.0 daemon + v0.21.0 plugin ecosystem complete | Federated belief search ([[cross-project-beliefs]]), environment ownership |
| **Distribution** | v0.23.0 grammar plugins + v0.22.0 SDK on crates.io | Slim binary (dynamic ONNX), `patina setup`, Homebrew |

**Milestones:**
```
0.9.0  ✓ Public release (fat binary)
0.9.1  ✓ Version/spec system alignment
0.9.2  ✓ Session system & adapter parity
0.9.3  ✓ Fix: session 0.9.2 hardening
0.9.4  ✓ Fix: spec archive command, belief verification
0.10.0 ✓ Epistemic layer complete (E4-E4.6c)
0.11.0 ✓ Mother delivery D0-D5 (unified search, BeliefOracle, three-layer, two-step, naming)
0.11.1 ✓ Fix: canonical SpecFrontmatter + auto-release prototype
0.11.2 ✓ Fix: spec list filter (status IS NOT NULL)
0.12.0 ✓ Feat: unify scrape layer (patterns + sessions)
0.13.0 ✓ Feat: spec-as-work-item (ready queue, blocked view, auto-release)
0.14.0 ✓ Feat: mother delivery A/B eval PASS
0.14.1 ✓ Fix: belief retrieval quality (co-retrieval 50%, D1 PASS)
0.14.2 ✓ Fix: database identity (UIDs for federation)
0.15.0 ✓ Feat: ref repo semantic training (13/13 repos indexed)
0.15.2 ✓ Refactor: semantic-structural split (scry=meaning, assay=facts, P@10 48.3%)
0.15.3 ✓ Refactor: spec closure for semantic-structural-split
0.16.0 ✓ Feat: Mother Architecture (daemon, graph, children, microserver)
0.16.1 ✓ Refactor: Version Consolidation (ReleaseStrategy, BumpType, auto-release)
0.16.2 ✓ Refactor: Security Hardening
0.17.0 ✓ Feat: Plugin System (WASM component model, command world, doctor extraction)
0.18.0 ✓ Feat: Host HTTP Interface (patina:host/http)
0.19.0 ✓ Feat: Task World (patina:task)
0.20.0 ✓ Feat: Pipeline World (patina:pipeline)
0.21.0 ✓ Feat: Plugin Ecosystem Complete (4 worlds, trap handling, WIT enforcement)
0.21.1 ✓ Fix: Collapse Spec Complete + Archive into One Command
0.21.2 ✓ Feat: Plugin Template Polish
0.22.0 ✓ Feat: Patina SDK — Consolidated Plugin Crate on crates.io
0.23.0 ✓ Feat: Grammar Extraction — 9 Grammars as Pipeline Plugins
0.23.1 ✓ Fix: Workspace Cleanup — 26 Root Dirs → 10
0.24.0 ✓ Release milestone (cross-project graph routing era begins)
0.25.0 ✓ Release milestone (belief graph and import workflows expanded)
0.26.0 ✓ Release milestone (integration hardening)
0.27.0 ✓ Release milestone (command surface iteration)
0.28.0 ✓ Release milestone (runtime and tooling stabilization)
0.29.0 ✓ Release milestone (retrieval and interface refinements)
0.30.0 ✓ Release milestone (session and orchestration improvements)
0.31.0 ✓ Release milestone (rapid patch train across reliability and UX)
0.32.0 ✓ Release milestone (surface cleanup + follow-through)
0.33.0 ✓ Release milestone (protocol ergonomics)
0.34.0 ✓ Release milestone (architecture polish)
0.35.0 ✓ Release milestone (command and runtime hardening)
0.36.0 ✓ Release milestone (interface parity)
0.37.0 ✓ Release milestone (delivery stabilization)
0.38.0 ✓ Release milestone (quality and correctness iteration)
0.39.0 ✓ Release milestone (cleanup + regression-proofing)
0.40.0 ✓ Release milestone (operability upgrades)
0.41.0 ✓ Release milestone (routing and session improvements)
0.42.0 ✓ Release milestone (surface consistency)
0.43.0 ✓ Release milestone (high-velocity patch cycle)
0.44.0 ✓ Release milestone (pre-remediation cleanup)
0.45.0 ✓ Release milestone (audit remediation kickoff)
0.45.1 ✓ Fix: code-audit remediation gate progress and cleanup
1.0.0  - All pillars complete
```

---

## Measurement Tools

Built-in quality measurement infrastructure:

| Command | Purpose | Ground Truth |
|---------|---------|--------------|
| `patina eval` | Retrieval quality by dimension | - |
| `patina eval --feedback` | Real-world precision from sessions | Session data |
| `patina bench retrieval` | MRR, Recall@k benchmarking | `resources/bench/*.json` |
| `patina report` | Full state report using own tools | Tool quality = report quality |

**Baseline metrics:** MRR 0.624, Recall@10 67.5%, Latency ~135ms

Run regularly to catch regressions.

---

## Live Spec State

For current spec status, use the tools — they're always accurate:

```bash
patina spec ready              # What can I work on now?
patina spec blocked            # What's waiting and why?
patina spec list               # Full inventory
patina spec list --status X    # Filter by status (draft, ready, active, complete, design)
```

**Reference docs** (not phased work):
- [reference/spec-pipeline.md](../surface/build/reference/spec-pipeline.md) - Pipeline architecture
- [reference/spec-architectural-alignment.md](../surface/build/reference/spec-architectural-alignment.md) - Command/library alignment
- [reference/spec-assay.md](../surface/build/reference/spec-assay.md) - Structural queries + signals

**Archived specs:** `git tag -l 'spec/*'` (55+ archived specs, viewable via `git show spec/<name>:path`)
