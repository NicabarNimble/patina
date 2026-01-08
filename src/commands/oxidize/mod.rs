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

    let db_path = ".patina/data/patina.db";
    let output_dir = format!(".patina/data/embeddings/{}/projections", model_name);
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

    println!(
        "   Indexed {} session events + {} code facts + {} patterns + {} commits",
        session_count, code_count, pattern_count, commit_count
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
