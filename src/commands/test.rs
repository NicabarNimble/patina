use anyhow::{Context, Result};
use patina::dev_env;
use std::fs;

pub fn execute() -> Result<()> {
    let project_root = std::env::current_dir().context("Failed to get current directory")?;

    // Read config to determine which dev environment to use
    let config_path = project_root.join(".patina/config.json");
    let dev_env_name = if config_path.exists() {
        let config_content = fs::read_to_string(&config_path)?;
        let config: serde_json::Value = serde_json::from_str(&config_content)?;

        config
            .get("dev")
            .and_then(|d| d.as_str())
            .unwrap_or("docker")
            .to_string()
    } else {
        "docker".to_string()
    };

    // Get the appropriate dev environment
    let dev_environment = dev_env::get_dev_env(&dev_env_name);

    // Try to test with the selected environment
    match dev_environment.test(&project_root) {
        Ok(()) => Ok(()),
        Err(e) => {
            // If it fails and has a fallback, try that
            if let Some(fallback_name) = dev_environment.fallback() {
                println!("⚠️  {} failed: {e}", dev_environment.name());
                println!("   Falling back to {fallback_name}...");

                let fallback_env = dev_env::get_dev_env(fallback_name);
                fallback_env.test(&project_root)
            } else {
                Err(e)
            }
        }
    }
}
