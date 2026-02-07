---
type: feat
id: skills-focused-adapter
status: ready
created: 2026-01-19
updated: 2026-02-06
blocked_by: []
related:
  - layer/surface/build/explore/adapter-polish/SPEC.md
  - layer/surface/build/explore/llm-adapter-refactor/SPEC.md
  - layer/surface/build/explore/skill-derive/SPEC.md
  - layer/surface/build/refactor/skill-enforcement/SPEC.md
references: [unix-philosophy, dependable-rust, adapter-pattern]
---

# feat: Skills-Focused Adapter

> All three LLM CLIs use identical SKILL.md format. Skills are the universal connector to Patina.

**Official spec:** https://agentskills.io/specification

---

## Core Insight

Skills deliver Patina's value. Supporting infrastructure (commands, scripts, bootstrap) is minimal scaffolding. Each component has one job:

| Component | Do X | Doesn't Do |
|-----------|------|------------|
| **Skills** | Guide LLM to use Patina tools | Execute code, store state |
| **Commands** | Provide explicit `/slash` triggers | Auto-activate, make decisions |
| **Scripts** | Execute operations (Bash) | Guide LLM, store state |
| **MCP Tools** | Query knowledge (Rust) | Store state, manage files |

---

## Architecture

```
PATINA CORE (Rust) → MCP tools (scry/context/assay), layer/, knowledge.db
        │
        ▼
SKILLS (Universal SKILL.md) → Same format: Claude, Gemini, OpenCode
        │
        ▼
ADAPTER-SPECIFIC → .claude/, .gemini/, .opencode/ (commands, scripts)
```

---

## Key Design Decisions

1. **SkillsAdapter trait** — extends existing adapter pattern with skills-specific paths
2. **Namespace ownership** — `patina-*` prefix for skills, `patina/` subdir for commands/bin
3. **Bootstrap markers** — `<!-- PATINA:START -->...<!-- PATINA:END -->` for safe CLAUDE.md updates (already in use)
4. **Composable deploy** — separate deploy_skills/deploy_scripts/deploy_commands, composed into refresh/setup
5. **No version tracking** — namespace-based ownership replaces adapter-manifest.json

## Patina Skills

| Skill | Purpose |
|-------|---------|
| `patina-codebase` | MCP tool guidance (scry/context/assay) |
| `patina-session` | Session workflow (start/update/note/end) |
| `patina-beliefs` | Belief capture (when/how to create beliefs) |
| `patina-review` | History review (sessions, git, layer changes) |

---

## Phases

### Phase 1: Adapter Consolidation
- [ ] Move `Adapter` enum from `launch.rs` to `adapters/mod.rs`
- [ ] Add `SkillsAdapter` trait with config_dir/skills_dir/commands_dir/scripts_dir
- [ ] Implement for Adapter enum
- [ ] Add `is_patina_owned()` namespace detection
- [ ] Remove adapter-manifest.json version tracking

### Phase 2: Skills + Resources
- [ ] Create `resources/skills/patina-{codebase,session,beliefs,review}/SKILL.md`
- [ ] Move scripts to `resources/scripts/` (universal)
- [ ] Create adapter-specific command directories (claude/gemini/opencode)
- [ ] Update `templates.rs` embeds for new paths

### Phase 3: Deploy Infrastructure
- [ ] Implement deploy_skills/deploy_scripts/deploy_commands
- [ ] Implement composed refresh() and setup()
- [ ] Implement update_bootstrap() with marker section
- [ ] Remove old install_*_templates() functions

### Phase 4: Verify
- [ ] `patina adapter refresh claude` deploys new structure
- [ ] Skills discovered by Claude Code via `skills/*/SKILL.md` glob
- [ ] User skills preserved (no patina-* prefix = untouched)
- [ ] Bootstrap marker section idempotent

---

## Exit Criteria

- [ ] `SkillsAdapter` trait implemented, Adapter enum uses it
- [ ] 4 patina-* skills deployed and discovered by Claude Code
- [ ] Namespace ownership: refresh only touches patina-* items
- [ ] Bootstrap markers: safe update without overwriting user content
- [ ] Old manifest/version tracking removed

---

## References

- [Agent Skills Specification](https://agentskills.io/specification)
- layer/core/adapter-pattern.md — trait-based integration
- layer/core/unix-philosophy.md — one tool, one job
- Supersedes: spec-adapter-non-destructive, spec-skills-universal
