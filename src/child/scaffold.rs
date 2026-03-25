//! Child scaffolding — `patina plugin init <name> --world <world>`.
//!
//! Generates a working child project from embedded templates.
//! Templates are compiled into the binary via `include_str!` — no
//! network, no cache, no sync. The binary is the single source of truth.

use std::path::{Path, PathBuf};

use anyhow::{bail, Result};

use super::engine::ChildKind;

// =========================================================================
// Embedded templates (compiled into the binary)
// =========================================================================

mod templates {
    pub mod mother_child {
        pub const CARGO_TOML: &str =
            include_str!("../../resources/templates/plugin/mother-child/Cargo.toml.tmpl");
        pub const MANIFEST_TOML: &str =
            include_str!("../../resources/templates/plugin/mother-child/child.toml.tmpl");
        pub const LIB_RS: &str =
            include_str!("../../resources/templates/plugin/mother-child/lib.rs.tmpl");
    }
    pub mod command {
        pub const CARGO_TOML: &str =
            include_str!("../../resources/templates/plugin/command/Cargo.toml.tmpl");
        pub const MANIFEST_TOML: &str =
            include_str!("../../resources/templates/plugin/command/child.toml.tmpl");
        pub const LIB_RS: &str =
            include_str!("../../resources/templates/plugin/command/lib.rs.tmpl");
    }
    pub mod task {
        pub const CARGO_TOML: &str =
            include_str!("../../resources/templates/plugin/task/Cargo.toml.tmpl");
        pub const MANIFEST_TOML: &str =
            include_str!("../../resources/templates/plugin/task/child.toml.tmpl");
        pub const LIB_RS: &str = include_str!("../../resources/templates/plugin/task/lib.rs.tmpl");
    }
    pub mod pipeline {
        pub const CARGO_TOML: &str =
            include_str!("../../resources/templates/plugin/pipeline/Cargo.toml.tmpl");
        pub const MANIFEST_TOML: &str =
            include_str!("../../resources/templates/plugin/pipeline/child.toml.tmpl");
        pub const LIB_RS: &str =
            include_str!("../../resources/templates/plugin/pipeline/lib.rs.tmpl");
    }
}

// =========================================================================
// Name validation
// =========================================================================

/// Validate a child name as a valid Rust crate name.
///
/// Rules: non-empty, starts with letter or underscore, contains only
/// alphanumeric, hyphens, or underscores. Matches Cargo's crate naming.
pub fn validate_name(name: &str) -> Result<()> {
    if name.is_empty() {
        bail!("child name cannot be empty");
    }
    let first = name.chars().next().unwrap();
    if !first.is_ascii_alphabetic() && first != '_' {
        bail!(
            "child name must start with a letter or underscore, got '{}'",
            first
        );
    }
    for ch in name.chars() {
        if !ch.is_ascii_alphanumeric() && ch != '-' && ch != '_' {
            bail!("child name contains invalid character: '{}'", ch);
        }
    }
    Ok(())
}

// =========================================================================
// Name transformation
// =========================================================================

/// Convert kebab-case name to PascalCase for Rust struct names.
///
/// "review-bot" → "ReviewBot", "my_plugin" → "MyPlugin"
pub fn to_pascal_case(name: &str) -> String {
    name.split(['-', '_'])
        .map(|segment| {
            let mut chars = segment.chars();
            match chars.next() {
                None => String::new(),
                Some(first) => {
                    let mut s = first.to_uppercase().to_string();
                    s.extend(chars);
                    s
                }
            }
        })
        .collect()
}

// =========================================================================
// Template substitution
// =========================================================================

/// SDK version embedded in scaffolded Cargo.toml files.
/// Uses the patina-sdk crate version, which tracks patina-ai major.minor.
const SDK_VERSION: &str = "0.21";

/// Apply name and version substitutions to a template string.
fn substitute(template: &str, name: &str) -> String {
    let struct_name = to_pascal_case(name);
    template
        .replace("__NAME_STRUCT__", &struct_name)
        .replace("__NAME__", name)
        .replace("__SDK_VERSION__", SDK_VERSION)
}

// =========================================================================
// Scaffold
// =========================================================================

/// Get the template set for a given world.
fn world_templates(world: &ChildKind) -> (&'static str, &'static str, &'static str) {
    match world {
        ChildKind::KnowledgeChild => (
            templates::mother_child::CARGO_TOML,
            templates::mother_child::MANIFEST_TOML,
            templates::mother_child::LIB_RS,
        ),
        ChildKind::MotherChild => (
            templates::mother_child::CARGO_TOML,
            templates::mother_child::MANIFEST_TOML,
            templates::mother_child::LIB_RS,
        ),
        ChildKind::Command => (
            templates::command::CARGO_TOML,
            templates::command::MANIFEST_TOML,
            templates::command::LIB_RS,
        ),
        ChildKind::Task => (
            templates::task::CARGO_TOML,
            templates::task::MANIFEST_TOML,
            templates::task::LIB_RS,
        ),
        ChildKind::Pipeline => (
            templates::pipeline::CARGO_TOML,
            templates::pipeline::MANIFEST_TOML,
            templates::pipeline::LIB_RS,
        ),
    }
}

