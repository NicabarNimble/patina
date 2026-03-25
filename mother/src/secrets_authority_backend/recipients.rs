use anyhow::{bail, Context, Result};
use std::fs;
use std::path::Path;

pub fn load_recipients(path: &Path) -> Result<Vec<String>> {
    if !path.exists() {
        return Ok(Vec::new());
    }

    let content = fs::read_to_string(path)
        .with_context(|| format!("Failed to read recipients file: {:?}", path))?;

    parse_recipients(&content)
}

pub fn parse_recipients(content: &str) -> Result<Vec<String>> {
    let mut recipients = Vec::new();

    for (line_num, line) in content.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        let key = line.split('#').next().unwrap_or(line).trim();
        if key.is_empty() {
            continue;
        }

        if !is_valid_age_recipient(key) {
            bail!(
                "Invalid age recipient on line {}: '{}'. Expected age1...",
                line_num + 1,
                key
            );
        }

        recipients.push(key.to_string());
    }

    Ok(recipients)
}

pub fn save_recipients(path: &Path, recipients: &[String]) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    let header = "# Patina Secrets Recipients\n\
                  # One age public key per line. Comments allowed.\n\n";

    let content: String = recipients.iter().map(|r| format!("{}\n", r)).collect();
    let full_content = format!("{}{}", header, content);

    fs::write(path, full_content)
        .with_context(|| format!("Failed to write recipients file: {:?}", path))?;

    Ok(())
}

pub fn is_valid_age_recipient(key: &str) -> bool {
    key.starts_with("age1") && key.len() >= 50 && key.len() <= 100
}

pub fn is_valid_age_identity(key: &str) -> bool {
    key.starts_with("AGE-SECRET-KEY-1") && key.len() >= 60 && key.len() <= 100
}
