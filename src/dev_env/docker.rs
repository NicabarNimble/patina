use super::DevEnvironment;
use anyhow::{Context, Result};
use std::fs;
use std::path::Path;
use std::process::Command;

pub const DOCKER_VERSION: &str = "0.1.0";

pub struct DockerEnvironment;

/// Detect project languages from manifest files
fn detect_project_languages(project_path: &Path) -> ProjectLanguages {
    ProjectLanguages {
        has_rust: project_path.join("Cargo.toml").exists(),
        has_node: project_path.join("package.json").exists(),
        has_python: project_path.join("requirements.txt").exists()
            || project_path.join("pyproject.toml").exists()
            || project_path.join("setup.py").exists(),
        has_go: project_path.join("go.mod").exists(),
    }
}

struct ProjectLanguages {
    has_rust: bool,
    has_node: bool,
    has_python: bool,
    has_go: bool,
}

impl DevEnvironment for DockerEnvironment {
    fn name(&self) -> &'static str {
        "docker"
    }

    fn version(&self) -> &'static str {
        DOCKER_VERSION
    }

    fn init_project(
        &self,
        project_path: &Path,
        project_name: &str,
        _project_type: &str,
    ) -> Result<()> {
        // Detect project languages
        let langs = detect_project_languages(project_path);

        // Create .devcontainer directory
        let devcontainer_dir = project_path.join(".devcontainer");
        fs::create_dir_all(&devcontainer_dir)?;

        // Generate Dockerfile with language-specific setup
        let mut dockerfile_content =
            include_str!("../../resources/templates/devcontainer/Dockerfile").to_string();

        // Replace language setup placeholders
        dockerfile_content = dockerfile_content.replace(
            "{{RUST_SETUP}}",
            if langs.has_rust {
                "# Install Rust\nRUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y\nENV PATH=\"/root/.cargo/bin:${PATH}\""
            } else {
                ""
            }
        );

        dockerfile_content = dockerfile_content.replace(
            "{{NODE_SETUP}}",
            if langs.has_node {
                "# Install Node.js\nRUN curl -fsSL https://deb.nodesource.com/setup_20.x | bash - \\\n    && apt-get install -y nodejs \\\n    && rm -rf /var/lib/apt/lists/*"
            } else {
                ""
            }
        );

        dockerfile_content = dockerfile_content.replace(
            "{{PYTHON_SETUP}}",
            if langs.has_python {
                "# Install Python\nRUN apt-get update && apt-get install -y \\\n    python3 \\\n    python3-pip \\\n    python3-venv \\\n    && rm -rf /var/lib/apt/lists/*"
            } else {
                ""
            }
        );

        dockerfile_content = dockerfile_content.replace(
            "{{GO_SETUP}}",
            if langs.has_go {
                "# Install Go\nRUN curl -L https://go.dev/dl/go1.22.0.linux-amd64.tar.gz | tar -C /usr/local -xzf - \\\n    && ln -s /usr/local/go/bin/go /usr/local/bin/go"
            } else {
                ""
            }
        );

        fs::write(devcontainer_dir.join("Dockerfile"), dockerfile_content)?;

        // Generate devcontainer.json
        let devcontainer_json =
            include_str!("../../resources/templates/devcontainer/devcontainer.json")
                .replace("{{PROJECT_NAME}}", project_name);

        fs::write(
            devcontainer_dir.join("devcontainer.json"),
            devcontainer_json,
        )?;

        Ok(())
    }

    fn build(&self, project_path: &Path) -> Result<()> {
        if !self.is_available() {
            anyhow::bail!("Docker is not installed");
        }

        if !project_path.join("Dockerfile").exists() {
            anyhow::bail!("No Dockerfile found in current directory");
        }

        println!("🐳 Building with Docker...");

        // Get project name from config
        let config_path = project_path.join(".patina/config.json");
        let project_name = if config_path.exists() {
            let config_content = fs::read_to_string(&config_path)?;
            let config: serde_json::Value = serde_json::from_str(&config_content)?;
            config
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or("app")
                .to_string()
        } else {
            "app".to_string()
        };

        let output = Command::new("docker")
            .current_dir(project_path)
            .args(["build", "-t", &format!("{project_name}:latest"), "."])
            .status()
            .context("Failed to run docker build")?;

        if output.success() {
            println!("✅ Successfully built {project_name}:latest");
            Ok(())
        } else {
            anyhow::bail!("Docker build failed")
        }
    }

    fn test(&self, project_path: &Path) -> Result<()> {
        // For now, run tests in Docker container
        self.build(project_path)?;

        println!("🧪 Running tests in Docker container...");

        let config_path = project_path.join(".patina/config.json");
        let project_name = if config_path.exists() {
            let config_content = fs::read_to_string(&config_path)?;
            let config: serde_json::Value = serde_json::from_str(&config_content)?;
            config
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or("app")
                .to_string()
        } else {
            "app".to_string()
        };

        let output = Command::new("docker")
            .current_dir(project_path)
            .args([
                "run",
                "--rm",
                &format!("{project_name}:latest"),
                "cargo",
                "test",
            ])
            .status()
            .context("Failed to run tests in Docker")?;

        if output.success() {
            println!("✅ Tests passed");
            Ok(())
        } else {
            anyhow::bail!("Tests failed")
        }
    }

    fn is_available(&self) -> bool {
        which::which("docker").is_ok()
    }
}
