use anyhow::Result;
use serde::Serialize;
use std::path::Path;

use patina::spec::SpecFrontmatter;

use super::archive::load_spec_read_only;
use super::queries::extract_section_items;

#[derive(Debug, Clone, Serialize)]
pub struct PromptPacket {
    pub spec_id: String,
    pub status: String,
    pub title: String,
    pub goal: String,
    pub read_first: Vec<String>,
    pub spec_path: String,
    pub design_path: Option<String>,
    pub direct_code_targets: Vec<String>,
    pub execution_order: Vec<String>,
    pub constraints: Vec<String>,
    pub verification: Vec<String>,
    pub definition_of_done: Vec<String>,
    pub session_workflow: Vec<String>,
    /// Slate's build/refactor/fix work-item view over the legacy spec packet.
    pub slate_work_item: SlateWorkItemPacket,
    /// Current spec capability coverage mapped into future Slate workflow terms.
    pub slate_capabilities: Vec<SlateCapabilityRow>,
}

#[derive(Debug, Clone, Serialize)]
pub struct HandoffPacket {
    pub spec_id: String,
    pub status: String,
    pub title: String,
    pub progress: ProgressSummary,
    pub resolved_decisions: Vec<String>,
    pub completed_items: Vec<String>,
    pub open_items: Vec<String>,
    pub next_steps: Vec<String>,
    pub verification: Vec<String>,
    pub spec_path: String,
    pub design_path: Option<String>,
    /// Closure-oriented Slate context: Allium alignment, proof, and belief harvest.
    pub slate_work_item: SlateWorkItemPacket,
    /// Current spec capability coverage mapped into future Slate workflow terms.
    pub slate_capabilities: Vec<SlateCapabilityRow>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ProgressSummary {
    pub checked: usize,
    pub total: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct PacketBundle {
    pub prompt: PromptPacket,
    pub handoff: HandoffPacket,
}

#[derive(Debug, Clone, Serialize)]
pub struct SlateWorkItemPacket {
    pub work_kind: String,
    pub human_request: String,
    pub allium: SlateAlliumContext,
    pub user_alignment: SlateUserAlignment,
    pub relevant_beliefs: Vec<String>,
    pub core_doctrine_refs: Vec<String>,
    pub implementation_plan: Vec<String>,
    pub proof_plan: Vec<String>,
    pub closure_evidence: Vec<String>,
    pub belief_harvest: SlateBeliefHarvest,
}

#[derive(Debug, Clone, Serialize)]
pub struct SlateAlliumContext {
    pub anchors: Vec<String>,
    pub intent_summary: String,
    pub intent_status: String,
    pub open_questions: Vec<String>,
    pub tool_commands: Vec<String>,
    pub skill_workflows: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SlateUserAlignment {
    pub aligned: bool,
    pub statement: String,
    pub unresolved_questions: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SlateBeliefHarvest {
    pub existing_beliefs: Vec<String>,
    pub evidence_to_add: Vec<String>,
    pub proposed_new_beliefs: Vec<String>,
    pub proposed_scopes: Vec<String>,
    pub proposed_attacks: Vec<String>,
    pub proposed_defeats_or_archives: Vec<String>,
    pub decision_required: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct SlateCapabilityRow {
    pub spec_action: &'static str,
    pub category: &'static str,
    pub slate_capability: &'static str,
    pub parity_policy: &'static str,
}

pub fn prompt_spec_value(id: &str) -> Result<PromptPacket> {
    let loaded = load_spec_read_only(id)?;
    let design_path = Path::new(&loaded.file_path)
        .parent()
        .map(|dir| dir.join("DESIGN.md"))
        .filter(|p| p.exists())
        .map(|p| p.to_string_lossy().to_string());
    let design_text = design_path
        .as_ref()
        .and_then(|p| std::fs::read_to_string(p).ok())
        .unwrap_or_default();

    let status = loaded.frontmatter.status.map_or("unknown", |s| s.as_str());
    let title = extract_title(&loaded.body).unwrap_or_else(|| loaded.frontmatter.id.clone());
    let goal = extract_section_paragraph(&loaded.body, "## Goal")
        .unwrap_or_else(|| "Execute this spec in small, verifiable slices.".to_string());
    let direct_code_targets = if !design_text.is_empty() {
        extract_section_items(&design_text, "## Direct Code Targets")
    } else {
        Vec::new()
    };
    let execution_order = extract_section_items(&loaded.body, "## Implementation Order");
    let constraints = extract_section_items(&loaded.body, "## Non-Goals");
    let verification = extract_section_items(&loaded.body, "## Verification");

    let mut definition_of_done: Vec<String> = loaded
        .frontmatter
        .exit_criteria
        .iter()
        .map(|c| format!("- {}", c.text))
        .collect();
    if definition_of_done.is_empty() {
        definition_of_done
            .push("- Exit criteria are explicitly defined and satisfied.".to_string());
    }

    let slate_work_item = build_slate_work_item(&loaded.frontmatter, &loaded.body, &design_text);

    Ok(PromptPacket {
        spec_id: loaded.frontmatter.id,
        status: status.to_string(),
        title,
        goal,
        read_first: vec![
            "layer/core/values/dependable-rust.md".to_string(),
            "layer/core/values/unix-philosophy.md".to_string(),
            "layer/core/values/spec-driven-design.md".to_string(),
            "layer/core/values/safety-boundaries.md".to_string(),
        ],
        spec_path: loaded.file_path,
        design_path,
        direct_code_targets,
        execution_order,
        constraints,
        verification,
        definition_of_done,
        session_workflow: vec![
            "Run /session-update periodically.".to_string(),
            "Run /session-note for important insights.".to_string(),
            "Run /session-end when complete.".to_string(),
        ],
        slate_work_item,
        slate_capabilities: slate_capability_matrix(),
    })
}

pub fn handoff_spec_value(id: &str) -> Result<HandoffPacket> {
    let loaded = load_spec_read_only(id)?;
    let design_path = Path::new(&loaded.file_path)
        .parent()
        .map(|dir| dir.join("DESIGN.md"))
        .filter(|p| p.exists())
        .map(|p| p.to_string_lossy().to_string());
    let design_text = design_path
        .as_ref()
        .and_then(|p| std::fs::read_to_string(p).ok())
        .unwrap_or_default();

    let status = loaded.frontmatter.status.map_or("unknown", |s| s.as_str());
    let title = extract_title(&loaded.body).unwrap_or_else(|| loaded.frontmatter.id.clone());
    let total = loaded.frontmatter.exit_criteria.len();
    let checked = loaded
        .frontmatter
        .exit_criteria
        .iter()
        .filter(|c| c.checked)
        .count();
    let completed_items = loaded
        .frontmatter
        .exit_criteria
        .iter()
        .filter(|c| c.checked)
        .map(|c| format!("- {}", c.text))
        .collect();
    let open_items = loaded
        .frontmatter
        .exit_criteria
        .iter()
        .filter(|c| !c.checked)
        .map(|c| format!("- {}", c.text))
        .collect();

    let resolved_decisions = extract_section_items(&loaded.body, "## Resolved Decisions");
    let next_steps = extract_section_items(&loaded.body, "## Implementation Order");
    let verification = extract_section_items(&loaded.body, "## Verification");
    let mut open_questions = if !design_text.is_empty() {
        extract_section_items(&design_text, "## Open Questions")
    } else {
        Vec::new()
    };
    if open_questions.is_empty() {
        open_questions.push("- No open questions documented.".to_string());
    }

    let slate_work_item = build_slate_work_item(&loaded.frontmatter, &loaded.body, &design_text);

    Ok(HandoffPacket {
        spec_id: loaded.frontmatter.id,
        status: status.to_string(),
        title,
        progress: ProgressSummary { checked, total },
        resolved_decisions,
        completed_items,
        open_items: [open_items, open_questions].concat(),
        next_steps,
        verification,
        spec_path: loaded.file_path,
        design_path,
        slate_work_item,
        slate_capabilities: slate_capability_matrix(),
    })
}

pub fn packet_spec_value(id: &str) -> Result<PacketBundle> {
    Ok(PacketBundle {
        prompt: prompt_spec_value(id)?,
        handoff: handoff_spec_value(id)?,
    })
}

fn build_slate_work_item(
    frontmatter: &SpecFrontmatter,
    body: &str,
    design_text: &str,
) -> SlateWorkItemPacket {
    let work_kind = extract_section_paragraph(body, "## Work Kind")
        .map(|s| normalize_work_kind(&s))
        .unwrap_or_else(|| infer_work_kind(&frontmatter.r#type));
    let human_request = extract_section_paragraph(body, "## Human Request")
        .or_else(|| extract_blockquote(body))
        .or_else(|| extract_section_paragraph(body, "## Problem"))
        .unwrap_or_else(|| "No human request captured yet.".to_string());
    let allium_intent = extract_section_paragraph(body, "## Allium Intent")
        .unwrap_or_else(|| "No Allium intent summary captured yet.".to_string());
    let allium_anchors = collect_allium_anchors(frontmatter, body);
    let allium_open_questions = extract_section_items(body, "## Open Questions")
        .into_iter()
        .chain(extract_section_items(design_text, "## Open Questions"))
        .collect::<Vec<_>>();
    let user_alignment_statement = extract_section_paragraph(body, "## User Alignment")
        .unwrap_or_else(|| "No HITL alignment captured yet.".to_string());
    let user_alignment = SlateUserAlignment {
        aligned: has_non_placeholder_section(body, "## User Alignment"),
        statement: user_alignment_statement,
        unresolved_questions: allium_open_questions.clone(),
    };
    let relevant_beliefs = collect_relevant_beliefs(frontmatter, body);
    let core_doctrine_refs = collect_core_doctrine_refs(frontmatter, body);
    let proof_plan = preferred_items(body, &["## Proof Plan", "## Verification"]);
    let implementation_plan =
        preferred_items(body, &["## Implementation Plan", "## Implementation Order"]);
    let closure_evidence = preferred_items(body, &["## Closure Evidence", "## Evidence"]);
    let belief_harvest = build_belief_harvest(&relevant_beliefs, body);

    SlateWorkItemPacket {
        work_kind: work_kind.clone(),
        human_request,
        allium: SlateAlliumContext {
            anchors: allium_anchors.clone(),
            intent_summary: allium_intent,
            intent_status: infer_allium_intent_status(&work_kind, &allium_anchors, body),
            open_questions: allium_open_questions,
            tool_commands: build_allium_tool_commands(&allium_anchors),
            skill_workflows: vec![
                "tend: update intended behavior when HITL changes business truth".to_string(),
                "weed: compare Allium intent against implementation drift".to_string(),
                "propagate: derive tests from Allium obligations".to_string(),
            ],
        },
        user_alignment,
        relevant_beliefs,
        core_doctrine_refs,
        implementation_plan,
        proof_plan,
        closure_evidence,
        belief_harvest,
    }
}

fn build_allium_tool_commands(anchors: &[String]) -> Vec<String> {
    if anchors.is_empty() {
        return vec![
            "allium check <allium-files>".to_string(),
            "allium analyse <allium-files>".to_string(),
            "allium plan <allium-files>".to_string(),
            "allium model <allium-files>".to_string(),
        ];
    }

    anchors
        .iter()
        .flat_map(|anchor| {
            let target = anchor.trim_start_matches("- ").to_string();
            [
                format!("allium check {}", target),
                format!("allium analyse {}", target),
                format!("allium plan {}", target),
                format!("allium model {}", target),
            ]
        })
        .collect()
}

fn build_belief_harvest(existing_beliefs: &[String], body: &str) -> SlateBeliefHarvest {
    SlateBeliefHarvest {
        existing_beliefs: existing_beliefs.to_vec(),
        evidence_to_add: preferred_items(body, &["## Belief Evidence", "## Closure Evidence"]),
        proposed_new_beliefs: extract_section_items(body, "## Proposed Beliefs"),
        proposed_scopes: extract_section_items(body, "## Belief Scopes"),
        proposed_attacks: extract_section_items(body, "## Belief Attacks"),
        proposed_defeats_or_archives: preferred_items(
            body,
            &["## Belief Defeats", "## Belief Archives"],
        ),
        decision_required: !has_non_placeholder_section(body, "## Belief Harvest"),
    }
}

fn slate_capability_matrix() -> Vec<SlateCapabilityRow> {
    vec![
        row(
            "create",
            "discovery",
            "capture human request and draft Slate work item",
            "intentional-divergence",
        ),
        row(
            "list",
            "discovery",
            "list Slate work items by status/target/work kind",
            "preserve-compat",
        ),
        row(
            "ready",
            "discovery",
            "show Slates ready after blockers and intent gates",
            "intentional-divergence",
        ),
        row(
            "blocked",
            "discovery",
            "show Slates blocked by dependencies or intent/proof gaps",
            "intentional-divergence",
        ),
        row(
            "next",
            "discovery",
            "recommend next Slate using status, blockers, queue, and intent readiness",
            "intentional-divergence",
        ),
        row(
            "show",
            "discovery",
            "show Slate, Allium context, beliefs, proof, and files",
            "intentional-divergence",
        ),
        row(
            "history",
            "discovery",
            "show Slate lifecycle and evidence history",
            "preserve-compat",
        ),
        row(
            "prompt",
            "planning",
            "build agent prompt with Allium intent and belief constraints",
            "intentional-divergence",
        ),
        row(
            "handoff",
            "planning",
            "summarize progress, proof gaps, Allium drift, and belief harvest",
            "intentional-divergence",
        ),
        row(
            "packet",
            "planning",
            "bundle prompt and handoff context",
            "intentional-divergence",
        ),
        row(
            "set",
            "shaping",
            "mutate Slate metadata and anchors",
            "intentional-divergence",
        ),
        row(
            "rename",
            "shaping",
            "rename Slate work item and update durable identity",
            "preserve-compat",
        ),
        row(
            "split",
            "shaping",
            "split Slate into smaller work items with inherited intent",
            "intentional-divergence",
        ),
        row(
            "reopen",
            "shaping",
            "reopen closed Slate when proof or intent changes",
            "intentional-divergence",
        ),
        row(
            "promote",
            "lifecycle",
            "advance draft→ready→active with Allium/HITL gates",
            "intentional-divergence",
        ),
        row(
            "pause",
            "lifecycle",
            "pause active Slate with WIP capture",
            "preserve-compat",
        ),
        row(
            "resume",
            "lifecycle",
            "resume paused/blocked Slate after blockers clear",
            "preserve-compat",
        ),
        row(
            "block",
            "lifecycle",
            "block Slate on dependencies, missing intent, or proof gaps",
            "intentional-divergence",
        ),
        row(
            "abandon",
            "lifecycle",
            "abandon Slate and preserve reason/evidence",
            "preserve-compat",
        ),
        row(
            "check",
            "closure",
            "check exit criteria plus intent/proof/belief gates",
            "intentional-divergence",
        ),
        row(
            "complete",
            "closure",
            "complete only after code, Allium, proof, and belief harvest reconcile",
            "intentional-divergence",
        ),
        row(
            "archive",
            "closure",
            "archive completed/abandoned Slate with recovery tag",
            "preserve-compat",
        ),
    ]
}

fn row(
    spec_action: &'static str,
    category: &'static str,
    slate_capability: &'static str,
    parity_policy: &'static str,
) -> SlateCapabilityRow {
    SlateCapabilityRow {
        spec_action,
        category,
        slate_capability,
        parity_policy,
    }
}

fn infer_work_kind(spec_type: &str) -> String {
    match spec_type {
        "fix" => "fix",
        "refactor" => "refactor",
        _ => "build",
    }
    .to_string()
}

fn normalize_work_kind(raw: &str) -> String {
    let lower = raw.to_ascii_lowercase();
    if lower.contains("refactor") {
        "refactor".to_string()
    } else if lower.contains("fix") || lower.contains("bug") {
        "fix".to_string()
    } else {
        "build".to_string()
    }
}

fn infer_allium_intent_status(work_kind: &str, anchors: &[String], body: &str) -> String {
    let allium_text = extract_section_paragraph(body, "## Allium Intent").unwrap_or_default();
    let lower = allium_text.to_ascii_lowercase();
    if work_kind == "refactor" && (lower.contains("no allium") || lower.contains("no behavior")) {
        return "not_behavioral_refactor".to_string();
    }
    if lower.contains("stale") || lower.contains("needs update") {
        return "needs_update".to_string();
    }
    if lower.contains("ambiguous") || lower.contains("unclear") {
        return "ambiguous".to_string();
    }
    if anchors.is_empty() && allium_text.is_empty() {
        return "missing".to_string();
    }
    "anchored".to_string()
}

fn collect_allium_anchors(frontmatter: &SpecFrontmatter, body: &str) -> Vec<String> {
    let mut anchors = Vec::new();
    for value in frontmatter
        .related
        .iter()
        .chain(frontmatter.references.iter())
    {
        if is_allium_ref(value) {
            anchors.push(value.clone());
        }
    }
    anchors.extend(extract_section_items(body, "## Allium Intent"));
    dedup(anchors)
}

fn collect_relevant_beliefs(frontmatter: &SpecFrontmatter, body: &str) -> Vec<String> {
    let mut refs = frontmatter.beliefs.clone();
    refs.extend(extract_section_items(body, "## Relevant Beliefs"));
    dedup(refs)
}

fn collect_core_doctrine_refs(frontmatter: &SpecFrontmatter, body: &str) -> Vec<String> {
    let mut refs: Vec<String> = frontmatter
        .references
        .iter()
        .chain(frontmatter.related.iter())
        .filter(|value| value.contains("layer/core"))
        .cloned()
        .collect();
    refs.extend(extract_section_items(body, "## Core Doctrine"));
    dedup(refs)
}

fn is_allium_ref(value: &str) -> bool {
    value.ends_with(".allium") || value.contains("layer/allium") || value.contains("/allium/")
}

fn preferred_items(body: &str, headings: &[&str]) -> Vec<String> {
    for heading in headings {
        let items = extract_section_items(body, heading);
        if !items.is_empty() {
            return items;
        }
    }
    Vec::new()
}

fn has_non_placeholder_section(text: &str, heading: &str) -> bool {
    extract_section_paragraph(text, heading)
        .map(|value| {
            let lower = value.to_ascii_lowercase();
            !lower.contains("not captured") && !lower.contains("todo") && !lower.is_empty()
        })
        .unwrap_or(false)
        || !extract_section_items(text, heading).is_empty()
}

fn extract_blockquote(text: &str) -> Option<String> {
    text.lines()
        .map(str::trim)
        .find(|line| line.starts_with("> "))
        .map(|line| line.trim_start_matches("> ").trim().to_string())
}

fn dedup(values: Vec<String>) -> Vec<String> {
    let mut out = Vec::new();
    for value in values {
        if !value.trim().is_empty() && !out.contains(&value) {
            out.push(value);
        }
    }
    out
}

fn extract_title(body: &str) -> Option<String> {
    body.lines()
        .find(|line| line.trim_start().starts_with("# "))
        .map(|line| line.trim_start_matches("# ").trim().to_string())
}

fn extract_section_paragraph(text: &str, heading: &str) -> Option<String> {
    let mut in_section = false;
    let mut lines = Vec::new();

    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed == heading {
            in_section = true;
            continue;
        }
        if in_section && trimmed.starts_with("## ") {
            break;
        }
        if in_section && !trimmed.is_empty() && !trimmed.starts_with('-') {
            lines.push(trimmed.to_string());
        }
    }

    if lines.is_empty() {
        None
    } else {
        Some(lines.join(" "))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slate_capability_matrix_covers_current_spec_actions() {
        let actions: Vec<&str> = slate_capability_matrix()
            .iter()
            .map(|row| row.spec_action)
            .collect();
        for expected in [
            "create", "list", "ready", "blocked", "next", "promote", "check", "show", "prompt",
            "handoff", "packet", "set", "pause", "resume", "block", "split", "complete", "abandon",
            "archive", "history", "rename", "reopen",
        ] {
            assert!(actions.contains(&expected), "missing {expected}");
        }
    }

    #[test]
    fn work_item_detects_allium_and_alignment() {
        let frontmatter = SpecFrontmatter {
            r#type: "feat".to_string(),
            related: vec!["layer/allium/example.allium".to_string()],
            beliefs: vec!["[[spec-driven-design]]".to_string()],
            references: vec!["layer/core/values/spec-driven-design.md".to_string()],
            ..Default::default()
        };
        let body = r#"
## Human Request
Change behavior.

## Allium Intent
- layer/allium/example.allium
The desired behavior is anchored.

## User Alignment
User confirmed this is the intended behavior.

## Verification
- cargo test
"#;
        let item = build_slate_work_item(&frontmatter, body, "");
        assert_eq!(item.work_kind, "build");
        assert_eq!(item.allium.intent_status, "anchored");
        assert!(item.user_alignment.aligned);
        assert!(item
            .relevant_beliefs
            .contains(&"[[spec-driven-design]]".to_string()));
        assert!(item
            .core_doctrine_refs
            .contains(&"layer/core/values/spec-driven-design.md".to_string()));
    }
}
