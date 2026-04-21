wit_bindgen::generate!({
    path: "wit",
    world: "slate-manager",
    generate_all,
});

use patina_sdk::toys;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

struct SlateManager;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct SpecFrontmatterLite {
    id: String,
    status: Option<String>,
    target: Option<String>,
    title: Option<String>,
    #[serde(default)]
    blocked_by: Vec<String>,
    #[serde(default)]
    paused_date: Option<String>,
    #[serde(default)]
    blocked_date: Option<String>,
    #[serde(default)]
    exit_criteria: Vec<ExitCriterionLite>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
enum ExitCriterionLite {
    Text(String),
    Full {
        #[serde(default)]
        id: Option<String>,
        text: String,
        #[serde(default)]
        checked: bool,
    },
}

#[derive(Debug, Clone)]
struct SpecRecord {
    frontmatter: SpecFrontmatterLite,
    path: String,
    body: String,
    design_path: Option<String>,
    design_body: Option<String>,
}

fn extract_command_name(payload: &serde_json::Value) -> Option<String> {
    let command = payload.get("command")?.as_object()?;
    let key = command.keys().next()?.to_ascii_lowercase();
    Some(key)
}

fn extract_backend_mode(payload: &serde_json::Value) -> String {
    payload
        .get("backend_mode")
        .and_then(|value| value.as_str())
        .map(|value| value.to_ascii_lowercase())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "off".to_string())
}

fn extract_command_args<'a>(
    payload: &'a serde_json::Value,
) -> Option<&'a serde_json::Map<String, serde_json::Value>> {
    let command = payload.get("command")?.as_object()?;
    let variant = command.values().next()?;
    variant.as_object()
}

fn is_patina_project_root(path: &Path) -> bool {
    path.join(".patina").is_dir() && path.join("layer").is_dir()
}

fn find_project_root() -> Result<PathBuf, String> {
    let mut current = std::env::current_dir().map_err(|e| e.to_string())?;
    loop {
        if is_patina_project_root(&current) {
            return Ok(current);
        }
        let Some(parent) = current.parent() else {
            return Err("not in a Patina project".to_string());
        };
        current = parent.to_path_buf();
    }
}

fn resolve_project_root_from_envelope(envelope: &serde_json::Value) -> Result<PathBuf, String> {
    if let Some(project) = envelope.get("project").and_then(|value| value.as_str()) {
        let trimmed = project.trim();
        if !trimmed.is_empty() {
            let candidate = PathBuf::from(trimmed);
            let resolved = if candidate.is_absolute() {
                candidate
            } else {
                std::env::current_dir()
                    .map_err(|e| e.to_string())?
                    .join(candidate)
            };
            if is_patina_project_root(&resolved) {
                return Ok(resolved);
            }
            return Err(format!(
                "invalid project root in slate envelope: {}",
                resolved.display()
            ));
        }
    }

    find_project_root()
}

fn extract_frontmatter_and_body(content: &str) -> Option<(&str, &str)> {
    let mut parts = content.splitn(3, "---");
    let first = parts.next()?;
    if !first.trim().is_empty() {
        return None;
    }
    let frontmatter = parts.next()?;
    let body = parts.next().unwrap_or_default();
    Some((frontmatter, body))
}

fn collect_spec_files(dir: &Path, out: &mut Vec<PathBuf>) -> Result<(), String> {
    let entries = fs::read_dir(dir).map_err(|e| format!("read_dir {}: {}", dir.display(), e))?;
    for entry in entries {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();
        if path.is_dir() {
            collect_spec_files(&path, out)?;
            continue;
        }
        if path.file_name().and_then(|n| n.to_str()) == Some("SPEC.md") {
            out.push(path);
        }
    }
    Ok(())
}

