# Design: Agents and Autonomous Workspaces

## Context

Two related concepts emerged from the spec review session that need exploration before any implementation decisions.

---

## Concept 1: Agents vs Adapters

From `spec/remove-codex` (archived 2026-01-13):

> **Adapters** = CLI tools patina launches in project context (Claude Code, Gemini CLI, OpenCode)
> **Agents** = tools that adapters can spawn for specific tasks

Codex was removed from adapters because it's conceptually an agent, not an adapter. But we never built an agent system.

### Current Adapter Model

```
User → patina adapter launch claude → Claude Code CLI
                                    → (has full project context)
                                    → (interactive session)
```

Adapters are **interactive CLI tools** that work in project context.

### Hypothetical Agent Model

```
Adapter → spawn agent → background task
                      → specific goal
                      → reports back to adapter
```

Agents would be **non-interactive tasks** spawned by adapters for specific work.

### Examples of Potential Agents

- Background indexer (continuous scrape/oxidize)
- Code reviewer (analyze PR, report issues)
- Test runner (run tests, summarize failures)
- Codex-style autonomous coder (work in isolated container)

---

## Concept 2: YOLO and Autonomous Workspaces

`patina yolo` generates devcontainers for autonomous AI development:

```
src/commands/yolo/
├── mod.rs          (main logic)
├── generator.rs    (devcontainer generation)
├── scanner.rs      (language/tool detection)
├── profile.rs      (workspace profiles)
└── features.rs     (devcontainer features)
```

**Total: 1,613 lines**

### What YOLO Does

1. Scans repo for languages (Rust, Python, Node, Go, etc.)
2. Detects tools and dependencies
3. Generates `.devcontainer/Dockerfile` and `devcontainer.json`
4. Configures for `--dangerously-skip-permissions` workflows
5. Supports profiles for different use cases

### The Question

Is devcontainer generation in scope for patina?

**Pro:**
- Enables the "autonomous AI development" vision
- Natural extension of AI-assisted development
- Already built and working

**Con:**
- Devcontainer generation isn't RAG/context orchestration
- Patina's core value is knowledge accumulation, not shipping
- Maintenance burden for tangential feature

---

## The Three-Layers Architecture

The `three-layers` spec proposes:

| Layer | Binary | Purpose |
|-------|--------|---------|
| Infrastructure | `mother` | Cross-project graph, federation |
| Product | `patina` | Context orchestration, knowledge |
| Shipping | `awaken` | Build, test, deploy, containers |

Under this model:
- `yolo` belongs in `awaken`
- `build` and `test` belong in `awaken` (being removed from patina)
- Agents might live in `awaken` or be their own thing

### Current Reality

- `mother` exists as `patina mother` subcommand
- `patina` is the main binary
- `awaken` doesn't exist
- Only `yolo` has substance in the "shipping" category

---

## Historical Sessions

| Session | Topic |
|---------|-------|
| 20260113-062119 | Codex removal, agent vs adapter distinction |
| 20260107-* | Early yolo development |
| 20260109-170426 | Autonomous workspace concepts |

---

## Decision Matrix

### Option A: Remove YOLO

- Delete 1,613 lines
- Users generate devcontainers manually or use other tools
- Patina focuses purely on knowledge/context

### Option B: Extract YOLO to Standalone Tool

- Create separate `yolo` or `awaken` binary
- Patina stays focused
- YOLO gets its own identity and can evolve independently

### Option C: Keep YOLO, Build Agent System

- YOLO becomes first "agent"
- Build agent infrastructure
- Adapters can spawn agents for tasks

### Option D: Keep YOLO As-Is, Freeze

- Don't remove, don't expand
- Accept it as tangential but useful
- No new development on it

---

## Recommendation

**Defer decision.** This is exploration, not urgent cleanup like `remove-dev-env`.

Questions to answer first:
1. Is anyone actually using `patina yolo`?
2. Does the three-layers architecture still make sense?
3. What's the minimum viable agent system?

---

## Next Steps (When Revisited)

1. Survey: Check if yolo is used in any ref repos or by users
2. Prototype: Sketch what an agent API would look like
3. Decide: Based on usage data and agent feasibility
