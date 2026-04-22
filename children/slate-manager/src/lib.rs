wit_bindgen::generate!({
    path: "wit",
    world: "slate-manager",
    generate_all,
});

use patina_sdk::toys;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

#[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
struct SlateManager;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct SpecFrontmatterLite {
    id: String,
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    r#type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    status: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    target: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    title: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    blocked_by: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    paused_date: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
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

#[derive(Debug, Clone, Copy)]
enum ReleaseBump {
    Patch,
    Minor,
    Major,
}

fn bump_from_spec_type(spec_type: &str) -> Option<ReleaseBump> {
    match spec_type {
        "fix" | "refactor" => Some(ReleaseBump::Patch),
        "feat" => Some(ReleaseBump::Minor),
        _ => None,
    }
}

fn compute_next_version(current: &str, bump: ReleaseBump) -> Result<String, String> {
    let parts: Vec<u32> = current
        .split('.')
        .map(|segment| {
            segment
                .parse::<u32>()
                .map_err(|_| format!("Invalid version component '{}'", segment))
        })
        .collect::<Result<Vec<_>, _>>()?;

    if parts.len() != 3 {
        return Err(format!("Expected semver format (x.y.z), got '{}'", current));
    }

    Ok(match bump {
        ReleaseBump::Patch => format!("{}.{}.{}", parts[0], parts[1], parts[2] + 1),
        ReleaseBump::Minor => format!("{}.{}.0", parts[0], parts[1] + 1),
        ReleaseBump::Major => format!("{}.0.0", parts[0] + 1),
    })
}

fn read_cargo_version(root: &Path) -> Result<String, String> {
    let content = fs::read_to_string(root.join("Cargo.toml"))
        .map_err(|e| format!("failed reading Cargo.toml: {}", e))?;
    let mut in_package_section = false;

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') {
            in_package_section = trimmed == "[package]";
            continue;
        }
        if in_package_section && trimmed.starts_with("version") && trimmed.contains('=') {
            let value = trimmed
                .split('=')
                .nth(1)
                .map(str::trim)
                .map(|v| v.trim_matches('"').trim_matches('\''))
                .filter(|v| !v.is_empty())
                .ok_or_else(|| "Could not parse version in Cargo.toml [package]".to_string())?;
            return Ok(value.to_string());
        }
    }

    Err("Could not find version in Cargo.toml [package]".to_string())
}

fn update_cargo_version(root: &Path, new_version: &str) -> Result<(), String> {
    let path = root.join("Cargo.toml");
    let content =
        fs::read_to_string(&path).map_err(|e| format!("read {}: {}", path.display(), e))?;

    let mut in_package_section = false;
    let mut version_updated = false;
    let mut new_content = String::new();

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') {
            in_package_section = trimmed == "[package]";
        }
        if in_package_section && !version_updated && trimmed.starts_with("version") {
            new_content.push_str(&format!("version = \"{}\"\n", new_version));
            version_updated = true;
        } else {
            new_content.push_str(line);
            new_content.push('\n');
        }
    }

    if !version_updated {
        return Err("Could not find version field in [package] section of Cargo.toml".to_string());
    }

    fs::write(&path, new_content).map_err(|e| format!("write {}: {}", path.display(), e))
}

fn ensure_release_safeguards(root: &Path, new_version: &str) -> Result<(), String> {
    if !patina::git::git::is_clean_tracked()? {
        return Err(
            "Working tree has uncommitted changes. Commit or stash before release.".to_string(),
        );
    }

    let behind = patina::git::git::commits_behind_upstream()?;
    if behind > 0 {
        return Err(format!(
            "Branch is {} commits behind remote. Pull changes first.",
            behind
        ));
    }

    if patina::git::git::is_diverged()? {
        return Err("Branch has diverged from remote. Resolve divergence first.".to_string());
    }

    let version_tag = format!("v{}", new_version);
    if patina::git::git::tag_exists(&version_tag)? {
        return Err(format!("Tag '{}' already exists", version_tag));
    }

    let index_path = root.join(".patina/local/data/patina.db");
    if !index_path.exists() {
        return Err(
            "No index found. Run 'patina scrape layer' first to build the index.".to_string(),
        );
    }

    Ok(())
}

