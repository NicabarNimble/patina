---
id: spec-skills-universal
status: design
created: 2026-01-19
tags: [spec, skills, adapter, architecture]
references: [unix-philosophy, dependable-rust, adapter-pattern]
---

# Spec: Skills as Universal Connector

**Insight:** All three LLM CLIs (Claude Code, Gemini CLI, OpenCode) use identical SKILL.md format. Skills are the universal interface to Patina.

**Strategy:** Lean heavy on Patina as universal value. Use skills as the thin connector layer.

**Official spec:** https://agentskills.io/specification

---

## Language Constraint

**Rust-first, TypeScript acceptable, no Python.**

| Component | Language | Rationale |
|-----------|----------|-----------|
| MCP tools | Rust | Core value, already implemented |
| Session scripts | Bash | Portable, already working |
| Skill instructions | Markdown | LLM reads, no runtime |
| Complex scripts | TypeScript | If needed, Bun available |

Most Patina skills are **instruction-only**. They guide the LLM to use MCP tools (scry, context, assay) which are Rust. Scripts are minimal glue.

---

## Official Spec Summary (agentskills.io)

### Required Structure
```
skill-name/
└── SKILL.md          # Required
```

### SKILL.md Format
```yaml
---
name: skill-name           # Required: 1-64 chars, lowercase, hyphens ok
description: |             # Required: 1-1024 chars, when to use
  What this skill does and when Claude should use it.
  Include keywords for activation triggers.
license: Apache-2.0        # Optional
compatibility: Requires git  # Optional: environment requirements
metadata:                  # Optional: arbitrary key-value
  author: patina
  version: "1.0"
allowed-tools: Bash(git:*) Read  # Optional/Experimental
---

# Instructions

Markdown body with procedural guidance.
```

### Optional Directories
```
skill-name/
├── SKILL.md              # Required
├── scripts/              # Executable code (Bash/TypeScript for Patina)
├── references/           # Documentation loaded on demand
└── assets/               # Templates, files for output
```

### Progressive Disclosure (3 levels)
1. **Metadata** (~100 tokens) - `name` + `description`, always in context
2. **Instructions** (<5000 tokens) - SKILL.md body, loaded when skill triggers
3. **Resources** (unlimited) - scripts/references/assets, loaded on demand

### Key Design Principles (from skill-creator)

1. **Concise is key** - Context window is shared. Only add what Claude doesn't know.
2. **Degrees of freedom** - Match specificity to task fragility:
   - High freedom: text instructions (multiple approaches valid)
   - Medium: pseudocode/parameterized scripts
   - Low: specific scripts (fragile operations)
3. **No extraneous files** - No README.md, CHANGELOG.md, etc. Skills are for agents.

---

## Why Skills

Skills solve the adapter problem elegantly:

| Problem | Old Approach | Skills Approach |
|---------|--------------|-----------------|
| Different CLI conventions | Adapter-specific code paths | Same SKILL.md everywhere |
| Keeping adapters in sync | Update 3 implementations | Update 1 skill, deploy to 3 locations |
| User training | Learn per-CLI commands | Skills auto-discovered by description |
| Testing | Can't test against real CLIs | Skills are just files, easy to verify |

**Core insight:** The LLM reads SKILL.md and decides when to use it. We write good descriptions, the CLI does the rest.

---

## Verified: Identical Format Across CLIs

From source code analysis (ref repos indexed in Patina):

### Claude Code
```
.claude/skills/*/SKILL.md
~/.claude/skills/*/SKILL.md
```

### Gemini CLI
```typescript
// packages/core/src/config/storage.ts
static getUserSkillsDir(): string {
  return path.join(Storage.getGlobalGeminiDir(), 'skills');  // ~/.gemini/skills/
}
getProjectSkillsDir(): string {
  return path.join(this.getGeminiDir(), 'skills');  // .gemini/skills/
}
```

### OpenCode
```typescript
// packages/opencode/src/skill/skill.ts
const OPENCODE_SKILL_GLOB = new Bun.Glob("{skill,skills}/**/SKILL.md")
const CLAUDE_SKILL_GLOB = new Bun.Glob("skills/**/SKILL.md")

// Explicitly discovers .claude/skills/ as fallback
if (!Flag.OPENCODE_DISABLE_CLAUDE_CODE_SKILLS) {
  for (const dir of claudeDirs) {
    // scans .claude/skills/**/SKILL.md
  }
}
```

