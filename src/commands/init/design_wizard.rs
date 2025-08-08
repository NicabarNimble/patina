use anyhow::Result;
use patina::environment::Environment;
use std::io::{self, Write};

/// Interactive wizard to create PROJECT_DESIGN.toml
pub fn create_project_design_wizard(default_name: &str, env: &Environment) -> Result<String> {
    println!("\n📝 Setting up PROJECT_DESIGN.toml");
    println!("I've detected your environment. Just need one quick question.\n");

    // Show what we detected
    println!("Detected environment:");
    if env.languages.get("rust").is_some_and(|info| info.available) {
        println!("  ✓ Rust (perfect for Patina projects!)");
    }
    if env.tools.get("docker").is_some_and(|info| info.available) {
        println!("  ✓ Docker (can containerize apps)");
    }
    if env.tools.get("go").is_some_and(|info| info.available)
        && env.tools.get("dagger").is_some_and(|info| info.available)
    {
        println!("  ✓ Go + Dagger (advanced CI/CD available)");
    }

    println!("\nProject type:");
    println!("1. app (deployable application)");
    println!("2. tool (CLI/utility)");
    println!("3. library (reusable code)");
    let type_choice = prompt_with_default("Choice", "2")?; // Default to tool
    let project_type = match type_choice.as_str() {
        "1" => "app",
        "2" => "tool",
        "3" => "library",
        _ => "tool",
    };

    // Build smart defaults from environment
    let has_rust = env.languages.get("rust").is_some_and(|info| info.available);
    let has_docker = env.tools.get("docker").is_some_and(|info| info.available);
    let has_go = env.tools.get("go").is_some_and(|info| info.available);

    // Determine primary language
    let primary_language = if has_rust {
        "rust"
    } else {
        "unknown" // Will be determined in session-zero
    };

    // Build required tools based on detected environment
    let mut required_tools = vec!["\"git\""];
    if has_rust {
        required_tools.push("\"rust\"");
        required_tools.push("\"cargo\"");
    }

    // Build recommended tools based on what's available
    let mut recommended_tools = vec![];
    if has_docker {
        recommended_tools.push("\"docker\"");
    }
    if has_go && project_type == "app" {
        recommended_tools.push("\"go\"");
        recommended_tools.push("\"dagger\"");
    }

    // Build development commands based on language
    let dev_commands = if has_rust {
        r#"build = "cargo build --release"
test = "cargo test"
lint = "cargo clippy"
format = "cargo fmt"
run = "cargo run --""#
    } else {
        r#"build = "TODO: Define build command"
test = "TODO: Define test command"
lint = "TODO: Define lint command"
format = "TODO: Define format command""#
    };

    // Generate the TOML with smart defaults
    let toml_content = format!(
        r#"[project]
name = "{}"
type = "{}"
purpose = "TODO: Define project purpose (use /session-zero)"

[why]
problem = "TODO: What problem does this solve? (use /session-zero)"
solution = "TODO: How does it solve it? (use /session-zero)"
users = "TODO: Who are the target users? (use /session-zero)"
value = "TODO: What unique value does this provide? (use /session-zero)"

[how]
architecture = "TODO: High-level architecture (use /session-zero)"
core_abstractions = ["TODO: Define during development"]
patterns = ["TODO: Will emerge from usage"]

[what]
core_features = ["TODO: Define during development"]
future_features = ["TODO: Will emerge from usage"]
non_goals = ["TODO: Define boundaries during development"]

[technical]
language = "{}"
{}dependencies = []

[development]
[development.environment]
required_tools = [{}]
recommended_tools = [{}]

[development.commands]
{}
"#,
        default_name,
        project_type,
        primary_language,
        if primary_language == "rust" {
            "min_rust_version = \"1.75\"\n"
        } else {
            ""
        },
        required_tools.join(", "),
        recommended_tools.join(", "),
        dev_commands
    );

    println!("\n✅ PROJECT_DESIGN.toml will be created with:");
    println!("   - Project type: {project_type}");
    println!("   - Detected language: {primary_language}");
    println!("   - Required tools: {}", required_tools.join(", "));
    if !recommended_tools.is_empty() {
        println!("   - Recommended tools: {}", recommended_tools.join(", "));
    }
    println!("\n💡 Use /session-zero after init to establish project purpose and architecture");

    Ok(toml_content)
}

/// Prompt for user input with a default value
fn prompt_with_default(prompt: &str, default: &str) -> Result<String> {
    print!("{prompt} [{default}]: ");
    io::stdout().flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;

    let trimmed = input.trim();
    Ok(if trimmed.is_empty() {
        default.to_string()
    } else {
        trimmed.to_string()
    })
}

/// Confirm prompt (Y/n)
pub fn confirm(prompt: &str) -> Result<bool> {
    print!("{prompt} [Y/n]: ");
    io::stdout().flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;

    let trimmed = input.trim().to_lowercase();
    Ok(trimmed.is_empty() || trimmed == "y" || trimmed == "yes")
}
