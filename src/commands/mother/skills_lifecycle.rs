use anyhow::{Context, Result};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use uuid::Uuid;

use super::skills_sandbox::{self, SandboxMetadata};

const STATUS_SCHEMA: &str = "patina.mother.skills.status.v1";
const SYNC_PLAN_SCHEMA: &str = "patina.mother.skills.sync-plan.v1";
const INSTALL_PLAN_SCHEMA: &str = "patina.mother.skills.install-plan.v1";
const UNINSTALL_PLAN_SCHEMA: &str = "patina.mother.skills.uninstall-plan.v1";
const MANIFEST_SCHEMA: &str = "patina.mother.skills.projection-manifest.v1";
const HANDSHAKE_SCHEMA: &str = "patina.mother.skills.handshake.v2";

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
    pub manifest_path: PathBuf,
    pub managed: bool,
    pub source_sha256: String,
    pub projection_sha256: Option<String>,
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
    pub force: bool,
    pub applied: bool,
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

#[derive(Debug, Clone)]
struct SkillLifecycleContext {
    sandbox: Option<SandboxMetadata>,
    project_root: PathBuf,
    home_root: PathBuf,
    child_store_root: PathBuf,
}

#[derive(Debug, Clone)]
struct HitlCapability {
    hitl: &'static str,
    project_roots: &'static [&'static str],
    global_roots: &'static [&'static str],
}

