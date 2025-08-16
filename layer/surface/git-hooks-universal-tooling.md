---
id: git-hooks-universal-tooling
status: active
created: 2025-08-16
session: learn-new-git-commands
tags: [git, hooks, integration, llm-agnostic, patina-commands]
references: [git-hooks-integration.md, git-universal-integration.md, git-integration-timeline.md]
---

# Git Hooks with Universal Tooling

Using Claude Code hooks that call Patina commands - making the integration work across any LLM that adopts similar hook patterns.

## The Architecture

```
Claude Code Hooks → Patina Commands → Git Operations
Gemini Hooks     → Patina Commands → Git Operations  
Any LLM Hooks    → Patina Commands → Git Operations
```

The hooks are thin wrappers that call Patina, which does the actual work.

## Implementation: Hooks Call Patina

### 1. Build Patina Commands for Hook Operations

First, extend Patina with commands that hooks can call:

```rust
// src/commands/mod.rs
pub mod hooks;

// src/commands/hooks.rs
use anyhow::Result;
use std::process::Command;
use serde_json::Value;

/// Called by Stop hook - auto-commit session
pub fn on_stop() -> Result<()> {
    // Read hook input from stdin
    let input: Value = serde_json::from_reader(std::io::stdin())?;
    
    let session_id = input["session_id"].as_str().unwrap_or("unknown");
    let cwd = input["cwd"].as_str().unwrap_or(".");
    
    // Change to project directory
    std::env::set_current_dir(cwd)?;
    
    // Check for active session
    if std::path::Path::new(".claude/context/active-session.md").exists() 
        || std::path::Path::new(".llm/context/active-session.md").exists() {
        
        // Add and commit session files
        Command::new("git")
            .args(&["add", ".claude/context/", ".llm/context/"])
            .output()?;
            
        Command::new("git")
            .args(&["commit", "-m", &format!("session: checkpoint {}", session_id)])
            .output()
            .ok(); // Don't fail if nothing to commit
    }
    
    // Also save any pattern changes
    save_patterns()?;
    
    Ok(())
}

/// Called by PostToolUse - track modifications
pub fn on_file_modified() -> Result<()> {
    let input: Value = serde_json::from_reader(std::io::stdin())?;
    
    let file_path = input["tool_input"]["file_path"].as_str();
    let session_id = input["session_id"].as_str().unwrap_or("unknown");
    
    if let Some(file) = file_path {
        // Track co-modification
        track_comodification(session_id, file)?;
        
        // Check survival and add to session
        let survival = check_survival(file)?;
        if survival.is_old {
            append_to_session(&format!("Modified {} (survived {})", file, survival.age))?;
        }
    }
    
    Ok(())
}

/// Called by PreToolUse - check survival
pub fn on_before_edit() -> Result<()> {
    let input: Value = serde_json::from_reader(std::io::stdin())?;
    
    let tool = input["tool"].as_str().unwrap_or("");
    let file_path = input["tool_input"]["file_path"].as_str();
    
    // Only check for edit operations
    if !["Edit", "MultiEdit", "Write"].contains(&tool) {
        return Ok(());
    }
    
    if let Some(file) = file_path {
        let survival = check_survival(file)?;
        
        if survival.months > 3 {
            eprintln!("⚠️  PATINA WARNING: {}", file);
            eprintln!("   Survived: {}", survival.age);
            eprintln!("   Commits: {}", survival.commits);
            eprintln!("   This is a stable pattern - modify carefully!");
            
            // Show co-modified files
            let comodified = get_comodified_files(file)?;
            if !comodified.is_empty() {
                eprintln!("   Often changes with:");
                for (count, file) in comodified.iter().take(3) {
                    eprintln!("     {} times: {}", count, file);
                }
            }
        }
    }
    
    Ok(())
}

// Helper functions
fn check_survival(file: &str) -> Result<SurvivalMetrics> {
    let age_output = Command::new("git")
        .args(&["log", "-1", "--format=%ar", "--", file])
        .output()?;
    
    let age = String::from_utf8_lossy(&age_output.stdout).trim().to_string();
    
    let commits_output = Command::new("git")
        .args(&["log", "--oneline", "--", file])
        .output()?;
    
    let commits = String::from_utf8_lossy(&commits_output.stdout)
        .lines()
        .count();
    
    Ok(SurvivalMetrics {
        age,
        commits,
        months: parse_months(&age),
        is_old: age.contains("month") || age.contains("year"),
    })
}

fn track_comodification(session_id: &str, file: &str) -> Result<()> {
    // Append to comodification log
    let log_entry = format!("{}:{}:{}", 
        session_id, 
        file, 
        chrono::Utc::now().to_rfc3339()
    );
    
    std::fs::create_dir_all(".patina")?;
    use std::io::Write;
    let mut f = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(".patina/comodifications.log")?;
    writeln!(f, "{}", log_entry)?;
    
    Ok(())
}
```

### 2. Create Patina Hook Commands

Add new CLI commands that hooks can call:

