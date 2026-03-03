// ============================================================================
// REFACTORED EXTRACTION WITH EMBEDDED SQLITE
// ============================================================================
//! New extraction pipeline using type-safe database operations.
//!
//! This replaces the unsafe SQL string concatenation with:
//! - Direct SQLite library integration
//! - Prepared statements and transactions
//! - Type-preserving data structures
//! - Batch operations for performance

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use anyhow::Result;
use ignore::WalkBuilder;

use super::database::Database;
use super::extracted_data::{ExtractedData, ExtractedPayload};
use super::languages::Language;
use super::types::FilePath;

use patina::paths;
use patina::plugin::{PipelineEngine, PluginManifest};

/// Process all source files and extract metadata using safe database operations
pub fn extract_code_metadata_v2(db_path: &str, work_dir: &Path, force: bool) -> Result<usize> {
    println!("🧠 Extracting code metadata with embedded SQLite...");

    // Open database connection
    let mut db = Database::open(db_path)?;
    db.init_schema()?;

    // Ensure forge materialized views exist for Issue/PullRequest routing
    crate::commands::scrape::forge::create_materialized_views(db.connection())?;

    // Open events.db for forge event writes (runtime events go to events.db)
    let events_conn = patina::eventlog::open_events_db()?;

    // Find all supported language files
    let mut all_files: Vec<(PathBuf, Language)> = Vec::new();

    for entry in WalkBuilder::new(work_dir)
        .hidden(false)
        .git_ignore(true)
        .build()
    {
        let entry = entry?;
        let path = entry.path();

        if path.is_file() {
            let language = Language::from_path(path);
            if !matches!(language, Language::Unknown) {
                all_files.push((path.to_path_buf(), language));
            }
        }
    }

    println!("  Found {} source files", all_files.len());

    // Discover pipeline plugins from ~/.patina/pipeline/
    let pipeline_plugins = discover_pipeline_plugins();

    // Scan staging tree for forge data (.forge-issue, .forge-pr files)
    // These are written by `patina scrape forge` and processed by grammar-forge plugin
    let staging_dir = paths::project::data_dir(work_dir).join("forge");
    let mut staged_files: Vec<PathBuf> = Vec::new();
    if staging_dir.is_dir() {
        for entry in WalkBuilder::new(&staging_dir)
            .hidden(false)
            .git_ignore(false)
            .build()
        {
            let entry = match entry {
                Ok(e) => e,
                Err(_) => continue,
            };
            let path = entry.path();
            if path.is_file() {
                let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
                if pipeline_plugins.contains_key(ext) {
                    staged_files.push(path.to_path_buf());
                }
            }
        }
        if !staged_files.is_empty() {
            println!("  Found {} staged forge files", staged_files.len());
        }
    }

    if all_files.is_empty() && staged_files.is_empty() {
        println!("  No source files found. Is this a code repository?");
        return Ok(0);
    }

    // Collect all extracted data in memory first
    let mut all_symbols = Vec::new();
    let mut all_functions = Vec::new();
    let mut all_types = Vec::new();
    let mut all_imports = Vec::new();
    let mut all_call_edges = Vec::new();
    let mut all_constants = Vec::new();
    let mut all_members = Vec::new();

    let mut files_with_errors = 0;
    let mut _files_processed = 0;
    let mut files_skipped_mtime = 0;
    let mut forge_issues_inserted = 0;
    let mut forge_prs_inserted = 0;
    let mut forge_skipped = 0;
    let mut walked_paths: HashSet<String> = HashSet::new();

    // Process each file and collect data
    for (file_path, language) in all_files {
        let relative_path = if let Ok(stripped) = file_path.strip_prefix(work_dir) {
            format!("./{}", stripped.to_string_lossy())
        } else {
            file_path.to_string_lossy().to_string()
        };

        // Track all walked paths for stale entry pruning
        walked_paths.insert(relative_path.clone());

        // Get file metadata for index state
        let mtime = std::fs::metadata(&file_path)
            .and_then(|m| m.modified())
            .unwrap_or(SystemTime::now())
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;

        // Check if file changed since last scrape (mtime skip optimization)
        if !force {
            if let Some((stored_mtime, stored_size)) = db.get_index_state(&relative_path)? {
                let file_size = std::fs::metadata(&file_path)
                    .map(|m| m.len() as i64)
                    .unwrap_or(-1);
                if mtime == stored_mtime && file_size == stored_size {
                    files_skipped_mtime += 1;
                    continue;
                }
            }
        }

        // Read file content (only after mtime check — avoid I/O for unchanged files)
        let content = match std::fs::read(&file_path) {
            Ok(content) => content,
            Err(e) => {
                eprintln!("  ⚠️  Failed to read {}: {}", relative_path, e);
                files_with_errors += 1;
                continue;
            }
        };

        let size = content.len() as i64;
        let line_count = content.iter().filter(|&&b| b == b'\n').count() as i64;

        // Update index state
        db.update_index_state(&relative_path, mtime, size, None, Some(line_count))?;

        // Process file: plugin-first dispatch with built-in fallback
        let ext = file_path.extension().and_then(|e| e.to_str()).unwrap_or("");
        match process_file_with_plugins(&relative_path, &content, language, ext, &pipeline_plugins)
        {
            Ok(payload) => {
                // #[non_exhaustive] requires wildcard arm for future variants
                #[allow(unreachable_patterns)]
                match payload {
                    ExtractedPayload::Code(extracted) => {
                        all_symbols.extend(extracted.symbols);
                        all_functions.extend(extracted.functions);
                        all_types.extend(extracted.types);
                        all_imports.extend(extracted.imports);
                        all_call_edges.extend(extracted.call_edges);
                        all_constants.extend(extracted.constants);
                        all_members.extend(extracted.members);
                        _files_processed += 1;
                    }
                    ExtractedPayload::Issue(issue) => {
                        // EC3: Validate against schema before DB insert
                        if let Ok(json) = serde_json::to_value(&issue) {
                            if let Err(e) =
                                crate::commands::schema::validate_fact("forge", "issue", &json)
                            {
                                eprintln!("  [pipeline] {} rejected: {}", relative_path, e);
                                files_with_errors += 1;
                                continue;
                            }
                        }
                        let conn = db.connection();
                        match crate::commands::scrape::forge::insert_issues(
                            conn,
                            &events_conn,
                            &[issue],
                        ) {
                            Ok(stats) => {
                                forge_issues_inserted += stats.inserted;
                                forge_skipped += stats.skipped;
                                _files_processed += 1;
                            }
                            Err(e) => {
                                eprintln!(
                                    "  [pipeline] forge issue insert failed for {}: {}",
                                    relative_path, e
                                );
                                files_with_errors += 1;
                            }
                        }
                    }
                    ExtractedPayload::PullRequest(pr) => {
                        // EC3: Validate against schema before DB insert
                        if let Ok(json) = serde_json::to_value(&pr) {
                            if let Err(e) = crate::commands::schema::validate_fact(
                                "forge",
                                "pull-request",
                                &json,
                            ) {
                                eprintln!("  [pipeline] {} rejected: {}", relative_path, e);
                                files_with_errors += 1;
                                continue;
                            }
                        }
                        let conn = db.connection();
                        match crate::commands::scrape::forge::insert_prs(conn, &events_conn, &[pr])
                        {
                            Ok(stats) => {
                                forge_prs_inserted += stats.inserted;
                                forge_skipped += stats.skipped;
                                _files_processed += 1;
                            }
                            Err(e) => {
                                eprintln!(
                                    "  [pipeline] forge PR insert failed for {}: {}",
                                    relative_path, e
                                );
                                files_with_errors += 1;
                            }
                        }
                    }
                    _ => {
                        // #[non_exhaustive] catch-all for future variants
                        eprintln!(
                            "  [pipeline] unknown payload kind from {} — skipping",
                            relative_path
                        );
                    }
                }
            }
            Err(e) => {
                eprintln!("  ⚠️  Processing error in {}: {}", relative_path, e);
                db.mark_skipped(&relative_path, &e.to_string())?;
                files_with_errors += 1;
            }
        }
    }

    // Process staged forge files through pipeline plugins
    for file_path in staged_files {
        let display_path = file_path.to_string_lossy().to_string();
        let content = match std::fs::read(&file_path) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("  ⚠️  Failed to read staged file {}: {}", display_path, e);
                files_with_errors += 1;
                continue;
            }
        };

        let ext = file_path.extension().and_then(|e| e.to_str()).unwrap_or("");

        // Staged files dispatch directly to plugin by extension (no Language detection)
        if let Some(plugin) = pipeline_plugins.get(ext) {
            let request = build_parse_envelope(&content, ext, &display_path);
            match plugin
                .engine
                .handle(&plugin.component, &plugin.manifest, &request)
            {
                Ok(response) => {
                    // Try ExtractedPayload (has "kind" field) — expected for forge plugins
                    if let Ok(payload) = serde_json::from_str::<ExtractedPayload>(&response) {
                        #[allow(unreachable_patterns)]
                        match payload {
                            ExtractedPayload::Issue(issue) => {
                                // EC3: Validate against schema before DB insert
                                if let Ok(json) = serde_json::to_value(&issue) {
                                    if let Err(e) = crate::commands::schema::validate_fact(
                                        "forge", "issue", &json,
                                    ) {
                                        eprintln!("  [pipeline] {} rejected: {}", display_path, e);
                                        files_with_errors += 1;
                                        continue;
                                    }
                                }
                                let conn = db.connection();
                                match crate::commands::scrape::forge::insert_issues(
                                    conn,
                                    &events_conn,
                                    &[issue],
                                ) {
                                    Ok(stats) => {
                                        forge_issues_inserted += stats.inserted;
                                        _files_processed += 1;
                                    }
                                    Err(e) => {
                                        eprintln!(
                                            "  [pipeline] forge issue insert failed for {}: {}",
                                            display_path, e
                                        );
                                        files_with_errors += 1;
                                    }
                                }
                            }
                            ExtractedPayload::PullRequest(pr) => {
                                // EC3: Validate against schema before DB insert
                                if let Ok(json) = serde_json::to_value(&pr) {
                                    if let Err(e) = crate::commands::schema::validate_fact(
                                        "forge",
                                        "pull-request",
                                        &json,
                                    ) {
                                        eprintln!("  [pipeline] {} rejected: {}", display_path, e);
                                        files_with_errors += 1;
                                        continue;
                                    }
                                }
                                let conn = db.connection();
                                match crate::commands::scrape::forge::insert_prs(
                                    conn,
                                    &events_conn,
                                    &[pr],
                                ) {
                                    Ok(stats) => {
                                        forge_prs_inserted += stats.inserted;
                                        _files_processed += 1;
                                    }
                                    Err(e) => {
                                        eprintln!(
                                            "  [pipeline] forge PR insert failed for {}: {}",
                                            display_path, e
                                        );
                                        files_with_errors += 1;
                                    }
                                }
                            }
                            ExtractedPayload::Code(extracted) => {
                                // Unlikely for forge files, but handle gracefully
                                all_symbols.extend(extracted.symbols);
                                all_functions.extend(extracted.functions);
                                all_types.extend(extracted.types);
                                all_imports.extend(extracted.imports);
                                all_call_edges.extend(extracted.call_edges);
                                all_constants.extend(extracted.constants);
                                all_members.extend(extracted.members);
                                _files_processed += 1;
                            }
                            _ => {
                                eprintln!(
                                    "  [pipeline] unknown payload kind from {} — skipping",
                                    display_path
                                );
                            }
                        }
                    } else {
                        eprintln!(
                            "  [pipeline:{}] invalid response for staged file {}: not ExtractedPayload",
                            plugin.manifest.name, display_path
                        );
                        files_with_errors += 1;
                    }
                }
                Err(e) => {
                    eprintln!(
                        "  [pipeline:{}] handle failed for staged file {}: {}",
                        plugin.manifest.name, display_path, e
                    );
                    files_with_errors += 1;
                }
            }
        }
    }

    if files_skipped_mtime > 0 {
        println!("  Skipped {} unchanged files (mtime)", files_skipped_mtime);
    }

    // Prune stale index_state rows for deleted/renamed files
    let pruned = db.prune_stale_paths(&walked_paths)?;
    if pruned > 0 {
        println!("  Pruned {} stale entries", pruned);
    }

    // Bulk insert all collected data
    println!("  💾 Writing to database using bulk operations...");

    let symbols_count = db.insert_symbols(&all_symbols)?;
    let functions_count = db.insert_functions(&all_functions)?;
    let types_count = db.insert_types(&all_types)?;
    let imports_count = db.insert_imports(&all_imports)?;
    let edges_count = db.insert_call_edges(&all_call_edges)?;
    let constants_count = db.insert_constants(&all_constants)?;
    let members_count = db.insert_members(&all_members)?;

    println!(
        "  ✅ Inserted: {} symbols, {} functions, {} types, {} imports, {} call edges, {} constants, {} members",
        symbols_count, functions_count, types_count, imports_count, edges_count, constants_count, members_count
    );

    if forge_issues_inserted > 0 || forge_prs_inserted > 0 {
        if forge_skipped > 0 {
            println!(
                "  📊 Forge via pipeline: {} issues, {} PRs ({} unchanged)",
                forge_issues_inserted, forge_prs_inserted, forge_skipped
            );
        } else {
            println!(
                "  📊 Forge via pipeline: {} issues, {} PRs",
                forge_issues_inserted, forge_prs_inserted
            );
        }
    }

    if files_with_errors > 0 {
        println!(
            "  ⚠️  {} files had parsing errors and were skipped",
            files_with_errors
        );
    }

    Ok(symbols_count + functions_count + types_count + imports_count)
}