fn load_specs(root: &Path) -> Result<Vec<SpecRecord>, String> {
    let build_root = root.join("layer/surface/build");
    let mut files = Vec::new();
    if !build_root.exists() {
        return Ok(Vec::new());
    }
    collect_spec_files(&build_root, &mut files)?;

    let mut records = Vec::new();
    for file in files {
        let content =
            fs::read_to_string(&file).map_err(|e| format!("read {}: {}", file.display(), e))?;
        let Some((frontmatter_text, body)) = extract_frontmatter_and_body(&content) else {
            continue;
        };
        let frontmatter: SpecFrontmatterLite = serde_yaml::from_str(frontmatter_text)
            .map_err(|e| format!("parse frontmatter {}: {}", file.display(), e))?;
        if frontmatter.id.trim().is_empty() {
            continue;
        }

        let design_path_buf = file.parent().map(|parent| parent.join("DESIGN.md"));
        let (design_path, design_body) = match design_path_buf {
            Some(path) if path.exists() => {
                let body = fs::read_to_string(&path)
                    .map_err(|e| format!("read {}: {}", path.display(), e))?;
                (Some(path.to_string_lossy().to_string()), Some(body))
            }
            _ => (None, None),
        };

        records.push(SpecRecord {
            frontmatter,
            path: file.to_string_lossy().to_string(),
            body: body.to_string(),
            design_path,
            design_body,
        });
    }

    records.sort_by(|a, b| a.frontmatter.id.cmp(&b.frontmatter.id));
    Ok(records)
}

fn require_id<'a>(
    args: Option<&'a serde_json::Map<String, serde_json::Value>>,
    command: &str,
) -> Result<&'a str, String> {
    args.and_then(|map| map.get("id"))
        .and_then(|v| v.as_str())
        .ok_or_else(|| format!("{} requires id", command))
}

fn normalize_criteria(frontmatter: &SpecFrontmatterLite) -> Vec<(String, String, bool)> {
    frontmatter
        .exit_criteria
        .iter()
        .map(|criterion| match criterion {
            ExitCriterionLite::Text(text) => (slugify(text), text.clone(), false),
            ExitCriterionLite::Full { id, text, checked } => (
                id.clone().unwrap_or_else(|| slugify(text)),
                text.clone(),
                *checked,
            ),
        })
        .collect()
}

fn status_or(frontmatter: &SpecFrontmatterLite, default: &str) -> String {
    frontmatter
        .status
        .clone()
        .unwrap_or_else(|| default.to_string())
}

fn find_spec<'a>(specs: &'a [SpecRecord], id: &str) -> Result<&'a SpecRecord, String> {
    specs
        .iter()
        .find(|record| record.frontmatter.id == id)
        .ok_or_else(|| format!("spec '{}' not found", id))
}

fn extract_title(text: &str) -> Option<String> {
    text.lines()
        .find(|line| line.trim_start().starts_with("# "))
        .map(|line| {
            line.trim_start()
                .trim_start_matches("# ")
                .trim()
                .to_string()
        })
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

fn extract_section_items(text: &str, heading: &str) -> Vec<String> {
    let mut in_section = false;
    let mut items = Vec::new();

    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed == heading {
            in_section = true;
            continue;
        }
        if in_section && trimmed.starts_with("## ") {
            break;
        }
        if in_section
            && (trimmed.starts_with("- ") || trimmed.starts_with(|c: char| c.is_ascii_digit()))
        {
            items.push(trimmed.to_string());
        }
    }

    items
}

fn extract_outline(text: &str) -> Vec<String> {
    let mut in_fence = false;
    let mut headings = Vec::new();

    for line in text.lines() {
        let trimmed = line.trim_start();
        if trimmed.starts_with("```") || trimmed.starts_with("~~~") {
            in_fence = !in_fence;
            continue;
        }
        if !in_fence && trimmed.starts_with('#') && trimmed.contains(' ') {
            headings.push(line.to_string());
        }
    }

    headings
}