fn complete_with_release(
    root: &Path,
    spec: &SpecRecord,
    bump: ReleaseBump,
) -> Result<String, String> {
    let old_version = read_cargo_version(root)?;
    let new_version = compute_next_version(&old_version, bump)?;
    ensure_release_safeguards(root, &new_version)?;

    update_cargo_version(root, &new_version)?;

    let spec_path = Path::new(&spec.path);
    let remove_target = spec_path
        .parent()
        .filter(|parent| parent.file_name().is_some())
        .map(|dir| to_repo_relative(root, dir))
        .unwrap_or_else(|| to_repo_relative(root, spec_path));

    patina::git::git::remove_paths(std::slice::from_ref(&remove_target))?;

    let mut stage_paths = vec!["Cargo.toml".to_string()];
    if root.join("Cargo.lock").exists() {
        stage_paths.push("Cargo.lock".to_string());
    }
    patina::git::git::add_paths(&stage_paths)?;

    let title = extract_title(&spec.body)
        .or(spec.frontmatter.title.clone())
        .unwrap_or_else(|| spec.frontmatter.id.clone());
    let commit_msg = format!("release: v{} — {}", new_version, title);
    patina::git::git::commit(&commit_msg)?;

    let version_tag = format!("v{}", new_version);
    patina::git::git::create_tag_at(&version_tag, "HEAD")?;

    let spec_tag = format!("spec/{}", spec.frontmatter.id);
    patina::git::git::create_tag_at(&spec_tag, "HEAD~1")?;

    Ok(new_version)
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

fn extract_command_args(
    payload: &serde_json::Value,
) -> Option<&serde_json::Map<String, serde_json::Value>> {
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

fn resolve_project_root_from_hint(project: Option<&str>) -> Result<PathBuf, String> {
    if let Some(project) = project {
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

fn resolve_project_root_from_envelope(envelope: &serde_json::Value) -> Result<PathBuf, String> {
    resolve_project_root_from_hint(envelope.get("project").and_then(|value| value.as_str()))
}

fn with_project_root_cwd<T>(
    project_root: &Path,
    f: impl FnOnce() -> Result<T, String>,
) -> Result<T, String> {
    let original = std::env::current_dir().map_err(|e| e.to_string())?;
    std::env::set_current_dir(project_root).map_err(|e| {
        format!(
            "failed to enter project root {}: {}",
            project_root.display(),
            e
        )
    })?;

    let result = f();

    let restore = std::env::set_current_dir(&original)
        .map_err(|e| format!("failed to restore cwd {}: {}", original.display(), e));

    match (result, restore) {
        (Ok(value), Ok(())) => Ok(value),
        (Err(error), Ok(())) => Err(error),
        (Ok(_), Err(restore_error)) => Err(restore_error),
        (Err(error), Err(_)) => Err(error),
    }
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
                (Some(to_repo_relative(root, &path)), Some(body))
            }
            _ => (None, None),
        };

        records.push(SpecRecord {
            frontmatter,
            path: to_repo_relative(root, &file),
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

fn arg_bool(
    args: Option<&serde_json::Map<String, serde_json::Value>>,
    key: &str,
    default: bool,
) -> bool {
    args.and_then(|map| map.get(key))
        .and_then(|v| v.as_bool())
        .unwrap_or(default)
}

fn arg_string(
    args: Option<&serde_json::Map<String, serde_json::Value>>,
    key: &str,
) -> Option<String> {
    args.and_then(|map| map.get(key))
        .and_then(|v| v.as_str())
        .map(|v| v.to_string())
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

fn is_terminal_status(status: &str) -> bool {
    matches!(status, "complete" | "completed" | "done" | "abandoned")
}

fn to_repo_relative(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .to_string()
}

fn archive_spec_record(root: &Path, spec: &SpecRecord, dry_run: bool) -> Result<(), String> {
    let status = status_or(&spec.frontmatter, "unknown");
    if !is_terminal_status(&status) {
        return Err(format!(
            "Spec '{}' has status '{}', expected 'complete' or 'abandoned'",
            spec.frontmatter.id, status
        ));
    }

    let tag_name = format!("spec/{}", spec.frontmatter.id);
    if patina::git::git::tag_exists(&tag_name)? {
        return Err(format!(
            "Tag '{}' already exists. Spec may have been archived previously.",
            tag_name
        ));
    }

    if dry_run {
        return Ok(());
    }

    if !patina::git::git::is_clean_tracked()? {
        return Err(
            "Working tree has uncommitted tracked changes. Commit or stash before archiving."
                .to_string(),
        );
    }

    let spec_path = Path::new(&spec.path);
    let remove_target = spec_path
        .parent()
        .filter(|parent| parent.file_name().is_some())
        .map(|dir| to_repo_relative(root, dir))
        .unwrap_or_else(|| to_repo_relative(root, spec_path));
    let spec_path_rel = to_repo_relative(root, spec_path);
    let description = spec
        .frontmatter
        .title
        .clone()
        .unwrap_or_else(|| spec.frontmatter.id.clone());

    patina::git::git::remove_paths(std::slice::from_ref(&remove_target))?;

    let commit_msg = format!(
        "docs: archive {} ({})\n\nSpec preserved via git tag: {}\nRecover with: git show {}:{}",
        tag_name, status, tag_name, tag_name, spec_path_rel
    );
    patina::git::git::commit(&commit_msg)?;
    patina::git::git::create_tag_at(&tag_name, "HEAD~1")?;

    toys::log::info(
        "slate-manager",
        &format!(
            "archived spec id={} status={} target={} description={}",
            spec.frontmatter.id, status, remove_target, description
        ),
    );

    Ok(())
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

fn handle_list(
    root: &Path,
    args: Option<&serde_json::Map<String, serde_json::Value>>,
) -> Result<serde_json::Value, String> {
    let status_filter = arg_string(args, "status");
    let target_filter = arg_string(args, "target");

    let specs = load_specs(root)?;
    let data: Vec<serde_json::Value> = specs
        .into_iter()
        .filter(|spec| {
            let status_ok = status_filter
                .as_deref()
                .is_none_or(|expected| spec.frontmatter.status.as_deref() == Some(expected));
            let target_ok = target_filter
                .as_deref()
                .is_none_or(|expected| spec.frontmatter.target.as_deref() == Some(expected));
            status_ok && target_ok
        })
        .map(|spec| {
            let title = extract_title(&spec.body)
                .or(spec.frontmatter.title.clone())
                .unwrap_or_else(|| spec.frontmatter.id.clone());
            serde_json::json!({
                "id": spec.frontmatter.id,
                "status": spec.frontmatter.status,
                "target": spec.frontmatter.target,
                "title": title,
                "unscraped": true,
            })
        })
        .collect();
    Ok(serde_json::Value::Array(data))
}

fn parse_queue_position(target: Option<&str>) -> Option<u32> {
    target.and_then(|t| t.trim().parse::<u32>().ok())
}

fn handle_next(root: &Path) -> Result<serde_json::Value, String> {
    let specs = load_specs(root)?;

    let mut status_map: HashMap<String, String> = HashMap::new();
    for spec in &specs {
        status_map.insert(
            spec.frontmatter.id.clone(),
            status_or(&spec.frontmatter, "draft"),
        );
    }

    let mut impact_counts: HashMap<String, usize> = HashMap::new();
    for spec in &specs {
        for blocker in &spec.frontmatter.blocked_by {
            *impact_counts.entry(blocker.clone()).or_insert(0) += 1;
        }
    }

    let mut out = Vec::new();

    for spec in specs {
        let status = status_or(&spec.frontmatter, "draft");
        let queue_position = parse_queue_position(spec.frontmatter.target.as_deref());
        let impact = impact_counts
            .get(&spec.frontmatter.id)
            .copied()
            .unwrap_or(0);

        match status.as_str() {
            "active" => {
                out.push(serde_json::json!({
                    "id": spec.frontmatter.id,
                    "status": status,
                    "reason": "Currently active — continue working",
                    "priority": 1,
                    "impact": impact,
                    "queue_position": queue_position,
                }));
            }
            "blocked" => {
                let all_blockers_done = spec.frontmatter.blocked_by.is_empty()
                    || spec.frontmatter.blocked_by.iter().all(|blocker_id| {
                        status_map
                            .get(blocker_id)
                            .map(|value| is_terminal_status(value))
                            .unwrap_or(true)
                    });
                if all_blockers_done {
                    out.push(serde_json::json!({
                        "id": spec.frontmatter.id,
                        "status": status,
                        "reason": "Blockers complete — ready to resume",
                        "priority": 2,
                        "impact": impact,
                        "queue_position": queue_position,
                    }));
                }
            }
            "paused" => {
                out.push(serde_json::json!({
                    "id": spec.frontmatter.id,
                    "status": status,
                    "reason": "Paused",
                    "priority": 4,
                    "impact": impact,
                    "queue_position": queue_position,
                }));
            }
            "ready" => {
                let reason = match queue_position {
                    Some(pos) => format!("Queue position #{}", pos),
                    None if impact > 0 => format!("Ready — blocks {} other spec(s)", impact),
                    None => "Ready to start".to_string(),
                };
                out.push(serde_json::json!({
                    "id": spec.frontmatter.id,
                    "status": status,
                    "reason": reason,
                    "priority": 5,
                    "impact": impact,
                    "queue_position": queue_position,
                }));
            }
            "draft" => {
                let reason = match queue_position {
                    Some(pos) => format!("Queue position #{} — needs audit", pos),
                    None => "Draft — unqueued".to_string(),
                };
                out.push(serde_json::json!({
                    "id": spec.frontmatter.id,
                    "status": status,
                    "reason": reason,
                    "priority": 6,
                    "impact": impact,
                    "queue_position": queue_position,
                }));
            }
            _ => {}
        }
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

        let aq = a.get("queue_position").and_then(|v| v.as_u64());
        let bq = b.get("queue_position").and_then(|v| v.as_u64());

        let ai = a.get("impact").and_then(|v| v.as_u64()).unwrap_or(0);
        let bi = b.get("impact").and_then(|v| v.as_u64()).unwrap_or(0);

        ap.cmp(&bp)
            .then_with(|| match (aq, bq) {
                (Some(la), Some(lb)) => la.cmp(&lb),
                (Some(_), None) => std::cmp::Ordering::Less,
                (None, Some(_)) => std::cmp::Ordering::Greater,
                (None, None) => std::cmp::Ordering::Equal,
            })
            .then_with(|| bi.cmp(&ai))
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

fn handle_complete(
    root: &Path,
    args: Option<&serde_json::Map<String, serde_json::Value>>,
) -> Result<serde_json::Value, String> {
    let id = require_id(args, "complete")?;
    let force = arg_bool(args, "force", false);
    let major = arg_bool(args, "major", false);

    let specs = load_specs(root)?;
    let spec = find_spec(&specs, id)?;
    let status = status_or(&spec.frontmatter, "unknown");
    if status != "active" {
        return Err(format!(
            "Cannot complete '{}' — status is '{}', expected 'active'",
            id, status
        ));
    }

    let criteria = normalize_criteria(&spec.frontmatter);
    let unchecked: Vec<(String, String)> = criteria
        .iter()
        .filter(|(_, _, checked)| !*checked)
        .map(|(criterion_id, text, _)| (criterion_id.clone(), text.clone()))
        .collect();

    if !unchecked.is_empty() && !force {
        let details = unchecked
            .iter()
            .map(|(criterion_id, text)| format!("  ✗ {} — {}", criterion_id, text))
            .collect::<Vec<_>>()
            .join("\n");
        return Err(format!(
            "Cannot complete '{}' — {} unchecked exit criteria:\n{}\n\n  Use --force to bypass.",
            id,
            unchecked.len(),
            details
        ));
    }

    let spec_type = spec
        .frontmatter
        .r#type
        .clone()
        .unwrap_or_else(|| "explore".to_string());

    let bump = if major {
        Some(ReleaseBump::Major)
    } else {
        bump_from_spec_type(&spec_type)
    };

    if let Some(bump) = bump {
        let new_version = complete_with_release(root, spec, bump)?;
        let _ = new_version;
        return Ok(serde_json::json!({
            "command": "complete",
            "spec_id": id,
            "new_status": "complete",
            "file": spec.path,
            "tag": format!("spec/{}", id),
            "archived": true,
        }));
    }

    let mut completed = spec.clone();
    completed.frontmatter.status = Some("complete".to_string());
    archive_spec_record(root, &completed, false)?;

    Ok(serde_json::json!({
        "command": "complete",
        "spec_id": id,
        "new_status": "complete",
        "file": completed.path,
        "tag": format!("spec/{}", id),
        "archived": true,
    }))
}

fn handle_archive(
    root: &Path,
    args: Option<&serde_json::Map<String, serde_json::Value>>,
) -> Result<serde_json::Value, String> {
    let stale = arg_bool(args, "stale", false);
    let dry_run = arg_bool(args, "dry_run", false);

    let specs = load_specs(root)?;

    if stale {
        let stale_specs: Vec<SpecRecord> = specs
            .into_iter()
            .filter(|spec| is_terminal_status(&status_or(&spec.frontmatter, "unknown")))
            .collect();

        if !dry_run {
            for spec in &stale_specs {
                archive_spec_record(root, spec, false)?;
            }
        }

        return Ok(serde_json::json!({
            "stale": true,
            "dry_run": dry_run,
        }));
    }

    let id = arg_string(args, "id")
        .ok_or_else(|| "Spec ID required. Use `patina spec archive <id>` or --stale".to_string())?;

    let spec = find_spec(&specs, &id)?;
    archive_spec_record(root, spec, dry_run)?;

    Ok(serde_json::json!({
        "id": id,
        "dry_run": dry_run,
    }))
}

fn dispatch_data_from_envelope(
    envelope: &serde_json::Value,
) -> Result<(String, String, PathBuf, serde_json::Value), String> {
    let command =
        extract_command_name(envelope).ok_or_else(|| "missing command payload".to_string())?;
    let backend_mode = extract_backend_mode(envelope);
    let args = extract_command_args(envelope);
    let project_root = resolve_project_root_from_envelope(envelope)?;

    let data = with_project_root_cwd(&project_root, || match command.as_str() {
        "list" => handle_list(&project_root, args),
        "next" => handle_next(&project_root),
        "check" => handle_check(&project_root, args),
        "show" => handle_show(&project_root, args),
        "prompt" => handle_prompt(&project_root, args),
        "handoff" => handle_handoff(&project_root, args),
        "packet" => handle_packet(&project_root, args),
        "complete" => handle_complete(&project_root, args),
        "archive" => handle_archive(&project_root, args),
        _ => Ok(serde_json::json!({
            "status": "scaffold",
            "message": format!("command '{}' not implemented", command),
            "command": command,
        })),
    })?;

    Ok((command, backend_mode, project_root, data))
}

pub fn dispatch_for_test(command_json: &str) -> Result<serde_json::Value, String> {
    let envelope: serde_json::Value = serde_json::from_str(command_json)
        .map_err(|error| format!("invalid command_json: {}", error))?;
    let (_, _, _, data) = dispatch_data_from_envelope(&envelope)?;
    Ok(data)
}

impl exports::patina::slate::control::Guest for SlateManager {
    fn list_specs(
        req: exports::patina::slate::control::ListRequest,
    ) -> Result<Vec<exports::patina::slate::control::SpecSummary>, String> {
        let project_root = resolve_project_root_from_hint(req.project.as_deref())?;
        with_project_root_cwd(&project_root, || {
            let specs = load_specs(&project_root)?;
            let rows = specs
                .into_iter()
                .filter(|spec| {
                    let status_ok = req.status.as_deref().is_none_or(|expected| {
                        spec.frontmatter.status.as_deref() == Some(expected)
                    });
                    let target_ok = req.target.as_deref().is_none_or(|expected| {
                        spec.frontmatter.target.as_deref() == Some(expected)
                    });
                    status_ok && target_ok
                })
                .map(|spec| {
                    let title = extract_title(&spec.body)
                        .or(spec.frontmatter.title.clone())
                        .unwrap_or_else(|| spec.frontmatter.id.clone());
                    exports::patina::slate::control::SpecSummary {
                        id: spec.frontmatter.id,
                        status: spec.frontmatter.status,
                        target: spec.frontmatter.target,
                        title,
                        unscraped: true,
                    }
                })
                .collect::<Vec<_>>();
            Ok(rows)
        })
    }

    fn next_specs(
        req: exports::patina::slate::control::NextRequest,
    ) -> Result<Vec<exports::patina::slate::control::NextRecommendation>, String> {
        let project_root = resolve_project_root_from_hint(req.project.as_deref())?;
        with_project_root_cwd(&project_root, || {
            let value = handle_next(&project_root)?;
            let rows = value
                .as_array()
                .ok_or_else(|| "next result must be an array".to_string())?
                .iter()
                .map(|item| {
                    let obj = item
                        .as_object()
                        .ok_or_else(|| "next item must be an object".to_string())?;
                    let id = obj
                        .get("id")
                        .and_then(|v| v.as_str())
                        .ok_or_else(|| "next item missing id".to_string())?
                        .to_string();
                    let status = obj
                        .get("status")
                        .and_then(|v| v.as_str())
                        .ok_or_else(|| "next item missing status".to_string())?
                        .to_string();
                    let reason = obj
                        .get("reason")
                        .and_then(|v| v.as_str())
                        .ok_or_else(|| "next item missing reason".to_string())?
                        .to_string();
                    let priority = obj
                        .get("priority")
                        .and_then(|v| v.as_u64())
                        .ok_or_else(|| "next item missing priority".to_string())?;
                    let impact = obj.get("impact").and_then(|v| v.as_u64()).unwrap_or(0);
                    let queue_position = obj.get("queue_position").and_then(|v| v.as_u64());
                    Ok(exports::patina::slate::control::NextRecommendation {
                        id,
                        status,
                        reason,
                        priority: u32::try_from(priority)
                            .map_err(|_| "priority exceeds u32".to_string())?,
                        impact: u32::try_from(impact)
                            .map_err(|_| "impact exceeds u32".to_string())?,
                        queue_position: queue_position
                            .map(|value| {
                                u32::try_from(value)
                                    .map_err(|_| "queue_position exceeds u32".to_string())
                            })
                            .transpose()?,
                    })
                })
                .collect::<Result<Vec<_>, String>>()?;
            Ok(rows)
        })
    }

    fn check_spec(
        req: exports::patina::slate::control::SpecIdRequest,
    ) -> Result<exports::patina::slate::control::CheckResult, String> {
        let project_root = resolve_project_root_from_hint(req.project.as_deref())?;
        with_project_root_cwd(&project_root, || {
            let value = handle_check(
                &project_root,
                Some(&serde_json::Map::from_iter([(
                    "id".to_string(),
                    serde_json::Value::String(req.id.clone()),
                )])),
            )?;
            let obj = value
                .as_object()
                .ok_or_else(|| "check result must be an object".to_string())?;

            let unchecked = obj
                .get("unchecked")
                .and_then(|v| v.as_array())
                .ok_or_else(|| "check result missing unchecked list".to_string())?
                .iter()
                .map(|item| {
                    let row = item
                        .as_object()
                        .ok_or_else(|| "unchecked item must be an object".to_string())?;
                    Ok(exports::patina::slate::control::UncheckedCriterion {
                        id: row
                            .get("id")
                            .and_then(|v| v.as_str())
                            .ok_or_else(|| "unchecked item missing id".to_string())?
                            .to_string(),
                        text: row
                            .get("text")
                            .and_then(|v| v.as_str())
                            .ok_or_else(|| "unchecked item missing text".to_string())?
                            .to_string(),
                    })
                })
                .collect::<Result<Vec<_>, String>>()?;

            Ok(exports::patina::slate::control::CheckResult {
                spec_id: obj
                    .get("spec_id")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| "check result missing spec_id".to_string())?
                    .to_string(),
                total: u32::try_from(
                    obj.get("total")
                        .and_then(|v| v.as_u64())
                        .ok_or_else(|| "check result missing total".to_string())?,
                )
                .map_err(|_| "check total exceeds u32".to_string())?,
                checked: u32::try_from(
                    obj.get("checked")
                        .and_then(|v| v.as_u64())
                        .ok_or_else(|| "check result missing checked".to_string())?,
                )
                .map_err(|_| "check checked exceeds u32".to_string())?,
                unchecked,
                passed: obj
                    .get("passed")
                    .and_then(|v| v.as_bool())
                    .ok_or_else(|| "check result missing passed".to_string())?,
            })
        })
    }

    fn show_spec(
        req: exports::patina::slate::control::SpecIdRequest,
    ) -> Result<exports::patina::slate::control::ShowResult, String> {
        let project_root = resolve_project_root_from_hint(req.project.as_deref())?;
        with_project_root_cwd(&project_root, || {
            let value = handle_show(
                &project_root,
                Some(&serde_json::Map::from_iter([(
                    "id".to_string(),
                    serde_json::Value::String(req.id.clone()),
                )])),
            )?;
            let obj = value
                .as_object()
                .ok_or_else(|| "show result must be an object".to_string())?;

            let parse_string_vec = |key: &str| -> Result<Vec<String>, String> {
                obj.get(key)
                    .and_then(|v| v.as_array())
                    .ok_or_else(|| format!("show result missing {}", key))?
                    .iter()
                    .map(|v| {
                        v.as_str()
                            .ok_or_else(|| format!("show {} element must be string", key))
                            .map(|s| s.to_string())
                    })
                    .collect::<Result<Vec<_>, String>>()
            };

            let design_outline = obj
                .get("design_outline")
                .and_then(|v| v.as_array())
                .map(|values| {
                    values
                        .iter()
                        .map(|v| {
                            v.as_str()
                                .ok_or_else(|| {
                                    "show design_outline element must be string".to_string()
                                })
                                .map(|s| s.to_string())
                        })
                        .collect::<Result<Vec<_>, String>>()
                })
                .transpose()?;

            let frontmatter_json = serde_json::to_string(
                obj.get("frontmatter")
                    .ok_or_else(|| "show result missing frontmatter".to_string())?,
            )
            .map_err(|error| format!("serialize frontmatter: {}", error))?;

            Ok(exports::patina::slate::control::ShowResult {
                id: obj
                    .get("id")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| "show result missing id".to_string())?
                    .to_string(),
                frontmatter_json,
                outline: parse_string_vec("outline")?,
                design_outline,
                files: parse_string_vec("files")?,
                direct_code_targets: parse_string_vec("direct_code_targets")?,
                resolved_decisions: parse_string_vec("resolved_decisions")?,
                implementation_order: parse_string_vec("implementation_order")?,
                verification_points: parse_string_vec("verification_points")?,
                open_questions: parse_string_vec("open_questions")?,
                path: obj
                    .get("path")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| "show result missing path".to_string())?
                    .to_string(),
                design_path: obj
                    .get("design_path")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string()),
            })
        })
    }

    fn prompt_spec(
        req: exports::patina::slate::control::SpecIdRequest,
    ) -> Result<exports::patina::slate::control::PromptResult, String> {
        let project_root = resolve_project_root_from_hint(req.project.as_deref())?;
        with_project_root_cwd(&project_root, || {
            let specs = load_specs(&project_root)?;
            let spec = find_spec(&specs, &req.id)?;
            let packet = build_prompt_packet(spec);
            let obj = packet
                .as_object()
                .ok_or_else(|| "prompt packet must be object".to_string())?;

            let parse_vec = |key: &str| -> Result<Vec<String>, String> {
                obj.get(key)
                    .and_then(|v| v.as_array())
                    .ok_or_else(|| format!("prompt missing {}", key))?
                    .iter()
                    .map(|v| {
                        v.as_str()
                            .ok_or_else(|| format!("prompt {} element must be string", key))
                            .map(|s| s.to_string())
                    })
                    .collect::<Result<Vec<_>, String>>()
            };

            Ok(exports::patina::slate::control::PromptResult {
                spec_id: obj
                    .get("spec_id")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| "prompt missing spec_id".to_string())?
                    .to_string(),
                status: obj
                    .get("status")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| "prompt missing status".to_string())?
                    .to_string(),
                title: obj
                    .get("title")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| "prompt missing title".to_string())?
                    .to_string(),
                goal: obj
                    .get("goal")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| "prompt missing goal".to_string())?
                    .to_string(),
                read_first: parse_vec("read_first")?,
                spec_path: obj
                    .get("spec_path")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| "prompt missing spec_path".to_string())?
                    .to_string(),
                design_path: obj
                    .get("design_path")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string()),
                direct_code_targets: parse_vec("direct_code_targets")?,
                execution_order: parse_vec("execution_order")?,
                constraints: parse_vec("constraints")?,
                verification: parse_vec("verification")?,
                definition_of_done: parse_vec("definition_of_done")?,
                session_workflow: parse_vec("session_workflow")?,
            })
        })
    }

    fn handoff_spec(
        req: exports::patina::slate::control::SpecIdRequest,
    ) -> Result<exports::patina::slate::control::HandoffResult, String> {
        let project_root = resolve_project_root_from_hint(req.project.as_deref())?;
        with_project_root_cwd(&project_root, || {
            let specs = load_specs(&project_root)?;
            let spec = find_spec(&specs, &req.id)?;
            let packet = build_handoff_packet(spec);
            let obj = packet
                .as_object()
                .ok_or_else(|| "handoff packet must be object".to_string())?;

            let parse_vec = |key: &str| -> Result<Vec<String>, String> {
                obj.get(key)
                    .and_then(|v| v.as_array())
                    .ok_or_else(|| format!("handoff missing {}", key))?
                    .iter()
                    .map(|v| {
                        v.as_str()
                            .ok_or_else(|| format!("handoff {} element must be string", key))
                            .map(|s| s.to_string())
                    })
                    .collect::<Result<Vec<_>, String>>()
            };

            let progress = obj
                .get("progress")
                .and_then(|v| v.as_object())
                .ok_or_else(|| "handoff missing progress".to_string())?;

            Ok(exports::patina::slate::control::HandoffResult {
                spec_id: obj
                    .get("spec_id")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| "handoff missing spec_id".to_string())?
                    .to_string(),
                status: obj
                    .get("status")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| "handoff missing status".to_string())?
                    .to_string(),
                title: obj
                    .get("title")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| "handoff missing title".to_string())?
                    .to_string(),
                progress: exports::patina::slate::control::ProgressSummary {
                    checked: u32::try_from(
                        progress
                            .get("checked")
                            .and_then(|v| v.as_u64())
                            .ok_or_else(|| "handoff progress missing checked".to_string())?,
                    )
                    .map_err(|_| "handoff progress checked exceeds u32".to_string())?,
                    total: u32::try_from(
                        progress
                            .get("total")
                            .and_then(|v| v.as_u64())
                            .ok_or_else(|| "handoff progress missing total".to_string())?,
                    )
                    .map_err(|_| "handoff progress total exceeds u32".to_string())?,
                },
                resolved_decisions: parse_vec("resolved_decisions")?,
                completed_items: parse_vec("completed_items")?,
                open_items: parse_vec("open_items")?,
                next_steps: parse_vec("next_steps")?,
                verification: parse_vec("verification")?,
                spec_path: obj
                    .get("spec_path")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| "handoff missing spec_path".to_string())?
                    .to_string(),
                design_path: obj
                    .get("design_path")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string()),
            })
        })
    }

    fn packet_spec(
        req: exports::patina::slate::control::SpecIdRequest,
    ) -> Result<exports::patina::slate::control::PacketResult, String> {
        let prompt = Self::prompt_spec(exports::patina::slate::control::SpecIdRequest {
            project: req.project.clone(),
            id: req.id.clone(),
        })?;
        let handoff = Self::handoff_spec(req)?;
        Ok(exports::patina::slate::control::PacketResult { prompt, handoff })
    }

    fn complete_spec(
        req: exports::patina::slate::control::CompleteRequest,
    ) -> Result<exports::patina::slate::control::CompleteResult, String> {
        let project_root = resolve_project_root_from_hint(req.project.as_deref())?;
        with_project_root_cwd(&project_root, || {
            let args = serde_json::Map::from_iter([
                ("id".to_string(), serde_json::Value::String(req.id.clone())),
                ("major".to_string(), serde_json::Value::Bool(req.major)),
                ("force".to_string(), serde_json::Value::Bool(req.force)),
            ]);
            let value = handle_complete(&project_root, Some(&args))?;
            let obj = value
                .as_object()
                .ok_or_else(|| "complete result must be an object".to_string())?;
            Ok(exports::patina::slate::control::CompleteResult {
                command: obj
                    .get("command")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| "complete result missing command".to_string())?
                    .to_string(),
                spec_id: obj
                    .get("spec_id")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| "complete result missing spec_id".to_string())?
                    .to_string(),
                new_status: obj
                    .get("new_status")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| "complete result missing new_status".to_string())?
                    .to_string(),
                file: obj
                    .get("file")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| "complete result missing file".to_string())?
                    .to_string(),
                tag: obj
                    .get("tag")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| "complete result missing tag".to_string())?
                    .to_string(),
                archived: obj
                    .get("archived")
                    .and_then(|v| v.as_bool())
                    .ok_or_else(|| "complete result missing archived".to_string())?,
            })
        })
    }

    fn archive_spec(
        req: exports::patina::slate::control::ArchiveRequest,
    ) -> Result<exports::patina::slate::control::ArchiveResult, String> {
        let project_root = resolve_project_root_from_hint(req.project.as_deref())?;
        with_project_root_cwd(&project_root, || {
            let mut args = serde_json::Map::new();
            if let Some(id) = req.id.clone() {
                args.insert("id".to_string(), serde_json::Value::String(id));
            }
            args.insert("stale".to_string(), serde_json::Value::Bool(req.stale));
            args.insert("dry_run".to_string(), serde_json::Value::Bool(req.dry_run));

            let value = handle_archive(&project_root, Some(&args))?;
            let obj = value
                .as_object()
                .ok_or_else(|| "archive result must be an object".to_string())?;

            Ok(exports::patina::slate::control::ArchiveResult {
                id: obj
                    .get("id")
                    .and_then(|v| v.as_str())
                    .map(|v| v.to_string()),
                stale: obj.get("stale").and_then(|v| v.as_bool()).unwrap_or(false),
                dry_run: obj
                    .get("dry_run")
                    .and_then(|v| v.as_bool())
                    .ok_or_else(|| "archive result missing dry_run".to_string())?,
            })
        })
    }

    fn dispatch(command_json: String) -> Result<String, String> {
        toys::measure::counter("slate_dispatch_calls", 1.0)?;

        let envelope: serde_json::Value = serde_json::from_str(&command_json)
            .map_err(|error| format!("invalid command_json: {}", error))?;
        let (command, backend_mode, project_root, data) = dispatch_data_from_envelope(&envelope)?;

        toys::measure::counter(&format!("slate_dispatch_command_{}", command), 1.0)?;

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

#[cfg(target_arch = "wasm32")]
export!(SlateManager);
