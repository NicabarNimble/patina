use anyhow::{Context, Result};
use patina::dev_env;
use patina::project;

pub fn execute() -> Result<()> {
    let project_root = std::env::current_dir().context("Failed to get current directory")?;

    // Load unified project config (with migration if needed)
    let config = project::load_with_migration(&project_root)?;
    let dev_env_name = config.dev.dev_type.clone();
    let _project_name = config.project.name.clone();

    // Get the appropriate dev environment
    let dev_environment = dev_env::get_dev_env(&dev_env_name);

    // Try to build with the selected environment
    match dev_environment.build(&project_root) {
        Ok(()) => Ok(()),
        Err(e) => {
            // If it fails and has a fallback, try that
            if let Some(fallback_name) = dev_environment.fallback() {
                println!("⚠️  {} failed: {e}", dev_environment.name());
                println!("   Falling back to {fallback_name}...");

                let fallback_env = dev_env::get_dev_env(fallback_name);
                fallback_env.build(&project_root)
            } else {
                Err(e)
            }
        }
    }
}
