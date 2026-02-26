//! Oxidize command - Build embeddings and projections from recipe
//!
//! Phase 2: Training + safetensors export + USearch index building

pub mod beliefs;
pub mod commits;
pub mod dependency;
pub mod pairs;
pub mod recipe;
pub mod temporal;
pub mod trainer;

use anyhow::{Context, Result};
use beliefs::generate_belief_pairs;
use commits::generate_commit_pairs;
use dependency::generate_dependency_pairs;
use pairs::TrainingPair;
use recipe::{OxidizeRecipe, ProjectionConfig};
use temporal::generate_temporal_pairs;
use trainer::Projection;

/// Run oxidize command
pub fn oxidize() -> Result<()> {
    println!("🧪 Oxidize - Build embeddings and projections");

    // Load recipe
    let recipe = OxidizeRecipe::load()?;

    let model_name = recipe.get_model_name()?;
    println!("✅ Recipe loaded: {}", model_name);
    println!("   Projections: {}", recipe.projections.len());

    for (name, config) in &recipe.projections {
        let is_raw = matches!(name.as_str(), "knowledge" | "semantic" | "sessions");
        if is_raw {
            println!(
                "   - {}: {}d raw E5 (no projection)",
                name,
                config.input_dim(&recipe)?,
            );
        } else {
            println!(
                "   - {}: {}→{}→{} ({} epochs)",
                name,
                config.input_dim(&recipe)?,
                config.hidden_dim(),
                config.output_dim(),
                config.epochs
            );
        }
    }

    let db_path = ".patina/local/data/patina.db";
    let output_dir = format!(".patina/local/data/embeddings/{}/projections", model_name);
    std::fs::create_dir_all(&output_dir)?;

    // Create embedder once, reuse for all projections
    use patina::embeddings::create_embedder;
    let mut embedder = create_embedder()?;

    // Build each domain (sorted for deterministic order)
    let mut sorted_projections: Vec<_> = recipe.projections.iter().collect();
    sorted_projections.sort_by(|a, b| a.0.cmp(b.0));
    let mut total_documents_embedded: usize = 0;
    for (name, config) in sorted_projections {
        println!("\n{}", "=".repeat(60));

        // Phase 5d: knowledge/sessions use raw E5 embeddings (no projection).
        // Raw E5 P@10=52.5% vs projected 9.2% — projection destroys E5's structure.
        let is_raw_domain = matches!(name.as_str(), "knowledge" | "semantic" | "sessions");

        if is_raw_domain {
            println!("🔮 Building {} index (raw E5, no projection)...", name);
            println!("{}", "=".repeat(60));

            // Delete stale projection weights if they exist
            let weights_path = format!("{}/{}.safetensors", output_dir, name);
            if std::path::Path::new(&weights_path).exists() {
                std::fs::remove_file(&weights_path)?;
                println!("   🗑️  Deleted stale projection: {}", weights_path);
            }

            // Build USearch index with raw embeddings (768-dim)
            let input_dim = config.input_dim(&recipe)?;
            println!("\n🔍 Building USearch index ({}d raw E5)...", input_dim);
            total_documents_embedded +=
                build_projection_index(name, db_path, &mut embedder, None, input_dim, &output_dir)?;
        } else {
            println!("📊 Training {} projection...", name);
            println!("{}", "=".repeat(60));

            let projection = train_projection(name, config, &recipe, db_path, &mut embedder)?;

            // Save trained weights
            println!("\n💾 Saving projection weights...");
            let weights_path = format!("{}/{}.safetensors", output_dir, name);
            projection.save_safetensors(std::path::Path::new(&weights_path))?;
            println!("   Saved to: {}", weights_path);

            // Build USearch index with projected embeddings
            println!("\n🔍 Building USearch index...");
            total_documents_embedded += build_projection_index(
                name,
                db_path,
                &mut embedder,
                Some(&projection),
                config.output_dim(),
                &output_dir,
            )?;
        }

        println!("\n✅ {} complete!", name);
    }

    println!("\n{}", "=".repeat(60));
    println!("✅ All domains built!");
    println!("   Output: {}", output_dir);

    // Emit measurement: index build metrics
    patina::measure::emit_or_warn(
        "index",
        "oxidize",
        "build",
        &serde_json::json!({
            "documents_embedded": total_documents_embedded,
            "projections_built": recipe.projections.len(),
        }),
    );

    Ok(())
}

