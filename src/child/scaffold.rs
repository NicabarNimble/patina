//! Child scaffolding — `patina child init <name>`.
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
    pub mod typed_child {
        pub const CARGO_TOML: &str =
            include_str!("../../resources/templates/child/child/Cargo.toml.tmpl");
        pub const MANIFEST_TOML: &str =
            include_str!("../../resources/templates/child/child/child.toml.tmpl");
        pub const LIB_RS: &str = include_str!("../../resources/templates/child/child/lib.rs.tmpl");
        pub const README_MD: &str =
            include_str!("../../resources/templates/child/child/README.md.tmpl");
        pub const DIAGNOSTICS_CARGO_TOML: &str =
            include_str!("../../resources/templates/child/child/diagnostics-Cargo.toml.tmpl");
        pub const DIAGNOSTICS_TEST_RS: &str =
            include_str!("../../resources/templates/child/child/diagnostics-test.rs.tmpl");
        pub const CHILDREN_DEV_TOML: &str =
            include_str!("../../resources/templates/child/child/children-dev.toml.tmpl");
        pub const WORLD_WIT: &str =
            include_str!("../../resources/templates/child/child/world.wit.tmpl");
        pub const DEPS_LOGGING_WIT: &str =
            include_str!("../../resources/templates/child/child/deps/logging.wit.tmpl");
        pub const DEPS_PATINA_MEASURE_WIT: &str =
            include_str!("../../resources/templates/child/child/deps/patina-measure.wit.tmpl");
        pub const DEPS_PATINA_RECORD_WIT: &str =
            include_str!("../../resources/templates/child/child/deps/patina-record.wit.tmpl");
    }

    pub mod legacy_child {
        pub const CARGO_TOML: &str =
            include_str!("../../resources/templates/child/legacy-child/Cargo.toml.tmpl");
        pub const MANIFEST_TOML: &str =
            include_str!("../../resources/templates/child/legacy-child/child.toml.tmpl");
        pub const LIB_RS: &str =
            include_str!("../../resources/templates/child/legacy-child/lib.rs.tmpl");
    }

    pub mod legacy_pipeline {
        pub const CARGO_TOML: &str =
            include_str!("../../resources/templates/child/pipeline/Cargo.toml.tmpl");
        pub const MANIFEST_TOML: &str =
            include_str!("../../resources/templates/child/pipeline/child.toml.tmpl");
        pub const LIB_RS: &str =
            include_str!("../../resources/templates/child/pipeline/lib.rs.tmpl");
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
const SDK_VERSION: &str = "0.22.0";

/// Apply name and version substitutions to a template string.
fn substitute(template: &str, name: &str) -> String {
    let struct_name = to_pascal_case(name);
    template
        .replace("__NAME_STRUCT__", &struct_name)
        .replace("__NAME__", name)
        .replace("__SDK_VERSION__", SDK_VERSION)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScaffoldLane {
    /// Typed SDK-first lane (default)
    Typed,
    /// Legacy handle(action,payload) lane (explicit opt-in)
    Legacy,
}

struct TemplateSet {
    cargo_toml: &'static str,
    manifest_toml: &'static str,
    lib_rs: &'static str,
    readme_md: Option<&'static str>,
    diagnostics_cargo_toml: Option<&'static str>,
    diagnostics_test_rs: Option<&'static str>,
    children_dev_toml: Option<&'static str>,
    world_wit: Option<&'static str>,
    wit_deps: &'static [(&'static str, &'static str)],
}

fn typed_templates(world: &ChildKind) -> Result<TemplateSet> {
    match world {
        ChildKind::Child => Ok(TemplateSet {
            cargo_toml: templates::typed_child::CARGO_TOML,
            manifest_toml: templates::typed_child::MANIFEST_TOML,
            lib_rs: templates::typed_child::LIB_RS,
            readme_md: Some(templates::typed_child::README_MD),
            diagnostics_cargo_toml: Some(templates::typed_child::DIAGNOSTICS_CARGO_TOML),
            diagnostics_test_rs: Some(templates::typed_child::DIAGNOSTICS_TEST_RS),
            children_dev_toml: Some(templates::typed_child::CHILDREN_DEV_TOML),
            world_wit: Some(templates::typed_child::WORLD_WIT),
            wit_deps: &[
                ("logging.wit", templates::typed_child::DEPS_LOGGING_WIT),
                (
                    "patina-measure.wit",
                    templates::typed_child::DEPS_PATINA_MEASURE_WIT,
                ),
                (
                    "patina-record.wit",
                    templates::typed_child::DEPS_PATINA_RECORD_WIT,
                ),
            ],
        }),
        ChildKind::Pipeline => {
            bail!(
                "typed scaffolding is currently available for --world child only; use --legacy --world pipeline for grammar lane"
            )
        }
    }
}

fn legacy_templates(world: &ChildKind) -> TemplateSet {
    match world {
        ChildKind::Child => TemplateSet {
            cargo_toml: templates::legacy_child::CARGO_TOML,
            manifest_toml: templates::legacy_child::MANIFEST_TOML,
            lib_rs: templates::legacy_child::LIB_RS,
            readme_md: None,
            diagnostics_cargo_toml: None,
            diagnostics_test_rs: None,
            children_dev_toml: None,
            world_wit: None,
            wit_deps: &[],
        },
        ChildKind::Pipeline => TemplateSet {
            cargo_toml: templates::legacy_pipeline::CARGO_TOML,
            manifest_toml: templates::legacy_pipeline::MANIFEST_TOML,
            lib_rs: templates::legacy_pipeline::LIB_RS,
            readme_md: None,
            diagnostics_cargo_toml: None,
            diagnostics_test_rs: None,
            children_dev_toml: None,
            world_wit: None,
            wit_deps: &[],
        },
    }
}

/// Scaffold a new child project.
///
/// Creates `<name>/` in the given parent directory with:
/// - `Cargo.toml`
/// - `child.toml`
/// - `src/lib.rs`
/// - `wit/world.wit` (typed lane only)
///
/// Returns the path to the created project directory.
pub fn scaffold(
    parent: &Path,
    name: &str,
    world: &ChildKind,
    lane: ScaffoldLane,
) -> Result<PathBuf> {
    validate_name(name)?;

    let project_dir = parent.join(name);
    if project_dir.exists() {
        bail!("directory '{}' already exists", project_dir.display());
    }

    let src_dir = project_dir.join("src");
    std::fs::create_dir_all(&src_dir)?;

    let templates = match lane {
        ScaffoldLane::Typed => typed_templates(world)?,
        ScaffoldLane::Legacy => legacy_templates(world),
    };

    std::fs::write(
        project_dir.join("Cargo.toml"),
        substitute(templates.cargo_toml, name),
    )?;
    std::fs::write(
        project_dir.join("child.toml"),
        substitute(templates.manifest_toml, name),
    )?;
    std::fs::write(src_dir.join("lib.rs"), substitute(templates.lib_rs, name))?;

    if let Some(readme_md) = templates.readme_md {
        std::fs::write(project_dir.join("README.md"), substitute(readme_md, name))?;
    }

    if let (Some(diagnostics_cargo_toml), Some(diagnostics_test_rs)) = (
        templates.diagnostics_cargo_toml,
        templates.diagnostics_test_rs,
    ) {
        let diagnostics_dir = project_dir.join("checks/diagnostics");
        std::fs::create_dir_all(diagnostics_dir.join("tests"))?;
        std::fs::write(
            diagnostics_dir.join("Cargo.toml"),
            substitute(diagnostics_cargo_toml, name),
        )?;
        std::fs::write(
            diagnostics_dir.join("tests/diagnostics.rs"),
            substitute(diagnostics_test_rs, name),
        )?;
    }

    if let Some(children_dev_toml) = templates.children_dev_toml {
        let patina_dir = project_dir.join(".patina");
        std::fs::create_dir_all(&patina_dir)?;
        std::fs::write(
            patina_dir.join("children-dev.toml"),
            substitute(children_dev_toml, name),
        )?;
    }

    if let Some(world_wit) = templates.world_wit {
        let wit_dir = project_dir.join("wit");
        std::fs::create_dir_all(&wit_dir)?;
        std::fs::write(wit_dir.join("world.wit"), substitute(world_wit, name))?;

        if !templates.wit_deps.is_empty() {
            let deps_dir = wit_dir.join("deps");
            std::fs::create_dir_all(&deps_dir)?;
            for (filename, content) in templates.wit_deps {
                std::fs::write(deps_dir.join(filename), substitute(content, name))?;
            }
        }
    }

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
            "name = \"review-bot\"\nversion = \"0.22.0\"\nstruct ReviewBot;"
        );
    }

    #[test]
    fn test_scaffold_typed_child_default() {
        let tmp = tempfile::tempdir().unwrap();
        let project = scaffold(
            tmp.path(),
            "test-plugin",
            &ChildKind::Child,
            ScaffoldLane::Typed,
        )
        .unwrap();

        assert!(project.join("Cargo.toml").exists());
        assert!(project.join("child.toml").exists());
        assert!(project.join("src/lib.rs").exists());
        assert!(project.join("README.md").exists());
        assert!(project.join("checks/diagnostics/Cargo.toml").exists());
        assert!(project
            .join("checks/diagnostics/tests/diagnostics.rs")
            .exists());
        assert!(project.join(".patina/children-dev.toml").exists());
        assert!(project.join("wit/world.wit").exists());
        assert!(project.join("wit/deps/logging.wit").exists());
        assert!(project.join("wit/deps/patina-measure.wit").exists());
        assert!(project.join("wit/deps/patina-record.wit").exists());

        let lib = std::fs::read_to_string(project.join("src/lib.rs")).unwrap();
        assert!(lib.contains("wit_bindgen::generate!"));
        assert!(lib.contains("export!(TestPlugin);"));
        assert!(!lib.contains("register_child!"));

        let manifest = std::fs::read_to_string(project.join("child.toml")).unwrap();
        assert!(manifest.contains("kind = \"child\""));
        assert!(manifest.contains("toys = [\"logging\", \"measure\"]"));

        let diagnostics =
            std::fs::read_to_string(project.join("checks/diagnostics/tests/diagnostics.rs"))
                .unwrap();
        assert!(diagnostics.contains("patina_child_diagnostics::check_local_dev"));

        let children_dev =
            std::fs::read_to_string(project.join(".patina/children-dev.toml")).unwrap();
        assert!(children_dev.contains("[children.test-plugin]"));
        assert!(children_dev.contains(".patina/dev/components/test-plugin.wasm"));
    }

    #[test]
    fn test_scaffold_legacy_child_explicit() {
        let tmp = tempfile::tempdir().unwrap();
        let project = scaffold(
            tmp.path(),
            "legacy-child",
            &ChildKind::Child,
            ScaffoldLane::Legacy,
        )
        .unwrap();

        assert!(project.join("Cargo.toml").exists());
        assert!(project.join("child.toml").exists());
        assert!(project.join("src/lib.rs").exists());
        assert!(!project.join("wit/world.wit").exists());

        let lib = std::fs::read_to_string(project.join("src/lib.rs")).unwrap();
        assert!(lib.contains("register_child!"));
    }

    #[test]
    fn test_scaffold_typed_pipeline_requires_legacy() {
        let tmp = tempfile::tempdir().unwrap();
        let err = scaffold(
            tmp.path(),
            "typed-pipeline",
            &ChildKind::Pipeline,
            ScaffoldLane::Typed,
        )
        .unwrap_err()
        .to_string();

        assert!(err.contains("--legacy --world pipeline"), "got: {err}");
    }

    #[test]
    fn test_scaffold_legacy_pipeline_world() {
        let tmp = tempfile::tempdir().unwrap();
        let project = scaffold(
            tmp.path(),
            "legacy-pipeline",
            &ChildKind::Pipeline,
            ScaffoldLane::Legacy,
        )
        .unwrap();

        let lib = std::fs::read_to_string(project.join("src/lib.rs")).unwrap();
        assert!(lib.contains("register_pipeline_child!"));
    }

    #[test]
    fn test_scaffold_rejects_existing_dir() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::create_dir(tmp.path().join("existing")).unwrap();
        let result = scaffold(
            tmp.path(),
            "existing",
            &ChildKind::Child,
            ScaffoldLane::Typed,
        );
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("already exists"));
    }
}