/// Loaded pipeline plugin — engine + component + manifest, ready to dispatch.
struct LoadedPipelinePlugin {
    engine: PipelineEngine,
    component: wasmtime::component::Component,
    manifest: PluginManifest,
}

/// Discover pipeline plugins from ~/.patina/pipeline/.
/// Returns a map of file extension → loaded plugin.
fn discover_pipeline_plugins() -> HashMap<String, LoadedPipelinePlugin> {
    let pipeline_dir = dirs::home_dir()
        .map(|h| h.join(".patina").join("pipeline"))
        .unwrap_or_default();

    if !pipeline_dir.is_dir() {
        return HashMap::new();
    }

    let engine = match PipelineEngine::new() {
        Ok(e) => e,
        Err(e) => {
            eprintln!("[pipeline] failed to create engine: {}", e);
            return HashMap::new();
        }
    };

    let discovered = engine.discover(&pipeline_dir);
    if !discovered.is_empty() {
        println!(
            "  Pipeline plugins: {} language(s) claimed",
            discovered.len()
        );
    }

    // Wrap into LoadedPipelinePlugin — we need the engine for each dispatch
    // Since PipelineEngine contains a Linker (not Clone), create one per plugin.
    // For efficiency, share a single engine across all plugins.
    let mut result = HashMap::new();
    for (lang, (component, manifest)) in discovered {
        // Re-create engine per entry since we can't clone it.
        // The wasmtime Engine singleton is shared (OnceLock), so this is cheap.
        let engine = match PipelineEngine::new() {
            Ok(e) => e,
            Err(_) => continue,
        };
        result.insert(
            lang,
            LoadedPipelinePlugin {
                engine,
                component,
                manifest,
            },
        );
    }
    result
}

