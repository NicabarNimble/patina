# Analysis: How Three Search Servers Emerged

> Historical analysis of the CLI / MCP / serve daemon bifurcation, traced through git history and session records. Context for [[d0-unified-search/SPEC.md]].

---

## The Three Paths Today

| Path | File | Pipeline | Result Type | Uses Oracles? |
|------|------|----------|-------------|---------------|
| CLI default | `src/commands/scry/mod.rs:execute()` | Heuristic → `scry_text()` OR `scry_lexical()`, persona bolted on after | `ScryResult` | **No** |
| MCP stdio | `src/mcp/server.rs` | `QueryEngine` → 4 oracles → RRF fusion | `FusedResult` | **Yes** |
| HTTP serve | `src/commands/serve/internal.rs:handle_scry()` | `if hybrid { QueryEngine } else { scry_text() }`, persona bolted on in non-hybrid path | `ScryResult` → `ScryResultJson` | **Only if hybrid** |

Each has its own:
- Display formatting (CLI inline, hybrid CLI in `hybrid.rs`, MCP in `format_results()`)
- Query logging (CLI `log_scry_query`, MCP `log_mcp_query`)
- Persona integration (CLI/serve bolt on post-query, MCP via PersonaOracle)
- Belief impact (CLI calls `find_belief_impact` inline, MCP wraps in `annotate_impact()`)

---

## Chronological Reconstruction

### Nov 25, 2025 — CLI scry born (`17c2b21e`)

The first scry was a simple vector search: embed query → search USearch → enrich from SQLite. It returned `ScryResult` (id, content, score, event_type, source_id, timestamp). No oracles, no fusion, just direct `scry_text()`. FTS5 lexical search was added the same day (`f9121b53`) as an alternative path with heuristic auto-detection (`is_lexical_query()`).

**Path 1 established. Direct search, returns `ScryResult`.**

### Dec 3, 2025 — Serve daemon born (`7885441d`)

Session [[20251203-160132]] records the design reasoning:
- **Purpose:** Ollama-style daemon for container workflows. Containers on remote machines query the Mac's knowledge.
- **Architecture choice:** HTTP REST over gRPC (simpler, curl-friendly). rouille (blocking) over tokio ("no async infection" — explicit architectural principle).
- Initially just `/health` and `/version` endpoints. No search yet.

### Dec 8, 2025 — Serve gets `/api/scry` (`f1b45a8b`)

The scry endpoint was added to the serve daemon. **It called `scry::scry_text()` directly** — the exact same function as CLI. It was a JSON wrapper around the CLI path. At this point serve was a thin HTTP transport layer. One search path, two transports. **No bifurcation yet.**

Session [[20251208-105433]] lists "Add include_persona to /api/scry" as an open item — persona integration was being added to CLI and needed parity in serve.

### Dec 12, 2025 — QueryEngine + MCP born together (`c597c15c`, `19a6af7c`)

The Oracle abstraction, QueryEngine with RRF fusion, and MCP server were all created on the **same day**. Session [[20251212-091705]] confirms: "Last session completed Phase 2: Built the agentic RAG system with parallel retrieval + RRF fusion. Created Oracle abstraction (`src/retrieval/`), MCP server (`src/mcp/`)."

The MCP server commit says: "Hand-rolled protocol (~150 lines, no external SDK). Blocking I/O. **patina_query tool with hybrid retrieval.**"

Session [[20251212-135526]] validates: "MCP Live Test: patina_query tool accessible from Claude Code. Queries return results from semantic, lexical, and persona oracles. RRF fusion working. Latency: ~196ms."

**Path 3 was born with QueryEngine. It never shared Path 1's code. It never used `scry_text()`.**

### Dec 16, 2025 — The bifurcation crystallizes (`49cf30c4`)

Session [[20251216-130440]] records the critical design decision:

> **"CLI vs MCP gap was intentional (direct control vs hybrid) - made discoverable via `--hybrid`"**

Rather than making QueryEngine the default CLI path, `--hybrid` was added as an **opt-in flag**. The reasoning: the oracle system was new and experimental — "needs feedback-driven tuning before becoming default" (D0 spec cites this from the original commit). MCP had it by default because MCP was built simultaneously with QueryEngine. CLI had 3 weeks of working direct-search history and the author didn't want to break it.

**On the same commit**, the serve daemon got its `hybrid` branch. The git diff shows `handle_scry()` being split into `if body.hybrid { QueryEngine } else { scry_text() }`. This is when the serve daemon went from "thin wrapper around CLI" to "duplicated bifurcation containing both paths."