/// Run oxidize for a registered external repo
///
/// Looks up repo path from registry, changes to that directory,
/// ensures recipe exists, and runs oxidize.
pub fn oxidize_for_repo(repo_name: &str) -> Result<()> {
    use std::os::unix::fs::symlink;

    // Look up repo path
    let repo_path = crate::commands::repo::get_path(repo_name)?;
    println!("🧪 Oxidize - Building embeddings for {}\n", repo_name);
    println!("   Path: {}", repo_path.display());

    // Save current directory (where patina project with models lives)
    let original_dir = std::env::current_dir()?;
    let resources_path = original_dir.join("resources");

    // Change to repo directory
    std::env::set_current_dir(&repo_path)?;

    // Ensure config.toml has embeddings section
    let config_path = repo_path.join(".patina/config.toml");
    if config_path.exists() {
        let config_content = std::fs::read_to_string(&config_path)?;
        if !config_content.contains("[embeddings]") {
            println!("   Adding embeddings config...");
            let updated = format!("{}\n[embeddings]\nmodel = \"e5-base-v2\"\n", config_content);
            std::fs::write(&config_path, updated)?;
        }
    }

    // Create oxidize.yaml if it doesn't exist
    let recipe_path = repo_path.join(".patina/oxidize.yaml");
    if !recipe_path.exists() {
        println!("   Creating oxidize.yaml recipe...\n");
        let recipe_content = r#"# Oxidize Recipe for reference repo
version: 1
embedding_model: e5-base-v2

projections:
  dependency:
    layers: [768, 1024, 256]
    epochs: 10
    batch_size: 32

  temporal:
    layers: [768, 1024, 256]
    epochs: 10
    batch_size: 32

  knowledge:
    layers: [768, 1024, 256]
    epochs: 10
    batch_size: 32
"#;
        std::fs::write(&recipe_path, recipe_content)?;
    }

    // Symlink resources directory if needed (for embedding models)
    let repo_resources = repo_path.join("resources");
    if !repo_resources.exists() && resources_path.exists() {
        println!("   Linking model resources...\n");
        symlink(&resources_path, &repo_resources).context("Failed to create resources symlink")?;
    }

    // Run oxidize
    let result = oxidize();

    // Clean up symlink
    if repo_resources.is_symlink() {
        let _ = std::fs::remove_file(&repo_resources);
    }

    // Restore directory
    std::env::set_current_dir(original_dir)?;

    result
}

