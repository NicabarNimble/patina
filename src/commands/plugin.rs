use anyhow::Result;

use patina::paths;
use patina::plugin::PluginEngine;

/// List installed plugins by scanning ~/.patina/plugins/ for .wasm + .toml pairs.
pub fn execute_list() -> Result<()> {
    let plugins_dir = paths::plugin::plugins_dir();

    if !plugins_dir.exists() {
        println!("No plugins directory found at {}", plugins_dir.display());
        println!("Install plugins by copying .wasm + .toml files to that directory.");
        return Ok(());
    }

    // Scan for .wasm files, look for matching .toml manifests
    let mut entries: Vec<_> = std::fs::read_dir(&plugins_dir)?
        .filter_map(Result::ok)
        .filter(|e| e.path().extension().is_some_and(|ext| ext == "wasm"))
        .collect();
    entries.sort_by_key(|e| e.file_name());

    if entries.is_empty() {
        println!("No plugins installed in {}", plugins_dir.display());
        println!("Install plugins by copying .wasm + .toml files to that directory.");
        return Ok(());
    }

    println!("Installed plugins ({}):\n", plugins_dir.display());
    println!(
        "  {:<20} {:<10} {:<15} {:<12} STATUS",
        "NAME", "VERSION", "WORLD", "ROLE"
    );
    println!("  {}", "-".repeat(72));

    for entry in &entries {
        let wasm_path = entry.path();
        let stem = wasm_path.file_stem().unwrap_or_default().to_string_lossy();
        let toml_path = plugins_dir.join(format!("{}.toml", stem));

        if toml_path.exists() {
            match PluginEngine::load_manifest(&toml_path) {
                Ok(manifest) => {
                    let status = match PluginEngine::check_capabilities(&manifest) {
                        Ok(()) => "ready",
                        Err(_) => "denied",
                    };
                    let role_str = manifest
                        .role
                        .as_ref()
                        .map(|r| r.to_string())
                        .unwrap_or_else(|| "-".into());
                    println!(
                        "  {:<20} {:<10} {:<15} {:<12} {}",
                        manifest.name, manifest.version, manifest.world, role_str, status
                    );
                }
                Err(e) => {
                    println!(
                        "  {:<20} {:<10} {:<15} {:<12} error: {}",
                        stem, "?", "?", "-", e
                    );
                }
            }
        } else {
            println!(
                "  {:<20} {:<10} {:<15} {:<12} no manifest",
                stem, "?", "?", "-"
            );
        }
    }

    println!();
    Ok(())
}
