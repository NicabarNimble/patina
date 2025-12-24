//! Recipients file parsing (recipients.txt / recipient.txt).
//!
//! Handles age public keys for multi-recipient encryption.
//!
//! Format:
//! ```text
//! # Recipients (one per line, comments allowed)
//! age1alice0qwerty...   # Alice
//! age1bob00asdfgh...    # Bob
//! age1ci000zxcvbn...    # GitHub Actions
//! ```

use anyhow::{bail, Context, Result};
use std::fs;
use std::path::Path;

/// Parse recipients from a file.
///
/// Returns a list of age public keys (age1...).
/// Strips comments and blank lines.
pub fn load_recipients(path: &Path) -> Result<Vec<String>> {
    if !path.exists() {
        return Ok(Vec::new());
    }

    let content = fs::read_to_string(path)
        .with_context(|| format!("Failed to read recipients file: {:?}", path))?;

    parse_recipients(&content)
}

/// Parse recipients from a string.
pub fn parse_recipients(content: &str) -> Result<Vec<String>> {
    let mut recipients = Vec::new();

    for (line_num, line) in content.lines().enumerate() {
        let line = line.trim();

        // Skip empty lines and comments
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        // Handle inline comments: "age1xxx...   # Alice"
        let key = line.split('#').next().unwrap_or(line).trim();

        if key.is_empty() {
            continue;
        }

        // Validate age public key format
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

/// Save recipients to a file.
pub fn save_recipients(path: &Path, recipients: &[String]) -> Result<()> {
    // Ensure parent directory exists
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

/// Validate age recipient (public key) format.
///
/// Age public keys start with "age1" and are ~62 characters.
/// We do a basic prefix check - age crate does full validation.
pub fn is_valid_age_recipient(key: &str) -> bool {
    // Age public keys: age1 followed by bech32 encoding
    // Typical length is 58-62 characters
    key.starts_with("age1") && key.len() >= 50 && key.len() <= 100
}

/// Validate age identity (private key) format.
///
/// Age identities start with "AGE-SECRET-KEY-1" and are ~74 characters.
pub fn is_valid_age_identity(key: &str) -> bool {
    key.starts_with("AGE-SECRET-KEY-1") && key.len() >= 60 && key.len() <= 100
}

/// Extract the public key (recipient) from an age identity.
///
/// Uses the age crate to derive the public key.
pub fn identity_to_recipient(identity: &str) -> Result<String> {
    use age::x25519;
    use std::str::FromStr;

    let identity = x25519::Identity::from_str(identity)
        .map_err(|e| anyhow::anyhow!("Invalid age identity: {}", e))?;

    Ok(identity.to_public().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_recipients() {
        let content = r#"
# Recipients
age1qwerty1234567890abcdefghijklmnopqrstuvwxyz12345678901234   # Alice
age1asdfgh1234567890abcdefghijklmnopqrstuvwxyz12345678901234   # Bob

# CI
age1zxcvbn1234567890abcdefghijklmnopqrstuvwxyz12345678901234
"#;

        let recipients = parse_recipients(content).unwrap();
        assert_eq!(recipients.len(), 3);
        assert!(recipients[0].starts_with("age1qwerty"));
        assert!(recipients[1].starts_with("age1asdfgh"));
        assert!(recipients[2].starts_with("age1zxcvbn"));
    }

    #[test]
    fn test_is_valid_age_recipient() {
        // Valid (62 chars typical)
        assert!(is_valid_age_recipient(
            "age1qwerty1234567890abcdefghijklmnopqrstuvwxyz12345678901234"
        ));

        // Invalid
        assert!(!is_valid_age_recipient("age1short"));
        assert!(!is_valid_age_recipient("notage1..."));
        assert!(!is_valid_age_recipient(""));
    }

    #[test]
    fn test_is_valid_age_identity() {
        // Valid (74 chars typical)
        assert!(is_valid_age_identity(
            "AGE-SECRET-KEY-1QWERTY1234567890ABCDEFGHIJKLMNOPQRSTUVWXYZ1234567890ABC"
        ));

        // Invalid
        assert!(!is_valid_age_identity("AGE-SECRET-KEY-1SHORT"));
        assert!(!is_valid_age_identity("age1..."));
        assert!(!is_valid_age_identity(""));
    }
}
