use anyhow::{Context, Result};
use serde::{Serialize, Deserialize};
use std::process::Command;

#[derive(Serialize, Deserialize)]
struct ExperimentConfig {
    name: String,
    commands: Vec<String>,
    port: u16,
}

pub fn execute(experiment_names: Vec<String>) -> Result<()> {
    // Build experiment configurations
    let experiments: Vec<ExperimentConfig> = experiment_names
        .into_iter()
        .enumerate()
        .map(|(i, name)| {
            // Patina decides what each experiment should do
            let commands = match name.as_str() {
                "async" => vec![
                    "cargo test --features async".to_string(),
                    "echo 'Async approach using tokio'".to_string(),
                ],
                "threads" => vec![
                    "cargo test --features threads".to_string(),
                    "echo 'Thread-based parallelism'".to_string(),
                ],
                "actors" => vec![
                    "cargo test --features actors".to_string(),
                    "echo 'Actor model with actix'".to_string(),
                ],
                _ => vec![
                    format!("cargo test --features {}", name),
                    format!("echo 'Testing {} approach'", name),
                ],
            };
            
            ExperimentConfig {
                name,
                commands,
                port: 8081 + i as u16,
            }
        })
        .collect();

    // Serialize experiments for Dagger
    let experiments_json = serde_json::to_string(&experiments)?;
    
    println!("🚀 Launching {} parallel experiments...", experiments.len());
    for exp in &experiments {
        println!("   - {}: {} commands", exp.name, exp.commands.len());
    }
    
    // Run the Dagger pipeline with experiments
    let status = Command::new("go")
        .current_dir("pipelines")
        .env("PATINA_EXPERIMENTS", experiments_json)
        .env("PATINA_PROJECT_ROOT", std::env::current_dir()?)
        .args(&["run", ".", "yolo"])
        .status()
        .context("Failed to run YOLO experiments")?;
    
    if !status.success() {
        anyhow::bail!("YOLO experiments failed");
    }
    
    Ok(())
}

// Usage: patina yolo async threads actors