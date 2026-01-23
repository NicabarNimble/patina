# Version History

> Retroactive version assignments using Phase.Milestone model.

**Policy:** See `versioning-policy.md`
**Audit:** See `git-history-audit.md`

---

## 0.1.x - Bootstrap Phase
**Era:** July 2025
**Theme:** Getting the project off the ground

| Version | Date | Milestone | Key Commit |
|---------|------|-----------|------------|
| 0.1.0 | 2025-07-16 | Initial commit | `1b31316e` |
| 0.1.1 | 2025-07-28 | brain→layer rename | `2fde68c6` |
| 0.1.2 | 2025-07-29 | CI pipeline working | `766660c2` |

**Phase summary:** Project foundation - CLI structure, Claude adapter scaffolding, layer concept, CI/CD.

---

## 0.2.x - Architecture Phase
**Era:** August 2025
**Theme:** Establishing patterns and module boundaries

| Version | Date | Milestone | Key Commit |
|---------|------|-----------|------------|
| 0.2.0 | 2025-08-09 | Dependable Rust pattern documented | `a9aaaebb` |
| 0.2.1 | 2025-08-10 | Black-box refactor complete | `22a5a800` |
| 0.2.2 | 2025-08-13 | Modular workspace (old removed) | `4244b1a8` |
| 0.2.3 | 2025-08-13 | Environment registry as Eternal Tool | `435367d2` |

**Phase summary:** Architecture solidified - dependable-rust pattern, black-box modules, clean boundaries.

---

## 0.3.x - Language Phase
**Era:** August - October 2025
**Theme:** Multi-language code extraction

| Version | Date | Milestone | Key Commit |
|---------|------|-----------|------------|
| 0.3.0 | 2025-08-27 | Scrape pipeline working | `35f927a6` |
| 0.3.1 | 2025-08-29 | Scrape reorganized (9 chapters) | `ee5cc9fb` |
| 0.3.2 | 2025-09-02 | C/C++ complete | `770be393` |
| 0.3.3 | 2025-09-10 | Go, Rust complete | `f3576fc1` |
| 0.3.4 | 2025-09-11 | Python, TypeScript complete | `802bc41f` |
| 0.3.5 | 2025-10-01 | 9/9 languages complete | `aeb5dab2` |

**Phase summary:** Full language extraction - Rust, Go, Python, TypeScript, JavaScript, C/C++, Solidity, Cairo.

---

## 0.4.x - Cleanup Phase
**Era:** October 2025
**Theme:** Removing legacy systems, simplifying

| Version | Date | Milestone | Key Commit |
|---------|------|-----------|------------|
| 0.4.0 | 2025-10-03 | Dagger fully removed | `0a55c4f3` |
| 0.4.1 | 2025-10-05 | DuckDB removed | `01ac2491` |
| 0.4.2 | 2025-10-21 | PROJECT_DESIGN.toml removed | `00a5643a` |
| 0.4.3 | 2025-11-03 | Database config system removed | `c7ff9306` |

**Phase summary:** Clean architecture - Dagger experiment concluded, pure Rust runtime, simplified dependencies.

---

## 0.5.x - Semantic Phase
**Era:** November 2025
**Theme:** Vector search and retrieval

| Version | Date | Milestone | Key Commit |
|---------|------|-----------|------------|
| 0.5.0 | 2025-11-25 | Oxidize pipeline (embeddings) | `9642ef3e` |
| 0.5.1 | 2025-11-25 | Scry MVP | `17c2b21e` |
| 0.5.2 | 2025-11-25 | FTS5 lexical search | `f9121b53` |
| 0.5.3 | 2025-11-25 | Temporal dimension | `4ff7e4ff` |
| 0.5.4 | 2025-11-25 | Evaluation framework | `fa9c9f25` |

**Phase summary:** Semantic search - oxidize pipeline, scry command, vector + lexical + temporal dimensions.

---

## 0.6.x - Server Phase
**Era:** November - December 2025
**Theme:** Production infrastructure and integrations

