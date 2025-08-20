use anyhow::Result;
use chrono::Utc;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::Path;

pub struct LearnCommand;

impl LearnCommand {
    pub fn execute(lesson: &str) -> Result<()> {
        // Simple but effective: append to a lessons file
        let lessons_file = Path::new(".patina/lessons.md");
        
        // Create .patina directory if it doesn't exist
        std::fs::create_dir_all(".patina")?;
        
        // Get current context
        let branch = get_current_branch()?;
        let timestamp = Utc::now().format("%Y-%m-%d %H:%M");
        
        // Format the lesson
        let formatted_lesson = format!(
            "\n## {} [{}]\n**Branch**: {}\n**Lesson**: {}\n",
            timestamp, 
            get_current_commit_short()?,
            branch,
            lesson
        );
        
        // Append to lessons file
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(lessons_file)?;
        
        writeln!(file, "{}", formatted_lesson)?;
        
        println!("📝 Lesson recorded: {}", lesson);
        
        // Also check if this is a repeated mistake
        check_for_repeated_mistake(lesson)?;
        
        Ok(())
    }
    
    pub fn execute_decision(decision: &str, reasoning: &str) -> Result<()> {
        let decisions_file = Path::new(".patina/decisions.md");
        
        std::fs::create_dir_all(".patina")?;
        
        let timestamp = Utc::now().format("%Y-%m-%d %H:%M");
        let formatted_decision = format!(
            "\n## {} [{}]\n**Decision**: {}\n**Reasoning**: {}\n",
            timestamp,
            get_current_commit_short()?,
            decision,
            reasoning
        );
        
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(decisions_file)?;
        
        writeln!(file, "{}", formatted_decision)?;
        
        println!("🎯 Decision recorded: {}", decision);
        println!("   Reasoning: {}", reasoning);
        
        Ok(())
    }
}

fn get_current_branch() -> Result<String> {
    let output = std::process::Command::new("git")
        .args(&["branch", "--show-current"])
        .output()?;
    
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn get_current_commit_short() -> Result<String> {
    let output = std::process::Command::new("git")
        .args(&["rev-parse", "--short", "HEAD"])
        .output()?;
    
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn check_for_repeated_mistake(lesson: &str) -> Result<()> {
    // Check if we've learned a similar lesson before
    let lessons_file = Path::new(".patina/lessons.md");
    
    if !lessons_file.exists() {
        return Ok(());
    }
    
    let content = std::fs::read_to_string(lessons_file)?;
    let lesson_lower = lesson.to_lowercase();
    
    // Simple similarity check - look for key words
    let key_words: Vec<&str> = lesson_lower
        .split_whitespace()
        .filter(|w| w.len() > 4) // Only check words longer than 4 chars
        .collect();
    
    let mut similar_lessons = Vec::new();
    
    for line in content.lines() {
        if line.starts_with("**Lesson**:") {
            let past_lesson = line.replace("**Lesson**:", "").trim().to_lowercase();
            
            // Count how many key words match
            let matches = key_words.iter()
                .filter(|w| past_lesson.contains(*w))
                .count();
            
            if matches >= 2 || (key_words.len() > 0 && matches as f32 / key_words.len() as f32 > 0.5) {
                similar_lessons.push(line.replace("**Lesson**:", "").trim().to_string());
            }
        }
    }
    
    if !similar_lessons.is_empty() {
        println!("\n⚠️  Similar lesson learned before:");
        for past in similar_lessons.iter().take(2) {
            println!("   - {}", past);
        }
        println!("   (Pattern detected: We might be repeating mistakes)");
    }
    
    Ok(())
}