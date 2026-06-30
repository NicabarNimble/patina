use anyhow::{Context, Result};
use clap::Subcommand;
use serde_json::Value;
use std::path::{Path, PathBuf};

use patina::child::engine::{FilesystemPreopen, GUEST_PROJECT_ROOT};

const SLATE_MANAGER: &str = "slate-manager";
const SLATE_WIT_PREFIX: &str = "patina:slate/control@0.1.0";

#[derive(Debug, Clone, Subcommand)]
pub enum SlateCommands {
    /// Create a draft Slate work item
    Create {
        /// Slate work id (kebab-case)
        id: String,

        /// Human-readable title; defaults to the id
        #[arg(long)]
        title: Option<String>,

        /// Original human request; defaults to the title
        #[arg(long, alias = "human-request")]
        request: Option<String>,

        /// Work kind: build, fix, or refactor
        #[arg(long, default_value = "build")]
        kind: String,

        /// Allium anchor path or id; repeatable
        #[arg(long = "allium-anchor", action = clap::ArgAction::Append)]
        allium_anchors: Vec<String>,

        /// User alignment statement; defaults to a command-created note
        #[arg(long, alias = "alignment")]
        user_alignment: Option<String>,

        /// Output raw JSON
        #[arg(long)]
        json: bool,
    },

    /// Recommend the next ready/active Slate work item
    Next {
        /// Optional status filter
        #[arg(long)]
        status: Option<String>,

        /// Optional kind filter
        #[arg(long)]
        kind: Option<String>,

        /// Output raw JSON
        #[arg(long)]
        json: bool,
    },

    /// Show a Slate work item
    Show {
        /// Slate work id
        id: String,

        /// Output raw JSON
        #[arg(long)]
        json: bool,
    },

    /// Check a Slate work item's proof plan
    Check {
        /// Slate work id
        id: String,

        /// Output raw JSON
        #[arg(long)]
        json: bool,
    },

    /// Complete a Slate work item after proof gates pass
    Complete {
        /// Slate work id
        id: String,

        /// Force completion even if gates fail
        #[arg(long)]
        force: bool,

        /// Output raw JSON
        #[arg(long)]
        json: bool,
    },

    /// Archive terminal Slate work with spec-parity recovery tag semantics
    Archive {
        /// Slate work id
        id: String,

        /// Force archive even if work is not terminal
        #[arg(long)]
        force: bool,

        /// Output raw JSON
        #[arg(long)]
        json: bool,
    },
}

pub fn execute(command: SlateCommands) -> Result<()> {
    match command {
        SlateCommands::Create {
            id,
            title,
            request,
            kind,
            allium_anchors,
            user_alignment,
            json,
        } => {
            let title = title.unwrap_or_else(|| id.clone());
            let human_request = request.unwrap_or_else(|| title.clone());
            let user_alignment = user_alignment.unwrap_or_else(|| {
                "Created from `patina slate create`; user alignment should be refined before promotion."
                    .to_string()
            });
            let result = call_slate_work("create-work", |project| {
                serde_json::json!([{
                    "project": project,
                    "id": id,
                    "title": title,
                    "kind": kind,
                    "human-request": human_request,
                    "allium-anchors": allium_anchors,
                    "user-alignment": user_alignment,
                }])
            })?;
            render_work_record(&result, json)
        }
        SlateCommands::Next { status, kind, json } => {
            let result = call_slate_work(
                "next-work",
                |project| serde_json::json!([{ "project": project, "status": status, "kind": kind }]),
            )?;
            render_next(&result, json)
        }
        SlateCommands::Show { id, json } => {
            let result = call_slate_work(
                "show-work",
                |project| serde_json::json!([{ "project": project, "id": id }]),
            )?;
            render_work_record(&result, json)
        }
        SlateCommands::Check { id, json } => {
            let result = call_slate_work(
                "check-work",
                |project| serde_json::json!([{ "project": project, "id": id }]),
            )?;
            render_check(&result, json)
        }
        SlateCommands::Complete { id, force, json } => {
            let result = call_slate_work(
                "complete-work",
                |project| serde_json::json!([{ "project": project, "id": id, "force": force }]),
            )?;
            render_work_record(&result, json)
        }
        SlateCommands::Archive { id, force, json } => {
            let result = call_slate_work(
                "archive-work",
                |project| serde_json::json!([{ "project": project, "id": id, "force": force }]),
            )?;
            render_archive(&result, json)
        }
    }
}

