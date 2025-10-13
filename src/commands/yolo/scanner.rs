//! Repository Scanner - Detects languages, tools, and services

use anyhow::Result;
use ignore::WalkBuilder;
use std::path::{Path, PathBuf};

use super::profile::{Language, LanguageInfo, RepoProfile, Service, Tool, ToolInfo};

pub struct Scanner {
    root_path: PathBuf,
}

impl Scanner {
    pub fn new(path: &Path) -> Self {
        Self {
            root_path: path.to_path_buf(),
        }
    }

    pub fn scan(&self) -> Result<RepoProfile> {
        let mut profile = RepoProfile::default();

        // Scan for manifests
        self.scan_manifests(&mut profile)?;

        // Scan for config files
        self.scan_configs(&mut profile)?;

        // Scan for source files
        self.scan_source_files(&mut profile)?;

        // Scan for services
        self.scan_services(&mut profile)?;

        // Apply smart inference
        self.apply_smart_inference(&mut profile)?;

        Ok(profile)
    }

    fn scan_manifests(&self, profile: &mut RepoProfile) -> Result<()> {
        // Node.js / JavaScript
        if self.root_path.join("package.json").exists() {
            let detected_by = vec!["package.json".to_string()];
            let version = self.read_node_version()?;

            profile.add_language(
                Language::JavaScript,
                LanguageInfo {
                    detected_by,
                    version,
                    file_count: 0, // Will be updated by scan_source_files
                },
            );

            // Detect package manager
            if self.root_path.join("pnpm-lock.yaml").exists() {
                profile.add_tool(
                    Tool::Pnpm,
                    ToolInfo {
                        detected_by: vec!["pnpm-lock.yaml".to_string()],
                        version: self.extract_pnpm_version()?,
                    },
                );
            } else if self.root_path.join("yarn.lock").exists() {
                profile.add_tool(
                    Tool::Yarn,
                    ToolInfo {
                        detected_by: vec!["yarn.lock".to_string()],
                        version: None,
                    },
                );
            } else if self.root_path.join("package-lock.json").exists() {
                profile.add_tool(
                    Tool::Npm,
                    ToolInfo {
                        detected_by: vec!["package-lock.json".to_string()],
                        version: None,
                    },
                );
            }
        }

        // Rust
        if self.root_path.join("Cargo.toml").exists() {
            let detected_by = vec!["Cargo.toml".to_string()];
            let version = self.read_rust_version()?;

            profile.add_language(
                Language::Rust,
                LanguageInfo {
                    detected_by,
                    version,
                    file_count: 0,
                },
            );
        }

        // Python
        if self.root_path.join("requirements.txt").exists()
            || self.root_path.join("pyproject.toml").exists()
            || self.root_path.join("setup.py").exists()
        {
            let mut detected_by = vec![];
            if self.root_path.join("requirements.txt").exists() {
                detected_by.push("requirements.txt".to_string());
            }
            if self.root_path.join("pyproject.toml").exists() {
                detected_by.push("pyproject.toml".to_string());
            }
            if self.root_path.join("setup.py").exists() {
                detected_by.push("setup.py".to_string());
            }

            let version = self.read_python_version()?;

            profile.add_language(
                Language::Python,
                LanguageInfo {
                    detected_by,
                    version,
                    file_count: 0,
                },
            );
        }

        // Go
        if self.root_path.join("go.mod").exists() {
            let detected_by = vec!["go.mod".to_string()];
            let version = self.read_go_version()?;

            profile.add_language(
                Language::Go,
                LanguageInfo {
                    detected_by,
                    version,
                    file_count: 0,
                },
            );
        }

        Ok(())
    }

