#![cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]

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
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    related: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    beliefs: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    references: Vec<String>,
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

const SLATE_WORK_DIR: &str = "layer/slate/work";
const SLATE_EVENTS_PATH: &str = "layer/slate/events.jsonl";

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct SlateWorkFile {
    id: String,
    title: String,
    kind: String,
    #[serde(default = "default_slate_status")]
    status: String,
    human_request: String,
    #[serde(default)]
    allium_anchors: Vec<String>,
    #[serde(default)]
    user_alignment: String,
    #[serde(default)]
    belief_refs: Vec<String>,
    #[serde(default)]
    proof_plan: Vec<String>,
    #[serde(default)]
    closure_evidence: Vec<String>,
    #[serde(default)]
    blocked_by: Vec<String>,
    #[serde(default)]
    target: Option<String>,
    #[serde(default)]
    implementation_plan: Vec<String>,
    #[serde(default)]
    belief_harvest_decision: Option<String>,
    #[serde(default)]
    created_at: Option<String>,
    #[serde(default)]
    updated_at: Option<String>,
    #[serde(default)]
    closed_at: Option<String>,
    #[serde(default)]
    block_reason: Option<String>,
    #[serde(default)]
    pause_reason: Option<String>,
}

#[derive(Debug, Clone)]
struct SlateWorkRecord {
    work: SlateWorkFile,
    path: String,
}

fn default_slate_status() -> String {
    "draft".to_string()
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

            // Mother passes host-absolute project roots. In WASI, filesystem access is
            // scoped to guest preopens (commonly mounted at /input), so host absolute
            // paths may need remapping to guest-visible paths.
            if resolved.is_absolute() {
                let remapped = PathBuf::from("/input")
                    .join(resolved.strip_prefix("/").unwrap_or(resolved.as_path()));
                if is_patina_project_root(&remapped) {
                    return Ok(remapped);
                }
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
    let original = std::env::current_dir().ok();
    std::env::set_current_dir(project_root).map_err(|e| {
        format!(
            "failed to enter project root {}: {}",
            project_root.display(),
            e
        )
    })?;

    let result = f();

    if let Some(original) = original {
        let _ = std::env::set_current_dir(&original);
    }

    result
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

fn slate_work_dir(root: &Path) -> PathBuf {
    root.join(SLATE_WORK_DIR)
}

fn slate_work_path(root: &Path, id: &str) -> PathBuf {
    slate_work_dir(root).join(id).join("work.toml")
}

fn validate_slate_id(id: &str) -> Result<(), String> {
    if id.is_empty()
        || !id
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
    {
        return Err(format!("invalid Slate work id '{}': use kebab-case", id));
    }
    Ok(())
}

fn normalize_slate_kind(kind: &str) -> String {
    match kind.trim().to_ascii_lowercase().as_str() {
        "fix" => "fix".to_string(),
        "refactor" => "refactor".to_string(),
        _ => "build".to_string(),
    }
}

fn load_slate_work(root: &Path) -> Result<Vec<SlateWorkRecord>, String> {
    let dir = slate_work_dir(root);
    if !dir.exists() {
        return Ok(Vec::new());
    }

    let mut records = Vec::new();
    fn walk(root: &Path, dir: &Path, out: &mut Vec<SlateWorkRecord>) -> Result<(), String> {
        for entry in fs::read_dir(dir).map_err(|e| format!("read {}: {}", dir.display(), e))? {
            let entry = entry.map_err(|e| e.to_string())?;
            let path = entry.path();
            if path.is_dir() {
                walk(root, &path, out)?;
                continue;
            }
            if path.file_name().and_then(|name| name.to_str()) != Some("work.toml") {
                continue;
            }
            let raw =
                fs::read_to_string(&path).map_err(|e| format!("read {}: {}", path.display(), e))?;
            let work: SlateWorkFile =
                toml::from_str(&raw).map_err(|e| format!("parse {}: {}", path.display(), e))?;
            if work.id.is_empty() {
                return Err(format!("Slate work file {} has empty id", path.display()));
            }
            out.push(SlateWorkRecord {
                work,
                path: to_repo_relative(root, &path),
            });
        }
        Ok(())
    }

    walk(root, &dir, &mut records)?;
    records.sort_by(|a, b| {
        a.work
            .status
            .cmp(&b.work.status)
            .then(a.work.kind.cmp(&b.work.kind))
            .then(a.work.id.cmp(&b.work.id))
    });
    Ok(records)
}

fn find_slate_work<'a>(
    records: &'a [SlateWorkRecord],
    id: &str,
) -> Result<&'a SlateWorkRecord, String> {
    records
        .iter()
        .find(|record| record.work.id == id)
        .ok_or_else(|| format!("Slate work '{}' not found", id))
}

fn timestamp() -> String {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_secs().to_string())
        .unwrap_or_else(|_| "0".to_string())
}

fn write_slate_work_file(root: &Path, work: &mut SlateWorkFile) -> Result<SlateWorkRecord, String> {
    validate_slate_id(&work.id)?;
    let now = timestamp();
    if work.created_at.is_none() {
        work.created_at = Some(now.clone());
    }
    work.updated_at = Some(now);

    let path = slate_work_path(root, &work.id);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("create {}: {}", parent.display(), e))?;
    }
    let content = toml::to_string_pretty(work).map_err(|e| e.to_string())?;
    fs::write(&path, content).map_err(|e| format!("write {}: {}", path.display(), e))?;
    Ok(SlateWorkRecord {
        work: work.clone(),
        path: to_repo_relative(root, &path),
    })
}