fn call_slate_work(operation: &str, args: impl FnOnce(Value) -> Value) -> Result<Value> {
    let (wasm_path, manifest_path) = slate_manager_paths()?;
    ensure_slate_manager_installed(&wasm_path, &manifest_path)?;

    let manifest = patina::child::engine::ChildManifest::from_path(&manifest_path)
        .with_context(|| format!("load {}", manifest_path.display()))?;
    if manifest.world != patina::child::engine::ChildKind::Child {
        anyhow::bail!("slate-manager is not a child-world component");
    }

    let project_root = patina::session::SessionManager::find_project_root()?;
    let preopens = slate_project_preopens(&manifest, &project_root)?;
    let payload = args(Value::String(GUEST_PROJECT_ROOT.to_string()));
    let operation_id = format!("{SLATE_WIT_PREFIX}.{operation}");

    let wasm_bytes =
        std::fs::read(&wasm_path).with_context(|| format!("read {}", wasm_path.display()))?;
    let engine = patina::child::engine::ChildEngine::new()?;
    let component = engine.load_component(&wasm_bytes)?;
    let mut child =
        engine.instantiate_child_with_preopens(&component, &manifest, None, &preopens)?;

    struct CliHost;
    impl patina::mother::MotherHost for CliHost {
        fn log(&self, child: &str, message: &str) {
            eprintln!("[{child}] {message}");
        }
    }
    child.on_load(&CliHost)?;

    let request = patina::mother::ChildCallRequest {
        operation_id,
        args: payload,
        correlation: None,
    };
    let response = child.call(&request)?;
    unwrap_typed_call_result(response.payload)
}

fn slate_manager_paths() -> Result<(PathBuf, PathBuf)> {
    let dir = patina::paths::child::command_children_dir();
    Ok((
        dir.join(format!("{SLATE_MANAGER}.wasm")),
        dir.join(format!("{SLATE_MANAGER}.toml")),
    ))
}

fn ensure_slate_manager_installed(wasm_path: &Path, manifest_path: &Path) -> Result<()> {
    if !wasm_path.exists() {
        anyhow::bail!(
            "slate-manager child not installed at {}\nInstall the Slate child before using `patina slate ...`.",
            wasm_path.display()
        );
    }
    if !manifest_path.exists() {
        anyhow::bail!(
            "slate-manager manifest not installed at {}\nInstall the Slate child manifest before using `patina slate ...`.",
            manifest_path.display()
        );
    }
    Ok(())
}

fn slate_project_preopens(
    manifest: &patina::child::engine::ChildManifest,
    project_root: &Path,
) -> Result<Vec<FilesystemPreopen>> {
    if !manifest.toys.filesystem {
        anyhow::bail!(
            "slate-manager manifest must request the filesystem toy so Patina can mount the project at {}",
            GUEST_PROJECT_ROOT
        );
    }

    Ok(vec![FilesystemPreopen::project_read_write(project_root)])
}

fn unwrap_typed_call_result(payload: Value) -> Result<Value> {
    let Some(first) = payload
        .get("results")
        .and_then(|value| value.as_array())
        .and_then(|results| results.first())
    else {
        return Ok(payload);
    };

    if let Some(ok) = first.get("ok") {
        return Ok(ok.clone());
    }
    if let Some(err) = first.get("err").and_then(|value| value.as_str()) {
        anyhow::bail!("slate-manager: {err}");
    }
    Ok(first.clone())
}

fn render_work_record(record: &Value, json: bool) -> Result<()> {
    if json {
        println!("{}", serde_json::to_string_pretty(record)?);
        return Ok(());
    }
    println!(
        "{}  {}  {}",
        record.get("id").and_then(Value::as_str).unwrap_or("-"),
        record.get("status").and_then(Value::as_str).unwrap_or("-"),
        record.get("title").and_then(Value::as_str).unwrap_or("-")
    );
    if let Some(path) = record.get("path").and_then(Value::as_str) {
        println!("path: {path}");
    }
    if let Some(request) = record.get("human-request").and_then(Value::as_str) {
        println!("request: {request}");
    }
    print_string_list("proof", record.get("proof-plan"));
    print_string_list("closure", record.get("closure-evidence"));
    Ok(())
}

fn render_next(value: &Value, json: bool) -> Result<()> {
    if json {
        println!("{}", serde_json::to_string_pretty(value)?);
        return Ok(());
    }
    let rows = value
        .as_array()
        .cloned()
        .unwrap_or_else(|| vec![value.clone()]);
    if rows.is_empty() {
        println!("No Slate work recommended.");
        return Ok(());
    }
    for row in rows {
        println!(
            "{}  {}  p{}  {}",
            row.get("id").and_then(Value::as_str).unwrap_or("-"),
            row.get("status").and_then(Value::as_str).unwrap_or("-"),
            row.get("priority").and_then(Value::as_u64).unwrap_or(0),
            row.get("reason").and_then(Value::as_str).unwrap_or("-")
        );
    }
    Ok(())
}

