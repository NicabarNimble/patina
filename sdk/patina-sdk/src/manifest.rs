//! Canonical child package manifest contract.
//!
//! This module owns the developer-facing `child.toml` shape used by Patina
//! children. Runtime backends should parse this surface through the SDK rather
//! than re-defining local manifest structs.

use std::fmt;
use std::path::{Component, Path, PathBuf};

pub const CHILD_MANIFEST_FILE: &str = "child.toml";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChildManifest {
    pub name: String,
    pub version: String,
    pub description: Option<String>,
    pub kind: String,
    pub role: Option<String>,
    pub ingress_mode: ChildIngressMode,
    pub artifact: ChildArtifact,
    pub contract: ChildContract,
    pub needs: ChildNeeds,
    pub relationships: ChildRelationships,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChildArtifact {
    pub wasm: Option<PathBuf>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChildContract {
    pub default_operation: Option<String>,
    pub allow_operations: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChildNeeds {
    pub toys: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChildRelationships {
    pub listens: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChildIngressMode {
    Handle,
    Hybrid,
    WitOnly,
}

impl ChildIngressMode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Handle => "handle",
            Self::Hybrid => "hybrid",
            Self::WitOnly => "wit-only",
        }
    }
}

impl Default for ChildIngressMode {
    fn default() -> Self {
        Self::Handle
    }
}

impl std::str::FromStr for ChildIngressMode {
    type Err = ChildManifestError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "handle" => Ok(Self::Handle),
            "hybrid" => Ok(Self::Hybrid),
            "wit-only" => Ok(Self::WitOnly),
            other => Err(ChildManifestError::InvalidIngressMode(other.to_string())),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChildPackage {
    pub manifest_path: PathBuf,
    pub artifact_path: PathBuf,
    pub manifest: ChildManifest,
}

impl ChildPackage {
    /// Load a release/package directory containing `child.toml` and its
    /// manifest-declared `.wasm` artifact.
    ///
    /// This is intentionally strict. Backends must not infer artifact identity
    /// from directory contents.
    pub fn from_package_dir(package_dir: impl AsRef<Path>) -> Result<Self, ChildManifestError> {
        let package_dir = package_dir.as_ref();
        let manifest_path = package_dir.join(CHILD_MANIFEST_FILE);
        let manifest = ChildManifest::from_path(&manifest_path)?;
        let artifact_relative_path = manifest.artifact.wasm.as_ref().ok_or(
            ChildManifestError::MissingArtifactDeclaration("child.artifact.wasm"),
        )?;
        let artifact_path = package_dir.join(artifact_relative_path);
        if !artifact_path.is_file() {
            return Err(ChildManifestError::MissingWasmArtifact(artifact_path));
        }
        Ok(Self {
            manifest_path,
            artifact_path,
            manifest,
        })
    }
}

impl ChildManifest {
    pub fn from_path(path: impl AsRef<Path>) -> Result<Self, ChildManifestError> {
        let path = path.as_ref();
        let contents = std::fs::read_to_string(path).map_err(|source| {
            if source.kind() == std::io::ErrorKind::NotFound {
                ChildManifestError::MissingManifest(path.to_path_buf())
            } else {
                ChildManifestError::ReadManifest {
                    path: path.to_path_buf(),
                    message: source.to_string(),
                }
            }
        })?;
        Self::from_toml_str(&contents)
    }

    pub fn from_toml_str(contents: &str) -> Result<Self, ChildManifestError> {
        let raw: RawChildManifest = toml::from_str(contents)
            .map_err(|error| ChildManifestError::ParseToml(error.to_string()))?;
        let child = raw
            .child
            .ok_or(ChildManifestError::MissingSection("child"))?;

        let name = required_string(child.name, "child.name")?;
        let version = required_string(child.version, "child.version")?;
        let kind = required_string(child.kind, "child.kind")?;
        let role = optional_string(child.role);
        let description = optional_string(child.description);
        let ingress_mode = child
            .ingress
            .and_then(|ingress| ingress.mode)
            .map(|mode| {
                required_string(Some(mode), "child.ingress.mode")?.parse::<ChildIngressMode>()
            })
            .transpose()?
            .unwrap_or_default();
        let artifact = child.artifact.unwrap_or_default();
        let contract = child.contract.unwrap_or_default();
        let needs = raw.needs.unwrap_or_default();
        let relationships = raw.relationships.unwrap_or_default();

        Ok(Self {
            name,
            version,
            description,
            kind,
            role,
            ingress_mode,
            artifact: ChildArtifact {
                wasm: optional_artifact_path(artifact.wasm, "child.artifact.wasm")?,
            },
            contract: ChildContract {
                default_operation: optional_string(contract.default),
                allow_operations: normalized_string_vec(contract.allow, "child.contract.allow")?,
            },
            needs: ChildNeeds {
                toys: normalized_string_vec(needs.toys, "needs.toys")?,
            },
            relationships: ChildRelationships {
                listens: normalized_string_vec(relationships.listens, "relationships.listens")?,
            },
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChildManifestError {
    MissingManifest(PathBuf),
    ReadManifest {
        path: PathBuf,
        message: String,
    },
    ReadPackageDir {
        path: PathBuf,
        message: String,
    },
    ParseToml(String),
    MissingSection(&'static str),
    MissingRequiredField(&'static str),
    EmptyStringField(&'static str),
    EmptyStringArrayItem {
        field: &'static str,
        index: usize,
    },
    InvalidIngressMode(String),
    MissingArtifactDeclaration(&'static str),
    InvalidArtifactPath {
        field: &'static str,
        path: String,
        reason: &'static str,
    },
    MissingWasmArtifact(PathBuf),
}

impl fmt::Display for ChildManifestError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingManifest(path) => write!(f, "missing child manifest {}", path.display()),
            Self::ReadManifest { path, message } => {
                write!(f, "read child manifest {}: {message}", path.display())
            }
            Self::ReadPackageDir { path, message } => {
                write!(
                    f,
                    "read child package directory {}: {message}",
                    path.display()
                )
            }
            Self::ParseToml(message) => write!(f, "parse child manifest TOML: {message}"),
            Self::MissingSection(section) => {
                write!(f, "child manifest missing [{section}] section")
            }
            Self::MissingRequiredField(field) => write!(f, "child manifest missing {field}"),
            Self::EmptyStringField(field) => {
                write!(f, "child manifest field {field} must not be empty")
            }
            Self::EmptyStringArrayItem { field, index } => {
                write!(f, "child manifest field {field}[{index}] must not be empty")
            }
            Self::InvalidIngressMode(mode) => write!(f, "unknown child ingress mode '{mode}'"),
            Self::MissingArtifactDeclaration(field) => {
                write!(f, "child package missing artifact declaration {field}")
            }
            Self::InvalidArtifactPath {
                field,
                path,
                reason,
            } => write!(
                f,
                "child manifest field {field} has invalid artifact path '{path}': {reason}"
            ),
            Self::MissingWasmArtifact(path) => {
                write!(
                    f,
                    "child package is missing wasm artifact {}",
                    path.display()
                )
            }
        }
    }
}

impl std::error::Error for ChildManifestError {}

#[derive(Debug, serde::Deserialize)]
struct RawChildManifest {
    child: Option<RawChildSection>,
    #[serde(default)]
    needs: Option<RawNeedsSection>,
    #[serde(default)]
    relationships: Option<RawRelationshipsSection>,
}

#[derive(Debug, serde::Deserialize)]
struct RawChildSection {
    name: Option<String>,
    version: Option<String>,
    #[serde(default)]
    description: Option<String>,
    kind: Option<String>,
    #[serde(default)]
    role: Option<String>,
    #[serde(default)]
    ingress: Option<RawIngressSection>,
    #[serde(default)]
    artifact: Option<RawArtifactSection>,
    #[serde(default)]
    contract: Option<RawContractSection>,
}

#[derive(Debug, serde::Deserialize)]
struct RawIngressSection {
    mode: Option<String>,
}

#[derive(Debug, Default, serde::Deserialize)]
struct RawArtifactSection {
    wasm: Option<String>,
}

#[derive(Debug, Default, serde::Deserialize)]
struct RawContractSection {
    #[serde(default)]
    default: Option<String>,
    #[serde(default)]
    allow: Vec<String>,
}

#[derive(Debug, Default, serde::Deserialize)]
struct RawNeedsSection {
    #[serde(default)]
    toys: Vec<String>,
}

#[derive(Debug, Default, serde::Deserialize)]
struct RawRelationshipsSection {
    #[serde(default)]
    listens: Vec<String>,
}

fn required_string(
    value: Option<String>,
    field: &'static str,
) -> Result<String, ChildManifestError> {
    let value = value.ok_or(ChildManifestError::MissingRequiredField(field))?;
    let value = value.trim();
    if value.is_empty() {
        return Err(ChildManifestError::EmptyStringField(field));
    }
    Ok(value.to_string())
}

fn optional_string(value: Option<String>) -> Option<String> {
    value
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn normalized_string_vec(
    values: Vec<String>,
    field: &'static str,
) -> Result<Vec<String>, ChildManifestError> {
    values
        .into_iter()
        .enumerate()
        .map(|(index, value)| {
            let value = value.trim().to_string();
            if value.is_empty() {
                Err(ChildManifestError::EmptyStringArrayItem { field, index })
            } else {
                Ok(value)
            }
        })
        .collect()
}

fn optional_artifact_path(
    value: Option<String>,
    field: &'static str,
) -> Result<Option<PathBuf>, ChildManifestError> {
    optional_string(value)
        .map(|value| normalized_artifact_path(value, field))
        .transpose()
}

fn normalized_artifact_path(
    value: String,
    field: &'static str,
) -> Result<PathBuf, ChildManifestError> {
    let path = PathBuf::from(&value);
    if path.extension().and_then(|ext| ext.to_str()) != Some("wasm") {
        return Err(ChildManifestError::InvalidArtifactPath {
            field,
            path: value,
            reason: "artifact path must end in .wasm",
        });
    }
    if path.components().any(|component| {
        matches!(
            component,
            Component::Prefix(_) | Component::RootDir | Component::ParentDir | Component::CurDir
        )
    }) {
        return Err(ChildManifestError::InvalidArtifactPath {
            field,
            path: value,
            reason: "artifact path must be a relative package path without . or .. components",
        });
    }
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn manifest() -> &'static str {
        r#"[child]
name = "slate-manager"
version = "0.4.0"
description = "Slate manager"
kind = "child"
role = "app"

[child.ingress]
mode = "wit-only"

[child.artifact]
wasm = "artifacts/slate.wasm"

[child.contract]
default = "patina:slate/control@0.1.0.list-work"
allow = [
  "patina:slate/control@0.1.0.list-work",
  "patina:slate/control@0.1.0.show-work",
]

[needs]
toys = ["logging", "measure", "git", "filesystem"]

[relationships]
listens = ["events.changed"]
"#
    }

    #[test]
    fn parses_current_child_manifest_shape() {
        let parsed = ChildManifest::from_toml_str(manifest()).unwrap();

        assert_eq!(parsed.name, "slate-manager");
        assert_eq!(parsed.version, "0.4.0");
        assert_eq!(parsed.kind, "child");
        assert_eq!(parsed.role.as_deref(), Some("app"));
        assert_eq!(parsed.ingress_mode, ChildIngressMode::WitOnly);
        assert_eq!(
            parsed.artifact.wasm,
            Some(PathBuf::from("artifacts/slate.wasm"))
        );
        assert_eq!(
            parsed.contract.default_operation.as_deref(),
            Some("patina:slate/control@0.1.0.list-work")
        );
        assert_eq!(parsed.contract.allow_operations.len(), 2);
        assert_eq!(
            parsed.needs.toys,
            vec!["logging", "measure", "git", "filesystem"]
        );
        assert_eq!(parsed.relationships.listens, vec!["events.changed"]);
    }

    #[test]
    fn rejects_missing_required_identity() {
        let error = ChildManifest::from_toml_str("[child]\nversion = \"0.1.0\"\nkind = \"child\"")
            .unwrap_err();

        assert_eq!(
            error,
            ChildManifestError::MissingRequiredField("child.name")
        );
    }

    #[test]
    fn rejects_empty_allow_item() {
        let error = ChildManifest::from_toml_str(
            "[child]\nname=\"x\"\nversion=\"0.1.0\"\nkind=\"child\"\n[child.contract]\nallow=[\"\"]",
        )
        .unwrap_err();

        assert_eq!(
            error,
            ChildManifestError::EmptyStringArrayItem {
                field: "child.contract.allow",
                index: 0
            }
        );
    }

    #[test]
    fn package_dir_loads_manifest_declared_wasm_artifact() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join("artifacts")).unwrap();
        std::fs::write(dir.path().join(CHILD_MANIFEST_FILE), manifest()).unwrap();
        std::fs::write(dir.path().join("artifacts/slate.wasm"), "wasm").unwrap();
        std::fs::write(dir.path().join("ignored.wasm"), "other wasm").unwrap();

        let package = ChildPackage::from_package_dir(dir.path()).unwrap();

        assert_eq!(package.manifest.name, "slate-manager");
        assert_eq!(
            package.artifact_path,
            dir.path().join("artifacts/slate.wasm")
        );
    }

    #[test]
    fn package_dir_rejects_missing_wasm_artifact_declaration() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join(CHILD_MANIFEST_FILE),
            "[child]\nname='x'\nversion='0.1.0'\nkind='child'\n",
        )
        .unwrap();
        std::fs::write(dir.path().join("x.wasm"), "wasm").unwrap();

        let error = ChildPackage::from_package_dir(dir.path()).unwrap_err();

        assert_eq!(
            error,
            ChildManifestError::MissingArtifactDeclaration("child.artifact.wasm")
        );
    }

    #[test]
    fn rejects_artifact_paths_that_escape_package() {
        let error = ChildManifest::from_toml_str(
            "[child]\nname='x'\nversion='0.1.0'\nkind='child'\n[child.artifact]\nwasm='../x.wasm'\n",
        )
        .unwrap_err();

        assert_eq!(
            error,
            ChildManifestError::InvalidArtifactPath {
                field: "child.artifact.wasm",
                path: "../x.wasm".into(),
                reason: "artifact path must be a relative package path without . or .. components"
            }
        );
    }
}
