---
type: explore
id: agent-protocol
status: design
created: 2026-02-05
sessions:
  origin: 20260205-102402
related:
  - layer/surface/build/feat/patina-platform/SPEC.md
references:
  - "Zed ACP (Agent Client Protocol)"
  - "MCP (Model Context Protocol)"
---

# explore: Agent Protocol — Patina as Host

> What if Patina controlled the agents instead of being a tool they call?

## Current State

Patina is a **tool** that AI agents call via MCP:

```
Claude Code (host) ──MCP──► Patina (tool)
```

Our "adapters" just generate config files. No runtime control.

## Observation: Zed's ACP

Zed flips it — the **editor is the host**, agents are guests:

```
Zed (host) ──ACP──► Claude Code (agent)
```

Editor controls what agents can do. Agents ask permission.

## Speculative: Patina as Host

What if Patina hosted agents?

```
Patina (host) ──protocol──► Claude/Gemini/Local (agents)
```

Patina would control:
- File access permissions
- Command execution approval
- Context injection
- Session continuity across agents

## Why This Might Matter

- **Trust boundary moves to Patina** — not "trust the agent"
- **Unified workflow** — same approval flow regardless of agent
- **Agent swapping** — switch Claude↔Gemini mid-session
- **Local LLMs** — same interface for ollama, llama.cpp

## Why This Might Not Matter

- **Complexity** — agents already work fine as hosts
- **Duplication** — reinventing what Claude Code already does
- **Adoption** — who would use this over native tools?

## Questions to Answer Before Pursuing

1. What does Patina-as-host enable that tool-mode doesn't?
2. Is there demand for agent-agnostic workflows?
3. Can we bridge MCP/ACP or need our own protocol?
4. Is this just NIH syndrome?

## Parking This

This is interesting but speculative. Focus on:
1. WASM plugin system (concrete)
2. WIT interfaces (concrete)
3. Extract plugins (concrete)

Revisit agent-as-host if/when there's a real use case.

---

## Status Log

| Date | Status | Note |
|------|--------|------|
| 2026-02-05 | draft | Parked. Interesting idea from ACP exploration but too meta. Focus on concrete plugin work first. |
