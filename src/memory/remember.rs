use anyhow::Result;
use std::path::Path;
use super::{MemorySystem, SessionSummary};
use chrono::Utc;

pub struct RememberCommand;

impl RememberCommand {
    pub fn execute() -> Result<()> {
        let memory_system = MemorySystem::new()?;
        let memory = memory_system.remember()?;
        
        // First, check for active session and last session files
        let active_session = read_active_session()?;
        let last_session = read_last_session()?;
        
        println!("🧠 Patina Memory\n");
        
        // Show last session summary
        if let Some(last) = last_session {
            println!("## Last Session: {}", last.title);
            println!("{}", last.summary);
            
            if !last.incomplete_tasks.is_empty() {
                println!("\n⚠️ Incomplete from last time:");
                for task in &last.incomplete_tasks {
                    println!("- {}", task);
                }
            }
        }
        
        // Show current context
        if let Some(active) = active_session {
            println!("\n## Current Session: {}", active.title);
            println!("Branch: {}", active.branch);
            println!("Started: {}", active.started);
            
            if !active.goals.is_empty() {
                println!("\nGoals:");
                for goal in &active.goals {
                    println!("- {}", goal);
                }
            }
        }
        
        // Show relevant patterns and decisions
        if !memory.lessons_learned.is_empty() {
            println!("\n## Remember These Lessons:");
            for lesson in memory.lessons_learned.iter().take(3) {
                println!("⚠️ {}", lesson.lesson);
            }
        }
        
        // Show files that might be relevant
        println!("\n## Relevant Context:");
        show_relevant_files()?;
        
        Ok(())
    }
}

#[derive(Debug)]
struct SessionInfo {
    title: String,
    summary: String,
    branch: String,
    started: String,
    goals: Vec<String>,
    incomplete_tasks: Vec<String>,
}

fn read_active_session() -> Result<Option<SessionInfo>> {
    let path = Path::new(".claude/context/active-session.md");
    if !path.exists() {
        return Ok(None);
    }
    
    let content = std::fs::read_to_string(path)?;
    parse_session_file(&content)
}

fn read_last_session() -> Result<Option<SessionInfo>> {
    let path = Path::new(".claude/context/last-session.md");
    if !path.exists() {
        return Ok(None);
    }
    
    let content = std::fs::read_to_string(path)?;
    
    // Extract the quick summary from last-session.md
    let mut title = String::new();
    let mut summary = String::new();
    let mut tasks = Vec::new();
    
    for line in content.lines() {
        if line.starts_with("# Last Session:") {
            title = line.replace("# Last Session:", "").trim().to_string();
        } else if line.starts_with("See:") {
            // This points to the full session file
            let session_file = line.replace("See:", "").trim();
            // Could read the full file for more details
        }
    }
    
    // For now, just show what we have
    if !title.is_empty() {
        Ok(Some(SessionInfo {
            title,
            summary: "Check layer/sessions/ for full details".to_string(),
            branch: String::new(),
            started: String::new(),
            goals: vec![],
            incomplete_tasks: tasks,
        }))
    } else {
        Ok(None)
    }
}

fn parse_session_file(content: &str) -> Result<Option<SessionInfo>> {
    let mut info = SessionInfo {
        title: String::new(),
        summary: String::new(),
        branch: String::new(),
        started: String::new(),
        goals: Vec::new(),
        incomplete_tasks: Vec::new(),
    };
    
    let mut in_goals = false;
    let mut in_previous = false;
    
    for line in content.lines() {
        if line.starts_with("# Session:") {
            info.title = line.replace("# Session:", "").trim().to_string();
        } else if line.starts_with("**Started**:") {
            info.started = line.replace("**Started**:", "").trim().to_string();
        } else if line.starts_with("**Git Branch**:") {
            info.branch = line.replace("**Git Branch**:", "").trim().to_string();
        } else if line.starts_with("## Goals") {
            in_goals = true;
            in_previous = false;
        } else if line.starts_with("## Previous Session Context") {
            in_goals = false;
            in_previous = true;
        } else if line.starts_with("##") {
            in_goals = false;
            in_previous = false;
        } else if in_goals && line.starts_with("- [ ]") {
            let goal = line.replace("- [ ]", "").trim().to_string();
            info.goals.push(goal.clone());
            info.incomplete_tasks.push(goal);
        } else if in_goals && line.starts_with("- [x]") {
            let goal = line.replace("- [x]", "").trim().to_string();
            info.goals.push(goal);
        } else if in_previous && !line.trim().is_empty() && !line.starts_with("<!--") {
            info.summary = line.trim().to_string();
            in_previous = false;
        }
    }
    
    if !info.title.is_empty() {
        Ok(Some(info))
    } else {
        Ok(None)
    }
}

fn show_relevant_files() -> Result<()> {
    // Show recently modified files that the LLM might need to know about
    let output = std::process::Command::new("git")
        .args(&["diff", "--name-only", "HEAD~3..HEAD"])
        .output()?;
    
    if output.status.success() {
        let files = String::from_utf8_lossy(&output.stdout);
        let file_list: Vec<&str> = files.lines().take(5).collect();
        
        if !file_list.is_empty() {
            println!("Recently changed files:");
            for file in file_list {
                println!("- {}", file);
            }
        }
    }
    
    // Also show files changed in current session
    let output = std::process::Command::new("git")
        .args(&["status", "--porcelain"])
        .output()?;
    
    if output.status.success() {
        let files = String::from_utf8_lossy(&output.stdout);
        let modified: Vec<&str> = files.lines()
            .filter(|l| l.starts_with(" M") || l.starts_with("M "))
            .map(|l| &l[3..])
            .take(5)
            .collect();
        
        if !modified.is_empty() {
            println!("\nModified in this session:");
            for file in modified {
                println!("- {}", file);
            }
        }
    }
    
    Ok(())
}