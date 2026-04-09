use anyhow::{Context, Result};
use sha2::{Digest, Sha256};
use std::collections::HashSet;
use std::path::{Path, PathBuf};

fn hash_sidecar_path(path: &Path) -> PathBuf {
    let mut sidecar = path.as_os_str().to_os_string();
    sidecar.push(".sha256");
    PathBuf::from(sidecar)
}

pub fn compute_file_hash(path: &Path) -> Result<String> {
    let bytes = std::fs::read(path).with_context(|| format!("reading {}", path.display()))?;
    Ok(format!("{:x}", Sha256::digest(&bytes)))
}

pub fn write_hash_file(path: &Path) -> Result<()> {
    let digest = compute_file_hash(path)?;
    std::fs::write(hash_sidecar_path(path), digest)
        .with_context(|| format!("writing hash for {}", path.display()))?;
    Ok(())
}

pub fn write_pando_hashes(pando_dir: &Path) -> Result<usize> {
    let pando_manifest = pando_dir.join("pando.toml");
    if !pando_manifest.exists() {
        anyhow::bail!("missing pando manifest at {}", pando_manifest.display());
    }

    let mut targets = HashSet::new();
    targets.insert(pando_manifest);

    if let Ok(entries) = std::fs::read_dir(pando_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_file() {
                continue;
            }
            match path.extension().and_then(|ext| ext.to_str()) {
                Some("toml") | Some("wasm") => {
                    targets.insert(path);
                }
                _ => {}
            }
        }
    }

    for target in &targets {
        write_hash_file(target)?;
    }
    Ok(targets.len())
}

pub fn write_all_pando_hashes(pandos_root: &Path) -> Result<(usize, usize)> {
    if !pandos_root.exists() {
        return Ok((0, 0));
    }

    let mut pando_dirs = std::fs::read_dir(pandos_root)
        .with_context(|| format!("reading pando root {}", pandos_root.display()))?
        .flatten()
        .filter(|entry| entry.path().is_dir())
        .collect::<Vec<_>>();
    pando_dirs.sort_by_key(|entry| entry.file_name());

    let mut pando_count = 0usize;
    let mut file_count = 0usize;
    for entry in pando_dirs {
        let written = write_pando_hashes(&entry.path())?;
        pando_count += 1;
        file_count += written;
    }

    Ok((pando_count, file_count))
}

pub fn verify_hash_file(path: &Path) -> Result<()> {
    let hash_path = hash_sidecar_path(path);
    if !hash_path.exists() {
        anyhow::bail!(
            "missing hash sidecar for {} at {}",
            path.display(),
            hash_path.display()
        );
    }

    let expected = std::fs::read_to_string(&hash_path)
        .with_context(|| format!("reading {}", hash_path.display()))?;
    let expected = expected.trim().to_string();
    let actual = compute_file_hash(path)?;
    if expected != actual {
        anyhow::bail!(
            "hash mismatch for {} (expected {}, got {})",
            path.display(),
            expected,
            actual
        );
    }
    Ok(())
}

pub fn verify_pando_integrity(pando_dir: &Path) -> Result<()> {
    let pando_name = pando_dir
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("unknown");
    let pando_manifest = pando_dir.join("pando.toml");
    if !pando_manifest.exists() {
        anyhow::bail!(
            "pando '{}' integrity check failed on {} — reinstall or run patina pando verify",
            pando_name,
            pando_manifest.display()
        );
    }

    let mut targets = HashSet::new();
    targets.insert(pando_manifest);

    if let Ok(entries) = std::fs::read_dir(pando_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_file() {
                continue;
            }
            match path.extension().and_then(|ext| ext.to_str()) {
                Some("toml") | Some("wasm") => {
                    targets.insert(path);
                }
                _ => {}
            }
        }
    }

    for target in targets {
        verify_hash_file(&target).map_err(|error| {
            anyhow::anyhow!(
                "pando '{}' integrity check failed on {} — reinstall or run patina pando verify: {}",
                pando_name,
                target.display(),
                error
            )
        })?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compute_file_hash_matches_known_digest() {
        let temp = tempfile::tempdir().unwrap();
        let file = temp.path().join("fixture.txt");
        std::fs::write(&file, b"abc").unwrap();
        let digest = compute_file_hash(&file).unwrap();
        assert_eq!(
            digest,
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn verify_hash_file_passes_for_unmodified_file() {
        let temp = tempfile::tempdir().unwrap();
        let file = temp.path().join("child.wasm");
        std::fs::write(&file, b"wasm-bytes").unwrap();
        write_hash_file(&file).unwrap();
        verify_hash_file(&file).unwrap();
    }

    #[test]
    fn verify_hash_file_fails_for_tampered_file() {
        let temp = tempfile::tempdir().unwrap();
        let file = temp.path().join("child.toml");
        std::fs::write(&file, "name='alpha'").unwrap();
        write_hash_file(&file).unwrap();
        std::fs::write(&file, "name='beta'").unwrap();
        let error = verify_hash_file(&file).unwrap_err();
        assert!(error.to_string().contains("hash mismatch"));
    }

    #[test]
    fn verify_hash_file_fails_for_missing_sidecar() {
        let temp = tempfile::tempdir().unwrap();
        let file = temp.path().join("pando.toml");
        std::fs::write(
            &file,
            "[pando]\nname='demo'\ndescription='d'\nversion='0.1.0'\n[[children]]\nname='x'\n",
        )
        .unwrap();
        let sidecar = hash_sidecar_path(&file);
        assert!(!sidecar.exists());
        let error = verify_hash_file(&file).unwrap_err();
        assert!(error.to_string().contains("missing hash sidecar"));
    }

    #[test]
    fn write_pando_hashes_writes_sidecars_for_manifest_and_wasm() {
        let temp = tempfile::tempdir().unwrap();
        let pando_dir = temp.path().join("demo");
        std::fs::create_dir_all(&pando_dir).unwrap();
        std::fs::write(
            pando_dir.join("pando.toml"),
            "[pando]\nname='demo'\ndescription='d'\nversion='0.1.0'\n[[children]]\nname='x'\n",
        )
        .unwrap();
        std::fs::write(pando_dir.join("x.wasm"), b"wasm-bytes").unwrap();

        let count = write_pando_hashes(&pando_dir).unwrap();
        assert!(count >= 2);
        assert!(pando_dir.join("pando.toml.sha256").exists());
        assert!(pando_dir.join("x.wasm.sha256").exists());
    }
}
