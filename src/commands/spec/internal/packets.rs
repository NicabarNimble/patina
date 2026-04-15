use anyhow::Result;
use serde::Serialize;
use std::path::Path;

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
    })
}

pub fn packet_spec_value(id: &str) -> Result<PacketBundle> {
    Ok(PacketBundle {
        prompt: prompt_spec_value(id)?,
        handoff: handoff_spec_value(id)?,
    })
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
