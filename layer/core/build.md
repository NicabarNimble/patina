# Build Recipe

**Version:** 0.15.3 — Three pillars: epistemic (complete), mother, distribution.

---

## What Patina IS

A local-first RAG network: portable project knowledge + personal mother.

- **Patina Projects:** `patina init .` - full RAG (semantic, temporal, dependency)
- **Reference Repos:** `patina repo add <url>` - lightweight index in `~/.patina/cache/repos/`
- **Mother:** `~/.patina/` - registry, personas, `patina serve` daemon

**Completed infrastructure:** Scrape pipeline, oxidize embeddings, query/scry, serve daemon, persona, rebuild command, MCP server, hybrid retrieval (MRR 0.624), model management, feedback loop, assay structural queries, spec work-item system (ready queue, auto-release). All working.

---

## The Architecture

**Spec:** [reference/spec-pipeline.md](../surface/build/reference/spec-pipeline.md)

```
                            GIT (source of truth)
                                    │
                   ┌────────────────┼────────────────┐
                   ▼                ▼                ▼
             scrape git      scrape code      scrape forge
           (commits+parsed)   (symbols)      (issues, PRs)
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
| scrape forge | Extract | Capture issues, PRs from GitHub/Gitea |
| oxidize | Prepare (semantic) | Build embeddings from facts |
| assay | Query (factual) | Structural signals, FTS5 search, temporal, belief grounding |
| scry | Query (semantic) | Multi-domain vector similarity — meaning, not keywords |

**Next:** [[mother-architecture]] — children as plugins. Mother is the plugin host (daemon lifecycle, child registry, heartbeat, request routing, toy management). Children are `MotherChild` trait implementors: [[mother-environment]] (models), [[mother-repos]] (ref repos). Native Rust traits now, WIT plugins later via [[patina-platform]].

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
| **Epistemic** | **COMPLETE** (v0.10.0) — 87 beliefs, verification, grounding, forge | E5/E6 deferred to mother scope |
| **Mother** | Registry + serve daemon | Federated query, persona fusion |
| **Distribution** | 52MB fat binary | Slim binary, `patina setup`, Homebrew |

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

**Archived specs:** `git tag -l 'spec/*'` (53+ archived specs, viewable via `git show spec/<name>:path`)