/// Train a projection based on its name
fn train_projection(
    name: &str,
    config: &ProjectionConfig,
    recipe: &OxidizeRecipe,
    db_path: &str,
    embedder: &mut Box<dyn patina::embeddings::EmbeddingEngine>,
) -> Result<Projection> {
    // Generate pairs based on projection type
    // Phase 5c: use ALL available pairs, not a random subset.
    // Phase 5d: knowledge domain combines commit + belief co-reference pairs.
    let pairs: Vec<TrainingPair> = match name {
        "knowledge" | "semantic" => {
            // Knowledge domain: commit pairs + belief-pattern co-reference pairs
            // Phase 5d: belief pairs teach vocabulary gap bridging between
            // conceptual queries and belief/pattern documents.
            println!("   Strategy: commit pairs + belief-pattern co-references");
            let mut all_pairs = generate_commit_pairs(db_path)?;
            match generate_belief_pairs(db_path) {
                Ok(belief_pairs) => {
                    println!("   Adding {} belief co-reference pairs", belief_pairs.len());
                    all_pairs.extend(belief_pairs);
                }
                Err(e) => {
                    println!("   ⚠️  Belief pairs skipped: {}", e);
                }
            }
            // Sort for determinism (pairs from different sources)
            all_pairs.sort_by(|a, b| a.anchor.cmp(&b.anchor));
            all_pairs
        }
        "sessions" => {
            // Session domain (Phase 5b): reuse commit-based training signal
            // Session content is natural language, same embedding space works
            println!("   Strategy: commit-based pairs (shared training signal)");
            generate_commit_pairs(db_path)?
        }
        "temporal" => {
            println!("   Strategy: files that co-change are related");
            generate_temporal_pairs(db_path)?
        }
        "dependency" => {
            println!("   Strategy: functions that call each other are related");
            generate_dependency_pairs(db_path)?
        }
        _ => {
            anyhow::bail!(
                "Unknown projection type: {}. Supported: knowledge, sessions, semantic, temporal, dependency",
                name
            );
        }
    };

    println!("   Generated {} training pairs", pairs.len());

    // Generate embeddings
    println!("\n🔮 Generating embeddings...");
    let mut anchors = Vec::new();
    let mut positives = Vec::new();
    let mut negatives = Vec::new();

    for pair in &pairs {
        anchors.push(embedder.embed_passage(&pair.anchor)?);
        positives.push(embedder.embed_passage(&pair.positive)?);
        negatives.push(embedder.embed_passage(&pair.negative)?);
    }

    println!("   Embedded {} triplets", anchors.len());

    // Train projection
    let input_dim = config.input_dim(recipe)?;
    println!(
        "\n🧠 Training MLP: {}→{}→{}...",
        input_dim,
        config.hidden_dim(),
        config.output_dim()
    );

    let mut projection = Projection::new(input_dim, config.hidden_dim(), config.output_dim());

    let learning_rate = 0.001;
    let _losses = projection.train(
        &anchors,
        &positives,
        &negatives,
        config.epochs,
        learning_rate,
    )?;

    println!("   Training complete!");

    Ok(projection)
}

/// Build USearch index from embeddings (projected or raw)
///
/// When projection is Some, embeddings are projected through the MLP.
/// When projection is None (knowledge/sessions domains), raw E5 embeddings
/// are indexed directly — Phase 5d proved raw E5 outperforms projection.
fn build_projection_index(
    projection_name: &str,
    db_path: &str,
    embedder: &mut Box<dyn patina::embeddings::EmbeddingEngine>,
    projection: Option<&Projection>,
    index_dim: usize,
    output_dir: &str,
) -> Result<usize> {
    use rusqlite::Connection;
    use usearch::{Index, IndexOptions, MetricKind, ScalarKind};

    // Open database
    let conn = Connection::open(db_path)
        .with_context(|| format!("Failed to open database: {}", db_path))?;

    // Get content to index based on projection type
    let events: Vec<(i64, String)> = match projection_name {
        "knowledge" | "semantic" => query_knowledge_corpus(&conn)?,
        "sessions" => query_session_corpus(&conn)?,
        "temporal" => query_file_events(&conn)?,
        "dependency" => dependency::query_function_events(&conn)?,
        _ => {
            println!("   ⚠️  No index builder for {} - skipping", projection_name);
            return Ok(0);
        }
    };

    println!("   Found {} items to index", events.len());

    if events.is_empty() {
        println!("   ⚠️  No items found - skipping index build");
        return Ok(0);
    }

    // Create USearch index
    let options = IndexOptions {
        dimensions: index_dim,
        metric: MetricKind::Cos,
        quantization: ScalarKind::F32,
        ..Default::default()
    };

    let index = Index::new(&options).context("Failed to create USearch index")?;
    index
        .reserve(events.len())
        .context("Failed to reserve index capacity")?;

    // Embed (and optionally project) and add to index
    let mode = if projection.is_some() {
        "projecting"
    } else {
        "raw"
    };
    println!("   Embedding vectors ({} mode)...", mode);
    for (id, content) in &events {
        let embedding = embedder
            .embed_passage(content)
            .context("Failed to generate embedding")?;
        let vector = match projection {
            Some(proj) => proj.forward(&embedding),
            None => embedding,
        };
        index
            .add(*id as u64, &vector)
            .context("Failed to add vector to index")?;
    }

    // Save index
    let index_path = format!("{}/{}.usearch", output_dir, projection_name);
    index
        .save(&index_path)
        .context("Failed to save USearch index")?;

    let count = events.len();
    println!("   ✅ Index built: {} vectors", count);
    println!("   Saved to: {}", index_path);

    Ok(count)
}