fn extract_key_files(body: &str) -> Vec<String> {
    let mut files = Vec::new();
    let mut in_key_files = false;
    let mut in_fence = false;

    for line in body.lines() {
        if line.starts_with("## Key Files") {
            in_key_files = true;
            continue;
        }
        if in_key_files && !in_fence && line.starts_with("## ") {
            break;
        }
        if in_key_files && line.trim_start().starts_with("```") {
            if in_fence {
                break;
            }
            in_fence = true;
            continue;
        }
        if in_key_files && in_fence {
            let trimmed = line.trim();
            if !trimmed.is_empty() && !trimmed.starts_with('#') {
                if let Some(path) = trimmed.split_whitespace().next() {
                    files.push(path.to_string());
                }
            }
        }
    }

    files
}

fn extract_code_targets(design_text: &str) -> Vec<String> {
    let mut targets = extract_section_items(design_text, "## Direct Code Targets");
    if targets.is_empty() {
        targets = extract_key_files(design_text);
    }
    targets
}

fn slugify(text: &str) -> String {
    let mut out = String::new();
    let mut prev_dash = false;

    for c in text.chars() {
        if c.is_ascii_alphanumeric() {
            out.push(c.to_ascii_lowercase());
            prev_dash = false;
        } else if !prev_dash {
            out.push('-');
            prev_dash = true;
        }
    }

    while out.ends_with('-') {
        out.pop();
    }

    if out.is_empty() {
        "criterion".to_string()
    } else {
        out
    }
}

fn handle_list(root: &Path) -> Result<serde_json::Value, String> {
    let specs = load_specs(root)?;
    let data: Vec<serde_json::Value> = specs
        .into_iter()
        .map(|spec| {
            let title = extract_title(&spec.body)
                .or(spec.frontmatter.title.clone())
                .unwrap_or_else(|| spec.frontmatter.id.clone());
            serde_json::json!({
                "id": spec.frontmatter.id,
                "status": spec.frontmatter.status,
                "target": spec.frontmatter.target,
                "title": title,
                "unscraped": false,
            })
        })
        .collect();
    Ok(serde_json::Value::Array(data))
}

fn handle_next(root: &Path) -> Result<serde_json::Value, String> {
    let specs = load_specs(root)?;
    let mut out = Vec::new();
    for spec in specs {
        let status = status_or(&spec.frontmatter, "draft");
        let (priority, reason) = match status.as_str() {
            "active" => (1_u32, "Currently active — continue working".to_string()),
            "ready" => (5_u32, "Ready to start".to_string()),
            "paused" => (4_u32, "Paused".to_string()),
            "blocked" => (3_u32, "Blocked".to_string()),
            _ => (6_u32, "Draft — unqueued".to_string()),
        };
        let queue_position = spec
            .frontmatter
            .target
            .as_deref()
            .and_then(|t| t.trim().parse::<u32>().ok());

        out.push(serde_json::json!({
            "id": spec.frontmatter.id,
            "status": status,
            "reason": reason,
            "priority": priority,
            "impact": 0,
            "queue_position": queue_position,
        }));
    }

    out.sort_by(|a, b| {
        let ap = a
            .get("priority")
            .and_then(|v| v.as_u64())
            .unwrap_or(u64::MAX);
        let bp = b
            .get("priority")
            .and_then(|v| v.as_u64())
            .unwrap_or(u64::MAX);
        ap.cmp(&bp)
    });

    Ok(serde_json::Value::Array(out))
}

fn handle_check(
    root: &Path,
    args: Option<&serde_json::Map<String, serde_json::Value>>,
) -> Result<serde_json::Value, String> {
    let id = require_id(args, "check")?;

    let specs = load_specs(root)?;
    let spec = find_spec(&specs, id)?;

    let criteria = normalize_criteria(&spec.frontmatter);
    let total = criteria.len();
    let checked = criteria
        .iter()
        .filter(|(_, _, is_checked)| *is_checked)
        .count();
    let unchecked: Vec<serde_json::Value> = criteria
        .into_iter()
        .filter(|(_, _, is_checked)| !*is_checked)
        .map(|(criterion_id, text, _)| {
            serde_json::json!({
                "id": criterion_id,
                "text": text,
            })
        })
        .collect();

    Ok(serde_json::json!({
        "spec_id": id,
        "total": total,
        "checked": checked,
        "unchecked": unchecked,
        "passed": checked == total,
    }))
}

