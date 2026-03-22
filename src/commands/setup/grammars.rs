//! Grammar plugin installer — copies pre-built WASM plugins to ~/.patina/pipeline/.
//!
//! Source: grammars/<lang>/ directories adjacent to the patina executable or repo root.
//! Target: ~/.patina/pipeline/grammar-<name>/plugin.{wasm,toml}

use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};

use super::GrammarOptions;

/// Default grammar names (embedded, no TOML parse needed at runtime).
const DEFAULT_GRAMMARS: &[&str] = &[
    "rust",
    "go",
    "python",
    "javascript",
    "typescript",
    "c",
    "cpp",
    "solidity",
    "cairo",
];

/// Install grammar plugins from local build artifacts.
pub fn install(options: GrammarOptions) -> Result<()> {
    let grammars: Vec<&str> = if let Some(ref only) = options.only {
        // Validate requested grammars exist in default set
        for name in only {
            if !DEFAULT_GRAMMARS.contains(&name.as_str()) {
                bail!(
                    "Unknown grammar '{}'. Available: {}",
                    name,
                    DEFAULT_GRAMMARS.join(", ")
                );
            }
        }
        only.iter().map(|s| s.as_str()).collect()
    } else {
        DEFAULT_GRAMMARS.to_vec()
    };

    let pipeline_dir = pipeline_dir()?;
    let source_root = find_source_root()?;

    if options.list {
        print_list(&grammars, &pipeline_dir, &source_root);
        return Ok(());
    }

    println!(
        "Installing grammar plugins to {}...",
        pipeline_dir.display()
    );

    std::fs::create_dir_all(&pipeline_dir)
        .with_context(|| format!("Failed to create {}", pipeline_dir.display()))?;

    let mut installed = 0;
    let mut skipped = 0;
    let mut failed = 0;

    for name in &grammars {
        let target_dir = pipeline_dir.join(format!("grammar-{}", name));
        let target_wasm = target_dir.join("plugin.wasm");
        let target_toml = target_dir.join("child.toml");

        // Check if already installed
        if target_wasm.exists() && target_toml.exists() && !options.force {
            println!("  grammar-{:<13} already installed", name);
            skipped += 1;
            continue;
        }

        // Find source artifacts
        let grammar_dir = source_root.join(format!("grammars/{}", name));
        let source_toml = grammar_dir.join("child.toml");
        let wasm_name = format!("grammar_{}.wasm", name);
        let source_wasm = grammar_dir
            .join("target")
            .join("wasm32-wasip2")
            .join("release")
            .join(&wasm_name);

        if !source_toml.exists() || !source_wasm.exists() {
            eprintln!(
                "  grammar-{:<13} MISSING (no build artifacts at {})",
                name,
                grammar_dir.display()
            );
            failed += 1;
            continue;
        }

        // Create target directory and copy files
        std::fs::create_dir_all(&target_dir)
            .with_context(|| format!("Failed to create {}", target_dir.display()))?;

        std::fs::copy(&source_wasm, &target_wasm).with_context(|| {
            format!(
                "Failed to copy {} -> {}",
                source_wasm.display(),
                target_wasm.display()
            )
        })?;

        std::fs::copy(&source_toml, &target_toml).with_context(|| {
            format!(
                "Failed to copy {} -> {}",
                source_toml.display(),
                target_toml.display()
            )
        })?;

        let size = std::fs::metadata(&target_wasm)
            .map(|m| format_size(m.len()))
            .unwrap_or_else(|_| "?".to_string());

        println!("  grammar-{:<13} installed ({})", name, size);
        installed += 1;
    }

    println!();
    let total = installed + skipped;
    println!(
        "  {}/{} grammars ready{}",
        total,
        grammars.len(),
        if failed > 0 {
            format!(" ({} missing)", failed)
        } else {
            String::new()
        }
    );

    Ok(())
}

/// Print what would be installed without doing it.
fn print_list(grammars: &[&str], pipeline_dir: &Path, source_root: &Path) {
    println!("Default grammar plugins:\n");
    println!("  {:<20} {:<12} {:<10} SOURCE", "GRAMMAR", "STATUS", "SIZE");
    println!("  {}", "-".repeat(65));

    for name in grammars {
        let target_dir = pipeline_dir.join(format!("grammar-{}", name));
        let installed =
            target_dir.join("plugin.wasm").exists() && target_dir.join("child.toml").exists();

        let grammar_dir = source_root.join(format!("grammars/{}", name));
        let wasm_name = format!("grammar_{}.wasm", name);
        let source_wasm = grammar_dir
            .join("target")
            .join("wasm32-wasip2")
            .join("release")
            .join(&wasm_name);

        let (status, size, source) = if installed {
            let sz = target_dir
                .join("plugin.wasm")
                .metadata()
                .map(|m| format_size(m.len()))
                .unwrap_or_else(|_| "?".into());
            ("installed", sz, "~/.patina/pipeline/".to_string())
        } else if source_wasm.exists() {
            let sz = source_wasm
                .metadata()
                .map(|m| format_size(m.len()))
                .unwrap_or_else(|_| "?".into());
            ("available", sz, format!("grammars/{}/", name))
        } else {
            ("missing", "-".to_string(), "-".to_string())
        };

        println!(
            "  grammar-{:<10} {:<12} {:<10} {}",
            name, status, size, source
        );
    }

    println!();
    println!("  Target: {}", pipeline_dir.display());
}

/// Find the root directory containing grammar-*/ build artifacts.
/// Searches: parent of the executable, then the current working directory.
fn find_source_root() -> Result<PathBuf> {
    // Try directory of the running executable first
    if let Ok(exe) = std::env::current_exe() {
        if let Some(parent) = exe.parent() {
            // For cargo install, exe is in ~/.cargo/bin — grammar dirs won't be there.
            // For dev builds, exe is in target/release/ or target/debug/.
            // Walk up to find the repo root.
            let candidates = [
                parent.to_path_buf(),                    // same dir as exe
                parent.join("..").join(".."),            // target/release/../../
                parent.join("..").join("..").join(".."), // target/release/../../../
            ];
            for candidate in &candidates {
                let resolved = candidate.canonicalize().unwrap_or(candidate.clone());
                if resolved.join("grammars/rust").join("child.toml").exists() {
                    return Ok(resolved);
                }
            }
        }
    }

    // Fall back to current working directory
    let cwd = std::env::current_dir()?;
    if cwd.join("grammars/rust").join("child.toml").exists() {
        return Ok(cwd);
    }

    // Walk up from cwd
    let mut dir = cwd.as_path();
    while let Some(parent) = dir.parent() {
        if parent.join("grammars/rust").join("child.toml").exists() {
            return Ok(parent.to_path_buf());
        }
        dir = parent;
    }

    bail!(
        "Cannot find grammar build artifacts (grammars/*/child.toml).\n\
         Run this command from the patina repo root, or ensure grammars/\n\
         directories exist alongside the patina binary."
    )
}

/// Get the pipeline directory (~/.patina/pipeline/).
fn pipeline_dir() -> Result<PathBuf> {
    Ok(patina::paths::plugin::pipeline_dir())
}

/// Format bytes as human-readable size.
fn format_size(bytes: u64) -> String {
    if bytes >= 1_048_576 {
        format!("{:.1}MB", bytes as f64 / 1_048_576.0)
    } else {
        format!("{}KB", bytes / 1024)
    }
}
