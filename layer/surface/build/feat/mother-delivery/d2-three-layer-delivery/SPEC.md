---
type: feat
id: d2-three-layer-delivery
status: design
created: 2026-02-02
updated: 2026-02-03
sessions:
  origin: 20260202-202802
  design-resolution: 20260203-065424
related:
  - layer/surface/build/feat/mother-delivery/SPEC.md
  - layer/surface/build/feat/mother-delivery/d1-belief-oracle/SPEC.md
  - layer/surface/build/feat/mother-delivery/design.md
---

# feat: D2 — Three-Layer Delivery (Description → Response → Graph Breadcrumbs)

> Deliver knowledge at multiple touch points so the LLM encounters it regardless of which tool it calls first.

## Problem

`context` reads static markdown files. `scry` returns full content with belief annotations as ignorable metadata. No recall instruction anywhere. The LLM has no nudge to use beliefs, no reinforcement when it does, and no actionable links to follow the knowledge graph.

---

## Design

Three delivery layers, consistent across both MCP and CLI. Belt-and-suspenders — knowledge delivered at multiple touch points so the LLM encounters it regardless of which tool it calls first.

OpenClaw puts recall instructions in both tool description AND system prompt. Patina adapts this: both tool/command description AND tool/command response carry delivery, but without adapter-specific system prompts.

### Layer 1: Tool/Command Description (the nudge)

The description text is the first thing the LLM sees — before it even calls the tool.

**MCP tool descriptions:**
```
scry: "Search codebase knowledge — USE THIS FIRST for any question about the code.
       Fast hybrid search over indexed symbols, functions, types, git history,
       and session learnings."

context: "Get project patterns and conventions — USE THIS to understand design rules
          before making architectural changes. Returns core patterns (eternal
          principles) and surface patterns (active architecture)."
```

**CLI `--help` text (consumed by LLM adapters calling via Bash/exec):**
Same content in `patina scry --help` and `patina context --help`. Claude Code reads `--help` output; OpenCode reads it via exec. The help text IS the tool description for CLI consumers.

### Layer 2: Response-Level Recall (in the output)

**In `context` response (MCP and CLI):**
```
## Core Patterns
[existing — layer/core/ principles]

## Active Beliefs (top N by relevance to topic)
  B-12: "Error handling should use thiserror derive macros" (entrenchment: 0.8)
  B-07: "Prefer explicit Result<T,E> over panics" (entrenchment: 0.9)
  B-23: "MCP tools should be adapter-agnostic" (entrenchment: 0.7)

## Cross-Project Beliefs (from Mother graph)
[beliefs from related projects via graph traversal]

## Recall Directive
Before answering questions about project conventions, design decisions, or
architectural patterns: search for relevant beliefs.
  CLI:  patina scry --belief <id>
  MCP:  scry(content_type="beliefs")
Project knowledge accumulates in beliefs — check them before assuming defaults.
```

The recall directive appears in the **tool/command response**, not in any adapter file. Every LLM that calls `context` (via MCP or CLI) sees it. Includes both CLI and MCP syntax so the LLM uses whichever interface it's connected through.

**In `scry` response (when beliefs are present):**
```
--- Belief Impact ---
3 beliefs matched — dig deeper:
  CLI:  patina scry --belief <id> --content-type code
  MCP:  scry(belief="<id>", content_type="code")
```

Lightweight hint, not a full directive. Only appears when belief results are present.

### Layer 3: Graph Breadcrumbs (link tracing in every result)

Every result is a node in the knowledge graph. The output shows its edges — what it links to, why, and how to follow. The LLM sees breadcrumbs and can self-direct exploration.

**Belief results include graph edges:**
```
2. [belief] error-handling-thiserror                       (0.83)
            "Use thiserror derive macros" (entrenchment: 0.8)
            Links:
              → attacks: panic-for-prototyping (defeated)
              → supports: explicit-result-types
              → reaches: src/retrieval/engine.rs, src/mcp/server.rs (+4 files)
            Dig deeper:
              patina scry --belief error-handling-thiserror --content-type code
```

**Code results include belief impact + structural edges:**
```
1. [code]   src/retrieval/engine.rs::query_with_options    (0.87)
            Belief impact:
              ← error-handling-thiserror (reach: 0.9)
              ← onnx-runtime-for-ml (reach: 0.7)
            Graph:
              → imports: src/retrieval/fusion.rs, src/retrieval/oracle.rs
              → co-changes: src/mcp/server.rs (28 times)
            Dig deeper:
              patina scry why src/retrieval/engine.rs::query_with_options
              patina assay --query-type callers --pattern query_with_options
```

The "Dig deeper" commands are CLI syntax (usable by Claude Code via Bash, OpenCode via exec). For MCP responses, the same section uses MCP tool syntax:
```
            Dig deeper:
              scry(mode="why", doc_id="src/retrieval/engine.rs::query_with_options")
              assay(query_type="callers", pattern="query_with_options")
```

The interface adapts to the delivery channel; the content is the same.

### Implementation

- Extend `get_project_context()` in `server.rs` to query beliefs by topic relevance
- Add cross-project belief aggregation via graph traversal
- Append recall directive to every context response (MCP and CLI)
- Add graph breadcrumbs to result formatting (belief links, belief impact, structural edges, dig-deeper commands)
- Detect delivery channel (MCP vs CLI) to format dig-deeper commands appropriately
- Rank beliefs by cosine similarity to topic when topic parameter is provided

---

## Exit Criteria

- [ ] Three-layer delivery — tool/command descriptions, response-level recall, and graph breadcrumbs all present
- [ ] Context returns dynamic beliefs — `context(topic="error handling")` returns relevant beliefs ranked by topic similarity
- [ ] Recall directive in context response — every context response includes recall instruction with both CLI and MCP syntax
- [ ] Graph breadcrumbs — belief results show links (attacks/supports/reaches), code results show belief impact + structural edges
- [ ] Dig-deeper commands — every result includes actionable commands to follow the graph, formatted for the delivery channel

---

## See Also

- [[design.md]] — ADR-3 (Why three-layer delivery)
- [[d1-belief-oracle/SPEC.md]] — Prerequisite: beliefs must be a search channel first
- [[d3-two-step-retrieval/SPEC.md]] — D3 must stabilize response shape before D2 references it in recall directives
- [[../SPEC.md]] — Parent spec (implementation order, non-goals)
