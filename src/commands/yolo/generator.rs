//! Generator - Creates devcontainer files from profile and features

use anyhow::{Result, Context};
use std::path::{Path, PathBuf};
use std::fs;
use serde_json::json;

use super::profile::{RepoProfile, Service};
use super::features::DevContainerFeature;

pub struct Generator {
    root_path: PathBuf,
}

impl Generator {
    pub fn new(path: &Path) -> Self {
        Self {
            root_path: path.to_path_buf(),
        }
    }

    pub fn generate(&self, profile: &RepoProfile, features: &[DevContainerFeature]) -> Result<()> {
        // Create .devcontainer directory
        let devcontainer_path = self.root_path.join(".devcontainer");
        fs::create_dir_all(&devcontainer_path)
            .context("Failed to create .devcontainer directory")?;

        // Generate devcontainer.json
        self.generate_devcontainer_json(&devcontainer_path, profile, features)?;

        // Generate Dockerfile if needed for custom features
        if self.needs_custom_dockerfile(features) {
            self.generate_dockerfile(&devcontainer_path, profile, features)?;
        }

        // Generate docker-compose.yml if services are needed
        if !profile.services.is_empty() {
            self.generate_docker_compose(&devcontainer_path, profile)?;
        }

        // Generate YOLO setup script
        self.generate_yolo_setup(&devcontainer_path, profile)?;

        Ok(())
    }

    fn generate_devcontainer_json(
        &self,
        devcontainer_path: &Path,
        profile: &RepoProfile,
        features: &[DevContainerFeature],
    ) -> Result<()> {
        let project_name = profile.project_name.as_deref()
            .or_else(|| self.root_path.file_name().and_then(|n| n.to_str()))
            .unwrap_or("workspace");

        // Build features object
        let mut features_obj = serde_json::Map::new();
        for feature in features {
            let (id, spec) = feature.to_feature_spec();
            features_obj.insert(id, spec);
        }

        let mut config = json!({
            "name": format!("{} - YOLO Workspace", project_name),
            "features": features_obj,
            "workspaceFolder": "/workspace",
            "workspaceMount": "source=${localWorkspaceFolder},target=/workspace,type=bind",

            // YOLO-specific environment variables
            "containerEnv": {
                "PATINA_YOLO": "1",
                "SKIP_PERMISSIONS": "1",
                "AI_WORKSPACE": "1",
                "IS_SANDBOX": "1"
            },

            // Mounts for credentials
            "mounts": [
                "source=${localWorkspaceFolder}/layer,target=/workspace/layer,type=bind",
                "source=${localEnv:HOME}/.patina/credentials,target=/root/.credentials,type=bind,readonly"
            ],

            // VSCode/Claude Code extensions
            "customizations": {
                "vscode": {
                    "extensions": self.get_vscode_extensions(features),
                    "settings": {
                        "terminal.integrated.defaultProfile.linux": "bash"
                    }
                }
            },

            // Forward common ports
            "forwardPorts": self.get_forward_ports(profile),

            // Run as root for full control in sandbox
            "remoteUser": "root",

            // Keep container running
            "overrideCommand": true,

            // Post-create command
            "postCreateCommand": "bash /workspace/.devcontainer/yolo-setup.sh"
        });

        // If we have a dockerfile, reference it
        if self.needs_custom_dockerfile(features) {
            config["build"] = json!({
                "dockerfile": "Dockerfile",
                "context": "."
            });
        } else {
            // Use base image
            config["image"] = json!("mcr.microsoft.com/devcontainers/base:ubuntu");
        }

        // If we have services, reference docker-compose
        if !profile.services.is_empty() {
            config["dockerComposeFile"] = json!("docker-compose.yml");
            config["service"] = json!("workspace");
            config["shutdownAction"] = json!("stopCompose");
        }

        let json_content = serde_json::to_string_pretty(&config)?;
        let json_path = devcontainer_path.join("devcontainer.json");
        fs::write(json_path, json_content)?;

        Ok(())
    }

    fn generate_dockerfile(
        &self,
        devcontainer_path: &Path,
        _profile: &RepoProfile,
        features: &[DevContainerFeature],
    ) -> Result<()> {
        let mut dockerfile = String::from(
            r#"# YOLO Development Container
# Autonomous AI workspace with all detected toolchains

FROM mcr.microsoft.com/devcontainers/base:ubuntu

# Avoid prompts during package installation
ENV DEBIAN_FRONTEND=noninteractive

"#
        );

        // Add custom installations for features not available as official features
        for feature in features {
            match feature {
                DevContainerFeature::Foundry { .. } => {
                    dockerfile.push_str(&self.get_foundry_install());
                }
                DevContainerFeature::Cairo { .. } => {
                    dockerfile.push_str(&self.get_cairo_install());
                }
                DevContainerFeature::Scarb { .. } => {
                    dockerfile.push_str(&self.get_scarb_install());
                }
                _ => {}
            }
        }

        dockerfile.push_str(
            r#"
# Create workspace directory
WORKDIR /workspace

# YOLO Mode indicator
ENV PATINA_YOLO=1

# Set up shell
CMD ["/bin/bash"]
"#
        );

        let dockerfile_path = devcontainer_path.join("Dockerfile");
        fs::write(dockerfile_path, dockerfile)?;

        Ok(())
    }

