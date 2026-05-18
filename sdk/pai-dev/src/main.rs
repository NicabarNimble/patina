use std::path::PathBuf;

use anyhow::Result;
use clap::{Parser, Subcommand, ValueEnum};
use patina_child_diagnostics::{
    check_children_dev_config_with_options, check_package, CheckOptions, ChildrenDevCheckOptions,
    DiagnosticReport, DiagnosticStage,
};

#[derive(Debug, Parser)]
#[command(
    name = "pai-dev",
    version,
    about = "Patina SDK developer tooling",
    long_about = "Patina SDK developer entry point for checking child packages, WIT contracts, components, and release candidates."
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    /// Work with one child package root.
    Child {
        #[command(subcommand)]
        command: ChildCommands,
    },
    /// Work with a repo-local .patina/children-dev.toml containing one or more children.
    Children {
        #[command(subcommand)]
        command: ChildrenCommands,
    },
}

#[derive(Debug, Subcommand)]
enum ChildCommands {
    /// Run diagnostics for one child package root.
    Check {
        /// Diagnostic stage to run.
        stage: CheckStage,
        /// Child package root containing child.toml and wit/.
        #[arg(default_value = ".")]
        path: PathBuf,
        /// Explicit built component artifact for component-built/release-candidate stages.
        #[arg(long)]
        component: Option<PathBuf>,
        /// Explicit release bundle directory for release-candidate stage.
        #[arg(long)]
        release: Option<PathBuf>,
        /// Optional release tag to compare against child.toml version.
        #[arg(long)]
        tag: Option<String>,
        /// Emit machine-readable JSON.
        #[arg(long)]
        json: bool,
    },
}

#[derive(Debug, Subcommand)]
enum ChildrenCommands {
    /// Run diagnostics using repo-local .patina/children-dev.toml.
    Check {
        /// Diagnostic stage to run.
        stage: CheckStage,
        /// Repository root containing .patina/children-dev.toml.
        #[arg(default_value = ".")]
        repo_root: PathBuf,
        /// Emit machine-readable JSON.
        #[arg(long)]
        json: bool,
    },
}

#[derive(Clone, Copy, Debug, ValueEnum)]
enum CheckStage {
    /// Validate child.toml and WIT only.
    LocalDev,
    /// Validate child.toml, WIT, and an explicit built component artifact.
    ComponentBuilt,
    /// Validate child.toml, WIT, component artifact, and release bundle assets.
    ReleaseCandidate,
}

impl From<CheckStage> for DiagnosticStage {
    fn from(stage: CheckStage) -> Self {
        match stage {
            CheckStage::LocalDev => DiagnosticStage::LocalDev,
            CheckStage::ComponentBuilt => DiagnosticStage::ComponentBuilt,
            CheckStage::ReleaseCandidate => DiagnosticStage::ReleaseCandidate,
        }
    }
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    let ok = match cli.command {
        Commands::Child { command } => run_child(command)?,
        Commands::Children { command } => run_children(command)?,
    };

    if !ok {
        std::process::exit(1);
    }

    Ok(())
}

fn run_child(command: ChildCommands) -> Result<bool> {
    match command {
        ChildCommands::Check {
            stage,
            path,
            component,
            release,
            tag,
            json,
        } => {
            let report = check_package(
                path,
                CheckOptions {
                    stage: stage.into(),
                    component_path: component,
                    release_path: release,
                    release_tag: tag,
                },
            );
            emit_report(&report, json)?;
            Ok(report.is_ok())
        }
    }
}

fn run_children(command: ChildrenCommands) -> Result<bool> {
    match command {
        ChildrenCommands::Check {
            stage,
            repo_root,
            json,
        } => {
            let report = check_children_dev_config_with_options(
                repo_root,
                ChildrenDevCheckOptions {
                    stage: stage.into(),
                },
            );

            if json {
                println!("{}", serde_json::to_string_pretty(&report)?);
            } else if report.is_ok() {
                println!("ok: children-dev diagnostics passed");
            } else {
                print!("{}", report.render_text());
            }

            Ok(report.is_ok())
        }
    }
}

fn emit_report(report: &DiagnosticReport, json: bool) -> Result<()> {
    if json {
        println!("{}", serde_json::to_string_pretty(report)?);
    } else if report.is_ok() {
        println!("ok: child diagnostics passed");
    } else {
        print!("{}", report.render_text());
    }

    Ok(())
}