fn handle_show(
    root: &Path,
    args: Option<&serde_json::Map<String, serde_json::Value>>,
) -> Result<serde_json::Value, String> {
    let id = require_id(args, "show")?;

    let specs = load_specs(root)?;
    let spec = find_spec(&specs, id)?;

    let design_outline = spec.design_body.as_ref().map(|d| extract_outline(d));
    let files = extract_key_files(&spec.body);
    let direct_code_targets = spec
        .design_body
        .as_deref()
        .map(extract_code_targets)
        .unwrap_or_default();
    let resolved_decisions = extract_section_items(&spec.body, "## Resolved Decisions");
    let implementation_order = extract_section_items(&spec.body, "## Implementation Order");
    let verification_points = extract_section_items(&spec.body, "## Verification");
    let open_questions = spec
        .design_body
        .as_deref()
        .map(|d| extract_section_items(d, "## Open Questions"))
        .unwrap_or_default();

    Ok(serde_json::json!({
        "id": spec.frontmatter.id,
        "frontmatter": spec.frontmatter,
        "outline": extract_outline(&spec.body),
        "design_outline": design_outline,
        "files": files,
        "direct_code_targets": direct_code_targets,
        "resolved_decisions": resolved_decisions,
        "implementation_order": implementation_order,
        "verification_points": verification_points,
        "open_questions": open_questions,
        "path": spec.path,
        "design_path": spec.design_path,
    }))
}

fn build_prompt_packet(spec: &SpecRecord) -> serde_json::Value {
    let status = status_or(&spec.frontmatter, "unknown");
    let title = extract_title(&spec.body)
        .or(spec.frontmatter.title.clone())
        .unwrap_or_else(|| spec.frontmatter.id.clone());
    let goal = extract_section_paragraph(&spec.body, "## Goal")
        .unwrap_or_else(|| "Execute this spec in small, verifiable slices.".to_string());
    let direct_code_targets = spec
        .design_body
        .as_deref()
        .map(extract_code_targets)
        .unwrap_or_default();
    let execution_order = extract_section_items(&spec.body, "## Implementation Order");
    let constraints = extract_section_items(&spec.body, "## Non-Goals");
    let verification = extract_section_items(&spec.body, "## Verification");

    let mut definition_of_done: Vec<String> = normalize_criteria(&spec.frontmatter)
        .into_iter()
        .map(|(_, text, _)| format!("- {}", text))
        .collect();
    if definition_of_done.is_empty() {
        definition_of_done
            .push("- Exit criteria are explicitly defined and satisfied.".to_string());
    }

    serde_json::json!({
        "spec_id": spec.frontmatter.id,
        "status": status,
        "title": title,
        "goal": goal,
        "read_first": [
            "layer/core/values/dependable-rust.md",
            "layer/core/values/unix-philosophy.md",
            "layer/core/values/spec-driven-design.md",
            "layer/core/values/safety-boundaries.md"
        ],
        "spec_path": spec.path,
        "design_path": spec.design_path,
        "direct_code_targets": direct_code_targets,
        "execution_order": execution_order,
        "constraints": constraints,
        "verification": verification,
        "definition_of_done": definition_of_done,
        "session_workflow": [
            "Run /session-update periodically.",
            "Run /session-note for important insights.",
            "Run /session-end when complete."
        ]
    })
}

