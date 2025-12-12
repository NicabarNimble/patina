# Concept: On-Device Orchestration Agent

**Status:** SUPERSEDED by [spec-agentic-rag.md](build/spec-agentic-rag.md)
**Created:** 2025-12-11
**Session:** 20251211-140201
**Superseded:** 2025-12-12 (Session 20251211-201645)

> **Note:** This concept proposed local LLM routing. After research review,
> we determined parallel retrieval + RRF fusion is simpler and more effective.
> See the new spec for the implemented approach.

---

## Executive Summary

An on-device orchestration agent that runs on Apple Silicon, receives queries from LLM frontends (Claude Code, Gemini CLI) via MCP, routes to appropriate oracles, and synthesizes contextual answers. The agent uses a small local LLM (Qwen3-0.6B or similar) for routing/synthesis while keeping all sensitive context on-device.

**Key Insight:** Patina becomes the intelligent middle layer, not just a tool provider.

---

## Pain Points This Solves

| Pain Point | Evidence | How Agent Solves |
|------------|----------|------------------|
| Stuck at semantic only | Session 20251125-065729: "1/6 dimensions implemented" | Agent coordinates multiple oracles |
| LLM orchestration burden | Session 20251026-072236: "Claude IS the intelligent agent" | Agent handles routing/combining |
| Context window limits | Can't send all 277 sessions to LLM | Agent retrieves TOP-K relevant |
| No smart retrieval | Session 20251029-084321: "Undefined how LLM queries beliefs" | Agent assembles context |
| Privacy concerns | Sessions contain sensitive project context | Sessions never leave device |
| Cost | Every routing decision = frontier tokens | Local decisions = 0 cloud tokens |
| Latency | Cloud roundtrip for simple decisions | Local inference ~60ms |
| Cross-project coordination | Mothership needs intelligence | Agent queries across nodes |

---

## Architecture

### The Two Models (Context)

**Model A: "Claude IS the Agent" (Current)**
```
Claude Code (Orchestrator)
    ↓ calls
Patina CLI / MCP Tools (Oracles)
    ↓ queries
Knowledge Graph + SQLite + Prolog
```
- LLM frontend does all orchestration
- Patina provides tools/oracles
- Simple, works now

**Model B: "On-Device Orchestration Agent" (Proposed)**
```
Claude Code / Gemini CLI
    ↓ calls via MCP
Patina Agent (Apple Silicon, ONNX Runtime)
    ↓ orchestrates
┌────────────┬────────────┬────────────┐
│ Semantic   │ Temporal   │ Session    │
│ Oracle     │ Oracle     │ Oracle     │
└────────────┴────────────┴────────────┘
    ↓ queries
Knowledge Graph + SQLite + Prolog
```
- Agent runs ON-DEVICE (Mac, Apple Silicon)
- LLM frontends call INTO the agent via MCP
- Agent decides WHAT to query and HOW to combine
- Small, fast model does routing/orchestration
- Frontier LLM stays focused on reasoning with user

---

### Full Architecture Diagram