/// Build a parse request envelope. Source code is sent as UTF-8 string.
fn build_parse_envelope(content: &[u8], language: &str, path: &str) -> String {
    let source = String::from_utf8_lossy(content);
    serde_json::json!({
        "op": "parse",
        "version": "1",
        "payload": {
            "source": source,
            "language": language,
            "path": path
        }
    })
    .to_string()
}

/// Try pipeline plugin first, fall back to built-in processor.
///
/// Deserialization order for plugin responses:
/// 1. Try `ExtractedPayload` (JSON has `kind` field)
/// 2. Try `ExtractedData` → wrap as `ExtractedPayload::Code` (backward compat)
/// 3. Fall through to built-in processor
fn process_file_with_plugins(
    file_path: &str,
    content: &[u8],
    language: Language,
    ext: &str,
    pipeline_plugins: &HashMap<String, LoadedPipelinePlugin>,
) -> Result<ExtractedPayload> {
    // Plugin-first dispatch: check if a pipeline plugin claims this extension
    if let Some(plugin) = pipeline_plugins.get(ext) {
        let request = build_parse_envelope(content, ext, file_path);
        match plugin
            .engine
            .handle(&plugin.component, &plugin.manifest, &request)
        {
            Ok(response) => {
                // 1. Try ExtractedPayload (has "kind" field)
                if let Ok(payload) = serde_json::from_str::<ExtractedPayload>(&response) {
                    return Ok(payload);
                }
                // 2. Try ExtractedData (no "kind" field — backward compat)
                match serde_json::from_str::<ExtractedData>(&response) {
                    Ok(extracted) => return Ok(ExtractedPayload::Code(extracted)),
                    Err(e) => {
                        eprintln!(
                            "  [pipeline:{}] parse response failed for {}: {}",
                            plugin.manifest.name, file_path, e
                        );
                        // Fall through to built-in
                    }
                }
            }
            Err(e) => {
                eprintln!(
                    "  [pipeline:{}] handle failed for {}: {}",
                    plugin.manifest.name, file_path, e
                );
                // Fall through to built-in
            }
        }
    }

    // Built-in Rust fallback — other languages require pipeline plugins
    process_file_by_language(file_path, content, language).map(ExtractedPayload::Code)
}

/// Compiled-in Rust fallback. All other languages dispatch via pipeline plugins.
/// Per [[graceful-extraction]], patina must always parse Rust even with zero plugins.
fn process_file_by_language(
    file_path: &str,
    content: &[u8],
    language: Language,
) -> Result<ExtractedData> {
    match language {
        Language::Rust => {
            use super::languages::rust::RustProcessor;
            RustProcessor::process_file(FilePath::from(file_path), content)
        }
        _ => Err(anyhow::anyhow!(
            "No pipeline plugin for {:?} — install with `patina plugin install`",
            language
        )),
    }
}
