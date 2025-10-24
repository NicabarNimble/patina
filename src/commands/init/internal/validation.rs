//! Environment and project validation

use anyhow::Result;

use patina::environment::Environment;

/// Validate environment against project requirements
pub fn validate_environment(env: &Environment) -> Result<Option<Vec<String>>> {
    let mut warnings = Vec::new();

    // All Patina projects benefit from Rust
    if !env.languages.get("rust").is_some_and(|info| info.available) {
        warnings.push(
            "⚠️  Rust not detected - Patina is built for Rust projects (install via rustup)"
                .to_string(),
        );
    }

    // Check for container tooling based on project type
    let has_docker = env.tools.get("docker").is_some_and(|info| info.available);
    let has_podman = env.tools.get("podman").is_some_and(|info| info.available);

    if !has_docker && !has_podman {
        warnings
            .push("⚠️  No container runtime detected (Docker or Podman recommended)".to_string());
    }

    // Check for git
    if !env.tools.get("git").is_some_and(|info| info.available) {
        warnings.push("⚠️  Git not detected - version control is essential".to_string());
    }

    if warnings.is_empty() {
        Ok(None)
    } else {
        Ok(Some(warnings))
    }
}

/// Determine the best development environment based on what's available
pub fn determine_dev_environment(_environment: &Environment) -> String {
    // If PATINA_DEV is set, respect it
    if let Ok(dev_env) = std::env::var("PATINA_DEV") {
        eprintln!("   Using PATINA_DEV={dev_env}");
        return dev_env;
    }

    // Docker-only now
    println!("🐳 Using Docker for development");
    println!("   💡 Docker provides containerized builds and tests");
    "docker".to_string()
}