    fn generate_docker_compose(
        &self,
        devcontainer_path: &Path,
        profile: &RepoProfile,
    ) -> Result<()> {
        let project_name = profile.project_name.as_deref()
            .or_else(|| self.root_path.file_name().and_then(|n| n.to_str()))
            .unwrap_or("workspace");

        let mut services = serde_json::Map::new();

        // Main workspace service
        let workspace_config = if self.needs_custom_dockerfile(&[]) {
            json!({
                "build": {
                    "context": ".",
                    "dockerfile": "Dockerfile"
                },
                "volumes": ["..:/workspace:cached"],
                "command": "sleep infinity"
            })
        } else {
            json!({
                "image": "mcr.microsoft.com/devcontainers/base:ubuntu",
                "volumes": ["..:/workspace:cached"],
                "command": "sleep infinity"
            })
        };
        services.insert("workspace".to_string(), workspace_config);

        // Add detected services
        for service in &profile.services {
            let service_config = self.create_service_config(service);
            services.insert(service.name.clone(), service_config);
        }

        let compose = json!({
            "version": "3.8",
            "services": services
        });

        let yaml_content = serde_yaml::to_string(&compose)?;
        let compose_path = devcontainer_path.join("docker-compose.yml");
        fs::write(compose_path, yaml_content)?;

        Ok(())
    }

    fn generate_yolo_setup(
        &self,
        devcontainer_path: &Path,
        _profile: &RepoProfile,
    ) -> Result<()> {
        let setup_script = r#"#!/bin/bash
# YOLO Workspace Setup Script

echo "🎯 Setting up YOLO workspace..."

# Ensure git is configured
if [ -z "$(git config --global user.email)" ]; then
    git config --global user.email "ai@patina.dev"
    git config --global user.name "AI Assistant"
fi

# Create credentials directory if needed
mkdir -p ~/.credentials

# Set up shell aliases for YOLO mode
cat >> ~/.bashrc <<'EOF'
alias yolo='echo "YOLO mode active - permissions bypassed"'
alias status='git status'
alias commit='git add -A && git commit -m'
EOF

# Install additional tools if needed
if command -v npm &> /dev/null; then
    echo "📦 Installing global npm packages..."
    npm install -g typescript ts-node
fi

echo "✅ YOLO workspace ready!"
echo ""
echo "💭 Tips:"
echo "  • Use 'claude --dangerously-skip-permissions' for autonomous work"
echo "  • All changes are isolated in this container"
echo "  • Git worktree provides safe experimentation"
echo ""
"#;

        let setup_path = devcontainer_path.join("yolo-setup.sh");
        fs::write(setup_path, setup_script)?;

        Ok(())
    }

    // Helper methods
    fn needs_custom_dockerfile(&self, features: &[DevContainerFeature]) -> bool {
        features.iter().any(|f| matches!(f,
            DevContainerFeature::Foundry { .. } |
            DevContainerFeature::Cairo { .. } |
            DevContainerFeature::Scarb { .. }
        ))
    }

    fn get_vscode_extensions(&self, features: &[DevContainerFeature]) -> Vec<String> {
        let mut extensions = vec![];

        for feature in features {
            match feature {
                DevContainerFeature::Rust { .. } => {
                    extensions.push("rust-lang.rust-analyzer".to_string());
                }
                DevContainerFeature::Python { .. } => {
                    extensions.push("ms-python.python".to_string());
                }
                DevContainerFeature::Go { .. } => {
                    extensions.push("golang.go".to_string());
                }
                DevContainerFeature::Node { .. } => {
                    extensions.push("dbaeumer.vscode-eslint".to_string());
                }
                DevContainerFeature::Solc { .. } |
                DevContainerFeature::Foundry { .. } => {
                    extensions.push("JuanBlanco.solidity".to_string());
                }
                _ => {}
            }
        }

        extensions
    }

    fn get_forward_ports(&self, profile: &RepoProfile) -> Vec<u16> {
        let mut ports = vec![3000, 8000, 8080]; // Common dev ports

        for service in &profile.services {
            ports.extend(&service.ports);
        }

        ports.sort_unstable();
        ports.dedup();
        ports
    }

    fn create_service_config(&self, service: &Service) -> serde_json::Value {
        match service.name.as_str() {
            "anvil" => json!({
                "image": "ghcr.io/foundry-rs/foundry:latest",
                "command": "anvil --host 0.0.0.0",
                "ports": ["8545:8545"]
            }),
            "indexer" => json!({
                "image": "ghcr.io/latticexyz/store-indexer:latest",
                "environment": ["RPC_HTTP_URL=http://anvil:8545"],
                "depends_on": ["anvil"]
            }),
            _ => json!({
                "image": service.image.clone().unwrap_or_else(|| "alpine:latest".to_string()),
                "ports": service.ports.iter().map(|p| format!("{}:{}", p, p)).collect::<Vec<_>>()
            })
        }
    }

    fn get_foundry_install(&self) -> String {
        r#"
# Install Foundry
RUN curl -L https://foundry.paradigm.xyz | bash && \
    /root/.foundry/bin/foundryup && \
    echo 'export PATH="/root/.foundry/bin:$PATH"' >> /etc/bash.bashrc

"#.to_string()
    }

    fn get_cairo_install(&self) -> String {
        r#"
# Install Cairo
RUN curl --proto '=https' --tlsv1.2 -sSf https://cairo-lang.org/install.sh | sh && \
    echo 'export PATH="/root/.cairo/bin:$PATH"' >> /etc/bash.bashrc

"#.to_string()
    }

    fn get_scarb_install(&self) -> String {
        r#"
# Install Scarb (Cairo package manager)
RUN curl --proto '=https' --tlsv1.2 -sSf https://docs.swmansion.com/scarb/install.sh | bash && \
    echo 'export PATH="/root/.scarb/bin:$PATH"' >> /etc/bash.bashrc

"#.to_string()
    }
}