use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::env;
use std::process::Command;

#[derive(Debug, Serialize, Deserialize)]
pub struct Environment {
    pub os: String,
    pub arch: String,
    pub home_dir: String,
    pub current_dir: String,
    pub tools: HashMap<String, ToolInfo>,
    pub languages: HashMap<String, LanguageInfo>,
    pub env_vars: HashMap<String, String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ToolInfo {
    pub available: bool,
    pub version: Option<String>,
    pub path: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LanguageInfo {
    pub available: bool,
    pub version: Option<String>,
    pub toolchain: Option<String>,
}

impl Environment {
    pub fn detect() -> Result<Self> {
        let mut env = Environment {
            os: env::consts::OS.to_string(),
            arch: env::consts::ARCH.to_string(),
            home_dir: dirs::home_dir()
                .map(|p| p.display().to_string())
                .unwrap_or_default(),
            current_dir: env::current_dir()?
                .display()
                .to_string(),
            tools: HashMap::new(),
            languages: HashMap::new(),
            env_vars: HashMap::new(),
        };
        
        // Detect common development tools
        env.detect_tools();
        
        // Detect programming languages
        env.detect_languages();
        
        // Collect relevant environment variables
        env.collect_env_vars();
        
        Ok(env)
    }
    
    fn detect_tools(&mut self) {
        let tools_to_check = vec![
            ("git", &["--version"]),
            ("docker", &["--version"]),
            ("docker-compose", &["--version"]),
            ("make", &["--version"]),
            ("cmake", &["--version"]),
            ("npm", &["--version"]),
            ("yarn", &["--version"]),
            ("pnpm", &["--version"]),
            ("brew", &["--version"]),
            ("apt", &["--version"]),
            ("yum", &["--version"]),
            ("code", &["--version"]),
            ("vim", &["--version"]),
            ("nvim", &["--version"]),
            ("psql", &["--version"]),
            ("mysql", &["--version"]),
            ("redis-cli", &["--version"]),
        ];
        
        for (tool_name, args) in tools_to_check {
            let mut tool_info = ToolInfo {
                available: false,
                version: None,
                path: None,
            };
            
            // Check if tool exists
            if let Ok(path) = which::which(tool_name) {
                tool_info.available = true;
                tool_info.path = Some(path.display().to_string());
                
                // Try to get version
                if let Ok(output) = Command::new(tool_name).args(args).output() {
                    let version_str = String::from_utf8_lossy(&output.stdout);
                    if !version_str.is_empty() {
                        tool_info.version = Some(version_str.lines().next().unwrap_or("").to_string());
                    }
                }
            }
            
            self.tools.insert(tool_name.to_string(), tool_info);
        }
    }
    
    fn detect_languages(&mut self) {
        // Rust
        let mut rust_info = LanguageInfo {
            available: false,
            version: None,
            toolchain: None,
        };
        
        if let Ok(output) = Command::new("rustc").arg("--version").output() {
            rust_info.available = true;
            rust_info.version = Some(String::from_utf8_lossy(&output.stdout).trim().to_string());
            
            if let Ok(toolchain_output) = Command::new("rustup").args(&["show", "active-toolchain"]).output() {
                rust_info.toolchain = Some(String::from_utf8_lossy(&toolchain_output.stdout).trim().to_string());
            }
        }
        self.languages.insert("rust".to_string(), rust_info);
        
        self.detect_rust_tools();
        
        // Python
        let mut python_info = LanguageInfo {
            available: false,
            version: None,
            toolchain: None,
        };
        
        if let Ok(output) = Command::new("python3").arg("--version").output() {
            python_info.available = true;
            python_info.version = Some(String::from_utf8_lossy(&output.stdout).trim().to_string());
        } else if let Ok(output) = Command::new("python").arg("--version").output() {
            python_info.available = true;
            python_info.version = Some(String::from_utf8_lossy(&output.stdout).trim().to_string());
        }
        self.languages.insert("python".to_string(), python_info);
        
        // Node.js
        let mut node_info = LanguageInfo {
            available: false,
            version: None,
            toolchain: None,
        };
        
        if let Ok(output) = Command::new("node").arg("--version").output() {
            node_info.available = true;
            node_info.version = Some(String::from_utf8_lossy(&output.stdout).trim().to_string());
        }
        self.languages.insert("node".to_string(), node_info);
    }
    
    fn detect_rust_tools(&mut self) {
        let rust_tools = vec![
            ("cargo", &["--version"]),
            ("cargo-watch", &["--version"]),
            ("cargo-edit", &["--version"]),
            ("cargo-expand", &["--version"]),
            ("cargo-audit", &["--version"]),
            ("cargo-outdated", &["--version"]),
            ("cargo-release", &["--version"]),
            ("cargo-fmt", &["--version"]),
            ("cargo-clippy", &["--version"]),
            ("sccache", &["--version"]),
            ("wasm-pack", &["--version"]),
            ("trunk", &["--version"]),
            ("sqlx", &["--version"]),
            ("sea-orm-cli", &["--version"]),
        ];
        
        for (tool_name, args) in rust_tools {
            let mut tool_info = ToolInfo {
                available: false,
                version: None,
                path: None,
            };
            
            if let Ok(path) = which::which(tool_name) {
                tool_info.available = true;
                tool_info.path = Some(path.display().to_string());
                
                if let Ok(output) = Command::new(tool_name).args(args).output() {
                    let version_str = String::from_utf8_lossy(&output.stdout);
                    if !version_str.is_empty() {
                        tool_info.version = Some(version_str.lines().next().unwrap_or("").to_string());
                    }
                }
            }
            
            self.tools.insert(tool_name.to_string(), tool_info);
        }
    }
    
    fn collect_env_vars(&mut self) {
        let relevant_vars = vec![
            "SHELL",
            "EDITOR",
            "VISUAL",
            "TERM",
            "USER",
            "HOME",
            "PATH",
            "LANG",
            "LC_ALL",
            "DOCKER_HOST",
            "VIRTUAL_ENV",
            "CONDA_DEFAULT_ENV",
            "NVM_DIR",
            "CARGO_HOME",
            "RUSTUP_HOME",
            "GOPATH",
            "CI",
        ];
        
        for var_name in relevant_vars {
            if let Ok(value) = env::var(var_name) {
                // Truncate PATH to avoid huge values
                if var_name == "PATH" {
                    let paths: Vec<&str> = value.split(':').collect();
                    let truncated = if paths.len() > 5 {
                        format!("{} paths (first 5: {}...)", 
                            paths.len(),
                            paths[..5].join(":")
                        )
                    } else {
                        value
                    };
                    self.env_vars.insert(var_name.to_string(), truncated);
                } else {
                    self.env_vars.insert(var_name.to_string(), value);
                }
            }
        }
    }
    
    pub fn to_markdown(&self) -> String {
        let mut md = String::new();
        
        md.push_str("## Environment\n\n");
        md.push_str(&format!("- **OS**: {} ({})\n", self.os, self.arch));
        md.push_str(&format!("- **Home**: {}\n", self.home_dir));
        md.push_str(&format!("- **Working Directory**: {}\n", self.current_dir));
        md.push_str("\n");
        
        md.push_str("### Development Tools\n\n");
        for (tool, info) in &self.tools {
            if info.available {
                md.push_str(&format!("- **{}**: ✓ ", tool));
                if let Some(version) = &info.version {
                    md.push_str(&format!("({})", version));
                }
                md.push_str("\n");
            }
        }
        md.push_str("\n");
        
        md.push_str("### Languages\n\n");
        for (lang, info) in &self.languages {
            if info.available {
                md.push_str(&format!("- **{}**: ", lang));
                if let Some(version) = &info.version {
                    md.push_str(version);
                }
                if let Some(toolchain) = &info.toolchain {
                    md.push_str(&format!(" ({})", toolchain));
                }
                md.push_str("\n");
            }
        }
        md.push_str("\n");
        
        md.push_str("### Key Environment Variables\n\n");
        for (var, value) in &self.env_vars {
            if var != "PATH" {  // Special handling for PATH above
                md.push_str(&format!("- **{}**: {}\n", var, value));
            }
        }
        
        md
    }
}