```rust
// src/cli.rs
#[derive(Subcommand)]
pub enum HookCommands {
    /// Called when Claude stops
    OnStop,
    /// Called after file modification
    OnModified,
    /// Called before file modification
    OnBeforeEdit,
    /// Process any hook event
    Hook {
        #[arg(value_name = "EVENT")]
        event: String,
    },
}

// In main command handler
Commands::Hook(cmd) => match cmd {
    HookCommands::OnStop => commands::hooks::on_stop(),
    HookCommands::OnModified => commands::hooks::on_file_modified(),
    HookCommands::OnBeforeEdit => commands::hooks::on_before_edit(),
    HookCommands::Hook { event } => commands::hooks::process_hook(event),
}
```

### 3. Simple Hook Scripts

Now the hooks are just thin wrappers:

```bash
#!/bin/bash
# ~/.claude/hooks/patina_stop.sh
# Called by Stop hook - just pass to Patina
patina hook on-stop

#!/bin/bash
# ~/.claude/hooks/patina_post_tool.sh
# Called by PostToolUse - just pass to Patina
patina hook on-modified

#!/bin/bash
# ~/.claude/hooks/patina_pre_tool.sh  
# Called by PreToolUse - just pass to Patina
patina hook on-before-edit
```

Or even simpler, hooks can directly call:

```json
{
  "hooks": {
    "Stop": [{
      "matcher": "",
      "hooks": [{
        "type": "command",
        "command": "patina hook on-stop"
      }]
    }],
    "PostToolUse": [{
      "matcher": "Edit|MultiEdit|Write",
      "hooks": [{
        "type": "command",
        "command": "patina hook on-modified"
      }]
    }],
    "PreToolUse": [{
      "matcher": "Edit|MultiEdit|Write",
      "hooks": [{
        "type": "command",
        "command": "patina hook on-before-edit"
      }]
    }]
  }
}
```

### 4. Make It Work for Other LLMs

If Gemini or another LLM has hooks, they just need to call the same Patina commands:

```python
# Hypothetical Gemini hook
def on_file_modified(context):
    # Just call Patina
    subprocess.run(
        ["patina", "hook", "on-modified"],
        input=json.dumps(context),
        text=True
    )
```

## Benefits of This Approach

### 1. Single Implementation
- All logic lives in Patina (Rust)
- Hooks are just triggers
- Easy to test: `echo '{"session_id":"test"}' | patina hook on-stop`

### 2. LLM Agnostic
- Any LLM with hooks can call `patina hook`
- Standardized interface via stdin JSON
- No Ruby/Python dependencies in hooks

### 3. Robust
- If Patina isn't installed, hooks fail gracefully
- Can add `|| true` to prevent breaking Claude
- Easy to debug: run Patina commands directly

### 4. Extensible
```bash
# Future hooks just call Patina
patina hook on-pr-created
patina hook on-test-run
patina hook on-compact
```

## Installation

```bash
# Install hooks via Patina
patina init --hooks

# This:
# 1. Checks if Claude Code is installed
# 2. Updates ~/.claude/settings.json
# 3. Shows instructions for other LLMs
```

```rust
// src/commands/init.rs
pub fn install_hooks() -> Result<()> {
    let home = dirs::home_dir().context("No home directory")?;
    let claude_settings = home.join(".claude/settings.json");
    
    if claude_settings.exists() {
        // Read existing settings
        let mut settings: Value = serde_json::from_reader(
            std::fs::File::open(&claude_settings)?
        )?;
        
        // Add our hooks
        settings["hooks"]["Stop"] = json!([{
            "matcher": "",
            "hooks": [{
                "type": "command",
                "command": "patina hook on-stop"
            }]
        }]);
        
        settings["hooks"]["PostToolUse"] = json!([{
            "matcher": "Edit|MultiEdit|Write",
            "hooks": [{
                "type": "command",
                "command": "patina hook on-modified"
            }]
        }]);
        
        // Write back
        std::fs::write(
            &claude_settings,
            serde_json::to_string_pretty(&settings)?
        )?;
        
        println!("✓ Installed Patina hooks for Claude Code");
    } else {
        println!("Claude Code not found. For other LLMs, configure hooks to call:");
        println!("  patina hook on-stop");
        println!("  patina hook on-modified");
        println!("  patina hook on-before-edit");
    }
    
    Ok(())
}
```

## Testing Without LLM

```bash
# Test stop hook
echo '{"session_id":"test","cwd":"."}' | patina hook on-stop

# Test modification tracking
echo '{"session_id":"test","tool_input":{"file_path":"src/main.rs"}}' | patina hook on-modified

# Test survival check
echo '{"tool":"Edit","tool_input":{"file_path":"README.md"}}' | patina hook on-before-edit
```

## Integration with Existing Patina

This connects all three systems:

1. **Session tracking** - `on_stop` commits session files
2. **Git survival** - `on_before_edit` checks file age
3. **Navigation** - Can query the comodification log

```rust
// Update navigate to use comodification data
pub fn get_related_patterns(file: &Path) -> Vec<(usize, PathBuf)> {
    // Read .patina/comodifications.log
    // Find files that appear together
    // Return sorted by frequency
}
```

## Conclusion

Hooks calling Patina commands gives us:
- **Auto-commit** functionality (when hooks fire)
- **Universal tooling** (Patina works everywhere)
- **LLM agnostic** (any LLM can call `patina hook`)
- **Single source of truth** (logic in Patina, not scripts)
- **Easy testing** (pipe JSON to Patina)

This is the bridge between LLM events and Git memory, implemented in a way that any LLM can use.