# Hook-Based Session System Implementation Plan

## Overview

This document outlines the implementation of a new hook-based session capture system for Patina. This will be developed on a feature branch and tested using Dagger, without altering the existing session system.

## Architecture

```
Hooks (Automatic Capture) → Processing (Rust) → Knowledge (Layer)
```

### Components

1. **Claude Code Hooks** - Automatic event capture
2. **Hook Logs** - Lightweight timestamp + event logs
3. **Sub-Agent Enrichment** - Periodic context addition
4. **Rust Processor** - Merges JSONL + logs + context
5. **Dagger Pipeline** - Consistent processing environment

## Feature Branch Setup

```bash
# Create feature branch
git checkout -b feature/hook-based-sessions

# Create new directories (separate from existing)
mkdir -p .claude/hooks
mkdir -p .claude/logs
mkdir -p src/commands/hooks
mkdir -p pipelines/hooks
```

## Phase 1: Hook Infrastructure

### 1.1 Hook Configuration

Create `.claude/settings.hooks.json` (separate from main settings):

```json
{
  "hooks": {
    "UserPromptSubmit": [{
      "hooks": [{
        "type": "command",
        "command": ".claude/hooks/log-prompt.sh"
      }]
    }],
    "PostToolUse": [{
      "matcher": ".*",
      "hooks": [{
        "type": "command",
        "command": ".claude/hooks/log-tool.sh"
      }]
    }],
    "Stop": [{
      "throttle": 300000,
      "hooks": [{
        "type": "command",
        "command": "touch .claude/logs/.enrich-needed"
      }]
    }]
  }
}
```

### 1.2 Hook Scripts

`.claude/hooks/log-prompt.sh`:
```bash
#!/bin/bash
TIMESTAMP=$(date -u +%Y-%m-%dT%H:%M:%S.%3NZ)
SESSION_ID=${CLAUDE_SESSION_ID:-"unknown"}
LOG_FILE=".claude/logs/hooks-${SESSION_ID}.log"

# Read prompt from stdin
PROMPT=$(cat)

# Log in simple format
echo "${TIMESTAMP}|PROMPT|${PROMPT}" >> "$LOG_FILE"
```

`.claude/hooks/log-tool.sh`:
```bash
#!/bin/bash
# Reads JSON from stdin, extracts tool info
TIMESTAMP=$(date -u +%Y-%m-%dT%H:%M:%S.%3NZ)
SESSION_ID=$(jq -r '.session_id' 2>/dev/null || echo "unknown")
TOOL_NAME=$(jq -r '.tool_name' 2>/dev/null)
FILE_PATH=$(jq -r '.tool_input.file_path // ""' 2>/dev/null)

LOG_FILE=".claude/logs/hooks-${SESSION_ID}.log"
echo "${TIMESTAMP}|TOOL|${TOOL_NAME}|${FILE_PATH}" >> "$LOG_FILE"
```

## Phase 2: Rust Processing

### 2.1 New Module Structure

```rust
// src/hooks/mod.rs
pub mod capture;
pub mod processor;
pub mod enrichment;

// src/hooks/capture.rs
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct HookEvent {
    pub timestamp: DateTime<Utc>,
    pub event_type: EventType,
    pub content: String,
    pub metadata: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum EventType {
    Prompt,
    Tool { name: String, file: Option<String> },
    Enrichment,
}

pub fn parse_hook_log(log_path: &Path) -> Result<Vec<HookEvent>> {
    let file = File::open(log_path)?;
    let reader = BufReader::new(file);
    let mut events = Vec::new();
    
    for line in reader.lines() {
        let line = line?;
        let parts: Vec<&str> = line.split('|').collect();
        
        if parts.len() >= 3 {
            let timestamp = DateTime::parse_from_rfc3339(parts[0])?;
            let event = match parts[1] {
                "PROMPT" => HookEvent {
                    timestamp: timestamp.into(),
                    event_type: EventType::Prompt,
                    content: parts[2].to_string(),
                    metadata: None,
                },
                "TOOL" => HookEvent {
                    timestamp: timestamp.into(),
                    event_type: EventType::Tool {
                        name: parts[2].to_string(),
                        file: parts.get(3).map(|s| s.to_string()),
                    },
                    content: String::new(),
                    metadata: None,
                },
                _ => continue,
            };
            events.push(event);
        }
    }
    
    Ok(events)
}
```