### SKILL.md Format (universal)
```yaml
---
name: skill-name
description: When to activate this skill. The LLM uses this to decide relevance.
---

# Instructions

Markdown body with procedural guidance for the model.
```

**All three parse YAML frontmatter with `name` + `description`, body is markdown.**

---

## Patina Skills Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                      PATINA VALUE                           │
│  MCP tools (scry/context/assay), layer/, session workflow   │
└─────────────────────────────────────────────────────────────┘
                            │
                            ▼
┌─────────────────────────────────────────────────────────────┐
│                    SKILLS (Universal)                        │
│  SKILL.md files that surface Patina value to LLM context    │
└─────────────────────────────────────────────────────────────┘
                            │
              ┌─────────────┼─────────────┐
              ▼             ▼             ▼
         .claude/       .gemini/      .opencode/
         skills/        skills/        skills/
```

Skills are the **only** adapter-specific deployment. Everything else is universal.

---

## Skill Categories

### 1. Knowledge Skills (MCP-backed)

Skills that guide the LLM to use Patina's MCP tools effectively.

**`patina-codebase`** - Primary skill for code questions
```yaml
---
name: patina-codebase
description: |
  Use this skill for any question about the codebase: finding code,
  understanding architecture, locating functions, searching history.
  Activates automatically for questions like "where is X", "how does Y work",
  "find the code that handles Z".
---

# Codebase Knowledge

This project uses Patina for codebase knowledge. Use these MCP tools:

## Tools Available

- **scry** - Search indexed knowledge. USE FIRST for any code question.
  - Semantic search: finds conceptually related code
  - Temporal search: finds recently changed code
  - Lexical search: finds exact matches

- **context** - Get project patterns and conventions.
  - Use BEFORE making architectural changes
  - Returns core patterns (eternal) and surface patterns (active)

- **assay** - Query code structure.
  - `inventory`: list modules, files, functions
  - `imports/importers`: dependency analysis
  - `callers/callees`: call graph

## When to Use What

| Question Type | Tool | Example |
|--------------|------|---------|
| "Where is X?" | scry | "where is error handling" |
| "How does Y work?" | scry + Read | "how does auth work" |
| "What calls Z?" | assay callers | "what calls validate()" |
| "Project conventions?" | context | "how should I structure this" |

Always try scry before manual file exploration. It searches pre-indexed knowledge.
```

### 2. Workflow Skills

Skills that guide the LLM through Patina workflows.

**`patina-session`** - Session management
```yaml
---
name: patina-session
description: |
  Use this skill for development session management: starting sessions,
  tracking progress, capturing insights, ending sessions.
  Activates for: "start a session", "let's begin work", "end session",
  "capture this insight", "what did we do".
---

# Session Workflow

Sessions track development work with Git integration.

## Commands

| Command | Purpose |
|---------|---------|
| `/session-start [name]` | Begin session, create Git tag |
| `/session-update` | Capture progress, Git activity |
| `/session-note [insight]` | Record important learning |
| `/session-end` | Archive session, classify work |

## Session State

- Active session: `.claude/context/active-session.md` (or .gemini/, .opencode/)
- Archives: `layer/sessions/`
- Git tags: `session-[timestamp]-start`, `session-[timestamp]-end`

## Best Practices

- Start session when beginning focused work
- Update periodically (especially after commits)
- Capture insights as they occur
- End session to preserve learnings
```

**`patina-beliefs`** - Epistemic belief capture
```yaml
---
name: patina-beliefs
description: |
  Use this skill for creating epistemic beliefs from session learnings.
  Activates when: synthesizing project decisions, user says "create a belief",
  "add belief", "capture this as a belief", distilling learnings.
  Beliefs capture decisions with evidence, confidence, and relationships.
---

# Epistemic Beliefs

Beliefs capture project decisions with evidence and confidence.

## When to Create Beliefs

- After discovering a pattern that worked well
- When making a decision that should persist
- When correcting a misconception
- After successful debugging reveals insight

## Belief Structure

Location: `layer/surface/epistemic/beliefs/[slug].md`

@./references/belief-example.md

## Creating a Belief

1. Identify the core claim (one sentence)
2. Gather evidence (commits, sessions, outcomes)
3. Assess confidence (high/medium/low)
4. Note relationships (supports/attacks other beliefs)

Run: `.claude/bin/patina/create-belief.sh "[claim]"`
```

### 3. Review Skills

Skills that help the LLM analyze and review.

**`patina-review`** - Session and history review
```yaml
---
name: patina-review
description: |
  Use this skill to review recent work: session history, Git activity,
  layer changes, belief evolution. Activates for: "what happened recently",
  "review last session", "summarize progress", "what did we learn".
