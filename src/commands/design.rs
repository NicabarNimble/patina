use anyhow::Result;
use clap::Args;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use toml::Value;
use std::collections::HashMap;

#[derive(Args, Debug)]
pub struct DesignCommand {
    /// Output path for the design TOML file
    #[arg(short, long, default_value = "PROJECT_DESIGN.toml")]
    output: PathBuf,

    /// Skip environment scanning
    #[arg(long)]
    no_scan: bool,

    /// Non-interactive mode (requires all values via flags)
    #[arg(long)]
    non_interactive: bool,
}

#[derive(Debug)]
struct ProjectContext {
    language: Option<String>,
    project_type: Option<String>,
    dependencies: Vec<String>,
    has_tests: bool,
    has_docker: bool,
    has_ci: bool,
    existing_commands: HashMap<String, String>,
}

#[derive(Debug)]
struct DesignAnswers {
    project_name: String,
    project_type: String,
    purpose: String,
    problem: String,
    solution: String,
    users: String,
    value: String,
    patterns: Vec<String>,
    architecture: String,
    core_abstractions: Vec<String>,
    core_features: Vec<String>,
    future_features: Vec<String>,
    non_goals: Vec<String>,
    language: String,
    dependencies: Vec<String>,
    constraints: Vec<String>,
}

impl DesignCommand {
    pub async fn execute(&self) -> Result<()> {
        println!("🎨 Patina Design Wizard\n");

        // Step 1: Scan environment unless disabled
        let context = if !self.no_scan {
            println!("🔍 Analyzing your project environment...");
            self.scan_environment().await?
        } else {
            ProjectContext::default()
        };

        // Display scan results
        if !self.no_scan {
            self.display_scan_results(&context);
        }

        // Step 2: Interactive interview
        let answers = if !self.non_interactive {
            self.conduct_interview(&context).await?
        } else {
            return Err(anyhow::anyhow!("Non-interactive mode not yet implemented"));
        };

        // Step 3: Generate TOML
        let design_toml = self.generate_toml(&answers)?;

        // Step 4: Review and refine
        let final_toml = if !self.non_interactive {
            self.review_and_refine(design_toml).await?
        } else {
            design_toml
        };

        // Step 5: Write to file
        std::fs::write(&self.output, final_toml)?;
        println!("\n✅ Design document created at: {}", self.output.display());

        Ok(())
    }

    async fn scan_environment(&self) -> Result<ProjectContext> {
        let mut context = ProjectContext::default();

        // Check for language indicators
        if Path::new("Cargo.toml").exists() {
            context.language = Some("rust".to_string());
            context.project_type = Some("rust".to_string());
            
            // Parse Cargo.toml for dependencies
            if let Ok(content) = std::fs::read_to_string("Cargo.toml") {
                if let Ok(cargo_toml) = toml::from_str::<Value>(&content) {
                    if let Some(deps) = cargo_toml.get("dependencies").and_then(|d| d.as_table()) {
                        context.dependencies = deps.keys().cloned().collect();
                    }
                }
            }
        } else if Path::new("package.json").exists() {
            context.language = Some("javascript".to_string());
            context.project_type = Some("node".to_string());
        } else if Path::new("go.mod").exists() {
            context.language = Some("go".to_string());
            context.project_type = Some("go".to_string());
        } else if Path::new("pyproject.toml").exists() || Path::new("setup.py").exists() {
            context.language = Some("python".to_string());
            context.project_type = Some("python".to_string());
        }

        // Check for common patterns
        context.has_tests = Path::new("tests").exists() || 
                           Path::new("test").exists() || 
                           Path::new("spec").exists();
        
        context.has_docker = Path::new("Dockerfile").exists() || 
                            Path::new("docker-compose.yml").exists();
        
        context.has_ci = Path::new(".github/workflows").exists() || 
                        Path::new(".gitlab-ci.yml").exists() ||
                        Path::new(".circleci").exists();

        // Check for Makefile or scripts
        if Path::new("Makefile").exists() {
            if let Ok(content) = std::fs::read_to_string("Makefile") {
                // Simple parser for common make targets
                for line in content.lines() {
                    if let Some(target) = line.strip_suffix(':') {
                        if !line.starts_with('\t') && !line.starts_with(' ') {
                            let target = target.trim();
                            if matches!(target, "test" | "build" | "run" | "lint" | "format" | "check") {
                                context.existing_commands.insert(
                                    target.to_string(),
                                    format!("make {}", target)
                                );
                            }
                        }
                    }
                }
            }
        }

        Ok(context)
    }