### Dec 16, 2025 – Feb 2026 — Divergence accelerates

Every new feature after this point had to be implemented in multiple places:

| Feature | CLI | Serve | MCP |
|---------|-----|-------|-----|
| Persona | Bolted on post-query (`mod.rs:174`) | Bolted on in non-hybrid path (`internal.rs:306`) | Via PersonaOracle in QueryEngine |
| Feedback logging | `log_scry_query()` | None | `log_mcp_query()` (different impl) |
| Display formatting | Inline in `execute()` | N/A (JSON) | `format_results()` + `format_results_with_query_id()` |
| Belief impact | `find_belief_impact()` inline | None | `annotate_impact()` wrapper |
| Orient/Recent/Why modes | CLI subcommands | Not supported | MCP modes with own handlers |
| Graph routing | Full implementation in `routing.rs` | Not supported | Via `all_repos` in QueryEngine |

---

## Why It Happened

This was **temporal layering**, not a design decision:

1. **CLI direct search** was built first (Nov 25) and worked well for 3 weeks
2. **Serve daemon** was built as a transport wrapper around CLI (Dec 3-8) — initially no duplication
3. **QueryEngine/Oracles** and **MCP server** were born together (Dec 12) as a new system alongside the old one
4. **`--hybrid` was an intentional bridge** (Dec 16) — the author chose to let both systems coexist while the oracle system proved itself
5. **Features accumulated on both paths** (Dec 16 – Feb 2026) because neither was retired

The serve daemon is the most damaged artifact. It started as Path 1's transport wrapper, then got Path 3's QueryEngine bolted on as an `if/else` branch, creating a hybrid of both paths in one function. It also accumulated its own type system (`ScryRequest`, `ScryResponse`, `ScryResultJson`) that duplicates both `ScryResult` and `FusedResult`.

---

## Implications for D0

The D0 spec correctly identifies the CLI/MCP bifurcation but does not explicitly address the serve daemon as a third path. D0 needs to unify **all three**:

1. **CLI `execute()`** — replace heuristic routing with QueryEngine as default
2. **MCP `server.rs`** — already uses QueryEngine; needs to delegate formatting to shared CLI code
3. **Serve `handle_scry()`** — remove the `if hybrid` branch entirely, delegate to QueryEngine

### The `ScryResult` migration

`ScryResult` (6 fields, flat) and `FusedResult` (with contributions, annotations, metadata) are different structs. Converting between them happens in:
- `hybrid.rs:38-47` (FusedResult → ScryResult for logging)
- `internal.rs:268-276` (FusedResult → ScryResult in serve hybrid path)
- `server.rs:1058-1073` (FusedResult → ScryResult for belief impact)

D0 should **retire `ScryResult`** and make `FusedResult` the universal result type. All consumers (logging, feedback, display, belief impact) migrate to `FusedResult`.

### The serve daemon simplification

After D0, `handle_scry()` should:
1. Parse `ScryRequest` JSON
2. Call QueryEngine (the only path)
3. Serialize `FusedResult` → JSON response

No `ScryResult` intermediary. No `if hybrid` branch. No persona bolting.

---

## Evidence Sources

| Source | Date | Key Content |
|--------|------|-------------|
| `17c2b21e` | 2025-11-25 | First scry MVP (vector search) |
| `7885441d` | 2025-12-03 | Serve daemon creation (health/version only) |
| [[20251203-160132]] | 2025-12-03 | Session: Ollama-style design, rouille over tokio |
| `f1b45a8b` | 2025-12-08 | Serve gets `/api/scry` (thin wrapper around `scry_text`) |
| [[20251208-105433]] | 2025-12-08 | Session: Phase 4e mothership client, persona open items |
| `c597c15c` | 2025-12-12 | Oracle abstraction + QueryEngine + RRF |
| `19a6af7c` | 2025-12-12 | MCP server born with QueryEngine |
| [[20251212-091705]] | 2025-12-12 | Session: Phase 2 review, values alignment check |
| [[20251212-135526]] | 2025-12-12 | Session: MCP live test, 196ms latency validated |
| `49cf30c4` | 2025-12-16 | `--hybrid` flag added to CLI + serve daemon |
| [[20251216-130440]] | 2025-12-16 | Session: "CLI vs MCP gap was intentional" |

---

## See Also

- [[d0-unified-search/SPEC.md]] — The fix: one pipeline for all three paths
- [[SPEC.md]] — Parent spec (D0 implementation order)
- [[design.md]] — ADRs and ref repo evidence