fn append_slate_event(
    root: &Path,
    work_id: &str,
    event_type: &str,
    payload: serde_json::Value,
) -> Result<(), String> {
    let path = root.join(SLATE_EVENTS_PATH);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("create {}: {}", parent.display(), e))?;
    }
    let event = serde_json::json!({
        "work_id": work_id,
        "event_type": event_type,
        "payload": payload,
        "created_at": timestamp(),
    });
    use std::io::Write;
    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .map_err(|e| format!("open {}: {}", path.display(), e))?;
    writeln!(file, "{}", event).map_err(|e| format!("append {}: {}", path.display(), e))
}

fn create_slate_work_file(
    root: &Path,
    work: &mut SlateWorkFile,
) -> Result<SlateWorkRecord, String> {
    validate_slate_id(&work.id)?;
    let path = slate_work_path(root, &work.id);
    if path.exists() {
        return Err(format!("Slate work '{}' already exists", work.id));
    }
    let record = write_slate_work_file(root, work)?;
    append_slate_event(
        root,
        &record.work.id,
        "created",
        serde_json::json!({"status": record.work.status}),
    )?;
    Ok(record)
}

fn update_slate_work(
    root: &Path,
    id: &str,
    event_type: &str,
    payload: serde_json::Value,
    mutate: impl FnOnce(&mut SlateWorkFile) -> Result<(), String>,
) -> Result<SlateWorkRecord, String> {
    let mut records = load_slate_work(root)?;
    let mut record = find_slate_work(&records, id)?.clone();
    mutate(&mut record.work)?;
    let saved = write_slate_work_file(root, &mut record.work)?;
    append_slate_event(root, &saved.work.id, event_type, payload)?;
    records.clear();
    Ok(saved)
}

fn validate_ready_gate(work: &SlateWorkFile) -> Result<(), String> {
    let allium_ok = !work.allium_anchors.is_empty()
        || (work.kind == "refactor"
            && work
                .user_alignment
                .to_ascii_lowercase()
                .contains("no behavior"));
    if work.kind.is_empty()
        || work.human_request.trim().is_empty()
        || work.user_alignment.trim().is_empty()
        || work.proof_plan.is_empty()
        || !allium_ok
    {
        return Err(format!(
            "Slate work '{}' is not ready: require kind, human request, Allium anchors or no-behavior-change rationale, user alignment, and proof plan",
            work.id
        ));
    }
    Ok(())
}