    fn scan_configs(&self, profile: &mut RepoProfile) -> Result<()> {
        // Foundry (Solidity framework)
        if self.root_path.join("foundry.toml").exists() {
            profile.add_tool(
                Tool::Foundry,
                ToolInfo {
                    detected_by: vec!["foundry.toml".to_string()],
                    version: self.extract_foundry_version()?,
                },
            );

            // Also implies Solidity
            profile.add_language(
                Language::Solidity,
                LanguageInfo {
                    detected_by: vec!["foundry.toml".to_string()],
                    version: self.extract_solc_version()?,
                    file_count: 0,
                },
            );
        }

        // Hardhat
        if self.root_path.join("hardhat.config.js").exists()
            || self.root_path.join("hardhat.config.ts").exists()
        {
            profile.add_tool(
                Tool::Hardhat,
                ToolInfo {
                    detected_by: vec!["hardhat.config.*".to_string()],
                    version: None,
                },
            );
        }

        // MUD Framework
        if self.root_path.join("mud.config.ts").exists() {
            profile.add_tool(
                Tool::MudFramework,
                ToolInfo {
                    detected_by: vec!["mud.config.ts".to_string()],
                    version: None,
                },
            );
        }

        // TypeScript
        if self.root_path.join("tsconfig.json").exists() {
            profile.add_language(
                Language::TypeScript,
                LanguageInfo {
                    detected_by: vec!["tsconfig.json".to_string()],
                    version: None,
                    file_count: 0,
                },
            );
        }

        // Dojo Framework (Cairo game engine)
        // Check for dojo config files (dojo_dev.toml, dojo_sepolia.toml, etc.)
        // Look in root directory and contracts/ subdirectory (common location)
        let dojo_configs = vec![
            "dojo_dev.toml",
            "dojo_sepolia.toml",
            "dojo_mainnet.toml",
            "dojo.toml",
        ];
        let has_dojo_config = dojo_configs.iter().any(|config| {
            self.root_path.join(config).exists()
                || self.root_path.join("contracts").join(config).exists()
        });

        if has_dojo_config {
            profile.add_tool(
                Tool::Dojo,
                ToolInfo {
                    detected_by: vec!["dojo_*.toml".to_string()],
                    version: None,
                },
            );

            // Dojo projects also need Scarb (Cairo package manager)
            // Check root and contracts/ subdirectory
            if self.root_path.join("Scarb.toml").exists()
                || self.root_path.join("contracts/Scarb.toml").exists()
            {
                profile.add_tool(
                    Tool::Scarb,
                    ToolInfo {
                        detected_by: vec!["Scarb.toml (Dojo project)".to_string()],
                        version: self.extract_scarb_version()?,
                    },
                );
            }

            // Also implies Cairo
            if !profile.languages.contains_key(&Language::Cairo) {
                profile.add_language(
                    Language::Cairo,
                    LanguageInfo {
                        detected_by: vec!["dojo_*.toml".to_string()],
                        version: self.extract_cairo_version()?,
                        file_count: 0,
                    },
                );
            }
        }

        Ok(())
    }

    fn scan_source_files(&self, profile: &mut RepoProfile) -> Result<()> {
        // Count Solidity files
        let sol_files = self.count_files_with_extension("sol")?;
        if sol_files > 0 && !profile.languages.contains_key(&Language::Solidity) {
            profile.add_language(
                Language::Solidity,
                LanguageInfo {
                    detected_by: vec![format!("{} .sol files", sol_files)],
                    version: None,
                    file_count: sol_files,
                },
            );
        }

        // Count Cairo files
        let cairo_files = self.count_files_with_extension("cairo")?;
        if cairo_files > 0 {
            profile.add_language(
                Language::Cairo,
                LanguageInfo {
                    detected_by: vec![format!("{} .cairo files", cairo_files)],
                    version: None,
                    file_count: cairo_files,
                },
            );
        }

        // Update file counts for detected languages
        if profile.languages.contains_key(&Language::JavaScript) {
            let js_count =
                self.count_files_with_extension("js")? + self.count_files_with_extension("jsx")?;
            if let Some(info) = profile.languages.get_mut(&Language::JavaScript) {
                info.file_count = js_count;
            }
        }

        if profile.languages.contains_key(&Language::TypeScript) {
            let ts_count =
                self.count_files_with_extension("ts")? + self.count_files_with_extension("tsx")?;
            if let Some(info) = profile.languages.get_mut(&Language::TypeScript) {
                info.file_count = ts_count;
            }
        }

        Ok(())
    }