| Version | Date | Milestone | Key Commit |
|---------|------|-----------|------------|
| 0.6.0 | 2025-11-26 | Mothership MVP | `a6be3e65` |
| 0.6.1 | 2025-12-03 | Serve command (daemon) | `7885441d` |
| 0.6.2 | 2025-12-12 | MCP server + tools | `19a6af7c` |
| 0.6.3 | 2025-12-16 | MCP tools renamed (scry/context) | `a5c2f3fd` |
| 0.6.4 | 2025-12-22 | Secrets (age encryption) | `f3aea67a` |
| 0.6.5 | 2025-12-23 | Scry modes (orient/recent/why) | `ada02b8b` |

**Phase summary:** Server infrastructure - serve daemon, MCP integration, secrets, cross-project queries.

**Note:** Git tag `v0.1.0` was placed at 2025-12-16 during this phase.

---

## 0.7.x - Epistemic Phase
**Era:** January 2026
**Theme:** Beliefs, knowledge management, polish

| Version | Date | Milestone | Key Commit |
|---------|------|-----------|------------|
| 0.7.0 | 2026-01-06 | Mother graph learning | `60a62757` |
| 0.7.1 | 2026-01-10 | Commits in semantic index | `2ee571cb` |
| 0.7.2 | 2026-01-14 | Ref repo lean storage | `34a0a65e` |
| 0.7.3 | 2026-01-21 | dev_env removed (~435 lines) | `1cfaeff5` |
| 0.7.4 | 2026-01-22 | Mother rename | `aaf36b60` |
| 0.7.5 | 2026-01-22 | Beliefs in scry (E3 complete) | `9aeceff3` |

**Phase summary:** Epistemic layer - beliefs indexed, mother branding, continued cleanup.

---

## 0.8.x - Go Public Phase
**Era:** January 2026 (current)
**Theme:** Open source readiness

| Version | Date | Milestone | Key Commit |
|---------|------|-----------|------------|
| 0.8.0 | 2026-01-23 | Go-public spec created | `fd6348bf` |
| 0.8.1 | 2026-01-23 | Versioning policy + history | (this session) |
| 0.8.2 | - | Session transparency | (pending) |
| 0.8.3 | - | Contributor system | (pending) |
| 0.8.4 | - | Release automation fixed | (pending) |
| 0.8.5 | - | CONTRIBUTING.md + docs | (pending) |

**Phase summary:** Going public - versioning, contributor systems, documentation.

---

## Summary Table

| Phase | Versions | Era | Theme |
|-------|----------|-----|-------|
| 0.1.x | 3 | Jul 2025 | Bootstrap |
| 0.2.x | 4 | Aug 2025 | Architecture |
| 0.3.x | 6 | Aug-Oct 2025 | Language Support |
| 0.4.x | 4 | Oct-Nov 2025 | Cleanup |
| 0.5.x | 5 | Nov 2025 | Semantic Search |
| 0.6.x | 6 | Nov-Dec 2025 | Server/MCP |
| 0.7.x | 6 | Jan 2026 | Epistemic |
| 0.8.x | 2+ | Jan 2026 | Go Public |

---

## Current Version

**Patina is at v0.8.1**

This reflects:
- 8 major development phases
- Current phase focused on going public
- Milestone 1 of go-public phase (versioning established)

---

## Git Tag Reconciliation

| Git Tag | Phase Version | Notes |
|---------|---------------|-------|
| `v0.1.0` (Dec 16, 2025) | ~0.6.3 | First official release, mid-Server phase |

The existing `v0.1.0` tag remains for historical reference. Future tags will use the Phase.Milestone scheme.

---

## Next Milestones

| Version | Milestone |
|---------|-----------|
| 0.8.2 | Session transparency implemented |
| 0.8.3 | Contributor system working |
| 0.8.4 | Release automation fixed |
| 0.8.5 | Documentation complete |
| 0.9.0 | Public release (repo public) |
| 1.0.0 | Production (contributors active) |
