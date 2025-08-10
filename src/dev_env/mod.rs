pub mod dagger;
pub mod dagger_refactored;
pub mod docker;

use anyhow::Result;
use std::path::Path;

/// Trait for development environment integrations
pub trait DevEnvironment {
    /// Get the name of this development environment
    fn name(&self) -> &'static str;

    /// Get the version of this integration
    fn version(&self) -> &'static str;

    /// Initialize dev environment files during project creation
    fn init_project(
        &self,
        project_path: &Path,
        project_name: &str,
        project_type: &str,
    ) -> Result<()>;

    /// Build the project using this environment
    fn build(&self, project_path: &Path) -> Result<()>;

    /// Run tests using this environment
    fn test(&self, project_path: &Path) -> Result<()>;

    /// Check if this environment is available
    fn is_available(&self) -> bool;

    /// Get fallback environment if this one isn't available
    fn fallback(&self) -> Option<&'static str> {
        None
    }
}

/// Get a development environment by name
pub fn get_dev_env(name: &str) -> Box<dyn DevEnvironment> {
    match name.to_lowercase().as_str() {
        "dagger" => {
            // Use refactored version if environment variable is set
            if crate::config::use_refactored_dagger() {
                Box::new(dagger_refactored::DaggerEnvironment)
            } else {
                Box::new(dagger::DaggerEnvironment)
            }
        }
        "docker" => Box::new(docker::DockerEnvironment),
        _ => Box::new(docker::DockerEnvironment), // Default to Docker
    }
}