fn build_handoff_packet(spec: &SpecRecord) -> serde_json::Value {
    let status = status_or(&spec.frontmatter, "unknown");
    let title = extract_title(&spec.body)
        .or(spec.frontmatter.title.clone())
        .unwrap_or_else(|| spec.frontmatter.id.clone());

    let criteria = normalize_criteria(&spec.frontmatter);
    let total = criteria.len();
    let checked = criteria
        .iter()
        .filter(|(_, _, is_checked)| *is_checked)
        .count();
    let completed_items: Vec<String> = criteria
        .iter()
        .filter(|(_, _, is_checked)| *is_checked)
        .map(|(_, text, _)| format!("- {}", text))
        .collect();
    let mut open_items: Vec<String> = criteria
        .iter()
        .filter(|(_, _, is_checked)| !*is_checked)
        .map(|(_, text, _)| format!("- {}", text))
        .collect();

    let mut open_questions = spec
        .design_body
        .as_deref()
        .map(|d| extract_section_items(d, "## Open Questions"))
        .unwrap_or_default();
    if open_questions.is_empty() {
        open_questions.push("- No open questions documented.".to_string());
    }
    open_items.extend(open_questions);

    serde_json::json!({
        "spec_id": spec.frontmatter.id,
        "status": status,
        "title": title,
        "progress": {
            "checked": checked,
            "total": total,
        },
        "resolved_decisions": extract_section_items(&spec.body, "## Resolved Decisions"),
        "completed_items": completed_items,
        "open_items": open_items,
        "next_steps": extract_section_items(&spec.body, "## Implementation Order"),
        "verification": extract_section_items(&spec.body, "## Verification"),
        "spec_path": spec.path,
        "design_path": spec.design_path,
    })
}

fn handle_prompt(
    root: &Path,
    args: Option<&serde_json::Map<String, serde_json::Value>>,
) -> Result<serde_json::Value, String> {
    let id = require_id(args, "prompt")?;
    let specs = load_specs(root)?;
    let spec = find_spec(&specs, id)?;
    Ok(build_prompt_packet(spec))
}

fn handle_handoff(
    root: &Path,
    args: Option<&serde_json::Map<String, serde_json::Value>>,
) -> Result<serde_json::Value, String> {
    let id = require_id(args, "handoff")?;
    let specs = load_specs(root)?;
    let spec = find_spec(&specs, id)?;
    Ok(build_handoff_packet(spec))
}

fn handle_packet(
    root: &Path,
    args: Option<&serde_json::Map<String, serde_json::Value>>,
) -> Result<serde_json::Value, String> {
    let id = require_id(args, "packet")?;
    let specs = load_specs(root)?;
    let spec = find_spec(&specs, id)?;
    Ok(serde_json::json!({
        "prompt": build_prompt_packet(spec),
        "handoff": build_handoff_packet(spec),
    }))
}

impl exports::patina::slate::control::Guest for SlateManager {
    fn dispatch(command_json: String) -> Result<String, String> {
        toys::measure::counter("slate_dispatch_calls", 1.0)?;

        let envelope: serde_json::Value = serde_json::from_str(&command_json)
            .map_err(|error| format!("invalid command_json: {}", error))?;
        let command =
            extract_command_name(&envelope).ok_or_else(|| "missing command payload".to_string())?;
        let backend_mode = extract_backend_mode(&envelope);
        let args = extract_command_args(&envelope);
        let project_root = resolve_project_root_from_envelope(&envelope)?;

        toys::measure::counter(&format!("slate_dispatch_command_{}", command), 1.0)?;

        let data = match command.as_str() {
            "list" => handle_list(&project_root)?,
            "next" => handle_next(&project_root)?,
            "check" => handle_check(&project_root, args)?,
            "show" => handle_show(&project_root, args)?,
            "prompt" => handle_prompt(&project_root, args)?,
            "handoff" => handle_handoff(&project_root, args)?,
            "packet" => handle_packet(&project_root, args)?,
            _ => {
                return Ok(serde_json::json!({
                    "status": "scaffold",
                    "message": format!("command '{}' not implemented", command),
                    "command": command,
                    "backend_mode": backend_mode,
                    "bytes": command_json.len(),
                })
                .to_string())
            }
        };

        toys::log::info(
            "slate-manager",
            &format!(
                "dispatch implemented command={} backend_mode={} project={} bytes={}",
                command,
                backend_mode,
                project_root.display(),
                command_json.len()
            ),
        );

        Ok(data.to_string())
    }
}

export!(SlateManager);
