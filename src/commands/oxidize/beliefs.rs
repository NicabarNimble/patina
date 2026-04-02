//! Belief-pattern co-reference training pair generator
//!
//! Phase 5d: Beliefs reference patterns and other beliefs via [[id]] links
//! in their Supports/Attacks/Applied-In sections. These co-references are
//! human-curated semantic relationships that directly teach vocabulary gap
//! bridging — e.g., a belief about "error analysis" references the
//! "dependable-rust" pattern, teaching the projection that these concepts
//! are related despite different vocabulary.
//!
//! Strategy:
//! - Anchor: belief enriched text (statement + evidence + context)
//! - Positive: referenced pattern or belief enriched text
//! - Negative: random unrelated pattern or belief

use super::pairs::TrainingPair;
use super::strip_frontmatter;
use anyhow::{Context, Result};
use rusqlite::Connection;
use std::collections::{HashMap, HashSet};

/// Generate training pairs from belief-pattern co-references
///
/// Parses [[id]] references from belief content, matches them to patterns
/// and other beliefs, and creates triplets for contrastive learning.
pub fn generate_belief_pairs(db_path: &str) -> Result<Vec<TrainingPair>> {
    let conn = Connection::open(db_path)
        .with_context(|| format!("Failed to open database: {}", db_path))?;

    // Load all belief enriched texts (same format as knowledge corpus)
    let belief_texts = load_belief_texts(&conn)?;
    if belief_texts.is_empty() {
        anyhow::bail!("No active beliefs found for training");
    }

    // Load all pattern enriched texts
    let pattern_texts = load_pattern_texts(&conn)?;

    // Combined lookup: both beliefs and patterns as potential targets
    let mut all_texts: HashMap<String, String> = HashMap::new();
    for (id, text) in &belief_texts {
        all_texts.insert(id.clone(), text.clone());
    }
    for (id, text) in &pattern_texts {
        all_texts.insert(id.clone(), text.clone());
    }

    println!(
        "   Found {} beliefs, {} patterns as training targets",
        belief_texts.len(),
        pattern_texts.len()
    );

    // Parse references from belief content
    let belief_refs = parse_belief_references(&conn)?;

    // Build sorted list of all target IDs for negative sampling
    let mut all_target_ids: Vec<&String> = all_texts.keys().collect();
    all_target_ids.sort();

    // Generate pairs
    // Sort belief_refs keys for deterministic iteration per [[hashmap-determinism-landmine]]
    let mut pairs = Vec::new();
    let mut rng = fastrand::Rng::with_seed(42);
    let mut sorted_belief_ids: Vec<&String> = belief_refs.keys().collect();
    sorted_belief_ids.sort();

    for belief_id in sorted_belief_ids {
        let referenced_ids = &belief_refs[belief_id];
        let anchor_text = match belief_texts.get(belief_id) {
            Some(t) => t,
            None => continue,
        };

        // Create one pair per valid reference
        for ref_id in referenced_ids {
            let positive_text = match all_texts.get(ref_id) {
                Some(t) => t,
                None => continue, // Referenced ID not in database
            };

            // Find a negative: random target not referenced by this belief
            let ref_set: HashSet<&str> = referenced_ids.iter().map(|s| s.as_str()).collect();
            let available_negatives: Vec<_> = all_target_ids
                .iter()
                .filter(|id| !ref_set.contains(id.as_str()) && **id != belief_id)
                .collect();

            if available_negatives.is_empty() {
                continue;
            }

            let neg_idx = rng.usize(..available_negatives.len());
            let negative_id = available_negatives[neg_idx];
            let negative_text = &all_texts[*negative_id];

            pairs.push(TrainingPair {
                anchor: anchor_text.clone(),
                positive: positive_text.clone(),
                negative: negative_text.clone(),
            });
        }
    }

    if pairs.is_empty() {
        anyhow::bail!("Could not generate any training pairs from belief references");
    }

    println!(
        "   Generated {} belief co-reference pairs from {} beliefs",
        pairs.len(),
        belief_refs.len()
    );

    Ok(pairs)
}

/// Load enriched text for all active beliefs (matches knowledge corpus format)
fn load_belief_texts(conn: &Connection) -> Result<HashMap<String, String>> {
    const MAX_CONTENT_CHARS: usize = 1500;

    let has_belief_fts: bool = conn
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='belief_fts'",
            [],
            |row| row.get::<_, i64>(0),
        )
        .map(|c| c > 0)
        .unwrap_or(false);

    let query = if has_belief_fts {
        "SELECT b.id, b.statement, b.persona, b.facets,
                b.confidence, b.entrenchment, bf.content
         FROM beliefs b
         LEFT JOIN belief_fts bf ON b.id = bf.id
         WHERE b.status = 'active'
         ORDER BY b.id"
    } else {
        "SELECT id, statement, persona, facets,
                confidence, entrenchment, NULL as content
         FROM beliefs
         WHERE status = 'active'
         ORDER BY id"
    };

    let mut stmt = conn.prepare(query)?;
    let mut texts = HashMap::new();
    let mut rows = stmt.query([])?;

    while let Some(row) = rows.next()? {
        let id: String = row.get(0)?;
        let statement: String = row.get(1)?;
        let persona: String = row.get(2)?;
        let facets: Option<String> = row.get(3)?;
        let confidence: f64 = row.get(4)?;
        let entrenchment: String = row.get(5)?;
        let fts_content: Option<String> = row.get(6)?;

        let mut desc = format!("Belief: {} - {}", id, statement);
        desc.push_str(&format!(". Persona: {}", persona));
        if let Some(f) = &facets {
            if !f.is_empty() {
                desc.push_str(&format!(". Facets: {}", f));
            }
        }
        desc.push_str(&format!(
            ". Confidence: {:.2}, Entrenchment: {}",
            confidence, entrenchment
        ));

        if let Some(content) = fts_content {
            let body = strip_frontmatter(&content);
            if !body.is_empty() {
                let remaining = MAX_CONTENT_CHARS.saturating_sub(desc.len());
                if remaining > 50 {
                    let preview: String = body.chars().take(remaining).collect();
                    desc.push_str(&format!(". {}", preview));
                }
            }
        }

        texts.insert(id, desc);
    }

    Ok(texts)
}