### 2.2 Session Processor

```rust
// src/hooks/processor.rs
use crate::hooks::capture::{HookEvent, parse_hook_log};

pub struct SessionProcessor {
    session_id: String,
    hook_events: Vec<HookEvent>,
    jsonl_events: Vec<JsonlEvent>,
    enrichments: Vec<String>,
}

impl SessionProcessor {
    pub fn new(session_id: String) -> Self {
        Self {
            session_id,
            hook_events: Vec::new(),
            jsonl_events: Vec::new(),
            enrichments: Vec::new(),
        }
    }
    
    pub fn load_hook_log(&mut self) -> Result<()> {
        let log_path = format!(".claude/logs/hooks-{}.log", self.session_id);
        self.hook_events = parse_hook_log(Path::new(&log_path))?;
        Ok(())
    }
    
    pub fn load_jsonl(&mut self) -> Result<()> {
        let jsonl_path = find_claude_jsonl(&self.session_id)?;
        self.jsonl_events = parse_jsonl(jsonl_path)?;
        Ok(())
    }
    
    pub fn merge_and_generate(&self) -> Result<String> {
        let mut output = String::new();
        
        // Merge events by timestamp
        let mut all_events = self.create_timeline()?;
        
        // Generate markdown
        for event in all_events {
            match event {
                TimelineEvent::User(e) => {
                    output.push_str(&format!("\n[{}] User: {}\n", 
                        e.timestamp.format("%H:%M:%S"), 
                        e.content
                    ));
                },
                TimelineEvent::Tool(e) => {
                    output.push_str(&format!("- Tool: {} {}\n", 
                        e.name, 
                        e.file.as_deref().unwrap_or("")
                    ));
                },
                TimelineEvent::Claude(e) => {
                    if let Some(enrichment) = self.find_enrichment(&e.timestamp) {
                        output.push_str(&format!("\n{}\n", enrichment));
                    }
                },
            }
        }
        
        Ok(output)
    }
}
```

### 2.3 New Command

```rust
// src/commands/hooks/process.rs
use clap::Args;

#[derive(Debug, Args)]
pub struct ProcessHooksArgs {
    /// Session ID to process (defaults to latest)
    #[arg(short, long)]
    session: Option<String>,
    
    /// Output format
    #[arg(short, long, default_value = "markdown")]
    format: String,
}

pub fn execute(args: ProcessHooksArgs) -> Result<()> {
    let session_id = args.session
        .or_else(|| find_latest_session())
        .ok_or_else(|| anyhow!("No session found"))?;
    
    let mut processor = SessionProcessor::new(session_id);
    processor.load_hook_log()?;
    processor.load_jsonl()?;
    
    let output = processor.merge_and_generate()?;
    
    // Save to new location (not interfering with existing)
    let output_path = format!("layer/hook-sessions/{}.md", processor.session_id);
    std::fs::create_dir_all("layer/hook-sessions")?;
    std::fs::write(output_path, output)?;
    
    println!("Processed session saved to layer/hook-sessions/");
    Ok(())
}
```

## Phase 3: Dagger Development Environment

### 3.1 Dagger Pipeline

```go
// pipelines/hooks/main.go
package main

import (
    "context"
    "fmt"
    "dagger.io/dagger"
)

func main() {
    ctx := context.Background()
    client, err := dagger.Connect(ctx, dagger.WithLogOutput(os.Stderr))
    if err != nil {
        panic(err)
    }
    defer client.Close()
    
    // Development container
    dev := client.Container().
        From("rust:1.75").
        WithDirectory("/app", client.Host().Directory(".")).
        WithWorkdir("/app").
        WithExec([]string{"cargo", "build", "--features", "hooks"})
    
    // Test hook processing
    _, err = dev.
        WithExec([]string{"cargo", "test", "--features", "hooks", "hooks::"}).
        Sync(ctx)
    
    if err != nil {
        panic(err)
    }
    
    fmt.Println("Hook system tests passed!")
}
```

