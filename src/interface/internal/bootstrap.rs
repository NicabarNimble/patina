use anyhow::{Context, Result};
use chrono::{DateTime, Local};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

use crate::interface::internal::bundle::{interface_bundle, InterfaceBundle, ManagedPathKind};
use crate::interface::runtime::{launch, templates};
use crate::project;

const MANAGED_DIR_METADATA_FILE: &str = ".patina-managed.toml";
const BACKUP_SUBDIR: &str = "interface";
const SUFFIX_ALPHABET: &[u8; 32] = b"ABCDEFGHJKLMNPQRSTUVWXYZ23456789";

#[derive(Debug, Clone)]
pub struct BootstrapResult {
    pub bootstrap_path: PathBuf,
    pub context_path: PathBuf,
    pub backup_snapshot: Option<PathBuf>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProjectionMode {
    CreateMissing,
    RefreshManaged,
    ForceRewrite,
}

#[derive(Debug, Clone)]
pub struct BundleDeploymentStatus {
    pub bundle_name: String,
    pub display_name: String,
    pub deployed: bool,
    pub current: bool,
    pub expected_version: String,
    pub observed_version: Option<String>,
    pub missing_paths: Vec<String>,
    pub unmanaged_paths: Vec<String>,
    pub stale_paths: Vec<String>,
}

#[derive(Debug, Clone, Copy)]
enum ExistingPathKind {
    File,
    Directory,
}

#[derive(Debug, Clone)]
struct BackupEntry {
    absolute_path: PathBuf,
    relative_path: PathBuf,
    kind: ExistingPathKind,
}

#[derive(Debug, Clone)]
struct SurfaceReconciliation {
    backup_snapshot: Option<PathBuf>,
}

#[derive(Debug, Serialize, Deserialize)]
struct ManagedDirectoryMetadata {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    adapter: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    bundle: Option<String>,
    version: String,
    managed_by: String,
}

impl ManagedDirectoryMetadata {
    fn owner_name(&self) -> Option<&str> {
        self.bundle.as_deref().or(self.adapter.as_deref())
    }
}

#[derive(Debug, Serialize)]
struct BackupManifest {
    adapter: String,
    mode: String,
    created_at: String,
    paths: Vec<String>,
}

pub fn ensure_bundle_bootstrap(bundle_name: &str, project_root: &Path) -> Result<BootstrapResult> {
    let status = bundle_deployment_status(bundle_name, project_root)?;
    if status.current {
        return bootstrap_result(bundle_name, project_root, None);
    }

    ensure_bundle_projection(bundle_name, project_root, ProjectionMode::RefreshManaged)
}

pub fn ensure_adapter_bootstrap(
    adapter_name: &str,
    project_root: &Path,
) -> Result<BootstrapResult> {
    ensure_bundle_bootstrap(adapter_name, project_root)
}

pub fn ensure_bundle_projection(
    bundle_name: &str,
    project_root: &Path,
    mode: ProjectionMode,
) -> Result<BootstrapResult> {
    let bundle = interface_bundle(bundle_name)?;
    let reconciliation = reconcile_interface_surface(bundle, project_root, mode)?;
    sync_managed_templates(bundle.name, project_root, mode)?;
    write_managed_directory_metadata(project_root, bundle)?;
    launch::generate_bootstrap(
        bundle.name,
        project_root,
        mode == ProjectionMode::ForceRewrite,
    )?;

    bootstrap_result(bundle.name, project_root, reconciliation.backup_snapshot)
}

pub fn ensure_adapter_projection(
    adapter_name: &str,
    project_root: &Path,
    mode: ProjectionMode,
) -> Result<BootstrapResult> {
    ensure_bundle_projection(adapter_name, project_root, mode)
}

pub fn bundle_deployment_status(
    bundle_name: &str,
    project_root: &Path,
) -> Result<BundleDeploymentStatus> {
    let bundle = interface_bundle(bundle_name)?;
    let mut missing_paths = Vec::new();
    let mut unmanaged_paths = Vec::new();
    let mut stale_paths = Vec::new();
    let mut observed_version = None;

    for path in bundle.managed_paths {
        let absolute_path = project_root.join(path.relative_path);
        if !absolute_path.exists() {
            missing_paths.push(path.relative_path.to_string());
            continue;
        }

        match path.kind {
            ManagedPathKind::File => {
                if !file_is_patina_managed(&absolute_path)? {
                    unmanaged_paths.push(path.relative_path.to_string());
                }
            }
            ManagedPathKind::Directory => match read_managed_directory_metadata(&absolute_path)? {
                Some(metadata) => {
                    let owner_matches = metadata
                        .owner_name()
                        .map(|owner| owner == bundle.name)
                        .unwrap_or(false);
                    if !owner_matches {
                        unmanaged_paths.push(path.relative_path.to_string());
                    } else if metadata.version != bundle.version {
                        observed_version = Some(metadata.version.clone());
                        stale_paths.push(path.relative_path.to_string());
                    }
                }
                None => unmanaged_paths.push(path.relative_path.to_string()),
            },
        }
    }

    let deployed = missing_paths.is_empty() && unmanaged_paths.is_empty();
    let current = deployed && stale_paths.is_empty();

    Ok(BundleDeploymentStatus {
        bundle_name: bundle.name.to_string(),
        display_name: bundle.display_name.to_string(),
        deployed,
        current,
        expected_version: bundle.version.to_string(),
        observed_version,
        missing_paths,
        unmanaged_paths,
        stale_paths,
    })
}

fn sync_managed_templates(
    adapter_name: &str,
    project_root: &Path,
    mode: ProjectionMode,
) -> Result<()> {
    let adapter_dir = project_root.join(format!(".{}", adapter_name));
    if mode == ProjectionMode::CreateMissing && adapter_dir.exists() {
        return Ok(());
    }

    templates::copy_to_project(adapter_name, project_root)
}

fn bootstrap_result(
    bundle_name: &str,
    project_root: &Path,
    backup_snapshot: Option<PathBuf>,
) -> Result<BootstrapResult> {
    let context_path = launch::canonical_agents_path(project_root);
    let bootstrap_path = launch::vendor_bootstrap_path(bundle_name, project_root)?
        .unwrap_or_else(|| context_path.clone());

    Ok(BootstrapResult {
        bootstrap_path,
        context_path,
        backup_snapshot,
    })
}

fn reconcile_interface_surface(
    bundle: &InterfaceBundle,
    project_root: &Path,
    mode: ProjectionMode,
) -> Result<SurfaceReconciliation> {
    if mode == ProjectionMode::CreateMissing {
        return Ok(SurfaceReconciliation {
            backup_snapshot: None,
        });
    }

    let mut entries = Vec::new();

    for path in bundle.managed_paths {
        let absolute_path = project_root.join(path.relative_path);
        if !absolute_path.exists() {
            continue;
        }

        let managed = match path.kind {
            ManagedPathKind::File => file_is_patina_managed(&absolute_path)?,
            ManagedPathKind::Directory => directory_is_patina_managed(&absolute_path, bundle)?,
        };

        if mode == ProjectionMode::ForceRewrite || !managed {
            let kind = existing_path_kind(&absolute_path)?;
            entries.push(BackupEntry {
                absolute_path,
                relative_path: PathBuf::from(path.relative_path),
                kind,
            });
        }
    }

    let backup_snapshot = if entries.is_empty() {
        None
    } else {
        Some(write_backup_snapshot(
            project_root,
            bundle.name,
            mode,
            &entries,
        )?)
    };

    for entry in entries {
        remove_existing_path(&entry.absolute_path, entry.kind)?;
    }

    Ok(SurfaceReconciliation { backup_snapshot })
}

fn file_is_patina_managed(path: &Path) -> Result<bool> {
    if !path.is_file() {
        return Ok(false);
    }

    let content =
        fs::read_to_string(path).with_context(|| format!("Failed to read {}", path.display()))?;
    Ok(content.contains("<!-- PATINA:START -->") && content.contains("<!-- PATINA:END -->"))
}

fn read_managed_directory_metadata(path: &Path) -> Result<Option<ManagedDirectoryMetadata>> {
    if !path.is_dir() {
        return Ok(None);
    }

    let metadata_path = path.join(MANAGED_DIR_METADATA_FILE);
    if !metadata_path.exists() {
        return Ok(None);
    }

    let content = fs::read_to_string(&metadata_path)
        .with_context(|| format!("Failed to read {}", metadata_path.display()))?;
    let metadata: ManagedDirectoryMetadata = toml::from_str(&content)
        .with_context(|| format!("Failed to parse {}", metadata_path.display()))?;
    Ok(Some(metadata))
}

fn directory_is_patina_managed(path: &Path, bundle: &InterfaceBundle) -> Result<bool> {
    let Some(metadata) = read_managed_directory_metadata(path)? else {
        return Ok(false);
    };
    Ok(metadata
        .owner_name()
        .map(|owner| owner == bundle.name)
        .unwrap_or(false))
}

fn write_managed_directory_metadata(project_root: &Path, bundle: &InterfaceBundle) -> Result<()> {
    let Some(directory) = bundle
        .managed_paths
        .iter()
        .find(|path| path.kind == ManagedPathKind::Directory)
    else {
        return Ok(());
    };

    let path = project_root.join(directory.relative_path);
    fs::create_dir_all(&path)?;
    let metadata_path = path.join(MANAGED_DIR_METADATA_FILE);
    let metadata = ManagedDirectoryMetadata {
        adapter: Some(bundle.name.to_string()),
        bundle: Some(bundle.name.to_string()),
        version: bundle.version.to_string(),
        managed_by: "patina ai refresh".to_string(),
    };
    fs::write(&metadata_path, toml::to_string_pretty(&metadata)?)
        .with_context(|| format!("Failed to write {}", metadata_path.display()))?;
    Ok(())
}

fn existing_path_kind(path: &Path) -> Result<ExistingPathKind> {
    let metadata =
        fs::metadata(path).with_context(|| format!("Failed to stat {}", path.display()))?;
    if metadata.is_dir() {
        Ok(ExistingPathKind::Directory)
    } else {
        Ok(ExistingPathKind::File)
    }
}

fn remove_existing_path(path: &Path, kind: ExistingPathKind) -> Result<()> {
    match kind {
        ExistingPathKind::File => {
            fs::remove_file(path).with_context(|| format!("Failed to remove {}", path.display()))?
        }
        ExistingPathKind::Directory => fs::remove_dir_all(path)
            .with_context(|| format!("Failed to remove {}", path.display()))?,
    }
    Ok(())
}

fn write_backup_snapshot(
    project_root: &Path,
    adapter_name: &str,
    mode: ProjectionMode,
    entries: &[BackupEntry],
) -> Result<PathBuf> {
    let backup_dir = project::backups_dir(project_root).join(BACKUP_SUBDIR);
    fs::create_dir_all(&backup_dir)
        .with_context(|| format!("Failed to create {}", backup_dir.display()))?;

    let backup_id = generate_backup_id(Local::now());
    let snapshot_path = backup_dir.join(format!("patina-setup-backup-{}", backup_id));
    fs::create_dir_all(&snapshot_path)
        .with_context(|| format!("Failed to create {}", snapshot_path.display()))?;

    let manifest_path = snapshot_path.join("manifest.toml");
    let manifest = BackupManifest {
        adapter: adapter_name.to_string(),
        mode: projection_mode_name(mode).to_string(),
        created_at: Local::now().to_rfc3339(),
        paths: entries
            .iter()
            .map(|entry| normalize_snapshot_path(&entry.relative_path))
            .collect(),
    };
    fs::write(&manifest_path, toml::to_string_pretty(&manifest)?)
        .with_context(|| format!("Failed to write {}", manifest_path.display()))?;

    for entry in entries {
        let dest_path = snapshot_path.join(&entry.relative_path);
        match entry.kind {
            ExistingPathKind::File => {
                if let Some(parent) = dest_path.parent() {
                    fs::create_dir_all(parent)
                        .with_context(|| format!("Failed to create {}", parent.display()))?;
                }
                fs::copy(&entry.absolute_path, &dest_path).with_context(|| {
                    format!(
                        "Failed to copy {} to {}",
                        entry.absolute_path.display(),
                        dest_path.display()
                    )
                })?;
            }
            ExistingPathKind::Directory => {
                copy_dir_recursive(&entry.absolute_path, &dest_path)?;
            }
        }
    }

    Ok(snapshot_path)
}

fn copy_dir_recursive(src: &Path, dest: &Path) -> Result<()> {
    fs::create_dir_all(dest).with_context(|| format!("Failed to create {}", dest.display()))?;

    for entry in WalkDir::new(src) {
        let entry = entry?;
        let path = entry.path();
        let relative = path
            .strip_prefix(src)
            .with_context(|| format!("Failed to strip prefix for {}", path.display()))?;
        if relative.as_os_str().is_empty() {
            continue;
        }
        let dest_path = dest.join(relative);

        if entry.file_type().is_dir() {
            fs::create_dir_all(&dest_path)
                .with_context(|| format!("Failed to create {}", dest_path.display()))?;
            continue;
        }

        if let Some(parent) = dest_path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create {}", parent.display()))?;
        }
        fs::copy(path, &dest_path).with_context(|| {
            format!(
                "Failed to copy {} to {}",
                path.display(),
                dest_path.display()
            )
        })?;
    }