---

# Review Recent Work

## Quick Commands

- Recent sessions: `ls -la layer/sessions/ | tail -10`
- Recent commits: `git log --oneline -20`
- Recent beliefs: `ls -la layer/surface/epistemic/beliefs/`
- Session detail: `cat layer/sessions/[session-id].md`

## Review Dimensions

1. **Git Activity** - commits, branches, tags
2. **Session Progress** - goals achieved, decisions made
3. **Beliefs Captured** - patterns formalized
4. **Layer Changes** - knowledge evolution

## Typical Review Flow

1. Check last session file for context
2. Review Git log since last session
3. Identify patterns worth capturing as beliefs
4. Note open items for next session
```

---

## Deployment Strategy

### Single Source, Multiple Targets

Skills live in `resources/skills/` as the single source of truth:

```
resources/skills/
├── patina-codebase/
│   ├── SKILL.md
│   └── references/
├── patina-session/
│   ├── SKILL.md
│   └── scripts/
├── patina-beliefs/
│   ├── SKILL.md
│   ├── scripts/
│   └── references/
└── patina-review/
    └── SKILL.md
```

### Adapter Deployment

On `patina adapter add/refresh`:

```rust
fn deploy_skills(adapter: &Adapter, project_path: &Path) -> Result<()> {
    let skills_source = resources_dir().join("skills");
    let skills_target = project_path
        .join(adapter.config_dir())  // .claude, .gemini, .opencode
        .join("skills");

    // Copy each skill directory
    for skill in fs::read_dir(&skills_source)? {
        let skill = skill?;
        let skill_name = skill.file_name();
        let target = skills_target.join(&skill_name);

        // Remove old version, copy fresh
        if target.exists() {
            fs::remove_dir_all(&target)?;
        }
        copy_dir_recursive(skill.path(), &target)?;
    }
    Ok(())
}
```

### OpenCode Optimization

Since OpenCode discovers `.claude/skills/` as fallback, we could:
- Only deploy to `.claude/skills/`
- OpenCode users get skills "for free"

But explicit deployment to `.opencode/skills/` is cleaner and doesn't require Claude adapter.

---

## Skill Naming Convention

**Pattern:** `patina-[domain]`

| Skill | Domain | Purpose |
|-------|--------|---------|
| `patina-codebase` | knowledge | MCP tool guidance |
| `patina-session` | workflow | Session management |
| `patina-beliefs` | workflow | Belief capture |
| `patina-review` | analysis | History review |

The `patina-` prefix:
- Clearly identifies Patina-owned skills
- Avoids collision with user skills
- Easy to identify for removal on `adapter remove`

---

## Discovery and Activation

### How CLIs Find Skills

All three CLIs scan skills directories and build a registry:
- Claude: On startup and `/skills reload`
- Gemini: On startup and `/skills list`
- OpenCode: On startup (lazy loaded)

### How LLMs Activate Skills

The LLM sees skill descriptions in context and decides when relevant:

```
User: "Where is the authentication code?"

LLM thinking: This is a codebase question. The patina-codebase skill
says to use scry for "where is X" questions.