```
┌─────────────────────────────────────────────────────────────────┐
│                    LLM Frontends                                │
│              (Claude Code, Gemini CLI)                          │
└─────────────────────────┬───────────────────────────────────────┘
                          │ MCP (JSON-RPC over stdio)
                          ▼
┌─────────────────────────────────────────────────────────────────┐
│              PATINA MCP SERVER (Rust)                           │
│                                                                 │
│  ┌──────────────────────────────────────────────────────────┐  │
│  │  Orchestration Agent                                      │  │
│  │  ┌────────────────────────────────────────────────────┐  │  │
│  │  │  Local LLM (ort crate, ONNX Runtime)               │  │  │
│  │  │  - Qwen3-0.6B or similar (ONNX format)             │  │  │
│  │  │  - Query routing, intent classification            │  │  │
│  │  │  - Result synthesis                                │  │  │
│  │  └────────────────────────────────────────────────────┘  │  │
│  └──────────────────────────────────────────────────────────┘  │
│                              │                                  │
│              ┌───────────────┼───────────────┐                  │
│              ▼               ▼               ▼                  │
│  ┌─────────────────┐ ┌─────────────────┐ ┌─────────────────┐   │
│  │ Semantic Oracle │ │ Temporal Oracle │ │ Session Oracle  │   │
│  │ (E5 + USearch)  │ │ (co_changes)    │ │ (obs + beliefs) │   │
│  └─────────────────┘ └─────────────────┘ └─────────────────┘   │
│                                                                 │
│  All Rust. No Python. ONNX Runtime for all ML.                 │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

---

## How Frontends Discover the Agent

MCP has built-in tool discovery. When a frontend connects, it asks "what tools do you have?" and the server responds with schemas.

### Tool Registration

```rust
ToolDefinition {
    name: "patina_query",
    description: "Query your codebase knowledge. Ask questions about \
        code, patterns, decisions, or how things work. The agent \
        routes to appropriate oracles and synthesizes answers.",
    input_schema: {
        "type": "object",
        "properties": {
            "query": {
                "type": "string",
                "description": "Natural language question about the codebase"
            },
            "intent": {
                "type": "string",
                "enum": ["search", "explain", "suggest"],
                "description": "Optional hint about query intent"
            }
        },
        "required": ["query"]
    }
}
```

### Frontend Configuration

**Claude Code** (`settings.json`):
```json
{
  "mcpServers": {
    "patina": {
      "command": "patina",
      "args": ["serve", "--mcp"]
    }
  }
}
```

### Discovery Flow

```
1. Frontend connects to Patina MCP Server
2. Frontend: "list_tools"
3. Patina returns: [patina_query, patina_context, patina_session_*, ...]
4. LLM reads descriptions, knows when to call each tool
5. User: "How does error handling work here?"
6. LLM reasons: "codebase question → patina_query"
7. LLM calls: patina_query({ query: "error handling" })
8. Agent routes, synthesizes, returns structured answer
9. LLM presents answer to user
```

---

## Integration with Scry & Scrape

### Scrape: Feeds the Oracles

```
patina scrape code     → Semantic Oracle (code.db, USearch, embeddings)
                       → Temporal Oracle (co_changes table)

patina scrape session  → Session Oracle (observations.db, beliefs)

patina scrape github   → GitHub Oracle (issues, PRs, bounties)
```

### Scry: Direct Oracle Access (CLI)

```bash
# Simple semantic search - NO agent needed
$ patina scry "error handling"

Results:
1. src/error.rs (0.92)
2. layer/core/patterns/error-context.md (0.87)
```

### Agent: Orchestrated Multi-Oracle (MCP)

```
Claude: patina_query("how is error handling done?")

Agent:
  → Classifies: "explain" intent
  → Routes to: semantic + session + temporal
  → Semantic: src/error.rs, patterns/error-context.md
  → Session: 3 sessions discussed error strategy
  → Temporal: error.rs changes with result.rs
  → Synthesizes combined answer
  → Returns with sources and confidence
```

### Command Interface

```bash
# INGESTION (unchanged)
patina scrape code
patina scrape session
patina scrape github

# QUERY
patina scry "query"           # Direct semantic (fast, CLI)
patina scry "query" --deep    # Agent-mediated (optional)

# SERVE (new)
patina serve                  # Start MCP server with agent
patina serve --mcp            # MCP-only mode (stdio)
patina serve --http :8080     # HTTP API mode
```

---

## Language Stack

**Constraint:** Rust, TypeScript, Swift. NO PYTHON.

### Why This Matters

MLX (Apple's ML framework) is Python-first. We need Rust-native solutions.

### Solution: ONNX Runtime via `ort` crate

We already use `ort` for E5 embeddings. Same runtime for local LLM.

```rust
// Current: embeddings
let embedding_model = ort::Session::from_file("e5-base-v2.onnx")?;

// New: local LLM (same runtime!)
let agent_model = ort::Session::from_file("qwen3-0.6b.onnx")?;
```

### Model Options (ONNX-compatible)

| Model | Size | Use Case |
|-------|------|----------|
| Qwen3-0.6B | 600MB | Query routing, classification |
| Phi-3-mini | 2.3GB | Synthesis, ranking |
| OLMo-1B | 1GB | General orchestration |

### Code Structure

```
src/
├── agent/
│   ├── mod.rs
│   ├── model.rs           # ONNX model loading (ort)
│   ├── router.rs          # Intent classification
│   ├── synthesizer.rs     # Result combination
│   └── oracles/
│       ├── semantic.rs    # USearch + E5
│       ├── temporal.rs    # co_changes queries
│       └── session.rs     # observations + beliefs
│
├── mcp/
│   ├── mod.rs
│   ├── server.rs          # JSON-RPC over stdio
│   ├── tools.rs           # Tool definitions
│   └── protocol.rs        # MCP types
│
└── commands/
    ├── serve.rs           # patina serve
    ├── scry.rs            # patina scry
    └── scrape/            # patina scrape