    fn scan_services(&self, profile: &mut RepoProfile) -> Result<()> {
        // Check for docker-compose files
        if self.root_path.join("docker-compose.yml").exists()
            || self.root_path.join("docker-compose.yaml").exists()
        {
            // TODO: Parse docker-compose and extract services
        }

        // Check for mprocs.yaml (process manager)
        if self.root_path.join("mprocs.yaml").exists() {
            // TODO: Parse mprocs and extract processes
            // For now, just note it exists
            profile.add_service(Service {
                name: "mprocs".to_string(),
                image: None,
                ports: vec![],
            });
        }

        Ok(())
    }

    fn apply_smart_inference(&self, profile: &mut RepoProfile) -> Result<()> {
        // If we have Foundry, we need Anvil for local blockchain
        if profile.tools.contains_key(&Tool::Foundry) {
            profile.add_service(Service {
                name: "anvil".to_string(),
                image: Some("ghcr.io/foundry-rs/foundry:latest".to_string()),
                ports: vec![8545],
            });
        }

        // If we have MUD Framework, we likely need an indexer
        if profile.tools.contains_key(&Tool::MudFramework) {
            profile.add_service(Service {
                name: "indexer".to_string(),
                image: Some("ghcr.io/latticexyz/store-indexer:latest".to_string()),
                ports: vec![],
            });
        }

        Ok(())
    }

    // Helper methods
    fn count_files_with_extension(&self, ext: &str) -> Result<usize> {
        // Use ignore crate which respects .gitignore and common patterns
        let count = WalkBuilder::new(&self.root_path)
            .hidden(false) // Include hidden files
            .git_ignore(true) // Respect .gitignore
            .git_global(true) // Respect global gitignore
            .git_exclude(true) // Respect .git/info/exclude
            .require_git(false) // Work even if not a git repo
            .max_depth(Some(10)) // Limit depth to avoid infinite recursion
            .build()
            .filter_map(Result::ok)
            .filter(|entry| {
                entry.file_type().map(|ft| ft.is_file()).unwrap_or(false)
                    && entry.path().extension().and_then(|e| e.to_str()) == Some(ext)
            })
            .count();
        Ok(count)
    }

    fn read_node_version(&self) -> Result<Option<String>> {
        // Check .nvmrc first
        let nvmrc_path = self.root_path.join(".nvmrc");
        if nvmrc_path.exists() {
            let content = std::fs::read_to_string(nvmrc_path)?;
            return Ok(Some(content.trim().to_string()));
        }

        // TODO: Parse package.json engines field
        Ok(None)
    }

    fn read_rust_version(&self) -> Result<Option<String>> {
        let toolchain_path = self.root_path.join("rust-toolchain.toml");
        if toolchain_path.exists() {
            // TODO: Parse rust-toolchain.toml
        }
        Ok(None)
    }

    fn read_python_version(&self) -> Result<Option<String>> {
        let version_path = self.root_path.join(".python-version");
        if version_path.exists() {
            let content = std::fs::read_to_string(version_path)?;
            return Ok(Some(content.trim().to_string()));
        }
        Ok(None)
    }

    fn read_go_version(&self) -> Result<Option<String>> {
        // TODO: Parse go.mod for go version
        Ok(None)
    }

    fn extract_pnpm_version(&self) -> Result<Option<String>> {
        // TODO: Parse from package.json packageManager field
        Ok(None)
    }

    fn extract_foundry_version(&self) -> Result<Option<String>> {
        // TODO: Parse foundry.toml
        Ok(None)
    }

    fn extract_solc_version(&self) -> Result<Option<String>> {
        // TODO: Parse foundry.toml for solc version
        Ok(None)
    }

    fn extract_scarb_version(&self) -> Result<Option<String>> {
        // TODO: Parse Scarb.toml for cairo-version
        Ok(None)
    }

    fn extract_cairo_version(&self) -> Result<Option<String>> {
        // TODO: Parse Scarb.toml for cairo-version field
        Ok(None)
    }
}
