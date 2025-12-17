//! Download infrastructure for models.
//!
//! - reqwest for HTTP (already a dependency)
//! - shasum -a 256 for verification (built into macOS)

use anyhow::{Context, Result};
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::Path;
use std::process::Command;

/// Compute SHA256 hash of a file using `shasum -a 256`.
pub fn sha256_file(path: &Path) -> Result<String> {
    let output = Command::new("shasum")
        .args(["-a", "256"])
        .arg(path)
        .output()
        .with_context(|| format!("Failed to run shasum on {:?}", path))?;

    if !output.status.success() {
        anyhow::bail!("shasum failed: {}", String::from_utf8_lossy(&output.stderr));
    }

    // Output format: "hash  filename\n"
    let stdout = String::from_utf8_lossy(&output.stdout);
    let hash = stdout
        .split_whitespace()
        .next()
        .ok_or_else(|| anyhow::anyhow!("Failed to parse shasum output"))?;

    Ok(hash.to_lowercase())
}

/// Download a file from URL to destination path.
pub fn download_file(url: &str, dest: &Path) -> Result<u64> {
    if let Some(parent) = dest.parent() {
        fs::create_dir_all(parent)?;
    }

    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(300))
        .build()?;

    let response = client
        .get(url)
        .send()
        .with_context(|| format!("Failed to GET {}", url))?;

    if !response.status().is_success() {
        anyhow::bail!("HTTP {}: {}", response.status(), url);
    }

    let total_size = response.content_length();
    let mut downloaded: u64 = 0;
    let mut file = File::create(dest)?;
    let mut response = response;

    let mut buffer = [0u8; 8192];
    loop {
        let bytes_read = response.read(&mut buffer)?;
        if bytes_read == 0 {
            break;
        }

        file.write_all(&buffer[..bytes_read])?;
        downloaded += bytes_read as u64;

        // Simple progress
        if let Some(total) = total_size {
            let mb_done = downloaded / (1024 * 1024);
            let mb_total = total / (1024 * 1024);
            print!("\r  {}/{} MB", mb_done, mb_total);
        } else {
            print!("\r  {} MB", downloaded / (1024 * 1024));
        }
        std::io::stdout().flush().ok();
    }

    println!();
    Ok(downloaded)
}

/// Download and verify a file. Returns computed SHA256.
pub fn download_and_verify(
    url: &str,
    dest: &Path,
    expected_sha256: Option<&str>,
) -> Result<String> {
    println!("  {}", url);
    download_file(url, dest)?;

    print!("  Verifying...");
    std::io::stdout().flush().ok();

    let hash = sha256_file(dest)?;

    if let Some(expected) = expected_sha256 {
        if hash != expected.to_lowercase() {
            fs::remove_file(dest).ok();
            anyhow::bail!(
                "Checksum mismatch!\n  Expected: {}\n  Got: {}",
                expected,
                hash
            );
        }
        println!(" ✓");
    } else {
        println!(" {}", &hash[..12]);
    }

    Ok(hash)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_sha256_empty_file() {
        let file = NamedTempFile::new().unwrap();
        let hash = sha256_file(file.path()).unwrap();
        // SHA256 of empty file is well-known
        assert_eq!(
            hash,
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
    }

    #[test]
    fn test_sha256_known_content() {
        let mut file = NamedTempFile::new().unwrap();
        file.write_all(b"test").unwrap();
        let hash = sha256_file(file.path()).unwrap();
        // SHA256 of "test" is well-known
        assert_eq!(
            hash,
            "9f86d081884c7d659a2feaa0c55ad015a3bf4f1b2b0b822cd15d6c15b0f00a08"
        );
    }
}