```

### When TS/Swift?

```
Rust:   CLI, Agent, MCP Server, Oracles (99%)
TS:     Future web dashboard, MCP testing utilities
Swift:  macOS menu bar app, Spotlight integration (future)
```

---

## Privacy Boundary

```
┌─────────────────────────────────────────┐
│  DEVICE BOUNDARY                        │
│                                         │
│  Sessions (sensitive)                   │
│  Beliefs (personal)                     │
│  Project context                        │
│              ↓                          │
│  Orchestration Agent                    │
│  Synthesizes → abstracts →              │
│  returns ANSWER not raw data            │
│              ↓                          │
│       Synthesized Answer                │
└─────────────────────────────────────────┘
              ↓
         To Cloud LLM
         Only sees: answer, not raw sessions
```

---

## Cost & Latency Benefits

### Token Savings

| Decision Type | Before (Cloud) | After (Local) |
|---------------|----------------|---------------|
| "Which oracle?" | ~500 tokens | 0 |
| "Rank results" | ~1000 tokens | 0 |
| "Combine answers" | ~800 tokens | 0 |
| Complex reasoning | Cloud | Cloud |

**Estimated savings:** 80%+ orchestration tokens stay local

### Latency

```
Before: Query → Cloud (200ms) → Response → Cloud (200ms) → ...
After:  Query → Local (20ms) → Oracles (10ms) → Synthesize (30ms)

~60ms local vs 400ms+ cloud roundtrips
```

---

## Implementation Path

### Phase 2a: Agent Foundation
- [ ] MCP server skeleton (Rust, stdio transport)
- [ ] Tool definitions (patina_query, patina_context)
- [ ] Basic routing (keyword-based, no ML yet)
- [ ] Integration with existing scry/semantic oracle

### Phase 2b: Local LLM Integration
- [ ] ONNX model loading via `ort`
- [ ] Tokenizer integration
- [ ] Query intent classification
- [ ] Result synthesis

### Phase 2c: Multi-Oracle Coordination
- [ ] Temporal oracle (co_changes)
- [ ] Session oracle (observations + beliefs)
- [ ] Result fusion and ranking
- [ ] Confidence scoring (Prolog integration)

### Phase 2d: Mothership Integration
- [ ] Cross-project queries
- [ ] Persona-aware responses
- [ ] Model management (`patina model pull`)

---

## Open Questions

### Architecture

1. **MCP transport:** stdio (simple) vs Unix socket (persistent)?
2. **Agent model serving:** In-process vs sidecar?
3. **Scry --deep:** Should CLI invoke agent, or agent-only via serve?

### Models

4. **Which small LLM?** Qwen3-0.6B vs Phi-3-mini vs OLMo-1B
5. **Model distribution:** Ship with patina? Download on first serve?
6. **Fine-tuning:** Train on session queries for better routing?

### Integration

7. **Scrape triggers refresh?** Agent needs to know about new data
8. **Fallback:** What if local model unavailable? Degrade to direct oracle?
9. **Caching:** Cache agent responses? Invalidation strategy?

---

## Key References

### Sessions
- 20251204-173633: "LLM-agnostic agentic RAG network" identity
- 20251120-110914: "LLM as Orchestrator, Patina as Oracle" + OLMo mention
- 20251116-223532: Mac Studio architecture, containers call Mac
- 20251026-072236: Three-layer architecture (observations → beliefs → agent)
- 20251029-084321: "Layer 3: Intelligent Agent" concept
- 20251125-065729: "Stuck at semantic only" pain point

### Existing Code
- `src/indexer/` - E5 embeddings via ort
- `src/commands/scry/` - Semantic search
- `src/commands/scrape/` - Knowledge ingestion
- `.patina/db/` - SQLite storage

### Core Principles
- `layer/core/unix-philosophy.md` - One tool, one job
- `layer/core/adapter-pattern.md` - Trait-based external integration
- `layer/core/dependable-rust.md` - Small stable interfaces

---

## Next Steps (Future Session)

1. Review and refine this concept
2. Decide on MCP transport (stdio vs socket)
3. Prototype MCP server skeleton
4. Evaluate small LLM options (benchmark on Mac)
5. Define tool schemas in detail
