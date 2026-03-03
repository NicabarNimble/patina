//! Grammar benchmark — measure WASM pipeline plugin throughput.
//!
//! Profiles plugin dispatch: engine creation, WASM handle, JSON deserialization.
//! Reports per-file latency and throughput for optimization.

use std::path::{Path, PathBuf};
use std::time::Instant;

use anyhow::{bail, Context, Result};
use ignore::WalkBuilder;

use crate::commands::scrape::code::extracted_data::ExtractedData;

use patina::plugin::PipelineEngine;

/// Options for grammar benchmark
pub struct GrammarBenchOptions {
    /// Maximum number of files to benchmark
    pub files: Option<usize>,
}

/// Run the grammar benchmark.
pub fn run(options: GrammarBenchOptions) -> Result<()> {
    let work_dir = std::env::current_dir()?;
    let rust_files = collect_rust_files(&work_dir, options.files)?;

    if rust_files.is_empty() {
        bail!("No Rust source files found in {}", work_dir.display());
    }

    println!(
        "Grammar plugin benchmark ({} Rust files):\n",
        rust_files.len()
    );

    // Read all file contents upfront (exclude I/O from timing)
    let file_contents: Vec<(String, Vec<u8>)> = rust_files
        .iter()
        .filter_map(|path| {
            let relative = path
                .strip_prefix(&work_dir)
                .map(|p| format!("./{}", p.display()))
                .unwrap_or_else(|_| path.display().to_string());
            std::fs::read(path).ok().map(|content| (relative, content))
        })
        .collect();

    let total_bytes: usize = file_contents.iter().map(|(_, c)| c.len()).sum();

    // === Discover plugin ===
    let pipeline_dir = patina::paths::plugin::pipeline_dir();

    let discovery_start = Instant::now();
    let engine = PipelineEngine::new().context("Failed to create PipelineEngine")?;
    let discovered = engine.discover(&pipeline_dir);
    let discovery_elapsed = discovery_start.elapsed();

    let rust_plugin = discovered.get("rs");
    if rust_plugin.is_none() {
        bail!(
            "grammar-rust plugin not installed at ~/.patina/pipeline/grammar-rust/.\n\
             Run `patina setup grammars` first."
        );
    }
    let (component, manifest) = rust_plugin.unwrap();

    // === Measure engine creation cost ===
    let engine_start = Instant::now();
    let mut engine_ok = 0;
    for _ in 0..10 {
        if PipelineEngine::new().is_ok() {
            engine_ok += 1;
        }
    }
    let engine_elapsed = engine_start.elapsed();
    let engine_cost_ms = engine_elapsed.as_secs_f64() * 1000.0 / engine_ok as f64;

    // === Dispatch: full pipeline (engine per call, matches extract_v2 pattern) ===
    let mut per_call_ok = 0usize;
    let mut per_call_errors = 0usize;
    let per_call_start = Instant::now();
    for (path, content) in &file_contents {
        let request = build_parse_envelope(content, "rs", path);
        let call_engine = match PipelineEngine::new() {
            Ok(e) => e,
            Err(_) => {
                per_call_errors += 1;
                continue;
            }
        };
        match call_engine.handle(component, manifest, &request) {
            Ok(response) => {
                if serde_json::from_str::<ExtractedData>(&response).is_ok() {
                    per_call_ok += 1;
                }
            }
            Err(_) => per_call_errors += 1,
        }
    }
    let per_call_elapsed = per_call_start.elapsed();

    // === Dispatch: shared engine (reuse single engine for all calls) ===
    let shared_engine = PipelineEngine::new()?;
    let mut shared_ok = 0usize;
    let shared_start = Instant::now();
    for (path, content) in &file_contents {
        let request = build_parse_envelope(content, "rs", path);
        if let Ok(response) = shared_engine.handle(component, manifest, &request) {
            if serde_json::from_str::<ExtractedData>(&response).is_ok() {
                shared_ok += 1;
            }
        }
    }
    let shared_elapsed = shared_start.elapsed();

    // === Report ===
    let n = file_contents.len() as f64;
    let per_call_ms = per_call_elapsed.as_secs_f64() * 1000.0;
    let shared_ms = shared_elapsed.as_secs_f64() * 1000.0;

    println!(
        "  Discovery:       {:.0}ms (load manifests + WASM components)",
        discovery_elapsed.as_secs_f64() * 1000.0
    );
    println!(
        "  Engine creation: {:.2}ms/engine (amortized over 10)",
        engine_cost_ms
    );
    println!();
    println!(
        "  Per-call engine: {:.1}s  ({:.1}ms/file, {}/{} ok)",
        per_call_elapsed.as_secs_f64(),
        per_call_ms / n,
        per_call_ok,
        file_contents.len()
    );
    println!(
        "  Shared engine:   {:.1}s  ({:.1}ms/file, {}/{} ok)",
        shared_elapsed.as_secs_f64(),
        shared_ms / n,
        shared_ok,
        file_contents.len()
    );

    let speedup = if shared_ms > 0.0 {
        per_call_ms / shared_ms
    } else {
        0.0
    };
    println!("  Reuse speedup:   {:.1}x", speedup);

    println!();
    let throughput = n / per_call_elapsed.as_secs_f64();
    let kb = total_bytes as f64 / 1024.0;
    println!(
        "  Throughput:      {:.0} files/s ({:.0} KB total input)",
        throughput, kb
    );

    if per_call_ms / n < 10.0 {
        println!("  FAST: <10ms/file — excellent for batch scrape");
    } else if per_call_ms / n < 50.0 {
        println!("  GOOD: <50ms/file — fine for batch scrape");
    } else {
        println!("  SLOW: >50ms/file — consider engine reuse optimization");
    }

    if per_call_errors > 0 {
        println!("  {} files had errors", per_call_errors);
    }

    Ok(())
}

/// Collect Rust source files from a directory.
fn collect_rust_files(dir: &Path, limit: Option<usize>) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    let max = limit.unwrap_or(usize::MAX);

    for entry in WalkBuilder::new(dir).hidden(false).git_ignore(true).build() {
        let entry = entry?;
        let path = entry.path();
        if path.is_file() && path.extension().is_some_and(|e| e == "rs") {
            files.push(path.to_path_buf());
            if files.len() >= max {
                break;
            }
        }
    }

    Ok(files)
}

/// Build a parse request envelope (same format as extract_v2.rs).
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
