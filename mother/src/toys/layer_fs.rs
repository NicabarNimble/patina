use anyhow::{bail, Context, Result};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct LayerFsHost {
    layer_root: PathBuf,
}

impl LayerFsHost {
    pub fn new(layer_root: impl Into<PathBuf>) -> Self {
        Self {
            layer_root: layer_root.into(),
        }
    }

    pub fn read_file(&self, path: &str) -> Result<String> {
        let resolved = self.resolve(path)?;
        std::fs::read_to_string(&resolved)
            .with_context(|| format!("reading {}", resolved.display()))
    }

    pub fn write_file(&self, path: &str, contents: &str) -> Result<()> {
        let resolved = self.resolve(path)?;
        if let Some(parent) = resolved.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("creating {}", parent.display()))?;
        }
        std::fs::write(&resolved, contents)
            .with_context(|| format!("writing {}", resolved.display()))
    }

    pub fn list_dir(&self, path: &str) -> Result<Vec<String>> {
        let resolved = self.resolve(path)?;
        let mut entries = Vec::new();
        for entry in std::fs::read_dir(&resolved)
            .with_context(|| format!("listing {}", resolved.display()))?
        {
            let entry = entry?;
            let full_path = entry.path();
            let rel = full_path
                .strip_prefix(&resolved)
                .with_context(|| "stripping layer prefix")?
                .to_string_lossy()
                .to_string();
            entries.push(rel);
        }
        entries.sort();
        Ok(entries)
    }

    pub fn delete_file(&self, path: &str) -> Result<()> {
        let resolved = self.resolve(path)?;
        if resolved.exists() {
            std::fs::remove_file(&resolved)
                .with_context(|| format!("deleting {}", resolved.display()))?;
        }
        Ok(())
    }

    pub fn move_path(&self, from: &str, to: &str) -> Result<()> {
        let from_path = self.resolve(from)?;
        let to_path = self.resolve(to)?;
        if let Some(parent) = to_path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("creating {}", parent.display()))?;
        }
        std::fs::rename(&from_path, &to_path)
            .with_context(|| format!("moving {} -> {}", from_path.display(), to_path.display()))
    }

    pub fn exists(&self, path: &str) -> Result<bool> {
        let resolved = self.resolve(path)?;
        Ok(resolved.exists())
    }

    fn resolve(&self, rel: &str) -> Result<PathBuf> {
        let candidate = Path::new(rel);
        if candidate.is_absolute() {
            bail!("absolute paths are not allowed: {}", rel);
        }
        normalize_no_escape(candidate, &self.layer_root)
    }
}

fn normalize_no_escape(path: &Path, root: &Path) -> Result<PathBuf> {
    use std::path::Component;

    let mut out = root.to_path_buf();
    for component in path.components() {
        match component {
            Component::Prefix(_) | Component::RootDir => {
                bail!("absolute paths are not allowed");
            }
            Component::CurDir => {}
            Component::ParentDir => {
                bail!("path escapes layer root");
            }
            Component::Normal(seg) => out.push(seg),
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn layer_fs_roundtrip() {
        let tmp = tempfile::tempdir().unwrap();
        let host = LayerFsHost::new(tmp.path());
        host.write_file("a/b.txt", "hello").unwrap();
        assert_eq!(host.read_file("a/b.txt").unwrap(), "hello");
        assert!(host.exists("a/b.txt").unwrap());
        host.move_path("a/b.txt", "a/c.txt").unwrap();
        assert!(host.exists("a/c.txt").unwrap());
        let listed = host.list_dir("a").unwrap();
        assert!(listed.contains(&"c.txt".to_string()));
        host.delete_file("a/c.txt").unwrap();
        assert!(!host.exists("a/c.txt").unwrap());
    }

    #[test]
    fn layer_fs_rejects_escape() {
        let tmp = tempfile::tempdir().unwrap();
        let host = LayerFsHost::new(tmp.path());
        assert!(host.read_file("../x").is_err());
        assert!(host.write_file("../x", "bad").is_err());
    }
}