/// Query knowledge corpus for semantic index — beliefs + patterns + commits only
///
/// pub(crate) for use by eval raw E5 diagnostic (Phase 5d).
///
/// Phase 2 of the semantic-structural split: build a clean knowledge domain
/// instead of the polluted 27K-item session-dominated index. Knowledge items
/// are natural language content where semantic matching adds value over FTS5.
///
/// Phase 5a: Corpus optimization — enriched belief/pattern text, filtered commits.
/// Root cause of 4/20 scry-vs-assay gap was commit dominance (92% of index).
/// See [[semantic-structural-split]] Phase 5a for diagnostic evidence.
pub(crate) fn query_knowledge_corpus(conn: &rusqlite::Connection) -> Result<Vec<(i64, String)>> {
    let mut events = Vec::new();

    use patina::embeddings::offsets::{
        BELIEF_ID_OFFSET, COMMIT_ID_OFFSET, FORGE_ID_OFFSET, PATTERN_ID_OFFSET,
    };

    // E5-base-v2 has a 512 token window (~2000 chars). Use up to 1500 chars
    // of content for beliefs/patterns to maximize semantic signal per item.
    const MAX_CONTENT_CHARS: usize = 1500;

    // 1. Layer patterns from patterns + pattern_fts tables (enriched text)
    let has_patterns: bool = conn
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='patterns'",
            [],
            |row| row.get::<_, i64>(0),
        )
        .map(|c| c > 0)
        .unwrap_or(false);

    if has_patterns {
        let mut stmt = conn.prepare(
            "SELECT p.rowid, p.id, p.title, p.purpose, f.content, p.tags, p.file_path
             FROM patterns p
             LEFT JOIN pattern_fts f ON p.id = f.id",
        )?;

        let mut rows = stmt.query([])?;
        while let Some(row) = rows.next()? {
            let rowid: i64 = row.get(0)?;
            let id: String = row.get(1)?;
            let title: String = row.get(2)?;
            let purpose: Option<String> = row.get(3)?;
            let content: Option<String> = row.get(4)?;
            let tags: Option<String> = row.get(5)?;
            let file_path: String = row.get(6)?;

            let mut desc = format!("Pattern: {} - {}", title, id);
            if let Some(p) = purpose {
                desc.push_str(&format!(". Purpose: {}", p));
            }
            if let Some(t) = tags {
                if !t.is_empty() {
                    desc.push_str(&format!(". Tags: {}", t));
                }
            }
            // Phase 5a: use up to 1500 chars of content (was 500)
            if let Some(c) = content {
                let content_preview: String = c.chars().take(MAX_CONTENT_CHARS).collect();
                desc.push_str(&format!(". Content: {}", content_preview));
            }
            desc.push_str(&format!(". File: {}", file_path));

            events.push((PATTERN_ID_OFFSET + rowid, desc));
        }
    }

    let pattern_count = events.len();

    // 2. Git commits — filtered to significant subset (Phase 5a)
    //
    // Original: all 1,824 commits with msg>30 chars (92% of index).
    // Now: only commits with rich messages (>75 chars), belief references,
    // release tags, or structural significance (>5 files changed).
    // This reduces commits to ~400 and shifts ratio from 92%/4%/4% to ~70%/15%/15%.
    let has_commit_files: bool = conn
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='commit_files'",
            [],
            |row| row.get::<_, i64>(0),
        )
        .map(|c| c > 0)
        .unwrap_or(false);

    let commit_query = if has_commit_files {
        "SELECT c.rowid, c.sha, c.message FROM commits c
         WHERE c.message IS NOT NULL AND length(c.message) > 30
         AND (
           length(c.message) > 75
           OR c.message LIKE '%belief%'
           OR c.message LIKE 'release%'
           OR (SELECT COUNT(*) FROM commit_files cf WHERE cf.sha = c.sha) > 5
         )
         ORDER BY c.rowid"
    } else {
        // Fallback if commit_files table doesn't exist: filter by message only
        "SELECT rowid, sha, message FROM commits
         WHERE message IS NOT NULL AND length(message) > 30
         AND (
           length(message) > 75
           OR message LIKE '%belief%'
           OR message LIKE 'release%'
         )
         ORDER BY rowid"
    };

    let mut stmt = conn.prepare(commit_query)?;

    let mut rows = stmt.query([])?;
    while let Some(row) = rows.next()? {
        let rowid: i64 = row.get(0)?;
        let sha: String = row.get(1)?;
        let message: String = row.get(2)?;

        let desc = format!("Commit {}: {}", &sha[..7.min(sha.len())], message);
        events.push((COMMIT_ID_OFFSET + rowid, desc));
    }

    let commit_count = events.len() - pattern_count;

    // 3. Epistemic beliefs — enriched with content from belief_fts (Phase 5a)
    //
    // Original: ~100 chars per belief (id + statement + persona + facets).
    // Now: includes body content from belief_fts (evidence, references, context)
    // for richer embeddings that bridge wider vocabulary gaps.
    let has_beliefs: bool = conn
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='beliefs'",
            [],
            |row| row.get::<_, i64>(0),
        )
        .map(|c| c > 0)
        .unwrap_or(false);

    let has_belief_fts: bool = conn
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='belief_fts'",
            [],
            |row| row.get::<_, i64>(0),
        )
        .map(|c| c > 0)
        .unwrap_or(false);

    if has_beliefs {
        let belief_query = if has_belief_fts {
            "SELECT b.rowid, b.id, b.statement, b.persona, b.facets,
                    b.confidence, b.entrenchment, bf.content
             FROM beliefs b
             LEFT JOIN belief_fts bf ON b.id = bf.id
             WHERE b.status = 'active'"
        } else {
            "SELECT rowid, id, statement, persona, facets,
                    confidence, entrenchment, NULL as content
             FROM beliefs
             WHERE status = 'active'"
        };

        let mut stmt = conn.prepare(belief_query)?;

        let mut rows = stmt.query([])?;
        while let Some(row) = rows.next()? {
            let rowid: i64 = row.get(0)?;
            let id: String = row.get(1)?;
            let statement: String = row.get(2)?;
            let persona: String = row.get(3)?;
            let facets: Option<String> = row.get(4)?;
            let confidence: f64 = row.get(5)?;
            let entrenchment: String = row.get(6)?;
            let fts_content: Option<String> = row.get(7)?;

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

            // Phase 5a: append body content from belief_fts for richer embeddings
            if let Some(content) = fts_content {
                // Strip YAML frontmatter (everything before first blank line after ---)
                let body = strip_frontmatter(&content);
                if !body.is_empty() {
                    let remaining = MAX_CONTENT_CHARS.saturating_sub(desc.len());
                    if remaining > 50 {
                        let preview: String = body.chars().take(remaining).collect();
                        desc.push_str(&format!(". {}", preview));
                    }
                }
            }

            events.push((BELIEF_ID_OFFSET + rowid, desc));
        }
    }

    let belief_count = events.len() - pattern_count - commit_count;

    // 4. Forge facts (issues + PRs) from eventlog
    //
    // Forge events are stored in eventlog with event_type 'forge.issue' or 'forge.pr'.
    // Key is FORGE_ID_OFFSET + seq so enrichment can look them back up by seq.
    let forge_count = {
        let mut stmt = conn.prepare(
            "SELECT seq, event_type, source_id,
                    json_extract(data, '$.title') as title,
                    json_extract(data, '$.body') as body
             FROM eventlog
             WHERE event_type IN ('forge.issue', 'forge.pr')
             ORDER BY seq",
        )?;

        let mut count = 0;
        let mut rows = stmt.query([])?;
        while let Some(row) = rows.next()? {
            let seq: i64 = row.get(0)?;
            let event_type: String = row.get(1)?;
            let source_id: String = row.get(2)?;
            let title: String = row.get::<_, Option<String>>(3)?.unwrap_or_default();
            let body: String = row.get::<_, Option<String>>(4)?.unwrap_or_default();

            let kind = if event_type == "forge.pr" {
                "PR"
            } else {
                "Issue"
            };

            let mut desc = format!("{} #{}: {}", kind, source_id, title);
            if !body.is_empty() {
                let preview: String = body.chars().take(MAX_CONTENT_CHARS).collect();
                desc.push_str(&format!(". {}", preview));
            }

            events.push((FORGE_ID_OFFSET + seq, desc));
            count += 1;
        }
        count
    };

    println!(
        "   Knowledge corpus: {} patterns + {} commits + {} beliefs + {} forge = {} items",
        pattern_count,
        commit_count,
        belief_count,
        forge_count,
        events.len()
    );

    Ok(events)
}