/// Load enriched text for all patterns (matches knowledge corpus format)
fn load_pattern_texts(conn: &Connection) -> Result<HashMap<String, String>> {
    const MAX_CONTENT_CHARS: usize = 1500;

    let has_patterns: bool = conn
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='patterns'",
            [],
            |row| row.get::<_, i64>(0),
        )
        .map(|c| c > 0)
        .unwrap_or(false);

    if !has_patterns {
        return Ok(HashMap::new());
    }

    let mut stmt = conn.prepare(
        "SELECT p.id, p.title, p.purpose, f.content, p.tags, p.file_path
         FROM patterns p
         LEFT JOIN pattern_fts f ON p.id = f.id
         ORDER BY p.id",
    )?;

    let mut texts = HashMap::new();
    let mut rows = stmt.query([])?;

    while let Some(row) = rows.next()? {
        let id: String = row.get(0)?;
        let title: String = row.get(1)?;
        let purpose: Option<String> = row.get(2)?;
        let content: Option<String> = row.get(3)?;
        let tags: Option<String> = row.get(4)?;
        let file_path: String = row.get(5)?;

        let mut desc = format!("Pattern: {} - {}", title, id);
        if let Some(p) = purpose {
            desc.push_str(&format!(". Purpose: {}", p));
        }
        if let Some(t) = tags {
            if !t.is_empty() {
                desc.push_str(&format!(". Tags: {}", t));
            }
        }
        if let Some(c) = content {
            let content_preview: String = c.chars().take(MAX_CONTENT_CHARS).collect();
            desc.push_str(&format!(". Content: {}", content_preview));
        }
        desc.push_str(&format!(". File: {}", file_path));

        texts.insert(id, desc);
    }

    Ok(texts)
}

/// Parse [[id]] references from belief content sections
///
/// Extracts references from Supports, Attacks, Attacked-By, and Applied-In
/// sections of each belief's markdown content.
fn parse_belief_references(conn: &Connection) -> Result<HashMap<String, Vec<String>>> {
    let has_belief_fts: bool = conn
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='belief_fts'",
            [],
            |row| row.get::<_, i64>(0),
        )
        .map(|c| c > 0)
        .unwrap_or(false);

    if !has_belief_fts {
        return Ok(HashMap::new());
    }

    let mut stmt = conn.prepare(
        "SELECT b.id, bf.content
         FROM beliefs b
         JOIN belief_fts bf ON b.id = bf.id
         WHERE b.status = 'active'
         ORDER BY b.id",
    )?;

    let mut refs = HashMap::new();
    let mut rows = stmt.query([])?;

    while let Some(row) = rows.next()? {
        let id: String = row.get(0)?;
        let content: String = row.get(1)?;

        let referenced = extract_wiki_links(&content);
        if !referenced.is_empty() {
            // Filter out self-references and session references
            let filtered: Vec<String> = referenced
                .into_iter()
                .filter(|r| r != &id && !r.starts_with("session-"))
                .collect();
            if !filtered.is_empty() {
                refs.insert(id, filtered);
            }
        }
    }

    Ok(refs)
}

/// Extract [[id]] wiki-style links from markdown content
fn extract_wiki_links(content: &str) -> Vec<String> {
    let mut links = Vec::new();
    let mut seen = HashSet::new();

    let bytes = content.as_bytes();
    let len = bytes.len();
    let mut i = 0;

    while i + 3 < len {
        if bytes[i] == b'[' && bytes[i + 1] == b'[' {
            // Found opening [[
            let start = i + 2;
            if let Some(end_offset) = content[start..].find("]]") {
                let link_text = &content[start..start + end_offset];
                // Take only the ID part (before any space or pipe)
                let id = link_text
                    .split_whitespace()
                    .next()
                    .unwrap_or(link_text)
                    .trim();
                if !id.is_empty() && !seen.contains(id) {
                    seen.insert(id.to_string());
                    links.push(id.to_string());
                }
                i = start + end_offset + 2;
            } else {
                i += 2;
            }
        } else {
            i += 1;
        }
    }

    links.sort();
    links
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_wiki_links() {
        let content = r#"## Supports
- [[error-analysis-over-architecture]]: some description
- [[andrew-ng-over-shoulder]]: another description

## Applied-In
- [[semantic-structural-split]] Phase 2: details
"#;
        let links = extract_wiki_links(content);
        assert_eq!(
            links,
            vec![
                "andrew-ng-over-shoulder",
                "error-analysis-over-architecture",
                "semantic-structural-split"
            ]
        );
    }

    #[test]
    fn test_extract_wiki_links_dedup() {
        let content = "[[foo]] and [[bar]] and [[foo]] again";
        let links = extract_wiki_links(content);
        assert_eq!(links, vec!["bar", "foo"]);
    }

    #[test]
    fn test_extract_wiki_links_empty() {
        let content = "No links here, just regular text.";
        let links = extract_wiki_links(content);
        assert!(links.is_empty());
    }

    #[test]
    fn test_strip_frontmatter() {
        let content = "---\ntype: belief\nid: test\n---\n\n# Title\nBody text";
        let body = crate::commands::oxidize::strip_frontmatter(content);
        assert_eq!(body, "# Title\nBody text");
    }
}
