use anyhow::{Context, Result};
use clap::{Subcommand, ValueEnum};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use uuid::Uuid;

use patina::paths;

const FIXTURE_SET: &str = "mct-skill-fixtures-v1";
const DEFAULT_SCENARIO: &str = "project-empty";

#[derive(Debug, Clone, Subcommand)]
pub enum SandboxCommands {
    /// Create an isolated Mother skill sandbox from a durable fixture scenario
    Create {
        /// Scenario fixture to materialize
        #[arg(long, default_value = DEFAULT_SCENARIO)]
        scenario: String,

        /// Default HITL interface recorded in sandbox metadata for inference tests
        #[arg(long = "default-interface", value_enum, default_value_t = SandboxInterface::Gemini)]
        default_interface: SandboxInterface,

        /// Output raw JSON
        #[arg(long)]
        json: bool,
    },

    /// List Mother skill sandboxes
    List {
        /// Output raw JSON
        #[arg(long)]
        json: bool,
    },

    /// Print a sandbox root path
    Path {
        /// Sandbox id
        id: String,
    },

    /// Reset a sandbox back to its scenario baseline
    Reset {
        /// Sandbox id
        id: String,

        /// Output raw JSON
        #[arg(long)]
        json: bool,
    },

    /// Remove a sandbox and its ephemeral root
    Remove {
        /// Sandbox id
        id: String,

        /// Output raw JSON
        #[arg(long)]
        json: bool,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ValueEnum)]
#[serde(rename_all = "kebab-case")]
pub enum SandboxInterface {
    Pi,
    Claude,
    Opencode,
    Gemini,
}