/// Strip YAML frontmatter from markdown content
fn strip_frontmatter(content: &str) -> &str {
    if !content.starts_with("---") {
        return content;
    }
    // Find the closing --- after the opening one
    if let Some(end) = content[3..].find("\n---") {
        let after_frontmatter = &content[3 + end + 4..];
        after_frontmatter.trim_start()
    } else {
        content
    }
}

/// Query session corpus for session-semantic index (Phase 5b)
///
/// Extracts unique session events (decisions, patterns, work, context) from the
/// eventlog. Deduplicates by (source_id, content) since each scrape re-inserts.
/// Uses eventlog seq as ID key (matches enrichment module's eventlog lookup).
///
/// Content filtering: only events with >50 chars, only high-value session types.
fn query_session_corpus(conn: &rusqlite::Connection) -> Result<Vec<(i64, String)>> {
    // Session events use their raw eventlog seq as the index key.
    // The enrichment module's eventlog branch (key < CODE_ID_OFFSET) handles these.
    let mut stmt = conn.prepare(
        "SELECT MIN(seq) as seq, source_id, event_type,
                json_extract(data, '$.content') as content
         FROM eventlog
         WHERE event_type IN ('session.decision', 'session.pattern',
                              'session.work', 'session.context')
         AND length(json_extract(data, '$.content')) > 50
         GROUP BY source_id, event_type, json_extract(data, '$.content')
         ORDER BY seq",
    )?;

    let mut events = Vec::new();
    let mut rows = stmt.query([])?;
    let mut type_counts = std::collections::HashMap::new();

    while let Some(row) = rows.next()? {
        let seq: i64 = row.get(0)?;
        let source_id: String = row.get(1)?;
        let event_type: String = row.get(2)?;
        let content: String = row.get(3)?;

        // Build descriptive text for embedding
        let type_label = event_type.strip_prefix("session.").unwrap_or(&event_type);
        let desc = format!("Session {} ({}): {}", source_id, type_label, content);

        events.push((seq, desc));
        *type_counts.entry(type_label.to_string()).or_insert(0) += 1;
    }

    let type_summary: Vec<String> = type_counts
        .iter()
        .map(|(k, v)| format!("{} {}", v, k))
        .collect();

    println!(
        "   Session corpus: {} items ({})",
        events.len(),
        type_summary.join(" + ")
    );

    Ok(events)
}

/// Query file events for temporal index
fn query_file_events(conn: &rusqlite::Connection) -> Result<Vec<(i64, String)>> {
    // Get unique files from co_changes with their index
    let mut stmt = conn.prepare(
        "SELECT DISTINCT file_a FROM co_changes
         UNION
         SELECT DISTINCT file_b FROM co_changes
         ORDER BY 1",
    )?;

    let mut events = Vec::new();
    let mut rows = stmt.query([])?;
    let mut idx: i64 = 0;
    while let Some(row) = rows.next()? {
        let file_path: String = row.get(0)?;
        // Convert file path to descriptive text for embedding
        let text = temporal::file_to_text(&file_path);
        events.push((idx, text));
        idx += 1;
    }

    Ok(events)
}