/// Scaffold a new child project.
///
/// Creates `<name>/` in the given parent directory with:
/// - `Cargo.toml` (cdylib, correct guest API dep)
/// - `child.toml` (kind, capabilities, provides)
/// - `src/lib.rs` (trait impl, register macro)
///
/// Templates use `patina-sdk` version dep — no absolute paths.
///
/// Returns the path to the created project directory.
pub fn scaffold(parent: &Path, name: &str, world: &ChildKind) -> Result<PathBuf> {
    validate_name(name)?;

    let project_dir = parent.join(name);
    if project_dir.exists() {
        bail!("directory '{}' already exists", project_dir.display());
    }

    let src_dir = project_dir.join("src");
    std::fs::create_dir_all(&src_dir)?;

    let (cargo_tmpl, manifest_tmpl, lib_tmpl) = world_templates(world);

    std::fs::write(project_dir.join("Cargo.toml"), substitute(cargo_tmpl, name))?;
    std::fs::write(
        project_dir.join("child.toml"),
        substitute(manifest_tmpl, name),
    )?;
    std::fs::write(src_dir.join("lib.rs"), substitute(lib_tmpl, name))?;

    Ok(project_dir)
}

// =========================================================================
// Tests
// =========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_name_valid() {
        assert!(validate_name("review-bot").is_ok());
        assert!(validate_name("my_plugin").is_ok());
        assert!(validate_name("a").is_ok());
        assert!(validate_name("_private").is_ok());
    }

    #[test]
    fn test_validate_name_invalid() {
        assert!(validate_name("").is_err());
        assert!(validate_name("123abc").is_err());
        assert!(validate_name("-leading-hyphen").is_err());
        assert!(validate_name("has space").is_err());
        assert!(validate_name("has.dot").is_err());
    }

    #[test]
    fn test_to_pascal_case() {
        assert_eq!(to_pascal_case("review-bot"), "ReviewBot");
        assert_eq!(to_pascal_case("my_plugin"), "MyPlugin");
        assert_eq!(to_pascal_case("simple"), "Simple");
        assert_eq!(to_pascal_case("a-b-c"), "ABC");
        assert_eq!(to_pascal_case("hello-world-plugin"), "HelloWorldPlugin");
    }

    #[test]
    fn test_substitute() {
        let template =
            "name = \"__NAME__\"\nversion = \"__SDK_VERSION__\"\nstruct __NAME_STRUCT__;";
        let result = substitute(template, "review-bot");
        assert_eq!(
            result,
            "name = \"review-bot\"\nversion = \"0.21\"\nstruct ReviewBot;"
        );
    }

    #[test]
    fn test_scaffold_creates_files() {
        let tmp = tempfile::tempdir().unwrap();
        let result = scaffold(tmp.path(), "test-plugin", &ChildKind::Task);
        assert!(result.is_ok());

        let project = result.unwrap();
        assert!(project.join("Cargo.toml").exists());
        assert!(project.join("child.toml").exists());
        assert!(project.join("src/lib.rs").exists());

        let cargo = std::fs::read_to_string(project.join("Cargo.toml")).unwrap();
        assert!(cargo.contains("name = \"test-plugin\""));

        let lib = std::fs::read_to_string(project.join("src/lib.rs")).unwrap();
        assert!(cargo.contains("test-plugin"));
        assert!(lib.contains("TestPlugin"));
        assert!(lib.contains("register_task_child!"));
    }

    #[test]
    fn test_scaffold_rejects_existing_dir() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::create_dir(tmp.path().join("existing")).unwrap();
        let result = scaffold(tmp.path(), "existing", &ChildKind::Task);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("already exists"));
    }

    #[test]
    fn test_scaffold_all_worlds() {
        let tmp = tempfile::tempdir().unwrap();
        for (world, expected_macro) in [
            (ChildKind::KnowledgeChild, "register_mother_child!"),
            (ChildKind::MotherChild, "register_mother_child!"),
            (ChildKind::Command, "register_command_child!"),
            (ChildKind::Task, "register_task_child!"),
            (ChildKind::Pipeline, "register_pipeline_child!"),
        ] {
            let name = format!("test-{}", world);
            let project = scaffold(tmp.path(), &name, &world).unwrap();
            let lib = std::fs::read_to_string(project.join("src/lib.rs")).unwrap();
            assert!(
                lib.contains(expected_macro),
                "world {} should use {}",
                world,
                expected_macro
            );
        }
    }
}
