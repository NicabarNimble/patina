//! Feature Mapper - Maps repository profile to Dev Container Features

use anyhow::Result;
use serde::{Serialize, Deserialize};
use super::profile::{RepoProfile, Language, Tool};

pub struct FeatureMapper;

impl FeatureMapper {
    pub fn new() -> Self {
        Self
    }

    pub fn map_profile(&self, profile: &RepoProfile) -> Result<Vec<DevContainerFeature>> {
        let mut features = Vec::new();

        // Map languages to features
        for (lang, info) in &profile.languages {
            match lang {
                Language::JavaScript | Language::TypeScript => {
                    // Node.js feature
                    if !features.iter().any(|f| matches!(f, DevContainerFeature::Node { .. })) {
                        features.push(DevContainerFeature::Node {
                            version: info.version.clone(),
                        });
                    }
                }
                Language::Rust => {
                    features.push(DevContainerFeature::Rust {
                        version: info.version.clone().unwrap_or_else(|| "stable".to_string()),
                    });
                }
                Language::Python => {
                    features.push(DevContainerFeature::Python {
                        version: info.version.clone().unwrap_or_else(|| "3.11".to_string()),
                    });
                }
                Language::Go => {
                    features.push(DevContainerFeature::Go {
                        version: info.version.clone().unwrap_or_else(|| "latest".to_string()),
                    });
                }
                Language::Solidity => {
                    // Solidity needs special handling
                    if !profile.tools.contains_key(&Tool::Foundry) {
                        features.push(DevContainerFeature::Solc {
                            version: info.version.clone().unwrap_or_else(|| "0.8.30".to_string()),
                        });
                    }
                }
                Language::Cairo => {
                    // Only add Cairo if Dojo is not present
                    // Dojo projects will be handled by Tool::Dojo below
                    if !profile.tools.contains_key(&Tool::Dojo) {
                        features.push(DevContainerFeature::Cairo {
                            version: info.version.clone().unwrap_or_else(|| "latest".to_string()),
                        });
                    }
                }
                _ => {}
            }
        }

        // Map tools to features
        for (tool, info) in &profile.tools {
            match tool {
                Tool::Pnpm => {
                    features.push(DevContainerFeature::Pnpm {
                        version: info.version.clone(),
                    });
                }
                Tool::Yarn => {
                    features.push(DevContainerFeature::Yarn {
                        version: info.version.clone(),
                    });
                }
                Tool::Foundry => {
                    features.push(DevContainerFeature::Foundry {
                        version: info.version.clone().unwrap_or_else(|| "latest".to_string()),
                    });
                }
                Tool::Hardhat => {
                    features.push(DevContainerFeature::Hardhat);
                }
                Tool::MudFramework => {
                    features.push(DevContainerFeature::MudCli);
                }
                Tool::Dojo => {
                    features.push(DevContainerFeature::Dojo {
                        version: info.version.clone().unwrap_or_else(|| "latest".to_string()),
                    });
                }
                Tool::Scarb => {
                    // Skip Scarb if Dojo is present (Dojo installs its own Scarb)
                    if !profile.tools.contains_key(&Tool::Dojo) {
                        features.push(DevContainerFeature::Scarb {
                            version: info.version.clone().unwrap_or_else(|| "latest".to_string()),
                        });
                    }
                }
                Tool::Poetry => {
                    features.push(DevContainerFeature::Poetry);
                }
                _ => {}
            }
        }

        // Always include Git and GitHub CLI for development
        features.push(DevContainerFeature::Git);
        features.push(DevContainerFeature::GitHubCli);

        Ok(features)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DevContainerFeature {
    // Language Features
    Node { version: Option<String> },
    Python { version: String },
    Rust { version: String },
    Go { version: String },

    // Blockchain Features
    Foundry { version: String },
    Solc { version: String },
    Cairo { version: String },
    Dojo { version: String },
    Scarb { version: String },
    Hardhat,
    MudCli,

    // Package Managers
    Pnpm { version: Option<String> },
    Yarn { version: Option<String> },
    Poetry,

    // Dev Tools
    Git,
    GitHubCli,
    DockerInDocker,
}

impl DevContainerFeature {
    /// Convert to Dev Container Feature JSON format
    pub fn to_feature_spec(&self) -> (String, serde_json::Value) {
        match self {
            DevContainerFeature::Node { version } => {
                let spec = if let Some(v) = version {
                    serde_json::json!({ "version": v })
                } else {
                    serde_json::json!({})
                };
                ("ghcr.io/devcontainers/features/node:1".to_string(), spec)
            }
            DevContainerFeature::Python { version } => {
                let spec = serde_json::json!({ "version": version });
                ("ghcr.io/devcontainers/features/python:1".to_string(), spec)
            }
            DevContainerFeature::Rust { version } => {
                let spec = serde_json::json!({ "version": version });
                ("ghcr.io/devcontainers/features/rust:1".to_string(), spec)
            }
            DevContainerFeature::Go { version } => {
                let spec = serde_json::json!({ "version": version });
                ("ghcr.io/devcontainers/features/go:1".to_string(), spec)
            }
            DevContainerFeature::Pnpm { version } => {
                let spec = if let Some(v) = version {
                    serde_json::json!({ "version": v })
                } else {
                    serde_json::json!({})
                };
                ("ghcr.io/devcontainers-contrib/features/pnpm:1".to_string(), spec)
            }
            DevContainerFeature::Yarn { version } => {
                let spec = if let Some(v) = version {
                    serde_json::json!({ "version": v })
                } else {
                    serde_json::json!({})
                };
                ("ghcr.io/devcontainers-contrib/features/yarn:1".to_string(), spec)
            }
            DevContainerFeature::Git => {
                ("ghcr.io/devcontainers/features/git:1".to_string(), serde_json::json!({}))
            }
            DevContainerFeature::GitHubCli => {
                ("ghcr.io/devcontainers/features/github-cli:1".to_string(), serde_json::json!({}))
            }
            DevContainerFeature::DockerInDocker => {
                ("ghcr.io/devcontainers/features/docker-in-docker:2".to_string(), serde_json::json!({}))
            }
            // Custom Patina features (will be published to ghcr.io/patina/features/)
            DevContainerFeature::Foundry { version } => {
                let spec = serde_json::json!({ "version": version });
                ("ghcr.io/patina/features/foundry:1".to_string(), spec)
            }
            DevContainerFeature::Cairo { version } => {
                let spec = serde_json::json!({ "version": version });
                ("ghcr.io/patina/features/cairo:1".to_string(), spec)
            }
            DevContainerFeature::Dojo { version } => {
                let spec = serde_json::json!({ "version": version });
                ("ghcr.io/patina/features/dojo:1".to_string(), spec)
            }
            DevContainerFeature::Scarb { version } => {
                let spec = serde_json::json!({ "version": version });
                ("ghcr.io/patina/features/scarb:1".to_string(), spec)
            }
            DevContainerFeature::Solc { version } => {
                let spec = serde_json::json!({ "version": version });
                ("ghcr.io/patina/features/solc:1".to_string(), spec)
            }
            DevContainerFeature::Hardhat => {
                ("ghcr.io/patina/features/hardhat:1".to_string(), serde_json::json!({}))
            }
            DevContainerFeature::MudCli => {
                ("ghcr.io/patina/features/mud:1".to_string(), serde_json::json!({}))
            }
            DevContainerFeature::Poetry => {
                ("ghcr.io/devcontainers-contrib/features/poetry:2".to_string(), serde_json::json!({}))
            }
        }
    }
}