impl SandboxInterface {
    fn as_str(self) -> &'static str {
        match self {
            SandboxInterface::Pi => "pi",
            SandboxInterface::Claude => "claude",
            SandboxInterface::Opencode => "opencode",
            SandboxInterface::Gemini => "gemini",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SandboxMetadata {
    pub id: String,
    pub fixture_set: String,
    pub scenario: String,
    pub root_path: PathBuf,
    pub project_root: PathBuf,
    pub home_root: PathBuf,
    pub child_store_root: PathBuf,
    pub logs_root: PathBuf,
    pub default_interface: String,
    pub materializes_all_hitls: bool,
    pub state: String,
}

pub fn execute(command: SandboxCommands) -> Result<()> {
    match command {
        SandboxCommands::Create {
            scenario,
            default_interface,
            json,
        } => create(&scenario, default_interface, json),
        SandboxCommands::List { json } => list(json),
        SandboxCommands::Path { id } => print_path(&id),
        SandboxCommands::Reset { id, json } => reset(&id, json),
        SandboxCommands::Remove { id, json } => remove(&id, json),
    }
}

fn create(scenario: &str, default_interface: SandboxInterface, json: bool) -> Result<()> {
    ensure_known_scenario(scenario)?;

    let id = Uuid::new_v4().simple().to_string()[..12].to_string();
    let root_path = sandboxes_dir().join(&id);
    let metadata = metadata_for(&id, scenario, default_interface, &root_path);

    materialize_sandbox(&metadata)?;
    write_metadata(&metadata)?;

    if json {
        println!("{}", serde_json::to_string_pretty(&metadata)?);
    } else {
        println!("Created Mother skill sandbox {}", metadata.id);
        println!("  scenario: {}", metadata.scenario);
        println!("  default interface: {}", metadata.default_interface);
        println!("  root: {}", metadata.root_path.display());
        println!("  project: {}", metadata.project_root.display());
        println!("  home: {}", metadata.home_root.display());
    }
    Ok(())
}

fn list(json: bool) -> Result<()> {
    let mut sandboxes = read_all_metadata()?;
    sandboxes.sort_by(|a, b| a.id.cmp(&b.id));

    if json {
        println!("{}", serde_json::to_string_pretty(&sandboxes)?);
        return Ok(());
    }

    if sandboxes.is_empty() {
        println!("No Mother skill sandboxes found.");
        return Ok(());
    }

    println!("Mother skill sandboxes:\n");
    for sandbox in sandboxes {
        println!(
            "  {}  scenario={} interface={} state={} root={}",
            sandbox.id,
            sandbox.scenario,
            sandbox.default_interface,
            sandbox.state,
            sandbox.root_path.display()
        );
    }
    Ok(())
}

fn print_path(id: &str) -> Result<()> {
    let metadata = read_metadata(id)?;
    println!("{}", metadata.root_path.display());
    Ok(())
}

fn reset(id: &str, json: bool) -> Result<()> {
    let mut metadata = read_metadata(id)?;
    remove_materialized_dirs(&metadata)?;
    materialize_sandbox(&metadata)?;
    metadata.state = "reset".to_string();
    write_metadata(&metadata)?;

    if json {
        println!("{}", serde_json::to_string_pretty(&metadata)?);
    } else {
        println!("Reset Mother skill sandbox {}", metadata.id);
    }
    Ok(())
}

fn remove(id: &str, json: bool) -> Result<()> {
    let metadata = read_metadata(id)?;
    if metadata.root_path.exists() {
        fs::remove_dir_all(&metadata.root_path)
            .with_context(|| format!("removing sandbox {}", metadata.root_path.display()))?;
    }

    if json {
        println!(
            "{}",
            serde_json::json!({
                "id": id,
                "removed": true,
                "root_path": metadata.root_path,
            })
        );
    } else {
        println!("Removed Mother skill sandbox {}", id);
    }
    Ok(())
}

fn metadata_for(
    id: &str,
    scenario: &str,
    default_interface: SandboxInterface,
    root_path: &Path,
) -> SandboxMetadata {
    SandboxMetadata {
        id: id.to_string(),
        fixture_set: FIXTURE_SET.to_string(),
        scenario: scenario.to_string(),
        root_path: root_path.to_path_buf(),
        project_root: root_path.join("project"),
        home_root: root_path.join("home"),
        child_store_root: root_path.join("child-store"),
        logs_root: root_path.join("logs"),
        default_interface: default_interface.as_str().to_string(),
        materializes_all_hitls: true,
        state: "created".to_string(),
    }
}

fn materialize_sandbox(metadata: &SandboxMetadata) -> Result<()> {
    fs::create_dir_all(&metadata.project_root)?;
    fs::create_dir_all(&metadata.home_root)?;
    fs::create_dir_all(&metadata.child_store_root)?;
    fs::create_dir_all(&metadata.logs_root)?;

    materialize_hitl_roots(metadata)?;
    materialize_child_store(metadata)?;
    materialize_scenario(metadata)?;
    Ok(())
}

fn materialize_hitl_roots(metadata: &SandboxMetadata) -> Result<()> {
    for path in [
        metadata.project_root.join(".pi/skills"),
        metadata.project_root.join(".claude/skills"),
        metadata.project_root.join(".opencode/skills"),
        metadata.project_root.join(".gemini/skills"),
        metadata.project_root.join(".agents/skills"),
        metadata.home_root.join(".pi/agent/skills"),
        metadata.home_root.join(".claude/skills"),
        metadata.home_root.join(".config/opencode/skills"),
        metadata.home_root.join(".opencode/skills"),
        metadata.home_root.join(".gemini/skills"),
        metadata.home_root.join(".agents/skills"),
    ] {
        fs::create_dir_all(path)?;
    }
    Ok(())
}

fn materialize_child_store(metadata: &SandboxMetadata) -> Result<()> {
    let src = fixtures_root().join("children/fixture-skill-app");
    let dst = metadata.child_store_root.join("fixture-skill-app");
    copy_dir_all(&src, &dst).with_context(|| {
        format!(
            "copying fixture child from {} to {}",
            src.display(),
            dst.display()
        )
    })?;
    Ok(())
}

fn materialize_scenario(metadata: &SandboxMetadata) -> Result<()> {
    let scenario_path = fixtures_root()
        .join("scenarios")
        .join(&metadata.scenario)
        .join("scenario.json");
    let dst = metadata.root_path.join("scenario.json");
    fs::copy(&scenario_path, &dst).with_context(|| {
        format!(
            "copying scenario metadata from {} to {}",
            scenario_path.display(),
            dst.display()
        )
    })?;

    match metadata.scenario.as_str() {
        "project-conflicted" => {
            let conflict = metadata.project_root.join(".gemini/skills/hello/SKILL.md");
            if let Some(parent) = conflict.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::write(
                conflict,
                "---\nname: hello\ndescription: unmanaged conflicting skill\n---\n\n# Unmanaged\n",
            )?;
        }
        "project-installed" | "project-stale" | "mixed-all" => {
            let installed = metadata
                .project_root
                .join(".gemini/skills/fixture-skill-app/hello/SKILL.md");
            if let Some(parent) = installed.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::copy(
                metadata
                    .child_store_root
                    .join("fixture-skill-app/skills/hello/SKILL.md"),
                installed,
            )?;
        }
        "global-installed" => {
            let installed = metadata
                .home_root
                .join(".gemini/skills/fixture-skill-app/hello/SKILL.md");
            if let Some(parent) = installed.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::copy(
                metadata
                    .child_store_root
                    .join("fixture-skill-app/skills/hello/SKILL.md"),
                installed,
            )?;
        }
        _ => {}
    }
    Ok(())
}

fn remove_materialized_dirs(metadata: &SandboxMetadata) -> Result<()> {
    for path in [
        &metadata.project_root,
        &metadata.home_root,
        &metadata.child_store_root,
        &metadata.logs_root,
    ] {
        if path.exists() {
            fs::remove_dir_all(path)?;
        }
    }
    Ok(())
}

fn write_metadata(metadata: &SandboxMetadata) -> Result<()> {
    fs::create_dir_all(&metadata.root_path)?;
    fs::write(
        metadata_path(&metadata.id),
        serde_json::to_string_pretty(metadata)?,
    )?;
    Ok(())
}

fn read_metadata(id: &str) -> Result<SandboxMetadata> {
    let path = metadata_path(id);
    let text = fs::read_to_string(&path)
        .with_context(|| format!("reading sandbox metadata {}", path.display()))?;
    Ok(serde_json::from_str(&text)?)
}

fn read_all_metadata() -> Result<Vec<SandboxMetadata>> {
    let dir = sandboxes_dir();
    if !dir.exists() {
        return Ok(Vec::new());
    }

    let mut out = Vec::new();
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        if !entry.file_type()?.is_dir() {
            continue;
        }
        let path = entry.path().join("sandbox.json");
        if !path.exists() {
            continue;
        }
        let text = fs::read_to_string(&path)?;
        out.push(serde_json::from_str(&text)?);
    }
    Ok(out)
}

fn ensure_known_scenario(scenario: &str) -> Result<()> {
    let path = fixtures_root()
        .join("scenarios")
        .join(scenario)
        .join("scenario.json");
    if !path.exists() {
        anyhow::bail!(
            "unknown Mother skill sandbox scenario '{}'; missing {}",
            scenario,
            path.display()
        );
    }
    Ok(())
}

fn copy_dir_all(src: &Path, dst: &Path) -> Result<()> {
    if dst.exists() {
        fs::remove_dir_all(dst)?;
    }
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        let next_dst = dst.join(entry.file_name());
        if ty.is_dir() {
            copy_dir_all(&entry.path(), &next_dst)?;
        } else {
            fs::copy(entry.path(), next_dst)?;
        }
    }
    Ok(())
}

fn metadata_path(id: &str) -> PathBuf {
    sandboxes_dir().join(id).join("sandbox.json")
}

fn sandboxes_dir() -> PathBuf {
    paths::patina_home().join("local/dev/mother-skill-sandboxes")
}

fn fixtures_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/mother")
}
