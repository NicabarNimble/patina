//! Generator - Creates devcontainer files from profile and features

use anyhow::{Context, Result};
use serde_json::json;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use super::features::DevContainerFeature;
use super::profile::{RepoProfile, Service};

pub struct Generator {
    root_path: PathBuf,
    has_1password_cli: bool,
}

impl Generator {
    pub fn new(path: &Path) -> Self {
        let has_1password_cli = Self::detect_1password_cli();
        Self {
            root_path: path.to_path_buf(),
            has_1password_cli,
        }
    }

    fn detect_1password_cli() -> bool {
        Command::new("which")
            .arg("op")
            .output()
            .map(|output| output.status.success())
            .unwrap_or(false)
    }

    pub fn generate(&self, profile: &RepoProfile, features: &[DevContainerFeature]) -> Result<()> {
        // Create .devcontainer directory
        let devcontainer_path = self.root_path.join(".devcontainer");
        fs::create_dir_all(&devcontainer_path)
            .context("Failed to create .devcontainer directory")?;

        // Generate devcontainer.json
        self.generate_devcontainer_json(&devcontainer_path, profile, features)?;

        // Always generate Dockerfile with Claude Code CLI
        self.generate_dockerfile(&devcontainer_path, profile, features)?;

        // Always generate docker-compose.yml for CLI usage
        self.generate_docker_compose(&devcontainer_path, profile, features)?;

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
        let project_name = profile
            .project_name
            .as_deref()
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
                "IS_SANDBOX": "1",
                "CLAUDE_CONFIG_DIR": "/root/.claude-linux"
            },

            // Mounts for shared Claude credentials (Max subscription)
            "mounts": [
                "source=${localWorkspaceFolder}/layer,target=/workspace/layer,type=bind",
                "source=${localEnv:HOME}/.patina/claude-linux,target=/root/.claude-linux,type=bind"
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

        // Always reference Dockerfile (includes Claude Code CLI)
        config["build"] = json!({
            "dockerfile": "Dockerfile",
            "context": "."
        });

        // Always reference docker-compose.yml (we always generate it)
        config["dockerComposeFile"] = json!("docker-compose.yml");
        config["service"] = json!("workspace");
        config["shutdownAction"] = json!("stopCompose");

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

# Install Node.js for Claude Code CLI
RUN curl -fsSL https://deb.nodesource.com/setup_20.x | bash - \
    && apt-get install -y nodejs \
    && npm install -g npm@latest \
    && npm install -g pnpm@latest

# Install Claude Code CLI for autonomous AI work
RUN npm install -g @anthropic-ai/claude-code@latest \
    && mkdir -p /root/.claude-linux

# Configure Claude settings using official settings.json API
# YOLO mode with extended timeouts (1 hour for long builds)
RUN echo '{"permissions":{"defaultMode":"bypassPermissions","allow":[],"deny":[]},"env":{"BASH_DEFAULT_TIMEOUT_MS":3600000,"BASH_MAX_TIMEOUT_MS":3600000}}' > /root/.claude-linux/settings.json

# Set Claude config directory for bind-mounted credentials
RUN echo 'export CLAUDE_CONFIG_DIR=/root/.claude-linux' >> /etc/bash.bashrc
"#,
        );

