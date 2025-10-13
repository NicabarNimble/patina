//! Repository Profile - Data structures for detected languages, tools, and services

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RepoProfile {
    pub languages: HashMap<Language, LanguageInfo>,
    pub tools: HashMap<Tool, ToolInfo>,
    pub services: Vec<Service>,
    pub project_name: Option<String>,
}

impl RepoProfile {
    pub fn add_language(&mut self, lang: Language, info: LanguageInfo) {
        self.languages.insert(lang, info);
    }

    pub fn add_tool(&mut self, tool: Tool, info: ToolInfo) {
        self.tools.insert(tool, info);
    }

    pub fn add_service(&mut self, service: Service) {
        self.services.push(service);
    }

    pub fn add_tool_override(&mut self, tool_name: &str) {
        if let Some(tool) = Tool::from_string(tool_name) {
            self.tools.insert(
                tool,
                ToolInfo {
                    detected_by: vec!["--with flag".to_string()],
                    version: None,
                },
            );
        }
    }

    pub fn exclude_tool(&mut self, tool_name: &str) {
        if let Some(tool) = Tool::from_string(tool_name) {
            self.tools.remove(&tool);
        }
        if let Some(lang) = Language::from_string(tool_name) {
            self.languages.remove(&lang);
        }
    }
}

#[derive(Debug, Clone, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub enum Language {
    Rust,
    Go,
    Python,
    JavaScript,
    TypeScript,
    Solidity,
    Cairo,
    C,
    Cpp,
}

impl Language {
    pub fn from_string(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "rust" | "rs" => Some(Language::Rust),
            "go" | "golang" => Some(Language::Go),
            "python" | "py" => Some(Language::Python),
            "javascript" | "js" => Some(Language::JavaScript),
            "typescript" | "ts" => Some(Language::TypeScript),
            "solidity" | "sol" => Some(Language::Solidity),
            "cairo" => Some(Language::Cairo),
            "c" => Some(Language::C),
            "cpp" | "c++" => Some(Language::Cpp),
            _ => None,
        }
    }
}

impl fmt::Display for Language {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Language::Rust => write!(f, "Rust"),
            Language::Go => write!(f, "Go"),
            Language::Python => write!(f, "Python"),
            Language::JavaScript => write!(f, "JavaScript"),
            Language::TypeScript => write!(f, "TypeScript"),
            Language::Solidity => write!(f, "Solidity"),
            Language::Cairo => write!(f, "Cairo"),
            Language::C => write!(f, "C"),
            Language::Cpp => write!(f, "C++"),
        }
    }
}

#[derive(Debug, Clone, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub enum Tool {
    // Package Managers
    Npm,
    Yarn,
    Pnpm,
    Cargo,
    Poetry,
    Pip,

    // Blockchain Tools
    Foundry,
    Hardhat,
    Truffle,
    MudFramework,
    Dojo,
    Scarb,

    // Dev Tools
    Git,
    Docker,
    DockerCompose,
}

impl Tool {
    pub fn from_string(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "npm" => Some(Tool::Npm),
            "yarn" => Some(Tool::Yarn),
            "pnpm" => Some(Tool::Pnpm),
            "cargo" => Some(Tool::Cargo),
            "poetry" => Some(Tool::Poetry),
            "pip" => Some(Tool::Pip),
            "foundry" | "forge" => Some(Tool::Foundry),
            "hardhat" => Some(Tool::Hardhat),
            "truffle" => Some(Tool::Truffle),
            "mud" | "mud-framework" => Some(Tool::MudFramework),
            "dojo" => Some(Tool::Dojo),
            "scarb" => Some(Tool::Scarb),
            "git" => Some(Tool::Git),
            "docker" => Some(Tool::Docker),
            "docker-compose" => Some(Tool::DockerCompose),
            _ => None,
        }
    }
}

impl fmt::Display for Tool {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Tool::Npm => write!(f, "npm"),
            Tool::Yarn => write!(f, "Yarn"),
            Tool::Pnpm => write!(f, "pnpm"),
            Tool::Cargo => write!(f, "Cargo"),
            Tool::Poetry => write!(f, "Poetry"),
            Tool::Pip => write!(f, "pip"),
            Tool::Foundry => write!(f, "Foundry"),
            Tool::Hardhat => write!(f, "Hardhat"),
            Tool::Truffle => write!(f, "Truffle"),
            Tool::MudFramework => write!(f, "MUD Framework"),
            Tool::Dojo => write!(f, "Dojo"),
            Tool::Scarb => write!(f, "Scarb"),
            Tool::Git => write!(f, "Git"),
            Tool::Docker => write!(f, "Docker"),
            Tool::DockerCompose => write!(f, "Docker Compose"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LanguageInfo {
    pub detected_by: Vec<String>,
    pub version: Option<String>,
    pub file_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolInfo {
    pub detected_by: Vec<String>,
    pub version: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Service {
    pub name: String,
    pub image: Option<String>,
    pub ports: Vec<u16>,
}
