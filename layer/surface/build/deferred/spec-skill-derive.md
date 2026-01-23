---
id: spec-skill-derive
status: design
created: 2026-01-20
tags: [skills, epistemic, adapters, generation]
references: [adapter-pattern, unix-philosophy, progressive-disclosure]
---

# Spec: Belief-Driven Skill Generation

**Problem:** Skills are manually written, coupled to specific adapters, and disconnected from Patina's epistemic layer.

**Solution:** Generate skills from beliefs - making skills projections of validated knowledge into adapter-specific formats.

**End goal:** `patina skill derive` generates cross-adapter skills from high-confidence beliefs, keeping Patina the source of truth.

---

## North Star

> Skills are not the intelligence. Skills are how Patina's intelligence manifests in LLM adapters.

The value lives in Patina (beliefs, patterns, knowledge). Skills are delivery mechanisms that inject that value into Claude Code, OpenCode, Gemini CLI, or any future adapter.

---

## Core Insight

What is a skill?

```
Skill = Structured Context + Tools + Trigger
            ↓               ↓        ↓
        SKILL.md         scripts/   name + description (router)
```

What is a belief?

```
Belief = Statement + Evidence + Confidence + Relationships
             ↓           ↓          ↓             ↓
         Knowledge    Sessions   Validated    Support/Attack
```

The connection: **A skill is a belief made executable.**

| Belief Property | Skill Property |
|-----------------|----------------|
| Statement | Instruction content |
| Evidence | Reference docs (provenance) |
| Confidence | Whether to generate at all |
| Facets | When to activate (routing) |
| Supports/Attacks | Dependencies / conflicts |

---

## Design Principles

1. **Patina First** - Beliefs are source of truth, skills are projections
2. **Confidence Threshold** - Only high-confidence beliefs earn skills
3. **Multi-Adapter** - One belief, multiple outputs (Claude, Codex, Gemini)
4. **Thin Wrappers** - Generated skills call Patina CLI, not duplicate logic
5. **Traceable** - Every skill links back to its source belief(s)

---

## Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                       EPISTEMIC LAYER                           │
│              layer/surface/epistemic/beliefs/*.md               │
│                                                                 │
│  ┌─────────────────┐  ┌─────────────────┐  ┌─────────────────┐ │
│  │  sync-first     │  │ progressive-    │  │ session-git-    │ │
│  │  (0.88)         │  │ disclosure      │  │ integration     │ │
│  │                 │  │ (0.82)          │  │ (0.85)          │ │
│  └────────┬────────┘  └────────┬────────┘  └────────┬────────┘ │
└───────────┼─────────────────────┼─────────────────────┼─────────┘
            │                     │                     │
            │         patina skill derive               │
            │                     │                     │
            ▼                     ▼                     ▼
┌─────────────────────────────────────────────────────────────────┐
│                    SKILL GENERATION ENGINE                       │
│                                                                 │
│  1. Filter beliefs by confidence (>= 0.80 default)              │
│  2. Check skill-derivable facet tag                             │
│  3. Load skill template for belief type                         │
│  4. Generate adapter-specific outputs                           │
└─────────────────────────────────────────────────────────────────┘
            │                     │                     │
            ▼                     ▼                     ▼
┌───────────────────┐ ┌───────────────────┐ ┌───────────────────┐
│   Claude Adapter  │ │   Codex Adapter   │ │   Gemini Adapter  │
├───────────────────┤ ├───────────────────┤ ├───────────────────┤
│ .claude/skills/   │ │ AGENTS.md         │ │ gemini-extension  │
│   {belief}/       │ │                   │ │   .json           │
│   SKILL.md        │ │ <skills>          │ │ contextFileName   │
│                   │ │ {belief}: desc    │ │                   │
│ marketplace.json  │ │ </skills>         │ │                   │
└───────────────────┘ └───────────────────┘ └───────────────────┘
```

---

## Belief Requirements for Derivation

Not every belief becomes a skill. Derivable beliefs need:

### Required

| Field | Requirement |
|-------|-------------|
| `confidence.score` | >= 0.80 (configurable threshold) |
| `status` | `active` (not defeated/archived) |
| `facets` | Contains `skill-derivable` tag |

### Recommended

| Field | Purpose |
|-------|---------|
| `applied-in` | Examples become skill instructions |
| `evidence` | Becomes reference documentation |
| `supports` | Defines skill dependencies |
| `attacks` | Defines skill conflicts |

### New Frontmatter (Optional)

```yaml
skill:
  name: session-management       # Override auto-generated name
  cli-command: patina session    # CLI command this wraps
  trigger-phrases:               # When to activate
    - "start a session"
    - "begin working"
    - "track my progress"
```

---

## Phase 1: Belief Annotation

Add skill-derivable annotation to existing beliefs.

### Checklist

- [ ] Add `skill-derivable` facet to applicable beliefs
- [ ] Add `skill:` frontmatter to beliefs with custom CLI mappings
- [ ] Document which beliefs map to which Patina CLI commands
- [ ] Create mapping table: belief → CLI command

### Candidate Beliefs

| Belief | CLI Mapping | Skill Purpose |
|--------|-------------|---------------|
| `session-git-integration` | `patina session *` | Session management |
| `progressive-disclosure` | `patina context *` | Context loading |
| `spec-first` | `patina spec *` | Spec management |
| `eventlog-is-truth` | `patina scry *` | Knowledge retrieval |

**CHECKPOINT: Review belief annotations before proceeding**

---

## Phase 2: Skill Template System

Create templates that transform beliefs into skills.

### Template Location

```
resources/skill-templates/
├── base.md.tmpl           # Common structure
├── cli-wrapper.md.tmpl    # For beliefs wrapping CLI commands
├── workflow.md.tmpl       # For multi-step workflows
└── guard.md.tmpl          # For constraint/validation beliefs
```

### Base Template Structure

```markdown
---
name: {{skill_name}}
description: {{belief_statement}} Use when {{trigger_conditions}}.
derived-from: {{belief_id}}
confidence: {{belief_confidence}}
generated: {{timestamp}}
---

# {{skill_title}}

{{belief_context}}

## When to Use

{{derived_from_facets_and_triggers}}

## Instructions

{{derived_from_applied_in}}

## Patina Integration

This skill wraps: `{{cli_command}}`

{{cli_usage_examples}}

## Evidence

{{derived_from_evidence_as_references}}
```

### Checklist

- [ ] Create `resources/skill-templates/` directory
- [ ] Design base template with placeholders
- [ ] Create cli-wrapper template
- [ ] Create workflow template
- [ ] Test template rendering with one belief

**CHECKPOINT: Review templates before building CLI**

---

## Phase 3: CLI Command

Implement `patina skill derive`.

### Command Signature

```bash
# Derive skill for single belief
patina skill derive session-git-integration

# Derive all derivable beliefs
patina skill derive --all

# Target specific adapter
patina skill derive --adapter claude session-git-integration
patina skill derive --adapter codex --all
patina skill derive --adapter gemini --all

# Custom confidence threshold
patina skill derive --min-confidence 0.75 --all

# Dry run (show what would be generated)
patina skill derive --dry-run --all

# Force regenerate (overwrite existing)
patina skill derive --force session-git-integration
```

### Output Locations

| Adapter | Output |
|---------|--------|
| Claude | `.claude/skills/{belief-id}/SKILL.md` |
| Codex | `agents/AGENTS.md` (appends/updates skill entry) |
| Gemini | `gemini-extension.json` (updates skills array) |

### Implementation Approach

```rust
// src/commands/skill/derive.rs

pub fn derive_skill(belief_id: &str, adapter: Adapter, force: bool) -> Result<()> {
    // 1. Load belief from epistemic layer
    let belief = load_belief(belief_id)?;

    // 2. Validate derivability
    if !belief.is_derivable() {
        return Err(anyhow!("Belief {} not marked as skill-derivable", belief_id));
    }

    // 3. Select template based on belief type
    let template = select_template(&belief)?;

    // 4. Render skill content
    let skill_content = render_template(template, &belief)?;

    // 5. Write to adapter-specific location
    write_skill(adapter, &belief, &skill_content, force)?;

    Ok(())
}
```

### Checklist

- [ ] Add `skill` subcommand to CLI
- [ ] Implement belief loading from epistemic layer
- [ ] Implement derivability check
- [ ] Implement template selection
- [ ] Implement template rendering
- [ ] Implement Claude adapter output
- [ ] Implement Codex adapter output
- [ ] Implement Gemini adapter output
- [ ] Add --dry-run flag
- [ ] Add --force flag
- [ ] Add --min-confidence flag

**CHECKPOINT: Test with one belief before full rollout**

---

## Phase 4: Adapter Index Generation

Generate adapter index files that list all derived skills.

### Claude: marketplace.json

```json
{
  "name": "patina-skills",
  "owner": { "name": "Patina" },
  "metadata": {
    "description": "Skills derived from Patina epistemic beliefs",
    "version": "{{version}}",
    "generated": "{{timestamp}}"
  },
  "plugins": [
    {
      "name": "{{skill_name}}",
      "source": "./skills/{{belief_id}}",
      "description": "{{belief_statement}}",
      "derived-from": "{{belief_id}}",
      "confidence": {{belief_confidence}}
    }
  ]
}
```

### Codex: AGENTS.md

```markdown
<skills>
You have additional SKILLs derived from Patina's epistemic beliefs.

<available_skills>
{{#each skills}}
{{name}}: `{{description}}`
{{/each}}
</available_skills>

IMPORTANT: Read the SKILL.md file when the description matches user intent.
</skills>
```

### Gemini: gemini-extension.json

```json
{
  "name": "patina",
  "description": "Patina knowledge and skills",
  "version": "{{version}}",
  "contextFileName": "agents/AGENTS.md"
}
```

### Checklist

- [ ] Generate `.claude-plugin/marketplace.json`
- [ ] Generate `agents/AGENTS.md`
- [ ] Generate `gemini-extension.json`
- [ ] Add `patina skill index` command to regenerate indices

**CHECKPOINT: Verify generated indices work with each adapter**

---

## Phase 5: Feedback Loop

Connect skill usage back to belief confidence.

### Usage Tracking

When a derived skill is used successfully:
1. Log usage event
2. Optionally update belief evidence
3. Consider confidence boost

When a derived skill fails or is rejected:
1. Log failure event
2. Queue for belief review
3. Consider confidence reduction

### Implementation (Future)

```rust
// In skill execution
pub fn track_skill_usage(skill_id: &str, outcome: Outcome) {
    // Find source belief
    let belief_id = get_source_belief(skill_id);

    match outcome {
        Outcome::Success => {
            add_evidence(belief_id, Evidence::SkillSuccess { skill_id, timestamp });
        }
        Outcome::Failure(reason) => {
            queue_review(belief_id, ReviewReason::SkillFailure { skill_id, reason });
        }
    }
}
```

### Checklist

- [ ] Design usage tracking schema
- [ ] Implement skill execution logging
- [ ] Connect to belief evidence system
- [ ] Add feedback to session-end distillation

**CHECKPOINT: Review feedback mechanism design**

---

## Testing

### Unit Tests

- [ ] Belief derivability check
- [ ] Template rendering
- [ ] Adapter output formatting

### Integration Tests

- [ ] Derive skill from real belief
- [ ] Verify Claude skill activates correctly
- [ ] Verify Codex AGENTS.md recognized
- [ ] Verify Gemini extension loads

### Manual Tests

1. Run `patina skill derive session-git-integration`
2. Start Claude Code session
3. Say "start a session"
4. Verify skill activates and calls Patina

---

## Success Criteria

1. **Derivation works**: `patina skill derive` produces valid skills
2. **Multi-adapter**: Same belief generates Claude, Codex, Gemini outputs
3. **Traceable**: Generated skills link back to source beliefs
4. **Thin wrappers**: Skills call Patina CLI, don't duplicate logic
5. **Threshold respected**: Only high-confidence beliefs become skills
6. **Index generated**: Adapter indices list all derived skills

---

## Future Extensions

### E1: Skill Composition

Beliefs with `supports` relationships could generate compound skills:
- `spec-first` + `measure-first` → `validated-development` skill

### E2: Conditional Skills

Beliefs with `attacked-by` could generate conditional skills:
- `sync-first` skill warns when `high-concurrency-needed` context detected

### E3: Cross-Project Skills

Mother beliefs could generate skills that work across all projects.

### E4: Skill Evolution

As beliefs are revised (AGM), automatically regenerate affected skills.

---

## References

- `layer/surface/epistemic/_index.md` - Belief inventory
- `layer/surface/architecture-persona-belief.md` - Belief architecture
- `spec-epistemic-layer.md` - Epistemic layer design
- `spec-spec-as-skill.md` - Related: spec management as skill
- HuggingFace skills repo - Multi-adapter reference implementation
- Agent Skills spec (agentskills.io) - Standard format
