use anyhow::Result;

use patina::child::engine::{check_capabilities, ChildManifest};
use patina::paths;

/// List installed command children by scanning the command children directory.
pub fn execute_list() -> Result<()> {
    let command_children_dir = paths::child::command_children_dir();

    if !command_children_dir.exists() {
        println!(
            "No command-children directory found at {}",
            command_children_dir.display()
        );
        println!("Install children by copying .wasm + .toml files to that directory.");
        return Ok(());
    }

    let mut entries: Vec<_> = std::fs::read_dir(&command_children_dir)?
        .filter_map(Result::ok)
        .filter(|e| e.path().extension().is_some_and(|ext| ext == "wasm"))
        .collect();
    entries.sort_by_key(|e| e.file_name());

    if entries.is_empty() {
        println!(
            "No command-children installed in {}",
            command_children_dir.display()
        );
        println!("Install children by copying .wasm + .toml files to that directory.");
        return Ok(());
    }

    println!(
        "Installed command-children ({}):\n",
        command_children_dir.display()
    );
    println!(
        "  {:<20} {:<10} {:<15} {:<12} {:<10} {:<8} STATUS",
        "NAME", "VERSION", "KIND", "ROLE", "INGRESS", "OPS"
    );
    println!("  {}", "-".repeat(120));

    for entry in &entries {
        let wasm_path = entry.path();
        let stem = wasm_path.file_stem().unwrap_or_default().to_string_lossy();
        let toml_path = command_children_dir.join(format!("{}.toml", stem));

        if toml_path.exists() {
            match ChildManifest::from_path(&toml_path) {
                Ok(manifest) => {
                    let status = match check_capabilities(&manifest) {
                        Ok(()) => "ready",
                        Err(_) => "denied",
                    };
                    let role_str = manifest
                        .role
                        .as_ref()
                        .map(|r| r.to_string())
                        .unwrap_or_else(|| "-".into());
                    let operations = if manifest.contract_allow_operations.is_empty() {
                        "-".to_string()
                    } else {
                        manifest.contract_allow_operations.join(",")
                    };
                    println!(
                        "  {:<20} {:<10} {:<15} {:<12} {:<10} {:<8} {}",
                        manifest.name,
                        manifest.version,
                        manifest.world,
                        role_str,
                        manifest.ingress_mode,
                        operations,
                        status
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
