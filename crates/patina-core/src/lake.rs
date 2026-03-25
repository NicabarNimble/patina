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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LakeConfigDocument {
    pub location: String,
    pub content: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListedLake {
    pub name: String,
    pub created_at: String,
    pub location: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreateLakeResult {
    pub name: String,
    pub location: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LakeServiceError {
    InvalidName(LakeNameError),
    AlreadyExists { name: String, location: String },
    Repository(String),
}

impl fmt::Display for LakeServiceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidName(err) => write!(f, "{}", err),
            Self::AlreadyExists { name, location } => {
                write!(f, "lake '{}' already exists at {}", name, location)
            }
            Self::Repository(message) => write!(f, "{}", message),
        }
    }
}

impl Error for LakeServiceError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::InvalidName(err) => Some(err),
            Self::AlreadyExists { .. } | Self::Repository(_) => None,
        }
    }
}

pub trait LakeRepository {
    fn lake_location(&self, name: &str) -> String;
    fn lake_exists(&self, name: &str) -> Result<bool, String>;
    fn create_lake_dir(&self, name: &str) -> Result<(), String>;
    fn write_lake_config(&self, name: &str, content: &str) -> Result<(), String>;
    fn list_lake_configs(&self) -> Result<Vec<LakeConfigDocument>, String>;
}

pub trait Clock {
    fn now_rfc3339(&self) -> String;
}

pub fn create_lake(
    repository: &dyn LakeRepository,
    clock: &dyn Clock,
    name: &str,
) -> Result<CreateLakeResult, LakeServiceError> {
    validate_lake_name(name).map_err(LakeServiceError::InvalidName)?;

    let location = repository.lake_location(name);
    let exists = repository
        .lake_exists(name)
        .map_err(LakeServiceError::Repository)?;
    if exists {
        return Err(LakeServiceError::AlreadyExists {
            name: name.to_string(),
            location,
        });
    }

    repository
        .create_lake_dir(name)
        .map_err(LakeServiceError::Repository)?;

    let now = clock.now_rfc3339();
    let config = render_lake_config(name, &now);
    repository
        .write_lake_config(name, &config)
        .map_err(LakeServiceError::Repository)?;

    Ok(CreateLakeResult {
        name: name.to_string(),
        location,
    })
}

pub fn list_lakes(repository: &dyn LakeRepository) -> Result<Vec<ListedLake>, LakeServiceError> {
    let configs = repository
        .list_lake_configs()
        .map_err(LakeServiceError::Repository)?;

    let mut lakes = configs
        .into_iter()
        .map(|document| {
            let metadata = parse_lake_metadata(&document.content);
            ListedLake {
                name: metadata.name,
                created_at: metadata.created_at,
                location: document.location,
            }
        })
        .collect::<Vec<_>>();
    lakes.sort_by(|left, right| left.name.cmp(&right.name));
    Ok(lakes)
}

#[cfg(test)]
mod tests {
    use super::{
        create_lake, list_lakes, parse_lake_metadata, render_lake_config, validate_lake_name,
        Clock, LakeConfigDocument, LakeRepository, LakeServiceError,
    };

    #[derive(Default)]
    struct FakeLakeRepository {
        existing: std::collections::HashSet<String>,
        created: Vec<String>,
        writes: std::collections::HashMap<String, String>,
        listed: Vec<LakeConfigDocument>,
    }

    impl LakeRepository for std::cell::RefCell<FakeLakeRepository> {
        fn lake_location(&self, name: &str) -> String {
            format!("/fake/lakes/{}", name)
        }

        fn lake_exists(&self, name: &str) -> Result<bool, String> {
            Ok(self.borrow().existing.contains(name))
        }

        fn create_lake_dir(&self, name: &str) -> Result<(), String> {
            self.borrow_mut().created.push(name.to_string());
            Ok(())
        }

        fn write_lake_config(&self, name: &str, content: &str) -> Result<(), String> {
            self.borrow_mut()
                .writes
                .insert(name.to_string(), content.to_string());
            Ok(())
        }

        fn list_lake_configs(&self) -> Result<Vec<LakeConfigDocument>, String> {
            Ok(self.borrow().listed.clone())
        }
    }

    struct FixedClock;

    impl Clock for FixedClock {
        fn now_rfc3339(&self) -> String {
            "2026-01-02T03:04:05Z".to_string()
        }
    }

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

    #[test]
    fn create_lake_uses_repository_and_clock_ports() {
        let repo = std::cell::RefCell::new(FakeLakeRepository::default());
        let result = create_lake(&repo, &FixedClock, "alpha").unwrap();

        assert_eq!(result.name, "alpha");
        assert_eq!(result.location, "/fake/lakes/alpha");

        let state = repo.borrow();
        assert_eq!(state.created, vec!["alpha"]);
        assert_eq!(
            state.writes.get("alpha").map(String::as_str),
            Some("name = \"alpha\"\ncreated_at = \"2026-01-02T03:04:05Z\"")
        );
    }

    #[test]
    fn create_lake_rejects_existing_name() {
        let mut repo = FakeLakeRepository::default();
        repo.existing.insert("alpha".to_string());
        let repo = std::cell::RefCell::new(repo);

        let err = create_lake(&repo, &FixedClock, "alpha").unwrap_err();
        assert_eq!(
            err,
            LakeServiceError::AlreadyExists {
                name: "alpha".to_string(),
                location: "/fake/lakes/alpha".to_string(),
            }
        );
    }

    #[test]
    fn list_lakes_parses_and_sorts_metadata() {
        let mut repo = FakeLakeRepository::default();
        repo.listed = vec![
            LakeConfigDocument {
                location: "/fake/lakes/zeta".to_string(),
                content: "name = \"zeta\"\ncreated_at = \"2026-01-03T00:00:00Z\"".to_string(),
            },
            LakeConfigDocument {
                location: "/fake/lakes/alpha".to_string(),
                content: "name = \"alpha\"\ncreated_at = \"2026-01-01T00:00:00Z\"".to_string(),
            },
        ];
        let repo = std::cell::RefCell::new(repo);

        let lakes = list_lakes(&repo).unwrap();

        assert_eq!(lakes.len(), 2);
        assert_eq!(lakes[0].name, "alpha");
        assert_eq!(lakes[0].location, "/fake/lakes/alpha");
        assert_eq!(lakes[1].name, "zeta");
        assert_eq!(lakes[1].location, "/fake/lakes/zeta");
    }
}