    fn display_scan_results(&self, context: &ProjectContext) {
        println!("\n📊 Environment scan results:");
        if let Some(lang) = &context.language {
            println!("   Language: {}", lang);
        }
        if !context.dependencies.is_empty() {
            println!("   Dependencies found: {}", context.dependencies.len());
        }
        if context.has_tests {
            println!("   ✓ Test directory detected");
        }
        if context.has_docker {
            println!("   ✓ Docker configuration found");
        }
        if context.has_ci {
            println!("   ✓ CI/CD configuration found");
        }
        if !context.existing_commands.is_empty() {
            println!("   ✓ Build commands detected");
        }
        println!();
    }

    async fn conduct_interview(&self, context: &ProjectContext) -> Result<DesignAnswers> {
        let mut answers = DesignAnswers {
            project_name: String::new(),
            project_type: String::new(),
            purpose: String::new(),
            problem: String::new(),
            solution: String::new(),
            users: String::new(),
            value: String::new(),
            patterns: Vec::new(),
            architecture: String::new(),
            core_abstractions: Vec::new(),
            core_features: Vec::new(),
            future_features: Vec::new(),
            non_goals: Vec::new(),
            language: context.language.clone().unwrap_or_default(),
            dependencies: context.dependencies.clone(),
            constraints: Vec::new(),
        };

        println!("Let's design your project together. Press Enter to use [defaults].\n");

        // Project basics
        answers.project_name = self.prompt("Project name", 
            Path::new(".").file_name()
                .and_then(|n| n.to_str())
                .map(|s| s.to_string()))?;

        println!("\nWhat type of project is this?");
        println!("  1. CLI tool");
        println!("  2. Web service/API");
        println!("  3. Library");
        println!("  4. Application");
        println!("  5. Other");
        
        let project_type_num = self.prompt("Select (1-5)", Some("1".to_string()))?;
        answers.project_type = match project_type_num.as_str() {
            "1" => "tool",
            "2" => "service",
            "3" => "library",
            "4" => "application",
            _ => "other",
        }.to_string();

        // Purpose and problem
        answers.purpose = self.prompt_multiline(
            "What's the one-line purpose of this project?",
            None
        )?;

        answers.problem = self.prompt_multiline(
            "What problem does this solve? (2-3 sentences)",
            None
        )?;

        answers.solution = self.prompt_multiline(
            "How does it solve this problem?",
            None
        )?;

        // Users and value
        answers.users = self.prompt(
            "Who will use this? (e.g., 'developers', 'data scientists', 'end users')",
            Some("developers".to_string())
        )?;

        answers.value = self.prompt_multiline(
            "What's the core value proposition? (one line)",
            None
        )?;

        // Technical details
        println!("\n🏗️  Architecture & Patterns");
        
        let patterns_input = self.prompt_multiline(
            "Key design patterns or principles? (comma-separated, or Enter to skip)",
            Some("".to_string())
        )?;
        if !patterns_input.trim().is_empty() {
            answers.patterns = patterns_input.split(',')
                .map(|s| s.trim().to_string())
                .collect();
        }

        answers.architecture = self.prompt_multiline(
            "High-level architecture description",
            Some("Modular design with clear separation of concerns".to_string())
        )?;

        // Features
        println!("\n✨ Features");
        
        let core_features = self.prompt_multiline(
            "Core features (comma-separated)",
            None
        )?;
        answers.core_features = core_features.split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();

        let future_features = self.prompt_multiline(
            "Future features to consider (comma-separated, or Enter to skip)",
            Some("".to_string())
        )?;
        if !future_features.trim().is_empty() {
            answers.future_features = future_features.split(',')
                .map(|s| s.trim().to_string())
                .collect();
        }

        let non_goals = self.prompt_multiline(
            "What is explicitly NOT a goal? (comma-separated, or Enter to skip)",
            Some("".to_string())
        )?;
        if !non_goals.trim().is_empty() {
            answers.non_goals = non_goals.split(',')
                .map(|s| s.trim().to_string())
                .collect();
        }

        Ok(answers)
    }

    fn prompt(&self, question: &str, default: Option<String>) -> Result<String> {
        print!("{}", question);
        if let Some(ref def) = default {
            print!(" [{}]", def);
        }
        print!(": ");
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let trimmed = input.trim();

        Ok(if trimmed.is_empty() && default.is_some() {
            default.unwrap()
        } else {
            trimmed.to_string()
        })
    }

    fn prompt_multiline(&self, question: &str, default: Option<String>) -> Result<String> {
        println!("{}", question);
        if let Some(ref def) = default {
            println!("(Enter for default: {})", def);
        } else {
            println!("(Press Enter twice to finish)");
        }

        let mut lines = Vec::new();
        let mut empty_count = 0;

        loop {
            let mut input = String::new();
            io::stdin().read_line(&mut input)?;
            
            if input.trim().is_empty() {
                empty_count += 1;
                if empty_count >= 1 && (default.is_some() || !lines.is_empty()) {
                    break;
                }
            } else {
                empty_count = 0;
                lines.push(input.trim().to_string());
            }
        }

        Ok(if lines.is_empty() && default.is_some() {
            default.unwrap()
        } else {
            lines.join(" ")
        })
    }

