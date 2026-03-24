use std::error::Error;
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LakeMetadata {
    pub name: String,
    pub created_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LakeNameError {
    Empty,
    InvalidCharacter,
}

impl fmt::Display for LakeNameError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => {
                write!(f, "lake name must be non-empty and use only [a-zA-Z0-9_-]")
            }
            Self::InvalidCharacter => {
                write!(f, "lake name must be non-empty and use only [a-zA-Z0-9_-]")
            }
        }
    }
}

impl Error for LakeNameError {}

pub fn validate_lake_name(name: &str) -> Result<(), LakeNameError> {
    if name.is_empty() {
        return Err(LakeNameError::Empty);
    }

    if !name
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || ch == '-' || ch == '_')
    {
        return Err(LakeNameError::InvalidCharacter);
    }

    Ok(())
}

pub fn parse_lake_metadata(config: &str) -> LakeMetadata {
    let name = config
        .lines()
        .find(|line| line.starts_with("name"))
        .and_then(|line| line.split('=').nth(1))
        .map(|value| value.trim().trim_matches('"'))
        .unwrap_or("?")
        .to_string();

    let created_at = config
        .lines()
        .find(|line| line.starts_with("created_at"))
        .and_then(|line| line.split('=').nth(1))
        .map(|value| value.trim().trim_matches('"'))
        .unwrap_or("?")
        .to_string();

    LakeMetadata { name, created_at }
}

pub fn render_lake_config(name: &str, created_at: &str) -> String {
    format!("name = \"{}\"\ncreated_at = \"{}\"", name, created_at)
}

#[cfg(test)]
mod tests {
    use super::{parse_lake_metadata, render_lake_config, validate_lake_name};

    #[test]
    fn validates_accepted_names() {
        assert!(validate_lake_name("good-name").is_ok());
        assert!(validate_lake_name("my_lake").is_ok());
        assert!(validate_lake_name("lake123").is_ok());
    }

    #[test]
    fn rejects_empty_and_invalid_names() {
        assert!(validate_lake_name("").is_err());
        assert!(validate_lake_name("has spaces").is_err());
        assert!(validate_lake_name("has/slash").is_err());
        assert!(validate_lake_name("has.dot").is_err());
    }

    #[test]
    fn parses_lake_metadata_from_config() {
        let metadata =
            parse_lake_metadata("name = \"test\"\ncreated_at = \"2026-01-01T00:00:00Z\"");
        assert_eq!(metadata.name, "test");
        assert_eq!(metadata.created_at, "2026-01-01T00:00:00Z");
    }

    #[test]
    fn falls_back_for_missing_fields() {
        let metadata = parse_lake_metadata("");
        assert_eq!(metadata.name, "?");
        assert_eq!(metadata.created_at, "?");
    }

    #[test]
    fn renders_config_with_expected_keys() {
        let config = render_lake_config("lake", "2026-01-01T00:00:00Z");
        assert_eq!(
            config,
            "name = \"lake\"\ncreated_at = \"2026-01-01T00:00:00Z\""
        );
    }
}
