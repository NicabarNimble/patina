use anyhow::Result;
use std::collections::HashMap;
use std::path::PathBuf;

pub struct ContextCommand;

impl ContextCommand {
    pub fn execute(topic: &str) -> Result<()> {
        println!("🔍 Context for: {}\n", topic);
        
        // Search for relevant information about this topic
        let files = find_related_files(topic)?;
        let sessions = find_related_sessions(topic)?;
        let patterns = find_applicable_patterns(topic)?;
        let previous_work = find_previous_work(topic)?;
        
        // Show related files
        if !files.is_empty() {
            println!("## Related Files:");
            for (file, relevance) in files.iter().take(5) {
                println!("- {} ({}% relevant)", file.display(), (relevance * 100.0) as i32);
            }
            println!();
        }
        
        // Show previous work on this topic
        if !previous_work.is_empty() {
            println!("## Previous Work on This:");
            for work in previous_work.iter().take(3) {
                println!("- {}", work);
            }
            println!();
        }
        
        // Show applicable patterns
        if !patterns.is_empty() {
            println!("## Patterns That Apply:");
            for pattern in patterns {
                println!("- {}", pattern);
            }
            println!();
        }
        
        // Show related sessions
        if !sessions.is_empty() {
            println!("## Related Sessions:");
            for session in sessions.iter().take(3) {
                println!("- {}", session);
            }
            println!();
        }
        
        // Search for failures/lessons about this topic
        show_relevant_lessons(topic)?;
        
        Ok(())
    }
}

fn find_related_files(topic: &str) -> Result<Vec<(PathBuf, f32)>> {
    let mut files = Vec::new();
    
    // Simple heuristic: find files with the topic in the name or path
    let topic_lower = topic.to_lowercase();
    
    // Search in src/
    let output = std::process::Command::new("find")
        .args(&["src", "-name", "*.rs", "-type", "f"])
        .output()?;
    
    if output.status.success() {
        let file_list = String::from_utf8_lossy(&output.stdout);
        for line in file_list.lines() {
            let path = PathBuf::from(line);
            let path_str = line.to_lowercase();
            
            // Calculate simple relevance score
            let mut relevance = 0.0;
            
            // Direct name match
            if path_str.contains(&topic_lower) {
                relevance = 0.9;
            }
            // Related terms (this could be much smarter)
            else if topic == "memory" && path_str.contains("remember") {
                relevance = 0.7;
            } else if topic == "navigate" && path_str.contains("index") {
                relevance = 0.6;
            } else if topic == "pattern" && path_str.contains("audit") {
                relevance = 0.5;
            }
            
            if relevance > 0.0 {
                files.push((path, relevance));
            }
        }
    }
    
    // Sort by relevance
    files.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
    
    Ok(files)
}

fn find_related_sessions(topic: &str) -> Result<Vec<String>> {
    let mut sessions = Vec::new();
    let topic_lower = topic.to_lowercase();
    
    // Search in session files
    let output = std::process::Command::new("grep")
        .args(&["-l", "-i", &topic_lower, "layer/sessions/*.md"])
        .output()?;
    
    if output.status.success() {
        let file_list = String::from_utf8_lossy(&output.stdout);
        for line in file_list.lines() {
            if let Some(filename) = PathBuf::from(line).file_name() {
                sessions.push(filename.to_string_lossy().to_string());
            }
        }
    }
    
    Ok(sessions)
}

fn find_applicable_patterns(topic: &str) -> Result<Vec<String>> {
    let mut patterns = Vec::new();
    
    // Map topics to patterns (this should be data-driven eventually)
    let pattern_map = HashMap::from([
        ("memory", vec!["session-capture", "context-orchestration"]),
        ("navigate", vec!["semantic-search", "pattern-indexing"]),
        ("pattern", vec!["dependable-rust", "pattern-selection-framework"]),
        ("adapter", vec!["adapter-pattern", "versioning"]),
        ("build", vec!["container-orchestration", "dagger-integration"]),
        ("test", vec!["test-driven", "validation"]),
    ]);
    
    if let Some(applicable) = pattern_map.get(topic) {
        for pattern in applicable {
            patterns.push(pattern.to_string());
        }
    }
    
    // Also check if the topic itself is a pattern
    let pattern_path = format!("layer/core/{}.md", topic);
    if std::path::Path::new(&pattern_path).exists() {
        patterns.insert(0, topic.to_string());
    }
    
    Ok(patterns)
}

fn find_previous_work(topic: &str) -> Result<Vec<String>> {
    let mut work = Vec::new();
    
    // Look at recent commits mentioning this topic
    let output = std::process::Command::new("git")
        .args(&["log", "--oneline", "--grep", topic, "-10"])
        .output()?;
    
    if output.status.success() {
        let commits = String::from_utf8_lossy(&output.stdout);
        for line in commits.lines().take(3) {
            work.push(line.to_string());
        }
    }
    
    Ok(work)
}

fn show_relevant_lessons(topic: &str) -> Result<()> {
    // For now, search in session files for lessons/failures
    // In the future, this would query the memory database
    
    let search_terms = vec![
        format!("{} failed", topic),
        format!("{} doesn't work", topic),
        format!("problem with {}", topic),
        format!("{} issue", topic),
    ];
    
    let mut found_lessons = Vec::new();
    
    for term in search_terms {
        let output = std::process::Command::new("grep")
            .args(&["-h", "-i", &term, "layer/sessions/*.md"])
            .output()?;
        
        if output.status.success() {
            let matches = String::from_utf8_lossy(&output.stdout);
            for line in matches.lines().take(1) {
                let cleaned = line.trim_start_matches('-').trim();
                if !cleaned.is_empty() {
                    found_lessons.push(cleaned.to_string());
                }
            }
        }
    }
    
    if !found_lessons.is_empty() {
        println!("## ⚠️ Previous Issues with {}:", topic);
        for lesson in found_lessons {
            println!("- {}", lesson);
        }
        println!();
    }
    
    Ok(())
}