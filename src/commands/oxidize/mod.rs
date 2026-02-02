//! Oxidize command - Build embeddings and projections from recipe
//!
//! Phase 2: Training + safetensors export + USearch index building

pub mod commits;
pub mod dependency;
pub mod pairs;
pub mod recipe;
pub mod temporal;
pub mod trainer;

use anyhow::{Context, Result};
use commits::{generate_commit_pairs, has_commits, has_sessions};
use dependency::generate_dependency_pairs;
use pairs::{generate_same_session_pairs, TrainingPair};
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
        println!(
            "   - {}: {}→{}→{} ({} epochs)",
            name,
            config.input_dim(&recipe)?,
            config.hidden_dim(),
            config.output_dim(),
            config.epochs
        );
    }

    let db_path = ".patina/local/data/patina.db";
    let output_dir = format!(".patina/local/data/embeddings/{}/projections", model_name);
    std::fs::create_dir_all(&output_dir)?;

    // Create embedder once, reuse for all projections
    use patina::embeddings::create_embedder;
    let mut embedder = create_embedder()?;

    // Train each projection
    for (name, config) in &recipe.projections {
        println!("\n{}", "=".repeat(60));
        println!("📊 Training {} projection...", name);
        println!("{}", "=".repeat(60));

        let projection = train_projection(name, config, &recipe, db_path, &mut embedder)?;

        // Save trained weights
        println!("\n💾 Saving projection weights...");
        let weights_path = format!("{}/{}.safetensors", output_dir, name);
        projection.save_safetensors(std::path::Path::new(&weights_path))?;
        println!("   Saved to: {}", weights_path);

        // Build USearch index
        println!("\n🔍 Building USearch index...");
        build_projection_index(
            name,
            db_path,
            &mut embedder,
            &projection,
            config.output_dim(),
            &output_dir,
        )?;

        println!("\n✅ {} projection complete!", name);
    }

    println!("\n{}", "=".repeat(60));
    println!("✅ All projections trained!");
    println!("   Output: {}", output_dir);

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

  semantic:
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
    let num_pairs = 100; // Start with 100 pairs for MVP

    // Generate pairs based on projection type
    let pairs: Vec<TrainingPair> = match name {
        "semantic" => {
            // Check which training signal is available
            let conn = rusqlite::Connection::open(db_path)
                .with_context(|| format!("Failed to open database: {}", db_path))?;

            if has_sessions(&conn)? {
                // Sessions capture user intent (what user thinks about together)
                println!("   Strategy: session observations capture user intent");
                drop(conn);
                generate_same_session_pairs(db_path, num_pairs)?
            } else if has_commits(&conn)? {
                // Commits capture code cohesion (what changes together)
                println!("   Strategy: commit messages capture code cohesion");
                drop(conn);
                generate_commit_pairs(db_path, num_pairs)?
            } else {
                anyhow::bail!("No training signal: neither sessions nor commits found")
            }
        }
        "temporal" => {
            println!("   Strategy: files that co-change are related");
            generate_temporal_pairs(db_path, num_pairs)?
        }
        "dependency" => {
            println!("   Strategy: functions that call each other are related");
            generate_dependency_pairs(db_path, num_pairs)?
        }
        _ => {
            anyhow::bail!(
                "Unknown projection type: {}. Supported: semantic, temporal, dependency",
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

/// Build USearch index from projected embeddings
fn build_projection_index(
    projection_name: &str,
    db_path: &str,
    embedder: &mut Box<dyn patina::embeddings::EmbeddingEngine>,
    projection: &Projection,
    output_dim: usize,
    output_dir: &str,
) -> Result<()> {
    use rusqlite::Connection;
    use usearch::{Index, IndexOptions, MetricKind, ScalarKind};

    // Open database
    let conn = Connection::open(db_path)
        .with_context(|| format!("Failed to open database: {}", db_path))?;

    // Get content to index based on projection type
    let events: Vec<(i64, String)> = match projection_name {
        "semantic" => query_session_events(&conn)?,
        "temporal" => query_file_events(&conn)?,
        "dependency" => dependency::query_function_events(&conn)?,
        _ => {
            println!("   ⚠️  No index builder for {} - skipping", projection_name);
            return Ok(());
        }
    };

    println!("   Found {} items to index", events.len());

    if events.is_empty() {
        println!("   ⚠️  No items found - skipping index build");
        return Ok(());
    }

    // Create USearch index
    let options = IndexOptions {
        dimensions: output_dim,
        metric: MetricKind::Cos,
        quantization: ScalarKind::F32,
        ..Default::default()
    };

    let index = Index::new(&options).context("Failed to create USearch index")?;
    index
        .reserve(events.len())
        .context("Failed to reserve index capacity")?;

    // Embed, project, and add to index
    println!("   Embedding and projecting vectors...");
    for (id, content) in &events {
        let embedding = embedder
            .embed_passage(content)
            .context("Failed to generate embedding")?;
        let projected = projection.forward(&embedding);
        index
            .add(*id as u64, &projected)
            .context("Failed to add vector to index")?;
    }

    // Save index
    let index_path = format!("{}/{}.usearch", output_dir, projection_name);
    index
        .save(&index_path)
        .context("Failed to save USearch index")?;

    println!("   ✅ Index built: {} vectors", events.len());
    println!("   Saved to: {}", index_path);

    Ok(())
}

/// Query session events for semantic index
fn query_session_events(conn: &rusqlite::Connection) -> Result<Vec<(i64, String)>> {
    let mut events = Vec::new();

    // 1. Session events from eventlog
    let mut stmt = conn.prepare(
        "SELECT seq, json_extract(data, '$.content') as content
         FROM eventlog
         WHERE event_type IN ('session.decision', 'session.pattern', 'session.goal', 'session.work', 'session.context')
           AND content IS NOT NULL
           AND length(content) > 20
         ORDER BY seq",
    )?;

    let mut rows = stmt.query([])?;
    while let Some(row) = rows.next()? {
        let seq: i64 = row.get(0)?;
        let content: String = row.get(1)?;
        events.push((seq, content));
    }

    let session_count = events.len();

    // 2. Code facts from function_facts (use offset to avoid ID collision)
    const CODE_ID_OFFSET: i64 = 1_000_000_000;
    let mut stmt = conn.prepare(
        "SELECT rowid, file, name, parameters, return_type, is_public, is_async
         FROM function_facts
         WHERE name != ''",
    )?;

    let mut rows = stmt.query([])?;
    while let Some(row) = rows.next()? {
        let rowid: i64 = row.get(0)?;
        let file: String = row.get(1)?;
        let name: String = row.get(2)?;
        let params: Option<String> = row.get(3)?;
        let return_type: Option<String> = row.get(4)?;
        let is_public: bool = row.get(5)?;
        let is_async: bool = row.get(6)?;

        // Create embeddable text for the function
        let mut desc = format!("Function `{}` in file `{}`", name, file);
        if is_public {
            desc.push_str(", public");
        }
        if is_async {
            desc.push_str(", async");
        }
        if let Some(p) = params {
            if !p.is_empty() {
                desc.push_str(&format!(", parameters: {}", p));
            }
        }
        if let Some(rt) = return_type {
            if !rt.is_empty() {
                desc.push_str(&format!(", returns: {}", rt));
            }
        }

        events.push((CODE_ID_OFFSET + rowid, desc));
    }

    let code_count = events.len() - session_count;

    // 3. Layer patterns from patterns + pattern_fts tables (use offset to avoid ID collision)
    // Note: patterns table may not exist in ref repos - skip gracefully
    const PATTERN_ID_OFFSET: i64 = 2_000_000_000;
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

            // Create embeddable text for the pattern
            // Include title, purpose, tags, and content for rich semantic matching
            let mut desc = format!("Pattern: {} - {}", title, id);
            if let Some(p) = purpose {
                desc.push_str(&format!(". Purpose: {}", p));
            }
            if let Some(t) = tags {
                if !t.is_empty() {
                    desc.push_str(&format!(". Tags: {}", t));
                }
            }
            // Include first ~500 chars of content for context
            if let Some(c) = content {
                let content_preview: String = c.chars().take(500).collect();
                desc.push_str(&format!(". Content: {}", content_preview));
            }
            desc.push_str(&format!(". File: {}", file_path));

            events.push((PATTERN_ID_OFFSET + rowid, desc));
        }
    }

    let pattern_count = events.len() - session_count - code_count;

    // 4. Git commits (the "why" behind code changes)
    const COMMIT_ID_OFFSET: i64 = 3_000_000_000;
    let mut stmt = conn.prepare(
        "SELECT rowid, sha, message FROM commits
         WHERE message IS NOT NULL AND length(message) > 30
         ORDER BY rowid",
    )?;

    let mut rows = stmt.query([])?;
    while let Some(row) = rows.next()? {
        let rowid: i64 = row.get(0)?;
        let sha: String = row.get(1)?;
        let message: String = row.get(2)?;

        // Use the full commit message for semantic search
        let desc = format!("Commit {}: {}", &sha[..7.min(sha.len())], message);
        events.push((COMMIT_ID_OFFSET + rowid, desc));
    }

    let commit_count = events.len() - session_count - code_count - pattern_count;

    // 5. Epistemic beliefs (project decisions with confidence)
    const BELIEF_ID_OFFSET: i64 = 4_000_000_000;
    let has_beliefs: bool = conn
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='beliefs'",
            [],
            |row| row.get::<_, i64>(0),
        )
        .map(|c| c > 0)
        .unwrap_or(false);

    if has_beliefs {
        let mut stmt = conn.prepare(
            "SELECT rowid, id, statement, persona, facets, confidence, entrenchment
             FROM beliefs
             WHERE status = 'active'",
        )?;

        let mut rows = stmt.query([])?;
        while let Some(row) = rows.next()? {
            let rowid: i64 = row.get(0)?;
            let id: String = row.get(1)?;
            let statement: String = row.get(2)?;
            let persona: String = row.get(3)?;
            let facets: Option<String> = row.get(4)?;
            let confidence: f64 = row.get(5)?;
            let entrenchment: String = row.get(6)?;

            // Create embeddable text for the belief
            // Rich context: statement, persona, facets, confidence for semantic matching
            let mut desc = format!("Belief: {} - {}", id, statement);
            desc.push_str(&format!(". Persona: {}", persona));
            if let Some(f) = facets {
                if !f.is_empty() {
                    desc.push_str(&format!(". Facets: {}", f));
                }
            }
            desc.push_str(&format!(
                ". Confidence: {:.2}, Entrenchment: {}",
                confidence, entrenchment
            ));

            events.push((BELIEF_ID_OFFSET + rowid, desc));
        }
    }

    let belief_count = events.len() - session_count - code_count - pattern_count - commit_count;

    // 6. Forge events (issues + PRs) — embed title+body for semantic search
    // Query eventlog directly (forge_issues/forge_prs views have broken event_seq)
    const FORGE_ID_OFFSET: i64 = 5_000_000_000;
    {
        let mut stmt = conn.prepare(
            "SELECT seq, event_type, source_id,
                    json_extract(data, '$.title') as title,
                    json_extract(data, '$.body') as body
             FROM eventlog
             WHERE event_type IN ('forge.issue', 'forge.pr')
               AND title IS NOT NULL",
        )?;

        let mut rows = stmt.query([])?;
        while let Some(row) = rows.next()? {
            let seq: i64 = row.get(0)?;
            let event_type: String = row.get(1)?;
            let number: String = row.get(2)?;
            let title: String = row.get(3)?;
            let body: Option<String> = row.get(4)?;

            let kind = if event_type == "forge.pr" { "PR" } else { "Issue" };
            let desc = match body {
                Some(b) if !b.is_empty() => format!("{} #{}: {}\n{}", kind, number, title, b),
                _ => format!("{} #{}: {}", kind, number, title),
            };

            events.push((FORGE_ID_OFFSET + seq, desc));
        }
    }

    let forge_count =
        events.len() - session_count - code_count - pattern_count - commit_count - belief_count;

    println!(
        "   Indexed {} session events + {} code facts + {} patterns + {} commits + {} beliefs + {} forge",
        session_count, code_count, pattern_count, commit_count, belief_count, forge_count
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
