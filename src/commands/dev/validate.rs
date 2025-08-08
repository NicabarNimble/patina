use anyhow::Result;
use std::fs;
use std::path::Path;

pub fn execute(json: bool) -> Result<()> {
    if !json {
        println!("🔍 Validating Patina resources...");
        println!();
    }
    
    let mut issues = Vec::new();
    let project_root = std::env::current_dir()?;
    
    // Check templates
    let template_checks = vec![
        ("dagger/main.go.tmpl", "Dagger main template"),
        ("dagger/go.mod.tmpl", "Dagger module template"),
        ("dagger/CONSTRAINTS.md", "Dagger constraints"),
    ];
    
    for (path, description) in &template_checks {
        let full_path = project_root.join("resources/templates").join(path);
        if !full_path.exists() {
            issues.push(format!("Missing template: {} ({})", path, description));
        } else if !json {
            println!("  ✓ {}", path);
        }
    }
    
    // Check Claude adapter resources
    let claude_checks = vec![
        "session-start.sh",
        "session-update.sh",
        "session-end.sh",
        "session-note.sh",
    ];
    
    if !json {
        println!();
        println!("🤖 Checking Claude adapter...");
    }
    
    for script in &claude_checks {
        let full_path = project_root.join("resources/claude").join(script);
        if !full_path.exists() {
            issues.push(format!("Missing Claude script: {}", script));
        } else if !json {
            println!("  ✓ {}", script);
        }
    }
    
    // Check brain patterns
    if !json {
        println!();
        println!("🧠 Checking brain patterns...");
    }
    
    for dir in ["core", "topics"] {
        let layer_path = project_root.join("layer").join(dir);
        if layer_path.exists() {
            let count = count_patterns(&layer_path)?;
            if !json {
                println!("  ✓ layer/{}/: {} patterns", dir, count);
            }
        } else {
            issues.push(format!("Missing layer directory: layer/{}/", dir));
        }
    }
    
    // Check Cargo.toml has proper features
    let cargo_toml = fs::read_to_string(project_root.join("Cargo.toml"))?;
    if !cargo_toml.contains("[features]") || !cargo_toml.contains("dev = ") {
        issues.push("Cargo.toml missing dev feature flag".to_string());
    } else if !json {
        println!();
        println!("  ✓ Cargo.toml has dev feature");
    }
    
    if json {
        let result = serde_json::json!({
            "valid": issues.is_empty(),
            "issues": issues,
            "checks_performed": template_checks.len() + claude_checks.len() + 3,
        });
        println!("{}", serde_json::to_string_pretty(&result)?);
    } else {
        println!();
        if issues.is_empty() {
            println!("✅ All resources validated successfully!");
        } else {
            println!("⚠️  Found {} issues:", issues.len());
            for issue in issues {
                println!("  - {}", issue);
            }
        }
    }
    
    Ok(())
}

fn count_patterns(dir: &Path) -> Result<usize> {
    let entries = fs::read_dir(dir)?;
    let count = entries
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.path().extension()
                .and_then(|ext| ext.to_str())
                .map(|ext| ext == "md")
                .unwrap_or(false)
        })
        .count();
    Ok(count)
}