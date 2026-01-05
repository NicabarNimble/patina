//! Query intent detection and oracle weighting
//!
//! Detects user intent from query text and provides intent-specific
//! oracle weights for RRF fusion.

/// Query intent categories
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum QueryIntent {
    /// Balanced search across all oracles
    #[default]
    General,
    /// When/history questions - boost commits, sessions
    Temporal,
    /// Why/decision questions - boost sessions, patterns
    Rationale,
    /// How/implementation questions - boost code
    Mechanism,
    /// What-is/definition questions - boost patterns
    Definition,
}

impl QueryIntent {
    /// Parse intent from string (for CLI/MCP parameter)
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "temporal" | "when" | "history" => Self::Temporal,
            "rationale" | "why" | "decision" => Self::Rationale,
            "mechanism" | "how" | "implementation" => Self::Mechanism,
            "definition" | "what" => Self::Definition,
            _ => Self::General,
        }
    }
}

/// Oracle weights for intent-aware RRF fusion
#[derive(Debug, Clone)]
pub struct IntentWeights {
    pub semantic: f32,
    pub lexical: f32,
    pub temporal: f32,
    pub persona: f32,
}

impl IntentWeights {
    /// Get weights for a specific intent
    ///
    /// Philosophy: Boost relevant oracles, but don't penalize others.
    /// Minimum weight is 1.0 to avoid hurting baseline performance.
    pub fn for_intent(intent: QueryIntent) -> Self {
        match intent {
            QueryIntent::General => Self {
                semantic: 1.0,
                lexical: 1.0,
                temporal: 1.0,
                persona: 1.0,
            },
            QueryIntent::Temporal => Self {
                semantic: 1.0,
                lexical: 2.0, // boost commits_fts, sessions
                temporal: 1.5,
                persona: 1.0,
            },
            QueryIntent::Rationale => Self {
                semantic: 1.0,
                lexical: 1.5, // boost patterns, sessions
                temporal: 1.0,
                persona: 1.5, // boost beliefs
            },
            QueryIntent::Mechanism => Self {
                semantic: 1.5, // boost code embeddings
                lexical: 1.0,
                temporal: 1.0,
                persona: 1.0,
            },
            QueryIntent::Definition => Self {
                semantic: 1.0,
                lexical: 1.5, // boost patterns
                temporal: 1.0,
                persona: 1.0,
            },
        }
    }

    /// Get weight for a specific oracle by name
    pub fn weight_for(&self, oracle_name: &str) -> f32 {
        match oracle_name.to_lowercase().as_str() {
            "semantic" => self.semantic,
            "lexical" => self.lexical,
            "temporal" => self.temporal,
            "persona" => self.persona,
            _ => 1.0,
        }
    }
}

/// Detect intent from query text
///
/// Uses simple keyword matching. The LLM can also provide explicit intent
/// via the `intent` parameter, which overrides detection.
pub fn detect_intent(query: &str) -> QueryIntent {
    let q = query.to_lowercase();

    // Temporal: when, history, added, changed, introduced
    if q.contains("when ")
        || q.starts_with("when")
        || q.contains(" added")
        || q.contains(" changed")
        || q.contains("history")
        || q.contains("introduced")
    {
        return QueryIntent::Temporal;
    }

    // Rationale: why, decided, chose, reason
    if q.contains("why ")
        || q.starts_with("why")
        || q.contains("decided")
        || q.contains("chose")
        || q.contains("reason")
    {
        return QueryIntent::Rationale;
    }

    // Mechanism: how X works, how does X
    if (q.contains("how ") || q.starts_with("how"))
        && (q.contains("work") || q.contains("implement") || q.contains("does"))
    {
        return QueryIntent::Mechanism;
    }

    // Definition: what is, explain, describe
    if q.contains("what is")
        || q.contains("what's")
        || q.starts_with("explain")
        || q.starts_with("describe")
    {
        return QueryIntent::Definition;
    }

    QueryIntent::General
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_temporal() {
        assert_eq!(
            detect_intent("when did we add commit message search"),
            QueryIntent::Temporal
        );
        assert_eq!(
            detect_intent("what changed in the last week"),
            QueryIntent::Temporal
        );
        assert_eq!(
            detect_intent("history of the retrieval module"),
            QueryIntent::Temporal
        );
    }

    #[test]
    fn test_detect_rationale() {
        assert_eq!(
            detect_intent("why did we choose RRF fusion"),
            QueryIntent::Rationale
        );
        assert_eq!(
            detect_intent("why is mother a federation layer"),
            QueryIntent::Rationale
        );
        assert_eq!(
            detect_intent("what was the reason for this change"),
            QueryIntent::Rationale
        );
    }

    #[test]
    fn test_detect_mechanism() {
        assert_eq!(
            detect_intent("how does moment detection work"),
            QueryIntent::Mechanism
        );
        assert_eq!(
            detect_intent("how is RRF fusion implemented"),
            QueryIntent::Mechanism
        );
    }

    #[test]
    fn test_detect_definition() {
        assert_eq!(
            detect_intent("what is the dependable rust pattern"),
            QueryIntent::Definition
        );
        assert_eq!(
            detect_intent("explain the adapter pattern"),
            QueryIntent::Definition
        );
    }

    #[test]
    fn test_detect_general() {
        assert_eq!(detect_intent("scry implementation"), QueryIntent::General);
        assert_eq!(detect_intent("find error handling"), QueryIntent::General);
    }
}
