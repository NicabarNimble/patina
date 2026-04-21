wit_bindgen::generate!({
    path: "wit",
    world: "slate-manager",
    generate_all,
});

use patina_sdk::toys;
use serde::Deserialize;
use std::fs;
use std::path::{Path, PathBuf};

struct SlateManager;

#[derive(Debug, Clone, Deserialize, Default)]
struct SpecFrontmatterLite {
    id: String,
    status: Option<String>,
    target: Option<String>,
    title: Option<String>,
    #[serde(default)]
    exit_criteria: Vec<ExitCriterionLite>,
}

#[derive(Debug, Clone, Deserialize)]
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
        }
    }

    find_project_root()
}

fn extract_frontmatter_block(content: &str) -> Option<&str> {
    let mut parts = content.splitn(3, "---");
    let first = parts.next()?;
    if !first.trim().is_empty() {
        return None;
    }
    let frontmatter = parts.next()?;
    Some(frontmatter)
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
        let Some(frontmatter_text) = extract_frontmatter_block(&content) else {
            continue;
        };
        let frontmatter: SpecFrontmatterLite = serde_yaml::from_str(frontmatter_text)
            .map_err(|e| format!("parse frontmatter {}: {}", file.display(), e))?;
        if frontmatter.id.trim().is_empty() {
            continue;
        }
        records.push(SpecRecord {
            frontmatter,
            path: file.to_string_lossy().to_string(),
        });
    }

    records.sort_by(|a, b| a.frontmatter.id.cmp(&b.frontmatter.id));
    Ok(records)
}

fn handle_list(root: &Path) -> Result<serde_json::Value, String> {
    let specs = load_specs(root)?;
    let data: Vec<serde_json::Value> = specs
        .into_iter()
        .map(|spec| {
            serde_json::json!({
                "id": spec.frontmatter.id,
                "status": spec.frontmatter.status,
                "target": spec.frontmatter.target,
                "title": spec.frontmatter.title.unwrap_or_default(),
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
        let status = spec
            .frontmatter
            .status
            .unwrap_or_else(|| "draft".to_string());
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
    let id = args
        .and_then(|map| map.get("id"))
        .and_then(|v| v.as_str())
        .ok_or_else(|| "check requires id".to_string())?;

    let specs = load_specs(root)?;
    let spec = specs
        .into_iter()
        .find(|record| record.frontmatter.id == id)
        .ok_or_else(|| format!("spec '{}' not found", id))?;

    let total = spec.frontmatter.exit_criteria.len();
    let mut checked = 0usize;
    let mut unchecked = Vec::new();

    for (idx, criterion) in spec.frontmatter.exit_criteria.iter().enumerate() {
        match criterion {
            ExitCriterionLite::Text(text) => {
                unchecked.push(serde_json::json!({
                    "id": format!("criterion-{}", idx + 1),
                    "text": text,
                }));
            }
            ExitCriterionLite::Full {
                id,
                text,
                checked: is_checked,
            } => {
                if *is_checked {
                    checked += 1;
                } else {
                    unchecked.push(serde_json::json!({
                        "id": id.clone().unwrap_or_else(|| format!("criterion-{}", idx + 1)),
                        "text": text,
                    }));
                }
            }
        }
    }

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
    let id = args
        .and_then(|map| map.get("id"))
        .and_then(|v| v.as_str())
        .ok_or_else(|| "show requires id".to_string())?;

    let specs = load_specs(root)?;
    let spec = specs
        .into_iter()
        .find(|record| record.frontmatter.id == id)
        .ok_or_else(|| format!("spec '{}' not found", id))?;

    Ok(serde_json::json!({
        "id": spec.frontmatter.id,
        "frontmatter": {
            "id": spec.frontmatter.id,
            "status": spec.frontmatter.status,
            "target": spec.frontmatter.target,
            "title": spec.frontmatter.title,
        },
        "outline": [],
        "files": [],
        "path": spec.path,
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