LLM action: Uses scry MCP tool with query "authentication code"
```

**Key insight:** Good descriptions = automatic activation. No slash commands needed.

### Explicit Activation

Users can also explicitly invoke:
- Claude: `/skills activate patina-codebase`
- Gemini: `/skills enable patina-codebase`
- OpenCode: (auto-activated when relevant)

---

## Adapter Responsibilities (Minimal)

With skills as universal connector, adapters only handle:

| Responsibility | What It Means |
|---------------|---------------|
| **Skills location** | Copy to `.claude/`, `.gemini/`, or `.opencode/` |
| **MCP config** | Wire scry/context/assay (all support MCP) |
| **Commands format** | Markdown (Claude/OpenCode) vs TOML (Gemini) |
| **Bootstrap hint** | Suggest adding @import to CLAUDE.md/GEMINI.md |

Everything else is universal Patina.

---

## Migration from Current State

### Current Structure
```
resources/claude/
├── skills/epistemic-beliefs/  # Only in Claude adapter
├── commands/*.md
└── bin/*.sh
```

### New Structure
```
resources/
├── skills/                    # Universal skills
│   ├── patina-codebase/
│   ├── patina-session/
│   ├── patina-beliefs/
│   └── patina-review/
├── claude/                    # Claude-specific (commands, scripts)
│   ├── commands/patina/
│   └── bin/patina/
├── gemini/                    # Gemini-specific
│   ├── commands/patina/       # TOML format
│   └── bin/patina/
└── opencode/                  # OpenCode-specific
    ├── commands/patina/
    └── bin/patina/
```

### Migration Steps

1. Move `resources/claude/skills/epistemic-beliefs/` to `resources/skills/patina-beliefs/`
2. Create new skills: `patina-codebase`, `patina-session`, `patina-review`
3. Update `templates.rs` to deploy from universal `resources/skills/`
4. Update each adapter to copy skills to its config directory

---

## Testing Strategy

### Unit Tests

```rust
#[test]
fn test_skill_md_format() {
    // Verify all skills have valid SKILL.md
    for skill_dir in fs::read_dir("resources/skills")? {
        let skill_md = skill_dir.path().join("SKILL.md");
        assert!(skill_md.exists());

        let content = fs::read_to_string(&skill_md)?;
        // Verify frontmatter
        assert!(content.starts_with("---"));
        // Verify required fields
        assert!(content.contains("name:"));
        assert!(content.contains("description:"));
    }
}

#[test]
fn test_skills_deployed_to_all_adapters() {
    // After adapter add, verify skills exist in target
    for adapter in ["claude", "gemini", "opencode"] {
        let skills_dir = format!(".{}/skills", adapter);
        assert!(Path::new(&skills_dir).join("patina-codebase/SKILL.md").exists());
    }
}
```

### Integration Tests

```rust
#[test]
fn test_skill_discovery_claude() {
    // Simulate Claude's skill discovery
    let skills = glob(".claude/skills/*/SKILL.md")?;
    assert!(skills.any(|s| s.contains("patina-codebase")));
}

#[test]
fn test_skill_discovery_opencode_claude_fallback() {
    // Verify OpenCode discovers Claude skills
    // (OpenCode scans .claude/skills/ when .opencode/skills/ is empty)
}
```

---

## Checklist

### Phase 1: Skill Consolidation
- [ ] Create `resources/skills/` directory
- [ ] Move `epistemic-beliefs` → `patina-beliefs`
- [ ] Create `patina-codebase` skill (MCP guidance)
- [ ] Create `patina-session` skill (workflow)
- [ ] Create `patina-review` skill (analysis)

### Phase 2: Universal Deployment
- [ ] Update `templates.rs` with `deploy_skills()` function
- [ ] Deploy skills to `.claude/skills/` on Claude adapter
- [ ] Deploy skills to `.gemini/skills/` on Gemini adapter
- [ ] Deploy skills to `.opencode/skills/` on OpenCode adapter

### Phase 3: Simplify Adapters
- [ ] Remove skill-specific code from individual adapters
- [ ] Verify MCP config works across all three
- [ ] Update commands to reference skill-based workflows

### Phase 4: Documentation
- [ ] Update CLAUDE.md guidance to mention skills
- [ ] Add skill discovery instructions per CLI
- [ ] Document how to extend with custom skills

---

## Future Possibilities

### User-Defined Skills
Users could add project-specific skills in `layer/skills/`:
```
layer/skills/
└── my-project-conventions/
    └── SKILL.md
```

Patina could deploy these alongside core skills.

### Skill Composition
Skills could reference other skills:
```yaml
---
name: patina-full
description: Complete Patina workflow including codebase, sessions, and beliefs.
---

@patina-codebase
@patina-session
@patina-beliefs
```

### Cross-Project Skills
Skills stored in `~/.patina/skills/` could be available globally:
```
~/.patina/skills/
└── my-coding-style/
    └── SKILL.md
```

---

## References

### Official
- [Agent Skills Specification](https://agentskills.io/specification)
- [Anthropic Skills Repo](https://github.com/anthropics/skills) - Reference examples (indexed in Patina)

### CLI Implementations (indexed as ref repos)
- [Gemini CLI Skills Docs](https://geminicli.com/docs/cli/skills/)
- [OpenCode skill.ts](~/.patina/cache/repos/opencode/packages/opencode/src/skill/skill.ts) - Shows Claude fallback discovery
- [Gemini CLI skillLoader.ts](~/.patina/cache/repos/gemini-cli/packages/core/src/skills/skillLoader.ts) - Same SKILL.md format

### Related Specs
- `spec-adapter-non-destructive.md` - Broader adapter concerns (namespacing, bootstrap files)