impl HitlCapability {
    fn roots_for_scope(&self, scope: &str) -> &'static [&'static str] {
        if scope == "global" {
            self.global_roots
        } else {
            self.project_roots
        }
    }

    fn supports_scope(&self, scope: &str) -> bool {
        !self.roots_for_scope(scope).is_empty()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ProjectionManifest {
    schema: String,
    child: String,
    hitl: String,
    scope: String,
    projection_root: PathBuf,
    entries: Vec<ProjectionManifestEntry>,
    updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ProjectionManifestEntry {
    skill: String,
    source_path: PathBuf,
    projection_path: PathBuf,
    source_sha256: String,
    projection_sha256: String,
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
    force: bool,
    json: bool,
) -> Result<()> {
    let status = build_status_response(child, hitl_arg, global)?;
    let mut response =
        build_plan_response(SYNC_PLAN_SCHEMA, status, dry_run, force, |tuple, force| {
            sync_action_for_tuple(tuple, force)
        });

    if !dry_run {
        apply_plan(&response)?;
        response.applied = true;
    }

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
    force: bool,
    json: bool,
) -> Result<()> {
    let status = build_status_response(Some(child), hitl_arg, global)?;
    if status.tuples.is_empty() {
        anyhow::bail!(
            "child '{}' has no skill bundle in Mother child store",
            child
        );
    }
    let mut response = build_plan_response(
        INSTALL_PLAN_SCHEMA,
        status,
        dry_run,
        force,
        |tuple, force| install_action_for_tuple(tuple, force),
    );

    if !dry_run {
        apply_plan(&response)?;
        response.applied = true;
    }

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
    force: bool,
    json: bool,
) -> Result<()> {
    let status = build_status_response(Some(child), hitl_arg, global)?;
    if status.tuples.is_empty() {
        anyhow::bail!(
            "child '{}' has no skill bundle in Mother child store",
            child
        );
    }
    let mut response = build_plan_response(
        UNINSTALL_PLAN_SCHEMA,
        status,
        dry_run,
        force,
        |tuple, force| uninstall_action_for_tuple(tuple, force),
    );

    if !dry_run {
        apply_plan(&response)?;
        response.applied = true;
    }

    if json {
        println!("{}", serde_json::to_string_pretty(&response)?);
    } else {
        print_human_plan(&response);
    }
    Ok(())
}

pub(crate) fn handshake_v2_payload(project_root: &Path, hitl_arg: &str) -> serde_json::Value {
    let hitl = normalize_hitl(hitl_arg);
    match context_for_project(project_root) {
        Ok(context) => {
            let project = build_status_for_context(&context, None, &hitl, "project");
            let global = build_status_for_context(&context, None, &hitl, "global");
            let project_plan = project.as_ref().ok().map(|status| {
                build_plan_response(
                    SYNC_PLAN_SCHEMA,
                    status.clone(),
                    true,
                    false,
                    |tuple, force| sync_action_for_tuple(tuple, force),
                )
            });
            serde_json::json!({
                "schema": HANDSHAKE_SCHEMA,
                "hitl": hitl,
                "project": project.ok(),
                "global": global.ok(),
                "auto_sync_plan": project_plan,
            })
        }
        Err(error) => serde_json::json!({
            "schema": HANDSHAKE_SCHEMA,
            "hitl": hitl,
            "error": error.to_string(),
        }),
    }
}

fn build_plan_response(
    schema: &'static str,
    status: SkillsStatusResponse,
    dry_run: bool,
    force: bool,
    planner: fn(&SkillTupleStatus, bool) -> Option<SkillSyncAction>,
) -> SkillsSyncPlanResponse {
    let actions = status
        .tuples
        .iter()
        .filter_map(|tuple| planner(tuple, force))
        .collect::<Vec<_>>();
    SkillsSyncPlanResponse {
        schema,
        sandbox_id: status.sandbox_id,
        scenario: status.scenario,
        hitl: status.hitl,
        scope: status.scope,
        dry_run,
        force,
        applied: false,
        actions,
    }
}

fn build_status_response(
    child: Option<&str>,
    hitl_arg: Option<&str>,
    global: bool,
) -> Result<SkillsStatusResponse> {
    let cwd = std::env::current_dir().context("resolving current directory")?;
    let context = context_for_cwd(&cwd)?;
    let hitl = resolve_hitl(hitl_arg, context.sandbox.as_ref())?;
    let scope = if global { "global" } else { "project" };
    build_status_for_context(&context, child, &hitl, scope)
}

fn build_status_for_context(
    context: &SkillLifecycleContext,
    child: Option<&str>,
    hitl: &str,
    scope: &str,
) -> Result<SkillsStatusResponse> {
    let tuples = status_tuples(context, child, hitl, scope)?;
    Ok(SkillsStatusResponse {
        schema: STATUS_SCHEMA,
        sandbox_id: context.sandbox.as_ref().map(|s| s.id.clone()),
        scenario: context.sandbox.as_ref().map(|s| s.scenario.clone()),
        hitl: hitl.to_string(),
        scope: scope.to_string(),
        tuples,
    })
}

fn context_for_cwd(cwd: &Path) -> Result<SkillLifecycleContext> {
    if let Some(metadata) = skills_sandbox::find_enclosing_sandbox(cwd)? {
        return Ok(SkillLifecycleContext {
            project_root: metadata.project_root.clone(),
            home_root: metadata.home_root.clone(),
            child_store_root: metadata.child_store_root.clone(),
            sandbox: Some(metadata),
        });
    }

    context_for_project(&resolve_project_root(cwd)?)
}

fn context_for_project(project_root: &Path) -> Result<SkillLifecycleContext> {
    Ok(SkillLifecycleContext {
        sandbox: None,
        project_root: project_root.to_path_buf(),
        home_root: dirs::home_dir().unwrap_or_else(|| PathBuf::from(".")),
        child_store_root: patina::paths::child::command_children_dir(),
    })
}

fn resolve_project_root(cwd: &Path) -> Result<PathBuf> {
    if let Ok(root) = patina::session::SessionManager::find_project_root() {
        return Ok(root);
    }
    Ok(cwd.to_path_buf())
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
    match hitl.trim() {
        "open-code" | "open_code" | "OpenCode" => "opencode".to_string(),
        other => other.to_ascii_lowercase(),
    }
}

fn hitl_capability(hitl: &str) -> Option<HitlCapability> {
    match hitl {
        "pi" => Some(HitlCapability {
            hitl: "pi",
            project_roots: &[".pi/skills", ".agents/skills"],
            global_roots: &["~/.pi/agent/skills", "~/.agents/skills"],
        }),
        "claude" => Some(HitlCapability {
            hitl: "claude",
            project_roots: &[".claude/skills"],
            global_roots: &["~/.claude/skills"],
        }),
        "opencode" => Some(HitlCapability {
            hitl: "opencode",
            project_roots: &[".opencode/skills", ".agents/skills"],
            global_roots: &["~/.config/opencode/skills", "~/.opencode/skills"],
        }),
        "gemini" => Some(HitlCapability {
            hitl: "gemini",
            project_roots: &[".gemini/skills", ".agents/skills"],
            global_roots: &["~/.gemini/skills", "~/.agents/skills"],
        }),
        _ => None,
    }
}

fn status_tuples(
    context: &SkillLifecycleContext,
    child_filter: Option<&str>,
    hitl: &str,
    scope: &str,
) -> Result<Vec<SkillTupleStatus>> {
    let mut tuples = Vec::new();
    if !context.child_store_root.exists() {
        return Ok(tuples);
    }

    let capability = hitl_capability(hitl);
    let supported = capability
        .as_ref()
        .map(|capability| capability.supports_scope(scope))
        .unwrap_or(false);
    let projection_root = capability
        .as_ref()
        .and_then(|capability| primary_projection_root(context, capability, scope))
        .unwrap_or_else(|| fallback_projection_root(context, scope));

    for entry in fs::read_dir(&context.child_store_root)? {
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
        let manifest_path = manifest_path(&context.project_root, hitl, scope, &child_name);
        let manifest = read_manifest(&manifest_path)?;
        let manifest_entries = manifest_entry_map(manifest.as_ref());

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
            let source_sha256 = file_sha256(&source_path)?;
            let projection_path = projection_path(&projection_root, hitl, &child_name, &skill_name);
            let manifest_entry = manifest_entries.get(skill_name.as_str());
            let (state, projection_sha256, conflict_reason, last_error, managed) = if !supported {
                (
                    "unsupported".to_string(),
                    None,
                    None,
                    Some(format!("HITL '{}' does not support {} scope", hitl, scope)),
                    manifest_entry.is_some(),
                )
            } else {
                evaluate_state(&projection_path, &source_sha256, manifest_entry)?
            };
            tuples.push(SkillTupleStatus {
                child: child_name.clone(),
                skill: skill_name,
                hitl: capability
                    .as_ref()
                    .map(|capability| capability.hitl.to_string())
                    .unwrap_or_else(|| hitl.to_string()),
                scope: scope.to_string(),
                state,
                projection_root: projection_root.clone(),
                projection_path,
                source_path,
                manifest_path: manifest_path.clone(),
                managed,
                source_sha256,
                projection_sha256,
                conflict_reason,
                last_error,
            });
        }
    }
    tuples.sort_by(|a, b| a.child.cmp(&b.child).then(a.skill.cmp(&b.skill)));
    Ok(tuples)
}

fn evaluate_state(
    projection_path: &Path,
    source_sha256: &str,
    manifest_entry: Option<&&ProjectionManifestEntry>,
) -> Result<(String, Option<String>, Option<String>, Option<String>, bool)> {
    let managed = manifest_entry.is_some();
    if !projection_path.exists() {
        return Ok(if managed {
            (
                "stale".to_string(),
                None,
                None,
                Some("projected_skill_missing".to_string()),
                true,
            )
        } else {
            ("absent".to_string(), None, None, None, false)
        });
    }

    let projection_sha256 = file_sha256(projection_path)?;
    match manifest_entry {
        Some(entry) if projection_sha256 != entry.projection_sha256 => Ok((
            "conflicted".to_string(),
            Some(projection_sha256),
            Some("managed_projection_modified".to_string()),
            None,
            true,
        )),
        Some(entry) if source_sha256 != entry.source_sha256 => Ok((
            "stale".to_string(),
            Some(projection_sha256),
            None,
            None,
            true,
        )),
        Some(_) => Ok((
            "installed".to_string(),
            Some(projection_sha256),
            None,
            None,
            true,
        )),
        None if projection_sha256 == source_sha256 => Ok((
            "installed".to_string(),
            Some(projection_sha256),
            None,
            Some("projection_matches_source_without_manifest".to_string()),
            false,
        )),
        None => Ok((
            "conflicted".to_string(),
            Some(projection_sha256),
            Some("projection_collision".to_string()),
            None,
            false,
        )),
    }
}

fn primary_projection_root(
    context: &SkillLifecycleContext,
    capability: &HitlCapability,
    scope: &str,
) -> Option<PathBuf> {
    capability
        .roots_for_scope(scope)
        .first()
        .map(|root| expand_hitl_root(context, scope, root))
}

fn fallback_projection_root(context: &SkillLifecycleContext, scope: &str) -> PathBuf {
    if scope == "global" {
        context.home_root.join(".agents/skills")
    } else {
        context.project_root.join(".agents/skills")
    }
}

fn expand_hitl_root(context: &SkillLifecycleContext, scope: &str, root: &str) -> PathBuf {
    if let Some(stripped) = root.strip_prefix("~/") {
        context.home_root.join(stripped)
    } else if scope == "global" {
        context.home_root.join(root)
    } else {
        context.project_root.join(root)
    }
}

fn projection_path(root: &Path, _hitl: &str, child: &str, skill: &str) -> PathBuf {
    root.join(child).join(skill).join("SKILL.md")
}

fn manifest_path(project_root: &Path, hitl: &str, scope: &str, child: &str) -> PathBuf {
    project_root
        .join(".patina/local/mother/skills")
        .join(hitl)
        .join(scope)
        .join(format!("{child}.json"))
}

fn read_manifest(path: &Path) -> Result<Option<ProjectionManifest>> {
    if !path.exists() {
        return Ok(None);
    }
    let text = fs::read_to_string(path)
        .with_context(|| format!("reading projection manifest {}", path.display()))?;
    let manifest: ProjectionManifest = serde_json::from_str(&text)
        .with_context(|| format!("parsing projection manifest {}", path.display()))?;
    Ok(Some(manifest))
}

fn manifest_entry_map(
    manifest: Option<&ProjectionManifest>,
) -> HashMap<&str, &ProjectionManifestEntry> {
    let mut map = HashMap::new();
    if let Some(manifest) = manifest {
        for entry in &manifest.entries {
            map.insert(entry.skill.as_str(), entry);
        }
    }
    map
}

fn install_action_for_tuple(tuple: &SkillTupleStatus, force: bool) -> Option<SkillSyncAction> {
    match tuple.state.as_str() {
        "absent" => Some(write_action(tuple, "install", "absent", false, true)),
        "stale" => Some(write_action(
            tuple,
            "install",
            "stale_projection",
            false,
            true,
        )),
        "conflicted" => Some(write_action(
            tuple,
            "install",
            tuple.conflict_reason.as_deref().unwrap_or("conflicted"),
            true,
            force,
        )),
        _ => None,
    }
}

fn uninstall_action_for_tuple(tuple: &SkillTupleStatus, force: bool) -> Option<SkillSyncAction> {
    match tuple.state.as_str() {
        "installed" | "stale" if tuple.managed => {
            Some(remove_action(tuple, "uninstall", &tuple.state, false, true))
        }
        "conflicted" if tuple.managed && tuple.projection_path.exists() => Some(remove_action(
            tuple,
            "uninstall",
            tuple.conflict_reason.as_deref().unwrap_or("conflicted"),
            true,
            force,
        )),
        _ => None,
    }
}

fn sync_action_for_tuple(tuple: &SkillTupleStatus, force: bool) -> Option<SkillSyncAction> {
    match tuple.state.as_str() {
        "stale" => Some(write_action(tuple, "sync", "stale", false, true)),
        "conflicted" => Some(write_action(
            tuple,
            "sync",
            tuple.conflict_reason.as_deref().unwrap_or("conflicted"),
            true,
            force,
        )),
        _ => None,
    }
}

fn write_action(
    tuple: &SkillTupleStatus,
    action: &str,
    reason: &str,
    requires_force: bool,
    safe_to_apply: bool,
) -> SkillSyncAction {
    SkillSyncAction {
        child: tuple.child.clone(),
        skill: tuple.skill.clone(),
        state: tuple.state.clone(),
        action: action.to_string(),
        reason: reason.to_string(),
        safe_to_apply,
        requires_force,
        writes: vec![tuple.projection_path.clone()],
        removes: Vec::new(),
    }
}

fn remove_action(
    tuple: &SkillTupleStatus,
    action: &str,
    reason: &str,
    requires_force: bool,
    safe_to_apply: bool,
) -> SkillSyncAction {
    SkillSyncAction {
        child: tuple.child.clone(),
        skill: tuple.skill.clone(),
        state: tuple.state.clone(),
        action: action.to_string(),
        reason: reason.to_string(),
        safe_to_apply,
        requires_force,
        writes: Vec::new(),
        removes: vec![tuple.projection_path.clone()],
    }
}

fn apply_plan(response: &SkillsSyncPlanResponse) -> Result<()> {
    if response
        .actions
        .iter()
        .any(|action| action.requires_force && !response.force)
    {
        anyhow::bail!(
            "{} has force-required actions; rerun with --force after reviewing dry-run output",
            response.schema
        );
    }
    if response
        .actions
        .iter()
        .any(|action| !action.safe_to_apply && !response.force)
    {
        anyhow::bail!("{} contains unsafe actions", response.schema);
    }

    let context = context_for_cwd(&std::env::current_dir()?)?;
    let scope = response.scope.as_str();
    let hitl = response.hitl.as_str();

    let mut affected_children = response
        .actions
        .iter()
        .map(|action| action.child.clone())
        .collect::<Vec<_>>();
    affected_children.sort();
    affected_children.dedup();

    if response.schema == UNINSTALL_PLAN_SCHEMA {
        apply_removes(response)?;
        for child in &affected_children {
            let manifest_path = manifest_path(&context.project_root, hitl, scope, child);
            if manifest_path.exists() {
                fs::remove_file(&manifest_path)
                    .with_context(|| format!("removing manifest {}", manifest_path.display()))?;
            }
            prune_empty_dirs(manifest_path.parent());
        }
    } else {
        stage_and_apply_writes(response)?;
        for child in &affected_children {
            write_manifest_for_child(&context, hitl, scope, child)?;
        }
    }

    emit_projection_audit(&context, response)?;
    Ok(())
}

fn stage_and_apply_writes(response: &SkillsSyncPlanResponse) -> Result<()> {
    let mut staged = Vec::new();
    for action in &response.actions {
        for projection_path in &action.writes {
            let Some(parent) = projection_path.parent() else {
                anyhow::bail!(
                    "projection path has no parent: {}",
                    projection_path.display()
                );
            };
            fs::create_dir_all(parent)
                .with_context(|| format!("creating projection dir {}", parent.display()))?;
            let stage_path = parent.join(format!(
                ".{}.{}.tmp",
                projection_path
                    .file_name()
                    .and_then(|name| name.to_str())
                    .unwrap_or("SKILL.md"),
                Uuid::new_v4().simple()
            ));
            fs::copy(source_path_for_action(response, action)?, &stage_path).with_context(
                || {
                    format!(
                        "staging projection {} -> {}",
                        action.skill,
                        stage_path.display()
                    )
                },
            )?;
            staged.push((stage_path, projection_path.clone()));
        }
    }

    for (stage_path, projection_path) in staged {
        atomic_replace(&stage_path, &projection_path)?;
    }
    Ok(())
}

fn source_path_for_action(
    response: &SkillsSyncPlanResponse,
    action: &SkillSyncAction,
) -> Result<PathBuf> {
    let status = build_status_response(
        Some(&action.child),
        Some(&response.hitl),
        response.scope == "global",
    )?;
    status
        .tuples
        .into_iter()
        .find(|tuple| tuple.skill == action.skill)
        .map(|tuple| tuple.source_path)
        .ok_or_else(|| {
            anyhow::anyhow!(
                "source skill not found for {}/{}",
                action.child,
                action.skill
            )
        })
}

fn apply_removes(response: &SkillsSyncPlanResponse) -> Result<()> {
    for action in &response.actions {
        for projection_path in &action.removes {
            if projection_path.exists() {
                fs::remove_file(projection_path).with_context(|| {
                    format!("removing projection {}", projection_path.display())
                })?;
            }
            prune_empty_dirs(projection_path.parent());
        }
    }
    Ok(())
}

fn write_manifest_for_child(
    context: &SkillLifecycleContext,
    hitl: &str,
    scope: &str,
    child: &str,
) -> Result<()> {
    let status = build_status_for_context(context, Some(child), hitl, scope)?;
    let mut entries = Vec::new();
    let projection_root = status
        .tuples
        .first()
        .map(|tuple| tuple.projection_root.clone())
        .unwrap_or_else(|| fallback_projection_root(context, scope));

    for tuple in &status.tuples {
        if tuple.projection_path.exists() {
            entries.push(ProjectionManifestEntry {
                skill: tuple.skill.clone(),
                source_path: tuple.source_path.clone(),
                projection_path: tuple.projection_path.clone(),
                source_sha256: file_sha256(&tuple.source_path)?,
                projection_sha256: file_sha256(&tuple.projection_path)?,
            });
        }
    }

    let manifest = ProjectionManifest {
        schema: MANIFEST_SCHEMA.to_string(),
        child: child.to_string(),
        hitl: hitl.to_string(),
        scope: scope.to_string(),
        projection_root,
        entries,
        updated_at: Utc::now().to_rfc3339(),
    };
    let path = manifest_path(&context.project_root, hitl, scope, child);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let text = serde_json::to_string_pretty(&manifest)?;
    write_atomic(&path, text.as_bytes())?;
    Ok(())
}

fn atomic_replace(from: &Path, to: &Path) -> Result<()> {
    match fs::rename(from, to) {
        Ok(()) => Ok(()),
        Err(_) => {
            let bytes = fs::read(from)?;
            fs::write(to, bytes)?;
            let _ = fs::remove_file(from);
            Ok(())
        }
    }
}

fn write_atomic(path: &Path, bytes: &[u8]) -> Result<()> {
    let parent = path
        .parent()
        .ok_or_else(|| anyhow::anyhow!("path has no parent: {}", path.display()))?;
    fs::create_dir_all(parent)?;
    let tmp = parent.join(format!(
        ".{}.{}.tmp",
        path.file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("manifest.json"),
        Uuid::new_v4().simple()
    ));
    fs::write(&tmp, bytes)?;
    atomic_replace(&tmp, path)
}

fn prune_empty_dirs(mut path: Option<&Path>) {
    while let Some(dir) = path {
        if fs::remove_dir(dir).is_err() {
            break;
        }
        path = dir.parent();
    }
}

fn file_sha256(path: &Path) -> Result<String> {
    let bytes = fs::read(path).with_context(|| format!("reading {}", path.display()))?;
    Ok(sha256_hex(&bytes))
}

fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    let digest = hasher.finalize();
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn emit_projection_audit(
    context: &SkillLifecycleContext,
    response: &SkillsSyncPlanResponse,
) -> Result<()> {
    let conn = patina::eventlog::open_events_db_at(&context.project_root)?;
    let source_id = format!(
        "mother.skills.{}.{}.{}",
        response.hitl,
        response.scope,
        Uuid::new_v4().simple()
    );
    let event_type = match response.schema {
        INSTALL_PLAN_SCHEMA => "mother.skills.projection.install",
        SYNC_PLAN_SCHEMA => "mother.skills.projection.sync",
        UNINSTALL_PLAN_SCHEMA => "mother.skills.projection.uninstall",
        _ => "mother.skills.projection.plan",
    };
    patina::eventlog::insert_event(
        &conn,
        event_type,
        &Utc::now().to_rfc3339(),
        &source_id,
        None,
        &serde_json::to_string(response)?,
    )?;
    Ok(())
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
            "  {}/{}  {}  managed={}  {}",
            tuple.child,
            tuple.skill,
            tuple.state,
            tuple.managed,
            tuple.projection_path.display()
        );
    }
}

fn print_human_plan(response: &SkillsSyncPlanResponse) {
    println!(
        "Mother skills plan: schema={} hitl={} scope={} dry_run={} force={} applied={} sandbox={}",
        response.schema,
        response.hitl,
        response.scope,
        response.dry_run,
        response.force,
        response.applied,
        response.sandbox_id.as_deref().unwrap_or("none")
    );
    if response.actions.is_empty() {
        println!("  no actions planned");
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
