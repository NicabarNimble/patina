// Intelligent memory for LLM sessions
// Goal: Give LLMs the context they need to work intelligently

use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

pub mod context;
pub mod database;
pub mod learn;
pub mod remember;

pub use context::ContextCommand;
pub use learn::LearnCommand;
pub use remember::RememberCommand;

// What LLMs actually need to remember
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Memory {
    pub last_session: Option<SessionSummary>,
    pub relevant_context: Vec<ContextItem>,
    pub lessons_learned: Vec<Lesson>,
    pub active_decisions: Vec<Decision>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionSummary {
    pub id: String,
    pub date: DateTime<Utc>,
    pub what_we_did: String,
    pub what_worked: Vec<String>,
    pub what_failed: Vec<String>,
    pub next_steps: Vec<String>,
    pub key_decisions: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextItem {
    pub topic: String,
    pub relevance: f32,
    pub summary: String,
    pub files: Vec<PathBuf>,
    pub related_sessions: Vec<String>,
    pub applicable_patterns: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Lesson {
    pub id: String,
    pub learned_at: DateTime<Utc>,
    pub lesson: String,
    pub context: String,
    pub tags: Vec<String>,
    pub prevent_repeat: bool, // Should warn if about to repeat this mistake
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Decision {
    pub id: String,
    pub made_at: DateTime<Utc>,
    pub decision: String,
    pub reasoning: String,
    pub alternatives_rejected: Vec<String>,
    pub still_valid: bool,
}

pub struct MemorySystem {
    _db_path: PathBuf,
}

impl MemorySystem {
    pub fn new() -> Result<Self> {
        let db_path = PathBuf::from(".patina/memory.db");
        std::fs::create_dir_all(".patina")?;
        Ok(Self { _db_path: db_path })
    }

    // Core function: What does the LLM need to know RIGHT NOW?
    pub fn remember(&self) -> Result<Memory> {
        let last_session = self.get_last_session()?;
        let relevant_context = self.get_active_context()?;
        let lessons_learned = self.get_relevant_lessons()?;
        let active_decisions = self.get_active_decisions()?;

        Ok(Memory {
            last_session,
            relevant_context,
            lessons_learned,
            active_decisions,
        })
    }

    // Get context for a specific topic
    pub fn context(&self, topic: &str) -> Result<Vec<ContextItem>> {
        // Find all relevant context for this topic
        // Including: files, past sessions, patterns, failures
        self.find_context(topic)
    }

    // Record a lesson learned
    pub fn learn(&mut self, lesson: &str, context: &str) -> Result<()> {
        let lesson = Lesson {
            id: format!("{}", Utc::now().timestamp()),
            learned_at: Utc::now(),
            lesson: lesson.to_string(),
            context: context.to_string(),
            tags: self.extract_tags(lesson),
            prevent_repeat: true,
        };

        self.save_lesson(lesson)
    }

    // Record a decision and why
    pub fn decide(
        &mut self,
        decision: &str,
        reasoning: &str,
        alternatives: Vec<String>,
    ) -> Result<()> {
        let decision = Decision {
            id: format!("{}", Utc::now().timestamp()),
            made_at: Utc::now(),
            decision: decision.to_string(),
            reasoning: reasoning.to_string(),
            alternatives_rejected: alternatives,
            still_valid: true,
        };

        self.save_decision(decision)
    }

    // Private methods that will be implemented
    fn get_last_session(&self) -> Result<Option<SessionSummary>> {
        // Read .claude/context/last-session.md and parse it intelligently
        // Not just raw text, but structured understanding
        Ok(None) // TODO: Implement
    }

    fn get_active_context(&self) -> Result<Vec<ContextItem>> {
        // Find what's relevant based on current branch, recent files, etc
        Ok(vec![]) // TODO: Implement
    }

    fn get_relevant_lessons(&self) -> Result<Vec<Lesson>> {
        // Get lessons that might apply to current work
        Ok(vec![]) // TODO: Implement
    }

    fn get_active_decisions(&self) -> Result<Vec<Decision>> {
        // Get decisions that still affect current work
        Ok(vec![]) // TODO: Implement
    }

    fn find_context(&self, _topic: &str) -> Result<Vec<ContextItem>> {
        // Smart context search - not just text matching
        Ok(vec![]) // TODO: Implement
    }

    fn save_lesson(&mut self, _lesson: Lesson) -> Result<()> {
        // Save to simple database
        Ok(()) // TODO: Implement
    }

    fn save_decision(&mut self, _decision: Decision) -> Result<()> {
        // Save to simple database
        Ok(()) // TODO: Implement
    }

    fn extract_tags(&self, _text: &str) -> Vec<String> {
        // Extract meaningful tags from text
        vec![] // TODO: Implement
    }
}

// Simple output formatting for LLMs
impl Memory {
    pub fn format_for_llm(&self) -> String {
        let mut output = String::new();

        if let Some(session) = &self.last_session {
            output.push_str(&format!("## Last Session ({})\n", session.id));
            output.push_str(&format!("What we did: {}\n", session.what_we_did));

            if !session.what_failed.is_empty() {
                output.push_str("\n⚠️ What failed:\n");
                for failure in &session.what_failed {
                    output.push_str(&format!("- {failure}\n"));
                }
            }

            if !session.next_steps.is_empty() {
                output.push_str("\n📍 Next steps:\n");
                for step in &session.next_steps {
                    output.push_str(&format!("- {step}\n"));
                }
            }
        }

        if !self.lessons_learned.is_empty() {
            output.push_str("\n## Relevant Lessons\n");
            for lesson in &self.lessons_learned {
                output.push_str(&format!("- {}\n", lesson.lesson));
            }
        }

        if !self.active_decisions.is_empty() {
            output.push_str("\n## Active Decisions\n");
            for decision in &self.active_decisions {
                output.push_str(&format!(
                    "- {}: {}\n",
                    decision.decision, decision.reasoning
                ));
            }
        }

        output
    }
}