    fn generate_toml(&self, answers: &DesignAnswers) -> Result<String> {
        let mut toml = String::new();
        
        // [project] section
        toml.push_str("[project]\n");
        toml.push_str(&format!("name = \"{}\"\n", answers.project_name));
        toml.push_str(&format!("type = \"{}\"\n", answers.project_type));
        toml.push_str(&format!("purpose = \"{}\"\n", answers.purpose));
        toml.push_str("\n");

        // [why] section
        toml.push_str("[why]\n");
        toml.push_str(&format!("problem = \"{}\"\n", answers.problem));
        toml.push_str(&format!("solution = \"{}\"\n", answers.solution));
        toml.push_str(&format!("users = \"{}\"\n", answers.users));
        toml.push_str(&format!("value = \"{}\"\n", answers.value));
        toml.push_str("\n");

        // [how] section
        toml.push_str("[how]\n");
        if !answers.patterns.is_empty() {
            toml.push_str("patterns = [\n");
            for pattern in &answers.patterns {
                toml.push_str(&format!("    \"{}\",\n", pattern));
            }
            toml.push_str("]\n");
        }
        toml.push_str(&format!("architecture = \"{}\"\n", answers.architecture));
        if !answers.core_abstractions.is_empty() {
            toml.push_str("core_abstractions = [\n");
            for abstraction in &answers.core_abstractions {
                toml.push_str(&format!("    \"{}\",\n", abstraction));
            }
            toml.push_str("]\n");
        }
        toml.push_str("\n");

        // [what] section
        toml.push_str("[what]\n");
        if !answers.core_features.is_empty() {
            toml.push_str("core_features = [\n");
            for feature in &answers.core_features {
                toml.push_str(&format!("    \"{}\",\n", feature));
            }
            toml.push_str("]\n");
        }
        if !answers.future_features.is_empty() {
            toml.push_str("future_features = [\n");
            for feature in &answers.future_features {
                toml.push_str(&format!("    \"{}\",\n", feature));
            }
            toml.push_str("]\n");
        }
        if !answers.non_goals.is_empty() {
            toml.push_str("non_goals = [\n");
            for goal in &answers.non_goals {
                toml.push_str(&format!("    \"{}\",\n", goal));
            }
            toml.push_str("]\n");
        }
        toml.push_str("\n");

        // [technical] section
        toml.push_str("[technical]\n");
        toml.push_str(&format!("language = \"{}\"\n", answers.language));
        if !answers.dependencies.is_empty() {
            toml.push_str("dependencies = [\n");
            for dep in &answers.dependencies {
                toml.push_str(&format!("    \"{}\",\n", dep));
            }
            toml.push_str("]\n");
        }
        if !answers.constraints.is_empty() {
            toml.push_str("constraints = [\n");
            for constraint in &answers.constraints {
                toml.push_str(&format!("    \"{}\",\n", constraint));
            }
            toml.push_str("]\n");
        }
        toml.push_str("\n");

        // [development] section
        toml.push_str("[development]\n");
        toml.push_str("[development.commands]\n");
        toml.push_str("# TODO: Add your development commands\n");
        toml.push_str("\n");

        Ok(toml)
    }

    async fn review_and_refine(&self, toml: String) -> Result<String> {
        println!("\n📋 Generated Design Document:");
        println!("================================");
        println!("{}", toml);
        println!("================================\n");

        let response = self.prompt(
            "Would you like to refine this? (y/n)",
            Some("n".to_string())
        )?;

        if response.to_lowercase() == "y" {
            println!("\nRefinement options:");
            println!("  1. Add technical constraints");
            println!("  2. Expand architecture section");
            println!("  3. Add development commands");
            println!("  4. Edit manually");
            println!("  5. Done");

            let choice = self.prompt("Select option (1-5)", Some("5".to_string()))?;
            
            match choice.as_str() {
                "1" | "2" | "3" => {
                    println!("Interactive refinement coming soon!");
                    Ok(toml)
                }
                "4" => {
                    println!("Opening in editor coming soon!");
                    Ok(toml)
                }
                _ => Ok(toml)
            }
        } else {
            Ok(toml)
        }
    }
}

impl Default for ProjectContext {
    fn default() -> Self {
        Self {
            language: None,
            project_type: None,
            dependencies: Vec::new(),
            has_tests: false,
            has_docker: false,
            has_ci: false,
            existing_commands: HashMap::new(),
        }
    }
}