use anyhow::{Context, Result};
use serde::Serialize;
use std::fs;
use std::path::{Path, PathBuf};

use super::skills_sandbox::{self, SandboxMetadata};

#[derive(Debug, Clone, Serialize)]
pub struct SkillsStatusResponse {
    pub schema: &'static str,
    pub sandbox_id: Option<String>,
    pub scenario: Option<String>,
    pub hitl: String,
    pub scope: String,
    pub tuples: Vec<SkillTupleStatus>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SkillTupleStatus {
    pub child: String,
    pub skill: String,
    pub hitl: String,
    pub scope: String,
    pub state: String,
    pub projection_root: PathBuf,
    pub projection_path: PathBuf,
    pub source_path: PathBuf,
    pub conflict_reason: Option<String>,
    pub last_error: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SkillsSyncPlanResponse {
    pub schema: &'static str,
    pub sandbox_id: Option<String>,
    pub scenario: Option<String>,
    pub hitl: String,
    pub scope: String,
    pub dry_run: bool,
    pub actions: Vec<SkillSyncAction>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SkillSyncAction {
    pub child: String,
    pub skill: String,
    pub state: String,
    pub action: String,
    pub reason: String,
    pub safe_to_apply: bool,
    pub requires_force: bool,
    pub writes: Vec<PathBuf>,
    pub removes: Vec<PathBuf>,
}

pub fn status(child: Option<&str>, hitl_arg: Option<&str>, global: bool, json: bool) -> Result<()> {
    let response = build_status_response(child, hitl_arg, global)?;

    if json {
        println!("{}", serde_json::to_string_pretty(&response)?);
    } else {
        print_human_status(&response);
    }
    Ok(())
}

pub fn sync(
    child: Option<&str>,
    hitl_arg: Option<&str>,
    global: bool,
    dry_run: bool,
    json: bool,
) -> Result<()> {
    if !dry_run {
        anyhow::bail!(
            "non-dry-run skill sync is not implemented in this harness slice; use --dry-run"
        )
    }

    let status = build_status_response(child, hitl_arg, global)?;
    let response = build_plan_response(
        "patina.mother.skills.sync-plan.v1",
        status,
        dry_run,
        sync_action_for_tuple,
    );

    if json {
        println!("{}", serde_json::to_string_pretty(&response)?);
    } else {
        print_human_plan(&response);
    }
    Ok(())
}

pub fn install(
    child: &str,
    hitl_arg: Option<&str>,
    global: bool,
    dry_run: bool,
    json: bool,
) -> Result<()> {
    if !dry_run {
        anyhow::bail!(
            "non-dry-run skill install is not implemented in this harness slice; use --dry-run"
        )
    }

    let status = build_status_response(Some(child), hitl_arg, global)?;
    let response = build_plan_response(
        "patina.mother.skills.install-plan.v1",
        status,
        dry_run,
        install_action_for_tuple,
    );

    if json {
        println!("{}", serde_json::to_string_pretty(&response)?);
    } else {
        print_human_plan(&response);
    }
    Ok(())
}

pub fn uninstall(
    child: &str,
    hitl_arg: Option<&str>,
    global: bool,
    dry_run: bool,
    json: bool,
) -> Result<()> {
    if !dry_run {
        anyhow::bail!(
            "non-dry-run skill uninstall is not implemented in this harness slice; use --dry-run"
        )
    }

    let status = build_status_response(Some(child), hitl_arg, global)?;
    let response = build_plan_response(
        "patina.mother.skills.uninstall-plan.v1",
        status,
        dry_run,
        uninstall_action_for_tuple,
    );

    if json {
        println!("{}", serde_json::to_string_pretty(&response)?);
    } else {
        print_human_plan(&response);
    }
    Ok(())
}

fn build_plan_response(
    schema: &'static str,
    status: SkillsStatusResponse,
    dry_run: bool,
    planner: fn(&SkillTupleStatus) -> Option<SkillSyncAction>,
) -> SkillsSyncPlanResponse {
    let actions = status.tuples.iter().filter_map(planner).collect::<Vec<_>>();
    SkillsSyncPlanResponse {
        schema,
        sandbox_id: status.sandbox_id,
        scenario: status.scenario,
        hitl: status.hitl,
        scope: status.scope,
        dry_run,
        actions,
    }
}

fn build_status_response(
    child: Option<&str>,
    hitl_arg: Option<&str>,
    global: bool,
) -> Result<SkillsStatusResponse> {
    let cwd = std::env::current_dir().context("resolving current directory")?;
    let sandbox = skills_sandbox::find_enclosing_sandbox(&cwd)?;
    let hitl = resolve_hitl(hitl_arg, sandbox.as_ref())?;
    let scope = if global { "global" } else { "project" };
    let tuples = match sandbox.as_ref() {
        Some(metadata) => sandbox_status_tuples(metadata, child, &hitl, scope)?,
        None => Vec::new(),
    };

    Ok(SkillsStatusResponse {
        schema: "patina.mother.skills.status.v1",
        sandbox_id: sandbox.as_ref().map(|s| s.id.clone()),
        scenario: sandbox.as_ref().map(|s| s.scenario.clone()),
        hitl,
        scope: scope.to_string(),
        tuples,
    })
}

fn resolve_hitl(hitl_arg: Option<&str>, sandbox: Option<&SandboxMetadata>) -> Result<String> {
    if let Some(hitl) = hitl_arg.filter(|value| !value.trim().is_empty()) {
        return Ok(normalize_hitl(hitl));
    }
    if let Some(metadata) = sandbox {
        return Ok(metadata.default_interface.clone());
    }
    anyhow::bail!(
        "could not infer HITL interface; pass global --interface <pi|claude|opencode|gemini>"
    )
}

fn normalize_hitl(hitl: &str) -> String {
    match hitl {
        "open-code" | "open_code" | "OpenCode" => "opencode".to_string(),
        other => other.to_ascii_lowercase(),
    }
}

fn sandbox_status_tuples(
    metadata: &SandboxMetadata,
    child_filter: Option<&str>,
    hitl: &str,
    scope: &str,
) -> Result<Vec<SkillTupleStatus>> {
    let mut tuples = Vec::new();
    if !metadata.child_store_root.exists() {
        return Ok(tuples);
    }

    for entry in fs::read_dir(&metadata.child_store_root)? {
        let entry = entry?;
        if !entry.file_type()?.is_dir() {
            continue;
        }
        let child_name = entry.file_name().to_string_lossy().to_string();
        if child_filter.is_some_and(|filter| filter != child_name) {
            continue;
        }
        let skills_dir = entry.path().join("skills");
        if !skills_dir.exists() {
            continue;
        }
        for skill_entry in fs::read_dir(&skills_dir)? {
            let skill_entry = skill_entry?;
            if !skill_entry.file_type()?.is_dir() {
                continue;
            }
            let skill_name = skill_entry.file_name().to_string_lossy().to_string();
            let source_path = skill_entry.path().join("SKILL.md");
            if !source_path.exists() {
                continue;
            }
            let projection_root = projection_root(metadata, hitl, scope);
            let projection_path = projection_path(&projection_root, hitl, &child_name, &skill_name);
            let (state, conflict_reason, last_error) =
                evaluate_sandbox_state(metadata, scope, &projection_path, &source_path);
            tuples.push(SkillTupleStatus {
                child: child_name.clone(),
                skill: skill_name,
                hitl: hitl.to_string(),
                scope: scope.to_string(),
                state,
                projection_root: projection_root.clone(),
                projection_path,
                source_path,
                conflict_reason,
                last_error,
            });
        }
    }
    tuples.sort_by(|a, b| a.child.cmp(&b.child).then(a.skill.cmp(&b.skill)));
    Ok(tuples)
}

fn evaluate_sandbox_state(
    metadata: &SandboxMetadata,
    scope: &str,
    projection_path: &Path,
    source_path: &Path,
) -> (String, Option<String>, Option<String>) {
    match metadata.scenario.as_str() {
        "project-empty" if scope == "project" => ("absent".to_string(), None, None),
        "project-installed" if scope == "project" => {
            state_from_projection(projection_path, source_path)
        }
        "project-stale" if scope == "project" => ("stale".to_string(), None, None),
        "project-conflicted" if scope == "project" => {
            if projection_path.exists() {
                (
                    "conflicted".to_string(),
                    Some("projection_collision".to_string()),
                    None,
                )
            } else {
                ("absent".to_string(), None, None)
            }
        }
        "global-installed" if scope == "global" => {
            state_from_projection(projection_path, source_path)
        }
        "mixed-all" => ("stale".to_string(), None, None),
        _ => ("absent".to_string(), None, None),
    }
}

fn state_from_projection(
    projection_path: &Path,
    source_path: &Path,
) -> (String, Option<String>, Option<String>) {
    if !projection_path.exists() {
        return (
            "stale".to_string(),
            None,
            Some("projected_skill_missing".to_string()),
        );
    }
    match (fs::read(projection_path), fs::read(source_path)) {
        (Ok(projected), Ok(source)) if projected == source => ("installed".to_string(), None, None),
        (Ok(_), Ok(_)) => ("stale".to_string(), None, None),
        (Err(error), _) => (
            "error".to_string(),
            None,
            Some(format!("failed_to_read_projection: {error}")),
        ),
        (_, Err(error)) => (
            "blocked".to_string(),
            None,
            Some(format!("failed_to_read_source: {error}")),
        ),
    }
}

fn projection_root(metadata: &SandboxMetadata, hitl: &str, scope: &str) -> PathBuf {
    match (hitl, scope) {
        ("pi", "project") => metadata.project_root.join(".pi/skills"),
        ("pi", "global") => metadata.home_root.join(".pi/agent/skills"),
        ("claude", "project") => metadata.project_root.join(".claude/skills"),
        ("claude", "global") => metadata.home_root.join(".claude/skills"),
        ("opencode", "project") => metadata.project_root.join(".opencode/skills"),
        ("opencode", "global") => metadata.home_root.join(".config/opencode/skills"),
        ("gemini", "project") => metadata.project_root.join(".gemini/skills"),
        ("gemini", "global") => metadata.home_root.join(".gemini/skills"),
        (_, "global") => metadata.home_root.join(".agents/skills"),
        _ => metadata.project_root.join(".agents/skills"),
    }
}

fn projection_path(root: &Path, hitl: &str, child: &str, skill: &str) -> PathBuf {
    match hitl {
        // Fixture policy: Claude/Gemini/OpenCode can use nested skill dirs. PI follows its
        // own managed root but still accepts nested dirs in the harness.
        "pi" | "claude" | "opencode" | "gemini" => root.join(child).join(skill).join("SKILL.md"),
        _ => root.join(child).join(skill).join("SKILL.md"),
    }
}

fn install_action_for_tuple(tuple: &SkillTupleStatus) -> Option<SkillSyncAction> {
    match tuple.state.as_str() {
        "absent" => Some(SkillSyncAction {
            child: tuple.child.clone(),
            skill: tuple.skill.clone(),
            state: tuple.state.clone(),
            action: "install".to_string(),
            reason: "absent".to_string(),
            safe_to_apply: true,
            requires_force: false,
            writes: vec![tuple.projection_path.clone()],
            removes: Vec::new(),
        }),
        "stale" => Some(SkillSyncAction {
            child: tuple.child.clone(),
            skill: tuple.skill.clone(),
            state: tuple.state.clone(),
            action: "install".to_string(),
            reason: "stale_projection".to_string(),
            safe_to_apply: true,
            requires_force: false,
            writes: vec![tuple.projection_path.clone()],
            removes: Vec::new(),
        }),
        "conflicted" => Some(SkillSyncAction {
            child: tuple.child.clone(),
            skill: tuple.skill.clone(),
            state: tuple.state.clone(),
            action: "install".to_string(),
            reason: tuple
                .conflict_reason
                .clone()
                .unwrap_or_else(|| "conflicted".to_string()),
            safe_to_apply: false,
            requires_force: true,
            writes: vec![tuple.projection_path.clone()],
            removes: Vec::new(),
        }),
        _ => None,
    }
}

fn uninstall_action_for_tuple(tuple: &SkillTupleStatus) -> Option<SkillSyncAction> {
    match tuple.state.as_str() {
        "installed" | "stale" => Some(SkillSyncAction {
            child: tuple.child.clone(),
            skill: tuple.skill.clone(),
            state: tuple.state.clone(),
            action: "uninstall".to_string(),
            reason: tuple.state.clone(),
            safe_to_apply: true,
            requires_force: false,
            writes: Vec::new(),
            removes: vec![tuple.projection_path.clone()],
        }),
        "conflicted" if tuple.projection_path.exists() => Some(SkillSyncAction {
            child: tuple.child.clone(),
            skill: tuple.skill.clone(),
            state: tuple.state.clone(),
            action: "uninstall".to_string(),
            reason: tuple
                .conflict_reason
                .clone()
                .unwrap_or_else(|| "conflicted".to_string()),
            safe_to_apply: false,
            requires_force: true,
            writes: Vec::new(),
            removes: vec![tuple.projection_path.clone()],
        }),
        _ => None,
    }
}

fn sync_action_for_tuple(tuple: &SkillTupleStatus) -> Option<SkillSyncAction> {
    match tuple.state.as_str() {
        "stale" => Some(SkillSyncAction {
            child: tuple.child.clone(),
            skill: tuple.skill.clone(),
            state: tuple.state.clone(),
            action: "sync".to_string(),
            reason: "stale".to_string(),
            safe_to_apply: true,
            requires_force: false,
            writes: vec![tuple.projection_path.clone()],
            removes: Vec::new(),
        }),
        "conflicted" => Some(SkillSyncAction {
            child: tuple.child.clone(),
            skill: tuple.skill.clone(),
            state: tuple.state.clone(),
            action: "sync".to_string(),
            reason: tuple
                .conflict_reason
                .clone()
                .unwrap_or_else(|| "conflicted".to_string()),
            safe_to_apply: false,
            requires_force: true,
            writes: vec![tuple.projection_path.clone()],
            removes: Vec::new(),
        }),
        _ => None,
    }
}

fn print_human_status(response: &SkillsStatusResponse) {
    println!(
        "Mother skills status: hitl={} scope={} sandbox={}",
        response.hitl,
        response.scope,
        response.sandbox_id.as_deref().unwrap_or("none")
    );
    if response.tuples.is_empty() {
        println!("  no child skills found");
        return;
    }
    for tuple in &response.tuples {
        println!(
            "  {}/{}  {}  {}",
            tuple.child,
            tuple.skill,
            tuple.state,
            tuple.projection_path.display()
        );
    }
}

fn print_human_plan(response: &SkillsSyncPlanResponse) {
    println!(
        "Mother skills plan: schema={} hitl={} scope={} dry_run={} sandbox={}",
        response.schema,
        response.hitl,
        response.scope,
        response.dry_run,
        response.sandbox_id.as_deref().unwrap_or("none")
    );
    if response.actions.is_empty() {
        println!("  no sync actions planned");
        return;
    }
    for action in &response.actions {
        println!(
            "  {}/{}  {} reason={} safe={} force_required={}",
            action.child,
            action.skill,
            action.action,
            action.reason,
            action.safe_to_apply,
            action.requires_force
        );
    }
}