    Ok(())
}

fn normalize_snapshot_path(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

fn generate_backup_id(now: DateTime<Local>) -> String {
    let mut value = fastrand::u32(..) & 0x000f_ffff;
    let mut suffix = ['A'; 4];
    for idx in (0..4).rev() {
        let alphabet_idx = (value & 0b1_1111) as usize;
        suffix[idx] = SUFFIX_ALPHABET[alphabet_idx] as char;
        value >>= 5;
    }

    format!(
        "{}-{}",
        now.format("%Y%m%d-%H%M%S"),
        suffix.iter().collect::<String>()
    )
}

fn projection_mode_name(mode: ProjectionMode) -> &'static str {
    match mode {
        ProjectionMode::CreateMissing => "create-missing",
        ProjectionMode::RefreshManaged => "refresh-managed",
        ProjectionMode::ForceRewrite => "force-rewrite",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::project::{self, ProjectConfig};
    use chrono::TimeZone;
    use tempfile::TempDir;

    fn setup_project(adapter_name: &str) -> TempDir {
        let temp = TempDir::new().unwrap();
        let mut config = ProjectConfig::with_name("patina");
        config.adapters.allowed = vec!["claude".to_string(), adapter_name.to_string()];
        config.adapters.default = adapter_name.to_string();
        project::save(temp.path(), &config).unwrap();
        temp
    }

    fn with_temp_patina_home<T>(temp: &TempDir, f: impl FnOnce() -> T) -> T {
        let _guard = crate::test_support::env_test_mutex()
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        let patina_home = temp.path().join("patina-home");
        std::fs::create_dir_all(&patina_home).unwrap();

        let old = std::env::var_os("PATINA_HOME");
        let old_home = std::env::var_os("HOME");
        unsafe {
            std::env::set_var("PATINA_HOME", &patina_home);
            std::env::set_var("HOME", temp.path());
        }
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(f));
        match old_home {
            Some(value) => unsafe {
                std::env::set_var("HOME", value);
            },
            None => unsafe {
                std::env::remove_var("HOME");
            },
        }
        match old {
            Some(value) => unsafe {
                std::env::set_var("PATINA_HOME", value);
            },
            None => unsafe {
                std::env::remove_var("PATINA_HOME");
            },
        }
        match result {
            Ok(value) => value,
            Err(panic) => std::panic::resume_unwind(panic),
        }
    }

    fn snapshot_entry_text(snapshot_path: &Path, name: &str) -> String {
        std::fs::read_to_string(snapshot_path.join(name)).unwrap()
    }

    #[test]
    fn creates_native_projection_for_opencode() {
        let temp = setup_project("opencode");

        let result = with_temp_patina_home(&temp, || {
            ensure_adapter_projection("opencode", temp.path(), ProjectionMode::RefreshManaged)
                .unwrap()
        });

        assert_eq!(result.bootstrap_path, temp.path().join("AGENTS.md"));
        assert_eq!(result.context_path, temp.path().join("AGENTS.md"));
        assert!(temp
            .path()
            .join(".opencode/commands/session-start.md")
            .exists());
        assert!(!temp.path().join(".opencode/PATINA.md").exists());
        assert!(!temp.path().join(".opencode/AGENTS.md").exists());
        assert!(temp.path().join(".opencode/.patina-managed.toml").exists());
        assert!(temp.path().join(".opencode/commands/spec.md").exists());
        assert!(temp
            .path()
            .join(".opencode/commands/epistemic-beliefs.md")
            .exists());
        assert!(temp.path().join(".opencode/bin/create-belief.sh").exists());
        let session_start =
            std::fs::read_to_string(temp.path().join(".opencode/commands/session-start.md"))
                .unwrap();
        assert!(session_start.contains(".opencode/bin/session-start.sh"));
        let belief_command =
            std::fs::read_to_string(temp.path().join(".opencode/commands/epistemic-beliefs.md"))
                .unwrap();
        assert!(belief_command.contains(".opencode/bin/create-belief.sh"));

        let agents = std::fs::read_to_string(temp.path().join("AGENTS.md")).unwrap();
        assert!(agents.contains("### OpenCode"));
        assert!(agents.contains("### Gemini CLI"));
        assert!(agents.contains(".opencode/bin/session-start.sh"));
        assert!(agents.contains(".gemini/bin/session-start.sh"));
        assert!(result.backup_snapshot.is_none());
    }

    #[test]
    fn bundle_status_reports_current_after_projection() {
        let temp = setup_project("gemini");

        with_temp_patina_home(&temp, || {
            ensure_bundle_projection("gemini", temp.path(), ProjectionMode::RefreshManaged).unwrap()
        });

        let status = bundle_deployment_status("gemini", temp.path()).unwrap();
        assert!(status.deployed);
        assert!(status.current);
        assert!(status.missing_paths.is_empty());
        assert!(status.unmanaged_paths.is_empty());
        assert!(status.stale_paths.is_empty());
    }

    #[test]
    fn ensure_bundle_bootstrap_skips_backup_when_bundle_is_current() {
        let temp = setup_project("claude");

        with_temp_patina_home(&temp, || {
            ensure_bundle_projection("claude", temp.path(), ProjectionMode::RefreshManaged).unwrap()
        });

        let result = with_temp_patina_home(&temp, || {
            ensure_bundle_bootstrap("claude", temp.path()).unwrap()
        });

        assert!(result.backup_snapshot.is_none());
        assert!(temp
            .path()
            .join(".claude/skills/epistemic-beliefs/SKILL.md")
            .exists());
        assert!(temp
            .path()
            .join(".claude/skills/epistemic-beliefs/references/verification-schema.md")
            .exists());
    }

    #[test]
    fn setup_takes_over_unmanaged_surface_and_creates_one_backup_snapshot() {
        let temp = setup_project("gemini");
        let adapter_dir = temp.path().join(".gemini");
        std::fs::create_dir_all(adapter_dir.join("commands")).unwrap();
        std::fs::write(temp.path().join("AGENTS.md"), "# Foreign root\n").unwrap();
        std::fs::write(temp.path().join("GEMINI.md"), "# Foreign shim\n").unwrap();
        std::fs::write(adapter_dir.join("GEMINI.md"), "# Foreign shell\n").unwrap();
        std::fs::write(adapter_dir.join("commands/custom.toml"), "stale = true\n").unwrap();

        let result = with_temp_patina_home(&temp, || {
            ensure_adapter_projection("gemini", temp.path(), ProjectionMode::RefreshManaged)
                .unwrap()
        });

        let snapshot_path = result.backup_snapshot.expect("expected backup snapshot");
        assert!(snapshot_path.starts_with(temp.path().join(".patina/local/backups/interface")));
        assert!(snapshot_path.is_dir());
        assert!(snapshot_entry_text(&snapshot_path, "AGENTS.md").contains("Foreign root"));
        assert!(snapshot_entry_text(&snapshot_path, "GEMINI.md").contains("Foreign shim"));
        assert!(snapshot_entry_text(&snapshot_path, ".gemini/GEMINI.md").contains("Foreign shell"));
        assert!(
            snapshot_entry_text(&snapshot_path, ".gemini/commands/custom.toml")
                .contains("stale = true")
        );

        let agents = std::fs::read_to_string(temp.path().join("AGENTS.md")).unwrap();
        assert!(agents.contains("<!-- PATINA:START -->"));
        assert!(agents.contains("### Gemini CLI"));

        let shim = std::fs::read_to_string(temp.path().join("GEMINI.md")).unwrap();
        assert!(shim.contains("Read `AGENTS.md` first."));
        assert!(!adapter_dir.join("GEMINI.md").exists());
        assert!(!adapter_dir.join("PATINA.md").exists());
        assert!(adapter_dir.join(".patina-managed.toml").exists());
    }

    #[test]
    fn rerun_refreshes_managed_surface_without_new_backup() {
        let temp = setup_project("gemini");

        with_temp_patina_home(&temp, || {
            ensure_adapter_projection("gemini", temp.path(), ProjectionMode::RefreshManaged)
                .unwrap()
        });

        let root_path = temp.path().join("AGENTS.md");
        let root_with_user_text = format!(
            "My notes stay here.\n\n{}",
            std::fs::read_to_string(&root_path).unwrap()
        );
        std::fs::write(&root_path, root_with_user_text).unwrap();

        let result = with_temp_patina_home(&temp, || {
            ensure_adapter_projection("gemini", temp.path(), ProjectionMode::RefreshManaged)
                .unwrap()
        });

        assert!(result.backup_snapshot.is_none());
        let updated_root = std::fs::read_to_string(&root_path).unwrap();
        assert!(updated_root.contains("My notes stay here."));
        assert!(updated_root.contains("<!-- PATINA:START -->"));
    }

    #[test]
    fn force_rewrite_creates_fresh_backup_of_managed_surface() {
        let temp = setup_project("opencode");

        with_temp_patina_home(&temp, || {
            ensure_adapter_projection("opencode", temp.path(), ProjectionMode::RefreshManaged)
                .unwrap()
        });
        std::fs::write(temp.path().join("AGENTS.md"), "# Replaced shell\n").unwrap();

        let result = with_temp_patina_home(&temp, || {
            ensure_adapter_projection("opencode", temp.path(), ProjectionMode::ForceRewrite)
                .unwrap()
        });

        let snapshot_path = result.backup_snapshot.expect("expected force backup");
        assert!(snapshot_entry_text(&snapshot_path, "AGENTS.md").contains("Replaced shell"));
        let bootstrap = std::fs::read_to_string(temp.path().join("AGENTS.md")).unwrap();
        assert!(bootstrap.contains("<!-- PATINA:START -->"));
        assert!(bootstrap.contains("### OpenCode"));
    }

    #[test]
    fn nested_agent_files_outside_managed_surface_are_left_alone() {
        let temp = setup_project("opencode");
        let nested = temp.path().join("packages/tool/AGENTS.md");
        std::fs::create_dir_all(nested.parent().unwrap()).unwrap();
        std::fs::write(&nested, "# Package local\n").unwrap();

        let result = with_temp_patina_home(&temp, || {
            ensure_adapter_projection("opencode", temp.path(), ProjectionMode::RefreshManaged)
                .unwrap()
        });

        assert!(result.backup_snapshot.is_none());
        assert_eq!(
            std::fs::read_to_string(&nested).unwrap(),
            "# Package local\n"
        );
    }

    #[test]
    fn backup_id_matches_session_style_shape() {
        let now = Local.with_ymd_and_hms(2026, 3, 11, 20, 15, 0).unwrap();
        let id = generate_backup_id(now);
        assert!(id.starts_with("20260311-201500-"));
        assert_eq!(id.len(), "20260311-201500-ABCD".len());
    }
}