fn render_check(value: &Value, json: bool) -> Result<()> {
    if json {
        println!("{}", serde_json::to_string_pretty(value)?);
        return Ok(());
    }
    let checked = value.get("checked").and_then(Value::as_u64).unwrap_or(0);
    let total = value.get("total").and_then(Value::as_u64).unwrap_or(0);
    let passed = value
        .get("passed")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    println!(
        "{}: {}/{} checked ({})",
        value
            .get("work-id")
            .and_then(Value::as_str)
            .unwrap_or("slate"),
        checked,
        total,
        if passed { "passed" } else { "open" }
    );
    if let Some(unchecked) = value.get("unchecked").and_then(Value::as_array) {
        for item in unchecked {
            println!(
                "  [ ] {} {}",
                item.get("id").and_then(Value::as_str).unwrap_or("-"),
                item.get("text").and_then(Value::as_str).unwrap_or("")
            );
        }
    }
    Ok(())
}

fn render_archive(value: &Value, json: bool) -> Result<()> {
    if json {
        println!("{}", serde_json::to_string_pretty(value)?);
        return Ok(());
    }
    println!(
        "archived {} -> {}",
        value.get("work-id").and_then(Value::as_str).unwrap_or("-"),
        value
            .get("new-status")
            .and_then(Value::as_str)
            .unwrap_or("-")
    );
    if let Some(path) = value.get("path").and_then(Value::as_str) {
        println!("path: {path}");
    }
    Ok(())
}

fn print_string_list(label: &str, value: Option<&Value>) {
    let Some(items) = value.and_then(Value::as_array) else {
        return;
    };
    if items.is_empty() {
        return;
    }
    println!("{label}:");
    for item in items {
        if let Some(text) = item.as_str() {
            println!("  - {text}");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unwraps_typed_call_ok_payload() {
        let payload = serde_json::json!({"results":[{"ok":{"id":"demo"}}]});
        let unwrapped = unwrap_typed_call_result(payload).unwrap();
        assert_eq!(unwrapped["id"], "demo");
    }

    #[test]
    fn unwraps_typed_call_error_as_failure() {
        let payload = serde_json::json!({"results":[{"err":"not installed"}]});
        let error = unwrap_typed_call_result(payload).unwrap_err().to_string();
        assert!(error.contains("slate-manager: not installed"));
    }

    #[test]
    fn operation_ids_use_slate_control_package() {
        assert_eq!(
            format!("{SLATE_WIT_PREFIX}.{}", "show-work"),
            "patina:slate/control@0.1.0.show-work"
        );
    }

    #[test]
    fn slate_project_preopen_mounts_guest_project() {
        let temp = tempfile::tempdir().unwrap();
        let manifest_path = temp.path().join("slate-manager.toml");
        std::fs::write(
            &manifest_path,
            r#"[child]
name = "slate-manager"
version = "0.1.0"
kind = "child"

[needs]
toys = ["filesystem"]
"#,
        )
        .unwrap();
        let manifest = patina::child::engine::ChildManifest::from_path(&manifest_path).unwrap();
        let preopens = slate_project_preopens(&manifest, temp.path()).unwrap();
        assert_eq!(preopens.len(), 1);
        assert_eq!(preopens[0].host_path, temp.path());
        assert_eq!(preopens[0].guest_path, GUEST_PROJECT_ROOT);
        assert_eq!(
            preopens[0].mode,
            patina::child::engine::FilesystemAccessMode::ReadWrite
        );
    }

    #[test]
    fn slate_project_preopen_requires_filesystem_toy() {
        let temp = tempfile::tempdir().unwrap();
        let manifest_path = temp.path().join("slate-manager.toml");
        std::fs::write(
            &manifest_path,
            r#"[child]
name = "slate-manager"
version = "0.1.0"
kind = "child"

[needs]
toys = ["logging"]
"#,
        )
        .unwrap();
        let manifest = patina::child::engine::ChildManifest::from_path(&manifest_path).unwrap();
        let error = slate_project_preopens(&manifest, temp.path())
            .expect_err("slate-manager without filesystem should fail");
        assert!(error.to_string().contains("filesystem toy"));
    }

    #[test]
    fn missing_slate_manager_fails_closed() {
        let temp = tempfile::tempdir().unwrap();
        let wasm = temp.path().join("slate-manager.wasm");
        let manifest = temp.path().join("slate-manager.toml");
        let error = ensure_slate_manager_installed(&wasm, &manifest)
            .expect_err("missing slate-manager should fail");
        assert!(error
            .to_string()
            .contains("slate-manager child not installed"));
    }
}
