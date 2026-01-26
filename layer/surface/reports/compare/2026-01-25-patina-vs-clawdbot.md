# Patina vs Clawdbot: Deep Dive Comparison

**Generated:** 2026-01-25
**Session:** clawdbot facts vs fiction
**Repos:** patina (local), clawdbot/clawdbot (ref repo)

---

## Architecture Overview

| Aspect | Patina | Clawdbot |
|--------|--------|----------|
| **Language** | Rust (with shell scripts for adapters) | TypeScript (ESM) |
| **Core Pattern** | Layer system (core → surface → dust) | Skills + Agents system |
| **Config Files** | CLAUDE.md | AGENTS.md + CLAUDE.md (identical) |
| **Session Storage** | `layer/sessions/*.md` (markdown) | `~/.clawdbot/agents/<id>/sessions/*.jsonl` |
| **Skill Location** | `resources/claude/skills/` | `skills/` directory at repo root |
| **Runtime** | CLI tool (`patina`) | CLI + Gateway + Mac/iOS/Android apps |

---

## Session Management

### Patina
- Markdown files in `layer/sessions/`
- Git tags for session boundaries (`session-YYYYMMDD-HHMMSS-start/end`)
- Committed to repo (transparency is the feature)
- Shell scripts orchestrate session lifecycle
- Human-readable format with goals, activity log, beliefs

### Clawdbot
- JSONL files per session (machine-readable)
- Session index (`sessions.json`) maps keys to session IDs
- Stored in user home (`~/.clawdbot/`)
- Includes cost tracking, token usage, tool calls
- Supports cross-agent spawning

---

## Skill System

### Patina
- Skills defined as markdown files with prompts (`.md`)
- Invoked via `/skill-name` syntax
- Shell scripts for commands (`session-start.sh`, etc.)
- Adapters provide skill loading for different LLMs

### Clawdbot
- Skills defined as `SKILL.md` with YAML frontmatter:
  ```yaml
  ---
  name: github
  description: "Interact with GitHub..."
  metadata: {"clawdbot":{"emoji":"🧩","requires":{"bins":["gh"]}}}
  ---
  ```
- 52+ packaged skills (1password, apple-notes, github, coding-agent, etc.)
- Rich metadata (emojis, binary requirements, etc.)
- Skills can be workspace-local or bundled
- Skill discovery via `collectSkillBins()` and `loadWorkspaceSkillEntries()`

---

## Key Stats

| Metric | Patina | Clawdbot |
|--------|--------|----------|
| Commits | ~1,500 | 7,594 |
| Source files | 176 | 2,988 |
| Functions | 1,488 | 10,827 |
| Skills | ~5 | 52+ |
| Language extractors | 9 | N/A |
| Vector indices | 3 (semantic, temporal, dependency) | N/A |

---

## Patterns Patina Should Adopt

### 1. SKILL.md Format with YAML Frontmatter
- **Current:** Patina skills are plain markdown
- **Clawdbot:** Structured frontmatter with `name`, `description`, `metadata`
- **Benefit:** Machine-readable requirements (`requires.bins`), emojis, discovery
- **Status:** Already planned in `spec-skills-universal.md`

### 2. Multi-Agent Safety Guidelines
- Clawdbot has explicit "multi-agent safety" rules in AGENTS.md
- No stash creation, no branch switching, scope commits to agent's changes
- Patina could adopt these as beliefs or core patterns

### 3. Skill Requirements Declaration
- `requires: {bins: ["gh", "jq"]}` - explicit binary dependencies
- `requires: {anyBins: ["claude", "codex"]}` - alternative dependencies
- Patina's `doctor` could validate skill requirements

### 4. Workflow Files (`.agent/workflows/`)
- Clawdbot has reusable workflow documents for complex procedures
- `update_clawdbot.md` - step-by-step upstream sync
- Pattern: "Runbook as code" with bash snippets

### 5. Learnings Section in Skills
- Clawdbot skills have "Learnings (Jan 2026)" sections
- Captures hard-won knowledge within the skill itself
- Aligns with Patina's epistemic beliefs concept

---

## What Rust Does Better

### 1. Performance
- Vector search (USearch) is already Rust
- Embedding generation (ONNX) is already Rust
- No Node.js/V8 overhead for core operations

### 2. Single Binary Distribution
- `patina` is one binary with everything included
- Clawdbot requires Node.js, npm, pnpm ecosystem
- Patina: `cargo install patina` vs Clawdbot's multi-step install

### 3. Type Safety at Compile Time
- Rust catches errors before runtime
- Clawdbot relies on TypeScript + tests
- Example: UTF-8 boundary check we fixed in `dependency.rs`

### 4. Cross-Platform Consistency
- Same binary on Mac/Linux/Windows
- Clawdbot has platform-specific installers and apps
- Patina's ONNX embeddings produce identical vectors everywhere

### 5. Memory Safety
- No garbage collection pauses
- Safe concurrency with ownership model
- Important for long-running MCP server

---

## Facts vs Fiction

| Claim | Reality |
|-------|---------|
| "Clawdbot is more mature" | **True** - 7594 commits, 52+ skills, multi-platform apps |
| "Patina has better search" | **True** - Multi-projection semantic search (dependency, temporal, semantic) |
| "Clawdbot sessions are richer" | **Mixed** - JSONL has more metadata, but Patina's markdown is more portable |
| "Skills are equivalent" | **Fiction** - Clawdbot's frontmatter + metadata is more sophisticated |
| "Rust is faster" | **True** for core ops, but shell scripts add overhead for session management |

---

## Recommended Actions

1. **Adopt SKILL.md format** with YAML frontmatter (in progress via spec-skills-universal)
2. **Add skill requirements** to `patina doctor`
3. **Create workflow directory** (`layer/workflows/` or `.agent/workflows/`)
4. **Add "Learnings" to beliefs** or skills
5. **Consider JSON sessions** for machine analysis (but keep markdown for portability)

---

## Key Takeaway

**Clawdbot** is a feature-rich personal assistant with messaging channels (Telegram, Discord, Slack, etc.) and multi-platform apps.

**Patina** is a context orchestration tool focused on making AI assistants smarter about codebases through semantic search, pattern evolution, and epistemic beliefs.

They solve different problems but share session tracking and skill concepts. The SKILL.md format from agentskills.io is the universal bridge.

---

## References

- Clawdbot repo: `~/.patina/cache/repos/clawdbot/clawdbot`
- Agent Skills Spec: https://agentskills.io/specification
- Patina spec: `layer/surface/build/deferred/spec-skills-universal.md`