        // Install 1Password CLI if available on host (for secure credential management)
        if self.has_1password_cli {
            dockerfile.push_str(r#"
# Install 1Password CLI for secure credential management
RUN curl -sS https://downloads.1password.com/linux/keys/1password.asc | \
    gpg --dearmor --output /usr/share/keyrings/1password-archive-keyring.gpg && \
    echo "deb [arch=$(dpkg --print-architecture) signed-by=/usr/share/keyrings/1password-archive-keyring.gpg] https://downloads.1password.com/linux/debian/$(dpkg --print-architecture) stable main" | \
    tee /etc/apt/sources.list.d/1password.list && \
    mkdir -p /etc/debsig/policies/AC2D62742012EA22/ && \
    curl -sS https://downloads.1password.com/linux/debian/debsig/1password.pol | \
    tee /etc/debsig/policies/AC2D62742012EA22/1password.pol && \
    mkdir -p /usr/share/debsig/keyrings/AC2D62742012EA22 && \
    curl -sS https://downloads.1password.com/linux/keys/1password.asc | \
    gpg --dearmor --output /usr/share/debsig/keyrings/AC2D62742012EA22/debsig.gpg && \
    apt-get update && apt-get install -y 1password-cli

"#);
        }

        // Add custom installations for features not available as official features
        for feature in features {
            match feature {
                DevContainerFeature::Foundry { .. } => {
                    dockerfile.push_str(&self.get_foundry_install());
                }
                DevContainerFeature::Dojo { .. } => {
                    dockerfile.push_str(&self.get_dojo_install());
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
"#,
        );

        let dockerfile_path = devcontainer_path.join("Dockerfile");
        fs::write(dockerfile_path, dockerfile)?;

        Ok(())
    }

    fn generate_docker_compose(
        &self,
        devcontainer_path: &Path,
        profile: &RepoProfile,
        _features: &[DevContainerFeature],
    ) -> Result<()> {
        let _project_name = profile
            .project_name
            .as_deref()
            .or_else(|| self.root_path.file_name().and_then(|n| n.to_str()))
            .unwrap_or("workspace");

        let mut services = serde_json::Map::new();

        // Common development ports to expose
        let common_ports = vec![
            "3000:3000", // Common web dev
            "3001:3001", // Alternative web
            "3008:3008", // MUD explorer
            "8000:8000", // Alternative web
            "8080:8080", // Alternative web
            "8545:8545", // Anvil/local blockchain
        ];

        // Main workspace service with YOLO environment variables
        // Always build from Dockerfile (includes Claude Code CLI)

        // Configure volumes based on 1Password CLI availability
        let mut volumes = vec![
            json!("../:/workspace:cached"),
            json!("${HOME}/.claude:/root/.claude-macos:ro"),
        ];

        // If 1Password CLI available, use tmpfs for credentials (RAM-only storage)
        // and mount op config for authentication
        if self.has_1password_cli {
            volumes.push(json!({
                "type": "tmpfs",
                "target": "/root/.claude-linux",
                "tmpfs": {
                    "size": 10485760  // 10MB tmpfs
                }
            }));
            // Mount 1Password config for authentication
            volumes.push(json!("${HOME}/.config/op:/root/.config/op:ro"));
        } else {
            volumes.push(json!(
                "${HOME}/.patina/claude-linux:/root/.claude-linux:cached"
            ));
        }

        let workspace_config = json!({
            "build": {
                "context": ".",
                "dockerfile": "Dockerfile"
            },
            "volumes": volumes,
            "working_dir": "/workspace",
            "command": "sleep infinity",
            "ports": common_ports,
            "environment": {
                "PATINA_YOLO": "1",
                "SKIP_PERMISSIONS": "1",
                "AI_WORKSPACE": "1",
                "IS_SANDBOX": "1",
                "CLAUDE_CONFIG_DIR": "/root/.claude-linux",
                "PATINA_USE_1PASSWORD": if self.has_1password_cli { "1" } else { "0" }
            }
        });
        services.insert("workspace".to_string(), workspace_config);

        // Add detected services
        for service in &profile.services {
            let service_config = self.create_service_config(service);
            services.insert(service.name.clone(), service_config);
        }

        let compose = json!({ "services": services });

        let yaml_content = serde_yaml::to_string(&compose)?;
        let compose_path = devcontainer_path.join("docker-compose.yml");
        fs::write(compose_path, yaml_content)?;

        Ok(())
    }

    fn generate_yolo_setup(&self, devcontainer_path: &Path, _profile: &RepoProfile) -> Result<()> {
        let setup_script = r#"#!/bin/bash
# YOLO Workspace Setup Script

echo "🎯 Setting up YOLO workspace..."

# Ensure git is configured
if [ -z "$(git config --global user.email)" ]; then
    git config --global user.email "ai@patina.dev"
    git config --global user.name "AI Assistant"
fi

# Create Claude config directory
mkdir -p ~/.claude-linux

# Configure Claude settings using official settings.json API
# This ensures settings are up-to-date even if Dockerfile settings were cached
echo "🔧 Configuring Claude settings..."
cat > ~/.claude-linux/settings.json <<'EOF'
{
  "permissions": {
    "defaultMode": "bypassPermissions",
    "allow": [],
    "deny": []
  },
  "env": {
    "BASH_DEFAULT_TIMEOUT_MS": 3600000,
    "BASH_MAX_TIMEOUT_MS": 3600000
  }
}
EOF

echo "✅ Claude configured with:"
echo "  - Permissions: YOLO mode (bypassed)"
echo "  - Bash timeout: 1 hour (3600000ms)"

# Set up shell aliases for YOLO mode
cat >> ~/.bashrc <<'EOF'
alias yolo='echo "YOLO mode active - permissions bypassed"'
alias status='git status'
alias commit='git add -A && git commit -m'
EOF

# Install additional tools if needed
if command -v npm &> /dev/null; then
    echo "📦 Installing global npm packages..."
    npm install -g typescript ts-node 2>/dev/null || true
fi

# Check Claude authentication
echo ""
echo "🤖 Checking Claude Code authentication..."

# If 1Password CLI is available, fetch credentials from vault
if [ "${PATINA_USE_1PASSWORD}" = "1" ]; then
    echo "🔐 Fetching credentials from 1Password vault..."

    # Check if op CLI is available
    if command -v op &> /dev/null; then
        # Fetch credential from 1Password and save to tmpfs
        if op document get "Patina Claude Max Subscription" --vault Private > ~/.claude-linux/.credentials.json 2>/dev/null; then
            chmod 600 ~/.claude-linux/.credentials.json
            echo "✅ Claude authenticated with Max subscription (from 1Password)"
            echo "   🔒 Credentials in RAM-only storage (tmpfs)"
            echo "   🔒 Credentials never touch disk"
        else
            echo "⚠️  Failed to fetch credentials from 1Password"
            echo ""
            echo "To fix:"
            echo "  1. Ensure you're signed in: op signin"
            echo "  2. Store credential: op document create ~/.patina/claude-linux/.credentials.json --title 'Patina Claude Max Subscription'"
            echo ""
        fi
    else
        echo "⚠️  1Password CLI not available in container"
        echo "   Install op CLI: https://developer.1password.com/docs/cli/get-started/"
    fi
elif [ -f ~/.claude-linux/.credentials.json ]; then
    echo "✅ Claude already authenticated with Max subscription"
    echo "   Credentials shared from ~/.patina/claude-linux/"
else
    echo "⚠️  Claude not authenticated yet"
    echo ""
    echo "To enable autonomous AI work with Max subscription:"
    echo "  1. On your HOST machine (Mac), run: claude login"
    echo "  2. Move credentials: mv ~/.claude/.credentials.json ~/.patina/claude-linux/"
    echo "  3. Credentials will work in ALL patina containers"
    echo ""
    echo "Or use 1Password for secure credential storage:"
    echo "  1. Install op CLI: brew install --cask 1password-cli"
    echo "  2. Store credential: op document create ~/.patina/claude-linux/.credentials.json --title 'Patina Claude Max Subscription'"
    echo "  3. Regenerate devcontainer: patina yolo"
    echo ""
fi

echo "✅ YOLO workspace ready!"
echo ""
echo "💭 Available Commands:"
echo "  • claude 'task' - Autonomous AI assistant (Max subscription shared)"
echo "  • Language tools based on detected stack"
echo "  • git, npm, node - Standard development tools"
echo ""
"#;

        let setup_path = devcontainer_path.join("yolo-setup.sh");
        fs::write(setup_path, setup_script)?;

        // Generate 1Password credential launcher script if op CLI available
        if self.has_1password_cli {
            self.generate_1password_launcher(devcontainer_path)?;
        }

        Ok(())
    }

    fn generate_1password_launcher(&self, devcontainer_path: &Path) -> Result<()> {
        let launcher_script = r#"#!/bin/bash
# 1Password Secure Launcher
# Fetches credentials from 1Password on HOST and injects into container

set -e

echo "🔐 Fetching credentials from 1Password..."

# Fetch credential from 1Password on the host (uses biometric auth)
CRED=$(op document get "Patina Claude Max Subscription" --vault Private 2>/dev/null)

if [ $? -ne 0 ]; then
    echo "❌ Failed to fetch credentials from 1Password"
    echo "   Make sure you're authenticated: op signin"
    exit 1
fi

# Encode credential to pass safely through environment
CRED_B64=$(echo "$CRED" | base64)

# Launch container with credential as environment variable
echo "🚀 Launching container with secure credentials..."
docker compose -f .devcontainer/docker-compose.yml up -d --build

# Inject credential into container's tmpfs
docker exec devcontainer-workspace-1 bash -c "
    echo '$CRED_B64' | base64 -d > /root/.claude-linux/.credentials.json
    chmod 600 /root/.claude-linux/.credentials.json
    echo '✅ Credentials injected into RAM-only storage'
"

echo ""
echo "✅ Container ready with secure credentials!"
echo "   Connect: docker exec -it devcontainer-workspace-1 bash"
echo "   🔒 Credentials in tmpfs (RAM-only)"
echo ""
"#;

        let launcher_path = devcontainer_path.join("launch-secure.sh");
        fs::write(&launcher_path, launcher_script)?;

        // Make executable
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(&launcher_path)?.permissions();
            perms.set_mode(0o755);
            fs::set_permissions(&launcher_path, perms)?;
        }

        Ok(())
    }

    // Helper methods
    fn _needs_custom_dockerfile(&self, features: &[DevContainerFeature]) -> bool {
        features.iter().any(|f| {
            matches!(
                f,
                DevContainerFeature::Foundry { .. }
                    | DevContainerFeature::Dojo { .. }
                    | DevContainerFeature::Cairo { .. }
                    | DevContainerFeature::Scarb { .. }
            )
        })
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
                DevContainerFeature::Solc { .. } | DevContainerFeature::Foundry { .. } => {
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
            }),
        }
    }

    fn get_foundry_install(&self) -> String {
        r#"
# Install Foundry
RUN curl -L https://foundry.paradigm.xyz | bash && \
    /root/.foundry/bin/foundryup && \
    echo 'export PATH="/root/.foundry/bin:$PATH"' >> /etc/bash.bashrc

# Add Foundry to PATH for docker exec
ENV PATH="/root/.foundry/bin:$PATH"

"#
        .to_string()
    }

    fn get_cairo_install(&self) -> String {
        r#"
# Install Cairo
RUN curl --proto '=https' --tlsv1.2 -sSf https://cairo-lang.org/install.sh | sh && \
    echo 'export PATH="/root/.cairo/bin:$PATH"' >> /etc/bash.bashrc

# Add Cairo to PATH for docker exec
ENV PATH="/root/.cairo/bin:$PATH"

"#
        .to_string()
    }

    fn get_dojo_install(&self) -> String {
        r#"
# Install asdf version manager for Dojo/Scarb version management
RUN mkdir -p /root/.local/bin && \
    wget -q https://github.com/asdf-vm/asdf/releases/download/v0.18.0/asdf-v0.18.0-linux-amd64.tar.gz && \
    tar -xzf asdf-v0.18.0-linux-amd64.tar.gz -C /root/.local/bin/ && \
    chmod +x /root/.local/bin/asdf && \
    rm asdf-v0.18.0-linux-amd64.tar.gz

# Configure asdf environment
ENV ASDF_DATA_DIR="/root/.asdf"
ENV PATH="/root/.local/bin:/root/.asdf/shims:$PATH"

# Install Scarb via asdf (multiple versions for compatibility)
RUN /root/.local/bin/asdf plugin add scarb && \
    /root/.local/bin/asdf install scarb 2.10.1 && \
    /root/.local/bin/asdf install scarb 2.12.2 && \
    /root/.local/bin/asdf reshim

# Install Dojo using dojoup (asdf-dojo plugin has bugs)
# Install latest as default, projects can specify version in .tool-versions for scarb
RUN curl -L https://install.dojoengine.org | bash && \
    . /root/.dojo/env && \
    dojoup install && \
    echo '. /root/.dojo/env' >> /etc/bash.bashrc

# Add asdf and Dojo to PATH
RUN echo 'export ASDF_DATA_DIR="/root/.asdf"' >> /etc/bash.bashrc && \
    echo 'export PATH="/root/.dojo/bin:/root/.local/bin:/root/.asdf/shims:$PATH"' >> /etc/bash.bashrc

# Set PATH for docker exec
ENV PATH="/root/.dojo/bin:/root/.local/bin:/root/.asdf/shims:$PATH"

"#.to_string()
    }

    fn get_scarb_install(&self) -> String {
        r#"
# Install Scarb (Cairo package manager)
RUN curl --proto '=https' --tlsv1.2 -sSf https://docs.swmansion.com/scarb/install.sh | bash && \
    echo 'export PATH="/root/.local/bin:$PATH"' >> /etc/bash.bashrc

# Add Scarb to PATH for docker exec
ENV PATH="/root/.local/bin:$PATH"

"#
        .to_string()
    }
}