### 3.2 Development Commands

```makefile
# Makefile.hooks (separate from main Makefile)
.PHONY: dev-hooks test-hooks build-hooks

dev-hooks:
	dagger run go run ./pipelines/hooks/main.go

test-hooks:
	cargo test --features hooks hooks::

build-hooks:
	cargo build --features hooks

watch-hooks:
	cargo watch -x "test --features hooks hooks::"
```

## Phase 4: Sub-Agent Integration

### 4.1 Session Enricher Sub-Agent

Create `.claude/sub-agents/session-enricher.json`:

```json
{
  "name": "session-enricher",
  "description": "Analyzes hook logs and adds context about development decisions",
  "system_prompt": "You analyze development session logs and add rich context about what happened and why. Focus on decisions, patterns discovered, and problems solved.",
  "tools": ["Read", "Write"],
  "config": {
    "trigger": "manual",
    "input_file": ".claude/logs/hooks-{session_id}.log",
    "output_file": ".claude/logs/enrichment-{session_id}.md"
  }
}
```

### 4.2 Enrichment Command

```markdown
# .claude/commands/enrich-hooks.md
---
allowed-tools: Read, Write
description: Enrich hook logs with context
---

Use the session-enricher sub agent to analyze the recent hook logs and add context about what we've been working on.
```

## Phase 5: Testing Strategy

### 5.1 Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_parse_hook_log() {
        let log_content = r#"
2025-07-28T10:45:20.463Z|PROMPT|Fix auth bug
2025-07-28T10:45:25.173Z|TOOL|Read|auth.rs
"#;
        
        let events = parse_hook_log_content(log_content).unwrap();
        assert_eq!(events.len(), 2);
        assert!(matches!(events[0].event_type, EventType::Prompt));
    }
    
    #[test]
    fn test_merge_timeline() {
        // Test merging hook events with JSONL events
    }
}
```

### 5.2 Integration Tests

```rust
#[test]
fn test_full_session_processing() {
    // Create test data
    create_test_hook_log("test-session-123");
    create_test_jsonl("test-session-123");
    
    // Process
    let processor = SessionProcessor::new("test-session-123".to_string());
    processor.load_hook_log().unwrap();
    processor.load_jsonl().unwrap();
    
    let output = processor.merge_and_generate().unwrap();
    
    // Verify output contains both sources
    assert!(output.contains("User:"));
    assert!(output.contains("Tool:"));
}
```

## Phase 6: Feature Flags

Add to `Cargo.toml`:

```toml
[features]
default = []
hooks = []

[dependencies]
# Existing deps...

# Hook-specific deps (only with feature)
notify = { version = "6.0", optional = true }
```

## Usage During Development

```bash
# Enable hooks for this session only
cp .claude/settings.hooks.json .claude/settings.json

# Work normally - hooks capture everything

# Process session with new system
cargo run --features hooks -- process-hooks

# View results (separate from existing sessions)
ls layer/hook-sessions/

# Disable hooks
rm .claude/settings.json
```

## Migration Plan

Once tested and proven:

1. Add hooks to main settings
2. Run both systems in parallel
3. Compare outputs
4. Gradually transition
5. Deprecate old system

## Benefits

1. **No Manual Timestamps** - Hooks handle everything
2. **Complete Capture** - Nothing missed
3. **Low Token Usage** - Only enrich periodically
4. **Backward Compatible** - Existing system untouched
5. **Testable** - Dagger ensures consistency

## Next Steps

1. Create feature branch
2. Implement Phase 1 (hooks)
3. Test capture accuracy
4. Implement Phase 2 (Rust processor)
5. Add Dagger pipeline
6. Test with real sessions
7. Add sub-agent enrichment
8. Compare with existing system
9. Plan migration