fn load_slate_events(root: &Path, id: &str) -> Result<Vec<serde_json::Value>, String> {
    let path = root.join(SLATE_EVENTS_PATH);
    if !path.exists() {
        return Ok(Vec::new());
    }
    let raw = fs::read_to_string(&path).map_err(|e| format!("read {}: {}", path.display(), e))?;
    let mut events = Vec::new();
    for line in raw.lines().filter(|line| !line.trim().is_empty()) {
        let event: serde_json::Value =
            serde_json::from_str(line).map_err(|e| format!("parse {}: {}", path.display(), e))?;
        if event.get("work_id").and_then(|value| value.as_str()) == Some(id) {
            events.push(event);
        }
    }
    Ok(events)
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

fn build_slate_work_item(
    frontmatter: &SpecFrontmatterLite,
    body: &str,
    design_text: &str,
) -> serde_json::Value {
    let work_kind = extract_section_paragraph(body, "## Work Kind")
        .map(|s| normalize_work_kind(&s))
        .unwrap_or_else(|| infer_work_kind(frontmatter.r#type.as_deref().unwrap_or("feat")));
    let human_request = extract_section_paragraph(body, "## Human Request")
        .or_else(|| extract_blockquote(body))
        .or_else(|| extract_section_paragraph(body, "## Problem"))
        .unwrap_or_else(|| "No human request captured yet.".to_string());
    let allium_intent = extract_section_paragraph(body, "## Allium Intent")
        .unwrap_or_else(|| "No Allium intent summary captured yet.".to_string());
    let allium_anchors = collect_allium_anchors(frontmatter, body);
    let open_questions: Vec<String> = extract_section_items(body, "## Open Questions")
        .into_iter()
        .chain(extract_section_items(design_text, "## Open Questions"))
        .collect();
    let unresolved_questions = open_questions.clone();
    let user_alignment_statement = extract_section_paragraph(body, "## User Alignment")
        .unwrap_or_else(|| "No HITL alignment captured yet.".to_string());
    let relevant_beliefs = collect_relevant_beliefs(frontmatter, body);
    let core_doctrine_refs = collect_core_doctrine_refs(frontmatter, body);

    serde_json::json!({
        "work_kind": work_kind,
        "human_request": human_request,
        "allium": {
            "anchors": allium_anchors,
            "intent_summary": allium_intent,
            "intent_status": infer_allium_intent_status(&work_kind, &collect_allium_anchors(frontmatter, body), body),
            "open_questions": open_questions,
            "tool_commands": build_allium_tool_commands(&collect_allium_anchors(frontmatter, body)),
            "skill_workflows": [
                "tend: update intended behavior when HITL changes business truth",
                "weed: compare Allium intent against implementation drift",
                "propagate: derive tests from Allium obligations"
            ],
        },
        "user_alignment": {
            "aligned": has_non_placeholder_section(body, "## User Alignment"),
            "statement": user_alignment_statement,
            "unresolved_questions": unresolved_questions,
        },
        "relevant_beliefs": relevant_beliefs,
        "core_doctrine_refs": core_doctrine_refs,
        "implementation_plan": preferred_items(body, &["## Implementation Plan", "## Implementation Order"]),
        "proof_plan": preferred_items(body, &["## Proof Plan", "## Verification"]),
        "closure_evidence": preferred_items(body, &["## Closure Evidence", "## Evidence"]),
        "belief_harvest": build_belief_harvest(&collect_relevant_beliefs(frontmatter, body), body),
    })
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

fn build_belief_harvest(existing_beliefs: &[String], body: &str) -> serde_json::Value {
    serde_json::json!({
        "existing_beliefs": existing_beliefs,
        "evidence_to_add": preferred_items(body, &["## Belief Evidence", "## Closure Evidence"]),
        "proposed_new_beliefs": extract_section_items(body, "## Proposed Beliefs"),
        "proposed_scopes": extract_section_items(body, "## Belief Scopes"),
        "proposed_attacks": extract_section_items(body, "## Belief Attacks"),
        "proposed_defeats_or_archives": preferred_items(body, &["## Belief Defeats", "## Belief Archives"]),
        "decision_required": !has_non_placeholder_section(body, "## Belief Harvest"),
    })
}

fn slate_capability_matrix() -> Vec<serde_json::Value> {
    vec![
        cap(
            "create",
            "discovery",
            "capture human request and draft Slate work item",
            "intentional-divergence",
        ),
        cap(
            "list",
            "discovery",
            "list Slate work items by status/target/work kind",
            "preserve-compat",
        ),
        cap(
            "ready",
            "discovery",
            "show Slates ready after blockers and intent gates",
            "intentional-divergence",
        ),
        cap(
            "blocked",
            "discovery",
            "show Slates blocked by dependencies or intent/proof gaps",
            "intentional-divergence",
        ),
        cap(
            "next",
            "discovery",
            "recommend next Slate using status, blockers, queue, and intent readiness",
            "intentional-divergence",
        ),
        cap(
            "show",
            "discovery",
            "show Slate, Allium context, beliefs, proof, and files",
            "intentional-divergence",
        ),
        cap(
            "history",
            "discovery",
            "show Slate lifecycle and evidence history",
            "preserve-compat",
        ),
        cap(
            "prompt",
            "planning",
            "build agent prompt with Allium intent and belief constraints",
            "intentional-divergence",
        ),
        cap(
            "handoff",
            "planning",
            "summarize progress, proof gaps, Allium drift, and belief harvest",
            "intentional-divergence",
        ),
        cap(
            "packet",
            "planning",
            "bundle prompt and handoff context",
            "intentional-divergence",
        ),
        cap(
            "set",
            "shaping",
            "mutate Slate metadata and anchors",
            "intentional-divergence",
        ),
        cap(
            "rename",
            "shaping",
            "rename Slate work item and update durable identity",
            "preserve-compat",
        ),
        cap(
            "split",
            "shaping",
            "split Slate into smaller work items with inherited intent",
            "intentional-divergence",
        ),
        cap(
            "reopen",
            "shaping",
            "reopen closed Slate when proof or intent changes",
            "intentional-divergence",
        ),
        cap(
            "promote",
            "lifecycle",
            "advance draft→ready→active with Allium/HITL gates",
            "intentional-divergence",
        ),
        cap(
            "pause",
            "lifecycle",
            "pause active Slate with WIP capture",
            "preserve-compat",
        ),
        cap(
            "resume",
            "lifecycle",
            "resume paused/blocked Slate after blockers clear",
            "preserve-compat",
        ),
        cap(
            "block",
            "lifecycle",
            "block Slate on dependencies, missing intent, or proof gaps",
            "intentional-divergence",
        ),
        cap(
            "abandon",
            "lifecycle",
            "abandon Slate and preserve reason/evidence",
            "preserve-compat",
        ),
        cap(
            "check",
            "closure",
            "check exit criteria plus intent/proof/belief gates",
            "intentional-divergence",
        ),
        cap(
            "complete",
            "closure",
            "complete only after code, Allium, proof, and belief harvest reconcile",
            "intentional-divergence",
        ),
        cap(
            "archive",
            "closure",
            "archive completed/abandoned Slate with recovery tag",
            "preserve-compat",
        ),
    ]
}

fn cap(
    spec_action: &'static str,
    category: &'static str,
    slate_capability: &'static str,
    parity_policy: &'static str,
) -> serde_json::Value {
    serde_json::json!({
        "spec_action": spec_action,
        "category": category,
        "slate_capability": slate_capability,
        "parity_policy": parity_policy,
    })
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

fn collect_allium_anchors(frontmatter: &SpecFrontmatterLite, body: &str) -> Vec<String> {
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

fn collect_relevant_beliefs(frontmatter: &SpecFrontmatterLite, body: &str) -> Vec<String> {
    let mut refs = frontmatter.beliefs.clone();
    refs.extend(extract_section_items(body, "## Relevant Beliefs"));
    dedup(refs)
}

fn collect_core_doctrine_refs(frontmatter: &SpecFrontmatterLite, body: &str) -> Vec<String> {
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

fn json_string(
    obj: &serde_json::Map<String, serde_json::Value>,
    key: &str,
) -> Result<String, String> {
    obj.get(key)
        .and_then(|v| v.as_str())
        .ok_or_else(|| format!("missing string field {}", key))
        .map(ToString::to_string)
}

fn json_bool(obj: &serde_json::Map<String, serde_json::Value>, key: &str) -> Result<bool, String> {
    obj.get(key)
        .and_then(|v| v.as_bool())
        .ok_or_else(|| format!("missing bool field {}", key))
}

fn json_string_vec(
    obj: &serde_json::Map<String, serde_json::Value>,
    key: &str,
) -> Result<Vec<String>, String> {
    obj.get(key)
        .and_then(|v| v.as_array())
        .ok_or_else(|| format!("missing list field {}", key))?
        .iter()
        .map(|v| {
            v.as_str()
                .ok_or_else(|| format!("{} element must be string", key))
                .map(ToString::to_string)
        })
        .collect()
}

fn json_object<'a>(
    obj: &'a serde_json::Map<String, serde_json::Value>,
    key: &str,
) -> Result<&'a serde_json::Map<String, serde_json::Value>, String> {
    obj.get(key)
        .and_then(|v| v.as_object())
        .ok_or_else(|| format!("missing object field {}", key))
}

fn slate_work_summary(record: SlateWorkRecord) -> exports::patina::slate::control::WorkSummary {
    exports::patina::slate::control::WorkSummary {
        id: record.work.id,
        title: record.work.title,
        kind: record.work.kind,
        status: record.work.status,
        path: record.path,
    }
}

fn slate_work_record(record: SlateWorkRecord) -> exports::patina::slate::control::WorkRecord {
    exports::patina::slate::control::WorkRecord {
        id: record.work.id,
        title: record.work.title,
        kind: record.work.kind,
        status: record.work.status,
        human_request: record.work.human_request,
        allium_anchors: record.work.allium_anchors,
        user_alignment: record.work.user_alignment,
        belief_refs: record.work.belief_refs,
        proof_plan: record.work.proof_plan,
        closure_evidence: record.work.closure_evidence,
        blocked_by: record.work.blocked_by,
        target: record.work.target,
        implementation_plan: record.work.implementation_plan,
        belief_harvest_decision: record.work.belief_harvest_decision,
        path: record.path,
    }
}

fn slate_work_event(value: serde_json::Value) -> exports::patina::slate::control::WorkEvent {
    exports::patina::slate::control::WorkEvent {
        work_id: value
            .get("work_id")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string(),
        event_type: value
            .get("event_type")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string(),
        payload_json: value
            .get("payload")
            .map(|v| v.to_string())
            .unwrap_or_else(|| "{}".to_string()),
        created_at: value
            .get("created_at")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string(),
    }
}

fn normalize_work_items(items: &[String]) -> Vec<(String, String, bool)> {
    items
        .iter()
        .map(|text| {
            let checked = text.contains("[x]") || text.contains("checked: true");
            (slugify(text), text.clone(), checked)
        })
        .collect()
}

fn work_progress(work: &SlateWorkFile) -> (usize, usize, Vec<String>, Vec<String>) {
    let criteria = normalize_work_items(&work.proof_plan);
    let total = criteria.len();
    let checked = criteria.iter().filter(|(_, _, checked)| *checked).count();
    let completed = criteria
        .iter()
        .filter(|(_, _, checked)| *checked)
        .map(|(_, text, _)| text.clone())
        .collect();
    let open = criteria
        .iter()
        .filter(|(_, _, checked)| !*checked)
        .map(|(_, text, _)| text.clone())
        .collect();
    (checked, total, completed, open)
}

fn closure_gates(work: &SlateWorkFile) -> Vec<String> {
    let allium_ok = !work.allium_anchors.is_empty()
        || (work.kind == "refactor"
            && work
                .user_alignment
                .to_ascii_lowercase()
                .contains("no behavior"));
    vec![
        format!("Allium intent aligned: {}", allium_ok),
        format!("Proof plan present: {}", !work.proof_plan.is_empty()),
        format!(
            "Closure evidence present: {}",
            !work.closure_evidence.is_empty()
        ),
        format!(
            "Belief harvest decision present: {}",
            work.belief_harvest_decision.is_some()
        ),
    ]
}

fn validate_complete_gate(work: &SlateWorkFile) -> Result<(), String> {
    let (checked, total, _, _) = work_progress(work);
    if total == 0 || checked != total {
        return Err(format!(
            "Slate work '{}' cannot complete: proof plan is not fully checked",
            work.id
        ));
    }
    if work.closure_evidence.is_empty() {
        return Err(format!(
            "Slate work '{}' cannot complete: missing closure evidence",
            work.id
        ));
    }
    if work.belief_harvest_decision.is_none() {
        return Err(format!(
            "Slate work '{}' cannot complete: missing belief harvest decision",
            work.id
        ));
    }
    Ok(())
}

fn work_prompt_result(
    record: SlateWorkRecord,
) -> exports::patina::slate::control::WorkPromptResult {
    let closure_gates = closure_gates(&record.work);
    exports::patina::slate::control::WorkPromptResult {
        work_id: record.work.id,
        status: record.work.status,
        title: record.work.title,
        human_request: record.work.human_request,
        read_first: vec![
            "layer/allium/".to_string(),
            "layer/surface/epistemic/beliefs/".to_string(),
            "layer/core/".to_string(),
            record.path.clone(),
        ],
        allium_anchors: record.work.allium_anchors,
        implementation_plan: record.work.implementation_plan,
        proof_plan: record.work.proof_plan,
        belief_refs: record.work.belief_refs,
        closure_gates,
        path: record.path,
    }
}

fn work_handoff_result(
    record: SlateWorkRecord,
) -> Result<exports::patina::slate::control::WorkHandoffResult, String> {
    let (checked, total, completed_items, open_items) = work_progress(&record.work);
    Ok(exports::patina::slate::control::WorkHandoffResult {
        work_id: record.work.id,
        status: record.work.status,
        title: record.work.title,
        progress: exports::patina::slate::control::ProgressSummary {
            checked: u32::try_from(checked).map_err(|_| "checked exceeds u32".to_string())?,
            total: u32::try_from(total).map_err(|_| "total exceeds u32".to_string())?,
        },
        completed_items,
        open_items,
        next_steps: record.work.implementation_plan,
        closure_evidence: record.work.closure_evidence,
        belief_harvest_decision: record.work.belief_harvest_decision,
        path: record.path,
    })
}

impl exports::patina::slate::control::Guest for SlateManager {
    fn list_work(
        req: exports::patina::slate::control::WorkListRequest,
    ) -> Result<Vec<exports::patina::slate::control::WorkSummary>, String> {
        let project_root = resolve_project_root_from_hint(req.project.as_deref())?;
        with_project_root_cwd(&project_root, || {
            let records = load_slate_work(&project_root)?;
            Ok(records
                .into_iter()
                .filter(|record| {
                    let status_ok = req
                        .status
                        .as_deref()
                        .is_none_or(|expected| record.work.status == expected);
                    let kind_ok = req
                        .kind
                        .as_deref()
                        .is_none_or(|expected| record.work.kind == normalize_slate_kind(expected));
                    status_ok && kind_ok
                })
                .map(slate_work_summary)
                .collect())
        })
    }

    fn ready_work(
        req: exports::patina::slate::control::WorkListRequest,
    ) -> Result<Vec<exports::patina::slate::control::WorkSummary>, String> {
        let project_root = resolve_project_root_from_hint(req.project.as_deref())?;
        with_project_root_cwd(&project_root, || {
            let records = load_slate_work(&project_root)?;
            Ok(records
                .into_iter()
                .filter(|record| record.work.status == "ready" || record.work.status == "active")
                .filter(|record| {
                    req.kind
                        .as_deref()
                        .is_none_or(|kind| record.work.kind == normalize_slate_kind(kind))
                })
                .map(slate_work_summary)
                .collect())
        })
    }

    fn blocked_work(
        req: exports::patina::slate::control::WorkListRequest,
    ) -> Result<Vec<exports::patina::slate::control::WorkSummary>, String> {
        let project_root = resolve_project_root_from_hint(req.project.as_deref())?;
        with_project_root_cwd(&project_root, || {
            let records = load_slate_work(&project_root)?;
            Ok(records
                .into_iter()
                .filter(|record| {
                    record.work.status == "blocked" || !record.work.blocked_by.is_empty()
                })
                .filter(|record| {
                    req.kind
                        .as_deref()
                        .is_none_or(|kind| record.work.kind == normalize_slate_kind(kind))
                })
                .map(slate_work_summary)
                .collect())
        })
    }

    fn next_work(
        req: exports::patina::slate::control::WorkListRequest,
    ) -> Result<Vec<exports::patina::slate::control::WorkRecommendation>, String> {
        let project_root = resolve_project_root_from_hint(req.project.as_deref())?;
        with_project_root_cwd(&project_root, || {
            let records = load_slate_work(&project_root)?;
            let mut rows = records
                .into_iter()
                .filter(|record| {
                    req.kind
                        .as_deref()
                        .is_none_or(|kind| record.work.kind == normalize_slate_kind(kind))
                })
                .filter_map(|record| {
                    let (priority, reason) = match record.work.status.as_str() {
                        "active" => (1, "Currently active".to_string()),
                        "ready" => (2, "Ready to start".to_string()),
                        "blocked" if record.work.blocked_by.is_empty() => {
                            (3, "Blocked without dependency".to_string())
                        }
                        "draft" => (4, "Draft needs intent/proof alignment".to_string()),
                        _ => return None,
                    };
                    Some(exports::patina::slate::control::WorkRecommendation {
                        id: record.work.id,
                        status: record.work.status,
                        reason,
                        priority,
                        path: record.path,
                    })
                })
                .collect::<Vec<_>>();
            rows.sort_by(|a, b| a.priority.cmp(&b.priority).then(a.id.cmp(&b.id)));
            Ok(rows)
        })
    }

    fn show_work(
        req: exports::patina::slate::control::WorkIdRequest,
    ) -> Result<exports::patina::slate::control::WorkRecord, String> {
        let project_root = resolve_project_root_from_hint(req.project.as_deref())?;
        with_project_root_cwd(&project_root, || {
            let records = load_slate_work(&project_root)?;
            Ok(slate_work_record(
                find_slate_work(&records, &req.id)?.clone(),
            ))
        })
    }

    fn history_work(
        req: exports::patina::slate::control::WorkIdRequest,
    ) -> Result<Vec<exports::patina::slate::control::WorkEvent>, String> {
        let project_root = resolve_project_root_from_hint(req.project.as_deref())?;
        with_project_root_cwd(&project_root, || {
            let events = load_slate_events(&project_root, &req.id)?;
            Ok(events.into_iter().map(slate_work_event).collect())
        })
    }

    fn create_work(
        req: exports::patina::slate::control::CreateWorkRequest,
    ) -> Result<exports::patina::slate::control::WorkRecord, String> {
        let project_root = resolve_project_root_from_hint(req.project.as_deref())?;
        with_project_root_cwd(&project_root, || {
            let mut work = SlateWorkFile {
                id: req.id,
                title: req.title,
                kind: normalize_slate_kind(&req.kind),
                status: default_slate_status(),
                human_request: req.human_request,
                allium_anchors: req.allium_anchors,
                user_alignment: req.user_alignment,
                belief_refs: Vec::new(),
                proof_plan: Vec::new(),
                closure_evidence: Vec::new(),
                blocked_by: Vec::new(),
                target: None,
                implementation_plan: Vec::new(),
                belief_harvest_decision: None,
                created_at: None,
                updated_at: None,
                closed_at: None,
                block_reason: None,
                pause_reason: None,
            };
            create_slate_work_file(&project_root, &mut work).map(slate_work_record)
        })
    }

    fn set_work(
        req: exports::patina::slate::control::SetWorkRequest,
    ) -> Result<exports::patina::slate::control::WorkRecord, String> {
        let project_root = resolve_project_root_from_hint(req.project.as_deref())?;
        with_project_root_cwd(&project_root, || {
            update_slate_work(
                &project_root,
                &req.id,
                "set",
                serde_json::json!({"field": req.field, "value": req.value}),
                |work| {
                    match req.field.as_str() {
                        "title" => work.title = req.value,
                        "status" => work.status = req.value,
                        "target" => work.target = Some(req.value),
                        "user_alignment" => work.user_alignment = req.value,
                        "belief_harvest_decision" => work.belief_harvest_decision = Some(req.value),
                        "proof_plan" => work.proof_plan.push(req.value),
                        "implementation_plan" => work.implementation_plan.push(req.value),
                        "closure_evidence" => work.closure_evidence.push(req.value),
                        "allium_anchor" => work.allium_anchors.push(req.value),
                        "belief_ref" => work.belief_refs.push(req.value),
                        other => return Err(format!("unsupported Slate field '{}'", other)),
                    }
                    Ok(())
                },
            )
            .map(slate_work_record)
        })
    }

    fn promote_work(
        req: exports::patina::slate::control::WorkStatusRequest,
    ) -> Result<exports::patina::slate::control::WorkRecord, String> {
        let project_root = resolve_project_root_from_hint(req.project.as_deref())?;
        with_project_root_cwd(&project_root, || {
            update_slate_work(
                &project_root,
                &req.id,
                "promoted",
                serde_json::json!({"force": req.force}),
                |work| {
                    match work.status.as_str() {
                        "draft" => {
                            if !req.force {
                                validate_ready_gate(work)?;
                            }
                            work.status = "ready".to_string();
                        }
                        "ready" => work.status = "active".to_string(),
                        other => {
                            return Err(format!(
                                "cannot promote Slate work '{}' from status '{}'",
                                work.id, other
                            ))
                        }
                    }
                    Ok(())
                },
            )
            .map(slate_work_record)
        })
    }

    fn check_work(
        req: exports::patina::slate::control::WorkIdRequest,
    ) -> Result<exports::patina::slate::control::WorkCheckResult, String> {
        let project_root = resolve_project_root_from_hint(req.project.as_deref())?;
        with_project_root_cwd(&project_root, || {
            let records = load_slate_work(&project_root)?;
            let record = find_slate_work(&records, &req.id)?;
            let criteria = normalize_work_items(&record.work.proof_plan);
            let total = criteria.len();
            let checked = criteria.iter().filter(|(_, _, checked)| *checked).count();
            let unchecked = criteria
                .into_iter()
                .filter(|(_, _, checked)| !*checked)
                .map(
                    |(id, text, _)| exports::patina::slate::control::UncheckedCriterion {
                        id,
                        text,
                    },
                )
                .collect::<Vec<_>>();
            Ok(exports::patina::slate::control::WorkCheckResult {
                work_id: req.id,
                total: u32::try_from(total).map_err(|_| "total exceeds u32".to_string())?,
                checked: u32::try_from(checked).map_err(|_| "checked exceeds u32".to_string())?,
                unchecked,
                passed: checked == total && total > 0,
            })
        })
    }

    fn pause_work(
        req: exports::patina::slate::control::PauseWorkRequest,
    ) -> Result<exports::patina::slate::control::WorkRecord, String> {
        let project_root = resolve_project_root_from_hint(req.project.as_deref())?;
        with_project_root_cwd(&project_root, || {
            update_slate_work(
                &project_root,
                &req.id,
                "paused",
                serde_json::json!({"reason": req.reason}),
                |work| {
                    work.status = "paused".to_string();
                    work.pause_reason = Some(req.reason);
                    Ok(())
                },
            )
            .map(slate_work_record)
        })
    }

    fn resume_work(
        req: exports::patina::slate::control::WorkStatusRequest,
    ) -> Result<exports::patina::slate::control::WorkRecord, String> {
        let project_root = resolve_project_root_from_hint(req.project.as_deref())?;
        with_project_root_cwd(&project_root, || {
            update_slate_work(
                &project_root,
                &req.id,
                "resumed",
                serde_json::json!({"force": req.force}),
                |work| {
                    if !matches!(work.status.as_str(), "paused" | "blocked") && !req.force {
                        return Err(format!(
                            "cannot resume Slate work '{}' from status '{}'",
                            work.id, work.status
                        ));
                    }
                    work.status = "ready".to_string();
                    work.pause_reason = None;
                    Ok(())
                },
            )
            .map(slate_work_record)
        })
    }

    fn block_work(
        req: exports::patina::slate::control::BlockWorkRequest,
    ) -> Result<exports::patina::slate::control::WorkRecord, String> {
        let project_root = resolve_project_root_from_hint(req.project.as_deref())?;
        with_project_root_cwd(&project_root, || {
            update_slate_work(
                &project_root,
                &req.id,
                "blocked",
                serde_json::json!({"reason": req.reason, "blocked_by": req.blocked_by}),
                |work| {
                    work.status = "blocked".to_string();
                    work.block_reason = Some(req.reason);
                    if let Some(blocker) = req.blocked_by {
                        if !work.blocked_by.contains(&blocker) {
                            work.blocked_by.push(blocker);
                        }
                    }
                    Ok(())
                },
            )
            .map(slate_work_record)
        })
    }

    fn abandon_work(
        req: exports::patina::slate::control::PauseWorkRequest,
    ) -> Result<exports::patina::slate::control::WorkRecord, String> {
        let project_root = resolve_project_root_from_hint(req.project.as_deref())?;
        with_project_root_cwd(&project_root, || {
            update_slate_work(
                &project_root,
                &req.id,
                "abandoned",
                serde_json::json!({"reason": req.reason}),
                |work| {
                    work.status = "abandoned".to_string();
                    work.closed_at = Some(timestamp());
                    work.block_reason = Some(req.reason);
                    Ok(())
                },
            )
            .map(slate_work_record)
        })
    }

    fn rename_work(
        req: exports::patina::slate::control::RenameWorkRequest,
    ) -> Result<exports::patina::slate::control::WorkRecord, String> {
        let project_root = resolve_project_root_from_hint(req.project.as_deref())?;
        with_project_root_cwd(&project_root, || {
            validate_slate_id(&req.new_id)?;
            let records = load_slate_work(&project_root)?;
            let mut record = find_slate_work(&records, &req.id)?.clone();
            let old_dir = slate_work_path(&project_root, &req.id)
                .parent()
                .unwrap()
                .to_path_buf();
            let new_path = slate_work_path(&project_root, &req.new_id);
            if new_path.exists() {
                return Err(format!("Slate work '{}' already exists", req.new_id));
            }
            record.work.id = req.new_id.clone();
            let saved = write_slate_work_file(&project_root, &mut record.work)?;
            if old_dir.exists() {
                fs::remove_dir_all(&old_dir)
                    .map_err(|e| format!("remove {}: {}", old_dir.display(), e))?;
            }
            append_slate_event(
                &project_root,
                &saved.work.id,
                "renamed",
                serde_json::json!({"old_id": req.id}),
            )?;
            Ok(slate_work_record(saved))
        })
    }

    fn reopen_work(
        req: exports::patina::slate::control::WorkStatusRequest,
    ) -> Result<exports::patina::slate::control::WorkRecord, String> {
        let project_root = resolve_project_root_from_hint(req.project.as_deref())?;
        with_project_root_cwd(&project_root, || {
            update_slate_work(
                &project_root,
                &req.id,
                "reopened",
                serde_json::json!({"force": req.force}),
                |work| {
                    if !matches!(work.status.as_str(), "complete" | "abandoned") && !req.force {
                        return Err(format!(
                            "cannot reopen Slate work '{}' from status '{}'",
                            work.id, work.status
                        ));
                    }
                    work.status = "active".to_string();
                    work.closed_at = None;
                    Ok(())
                },
            )
            .map(slate_work_record)
        })
    }

    fn split_work(
        req: exports::patina::slate::control::SplitWorkRequest,
    ) -> Result<exports::patina::slate::control::WorkRecord, String> {
        let project_root = resolve_project_root_from_hint(req.project.as_deref())?;
        with_project_root_cwd(&project_root, || {
            let records = load_slate_work(&project_root)?;
            let parent = find_slate_work(&records, &req.id)?;
            let mut child = parent.work.clone();
            child.id = req.new_id;
            child.title = req.title;
            child.status = "draft".to_string();
            child.closure_evidence.clear();
            child.closed_at = None;
            child.created_at = None;
            child.updated_at = None;
            let saved = create_slate_work_file(&project_root, &mut child)?;
            append_slate_event(
                &project_root,
                &parent.work.id,
                "split",
                serde_json::json!({"child_id": saved.work.id}),
            )?;
            Ok(slate_work_record(saved))
        })
    }

    fn prompt_work(
        req: exports::patina::slate::control::WorkIdRequest,
    ) -> Result<exports::patina::slate::control::WorkPromptResult, String> {
        let project_root = resolve_project_root_from_hint(req.project.as_deref())?;
        with_project_root_cwd(&project_root, || {
            let records = load_slate_work(&project_root)?;
            Ok(work_prompt_result(
                find_slate_work(&records, &req.id)?.clone(),
            ))
        })
    }

    fn handoff_work(
        req: exports::patina::slate::control::WorkIdRequest,
    ) -> Result<exports::patina::slate::control::WorkHandoffResult, String> {
        let project_root = resolve_project_root_from_hint(req.project.as_deref())?;
        with_project_root_cwd(&project_root, || {
            let records = load_slate_work(&project_root)?;
            work_handoff_result(find_slate_work(&records, &req.id)?.clone())
        })
    }

    fn packet_work(
        req: exports::patina::slate::control::WorkIdRequest,
    ) -> Result<exports::patina::slate::control::WorkPacketResult, String> {
        let prompt = Self::prompt_work(exports::patina::slate::control::WorkIdRequest {
            project: req.project.clone(),
            id: req.id.clone(),
        })?;
        let handoff = Self::handoff_work(req)?;
        Ok(exports::patina::slate::control::WorkPacketResult { prompt, handoff })
    }

    fn complete_work(
        req: exports::patina::slate::control::WorkStatusRequest,
    ) -> Result<exports::patina::slate::control::WorkRecord, String> {
        let project_root = resolve_project_root_from_hint(req.project.as_deref())?;
        with_project_root_cwd(&project_root, || {
            update_slate_work(
                &project_root,
                &req.id,
                "completed",
                serde_json::json!({"force": req.force}),
                |work| {
                    if !req.force {
                        validate_complete_gate(work)?;
                    }
                    work.status = "complete".to_string();
                    work.closed_at = Some(timestamp());
                    Ok(())
                },
            )
            .map(slate_work_record)
        })
    }

    fn archive_work(
        req: exports::patina::slate::control::WorkStatusRequest,
    ) -> Result<exports::patina::slate::control::WorkArchiveResult, String> {
        let project_root = resolve_project_root_from_hint(req.project.as_deref())?;
        with_project_root_cwd(&project_root, || {
            let records = load_slate_work(&project_root)?;
            let record = find_slate_work(&records, &req.id)?;
            let status = record.work.status.clone();
            if !matches!(status.as_str(), "complete" | "abandoned") && !req.force {
                return Err(format!(
                    "cannot archive Slate work '{}' from status '{}'",
                    record.work.id, status
                ));
            }

            let tag_name = format!("slate/{}", record.work.id);
            if patina::git::git::tag_exists(&tag_name)? {
                return Err(format!(
                    "Tag '{}' already exists. Slate work may have been archived previously.",
                    tag_name
                ));
            }

            if !patina::git::git::is_clean_tracked()? {
                return Err(
                    "Working tree has uncommitted tracked changes. Commit or stash before archiving."
                        .to_string(),
                );
            }

            let work_dir = slate_work_path(&project_root, &record.work.id)
                .parent()
                .ok_or_else(|| format!("Slate work '{}' has no parent directory", record.work.id))?
                .to_path_buf();
            let remove_target = to_repo_relative(&project_root, &work_dir);
            let work_file_rel = record.path.clone();
            let title = record.work.title.clone();

            append_slate_event(
                &project_root,
                &record.work.id,
                "archived",
                serde_json::json!({"force": req.force, "tag": tag_name}),
            )?;
            patina::git::git::add_paths(std::slice::from_ref(&SLATE_EVENTS_PATH.to_string()))?;
            patina::git::git::remove_paths(std::slice::from_ref(&remove_target))?;

            let commit_msg = format!(
                "docs: archive {} ({})\n\nSlate work preserved via git tag: {}\nRecover with: git show {}:{}",
                tag_name, status, tag_name, tag_name, work_file_rel
            );
            patina::git::git::commit(&commit_msg)?;
            patina::git::git::create_tag_at(&tag_name, "HEAD~1")?;

            toys::log::info(
                "slate-manager",
                &format!(
                    "archived slate id={} status={} target={} title={}",
                    record.work.id, status, remove_target, title
                ),
            );

            Ok(exports::patina::slate::control::WorkArchiveResult {
                work_id: record.work.id.clone(),
                new_status: "archived".to_string(),
                path: remove_target,
                archived: true,
            })
        })
    }

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

#[cfg(test)]
mod slate_native_tests {
    use super::*;

    fn temp_project() -> tempfile::TempDir {
        let temp = tempfile::tempdir().expect("temp project");
        fs::create_dir_all(temp.path().join(".patina")).expect("patina dir");
        fs::create_dir_all(temp.path().join("layer")).expect("layer dir");
        temp
    }

    #[test]
    fn native_slate_create_update_and_history_are_project_living() {
        let temp = temp_project();
        let mut work = SlateWorkFile {
            id: "demo".to_string(),
            title: "Demo".to_string(),
            kind: "build".to_string(),
            status: "draft".to_string(),
            human_request: "Build it".to_string(),
            allium_anchors: vec!["layer/allium/demo.allium".to_string()],
            user_alignment: "User confirmed".to_string(),
            proof_plan: vec!["[x] cargo test".to_string()],
            ..Default::default()
        };

        let created = create_slate_work_file(temp.path(), &mut work).expect("create work");
        assert_eq!(created.path, "layer/slate/work/demo/work.toml");

        let promoted = update_slate_work(
            temp.path(),
            "demo",
            "promoted",
            serde_json::json!({"to": "ready"}),
            |work| {
                validate_ready_gate(work)?;
                work.status = "ready".to_string();
                Ok(())
            },
        )
        .expect("promote");
        assert_eq!(promoted.work.status, "ready");

        let events = load_slate_events(temp.path(), "demo").expect("events");
        assert_eq!(events.len(), 2);
        assert!(temp.path().join("layer/slate/work/demo/work.toml").exists());
        assert!(temp.path().join("layer/slate/events.jsonl").exists());
    }

    #[test]
    fn native_slate_ready_gate_blocks_missing_intent() {
        let work = SlateWorkFile {
            id: "demo".to_string(),
            title: "Demo".to_string(),
            kind: "build".to_string(),
            human_request: "Build it".to_string(),
            user_alignment: "User confirmed".to_string(),
            proof_plan: vec!["cargo test".to_string()],
            ..Default::default()
        };
        let err = validate_ready_gate(&work).expect_err("missing allium should block");
        assert!(err.contains("Allium"));
    }

    #[test]
    fn native_slate_packet_and_completion_gates_use_work_artifacts() {
        let temp = temp_project();
        let mut work = SlateWorkFile {
            id: "demo".to_string(),
            title: "Demo".to_string(),
            kind: "build".to_string(),
            status: "active".to_string(),
            human_request: "Build it".to_string(),
            allium_anchors: vec!["layer/allium/demo.allium".to_string()],
            user_alignment: "User confirmed".to_string(),
            implementation_plan: vec!["edit src/lib.rs".to_string()],
            proof_plan: vec!["[x] cargo test".to_string()],
            belief_refs: vec!["[[dependable-rust]]".to_string()],
            closure_evidence: vec!["cargo test passed".to_string()],
            belief_harvest_decision: Some("no belief change".to_string()),
            ..Default::default()
        };
        let record = create_slate_work_file(temp.path(), &mut work).expect("create work");
        let prompt = work_prompt_result(record.clone());
        assert_eq!(prompt.work_id, "demo");
        assert!(prompt.read_first.iter().any(|item| item == "layer/allium/"));
        let handoff = work_handoff_result(record.clone()).expect("handoff");
        assert_eq!(handoff.progress.checked, 1);
        validate_complete_gate(&record.work).expect("complete gate");

        let incomplete = SlateWorkFile {
            id: "bad".to_string(),
            title: "Bad".to_string(),
            kind: "build".to_string(),
            status: "active".to_string(),
            human_request: "Build it".to_string(),
            allium_anchors: vec!["layer/allium/demo.allium".to_string()],
            user_alignment: "User confirmed".to_string(),
            proof_plan: vec!["[x] cargo test".to_string()],
            ..Default::default()
        };
        let err = validate_complete_gate(&incomplete).expect_err("missing closure should block");
        assert!(err.contains("closure evidence"));
    }
}

#[cfg(target_arch = "wasm32")]
export!(SlateManager);
