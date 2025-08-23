use crate::Metal;
use anyhow::Result;

/// Wrapper for metal-specific parsing logic
pub struct MetalParser;

impl MetalParser {
    /// Find all source files of a specific metal in a directory
    pub fn find_files(dir: &std::path::Path, metal: Metal) -> Result<Vec<std::path::PathBuf>> {
        use std::process::Command;

        let pattern = metal.file_pattern();
        let output = Command::new("find")
            .current_dir(dir)
            .args([".", "-name", pattern, "-type", "f"])
            .output()?;

        if !output.status.success() {
            return Ok(Vec::new());
        }

        let files = String::from_utf8_lossy(&output.stdout)
            .lines()
            .filter(|line| !line.is_empty())
            .map(|line| dir.join(line))
            .collect();

        Ok(files)
    }

    /// Detect all metals present in a directory
    pub fn detect_metals(dir: &std::path::Path) -> Result<Vec<Metal>> {
        let mut metals = Vec::new();

        for metal in Metal::all() {
            let files = Self::find_files(dir, metal)?;
            if !files.is_empty() {
                metals.push(metal);
            }
        }

        Ok(metals)
    }
}
