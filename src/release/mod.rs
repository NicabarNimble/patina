//! Release strategy for Patina projects
//!
//! Decouples spec lifecycle from version management. Specs are universal —
//! every patina project has them. Versioning is a plugin, dispatched via
//! `ReleaseStrategy`.
//!
//! Follows [[dependable-rust]]: mod.rs (interface) + internal.rs (implementation).
//! Follows [[compiler-enforced-safety]]: enums over strings, typestate over docs.
//!
//! # Example
//!
//! ```ignore
//! use patina::release::{BumpType, ReleaseStrategy};
//!
//! let strategy = ReleaseStrategy::from_project(project_path);
//! let bump = BumpType::from_spec_type("feat");
//! if let Some(bump) = bump {
//!     let prepared = strategy.preflight(bump, "layer/surface/build/feat/my-feature/SPEC.md")?;
//!     prepared.execute("my feature", "layer/surface/build/feat/my-feature/SPEC.md", None)?;
//! }
//! ```

pub(crate) mod internal;

use anyhow::Result;
use std::path::Path;

pub fn read_cargo_version() -> Result<String> {
    internal::read_cargo_version()
}

pub fn compute_next_version(current: &str, bump: BumpType) -> Result<String> {
    internal::compute_next_version(current, bump)
}

pub fn update_cargo_version(new_version: &str) -> Result<()> {
    internal::update_cargo_version(new_version)
}

// ============================================================================
// Core Types
// ============================================================================

/// Version bump types — exhaustive enum, not stringly-typed.
///
/// Per [[compiler-enforced-safety]]: the compiler enforces that every
/// call site handles all variants.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BumpType {
    Patch,
    Minor,
    Major,
}

impl BumpType {
    /// Determine bump type from spec type.
    ///
    /// Returns `None` for spec types that don't warrant a version bump
    /// (e.g., `explore`).
    pub fn from_spec_type(spec_type: &str) -> Option<Self> {
        match spec_type {
            "fix" | "refactor" => Some(BumpType::Patch),
            "feat" => Some(BumpType::Minor),
            _ => None, // explore, unknown types → no bump
        }
    }

    /// Human-readable label for display
    pub fn label(&self) -> &'static str {
        match self {
            BumpType::Patch => "patch",
            BumpType::Minor => "minor",
            BumpType::Major => "major",
        }
    }
}

/// Release strategy — enum today, trait/WIT when plugin infra lands.
///
/// Three user profiles:
/// - `Cargo`: Patina-native — Patina owns Cargo.toml (safeguards → bump → commit → tag)
/// - `External`: BYO-version — print reminder, don't touch files
/// - `None`: Spec-only — silent no-op
///
/// Evolution path: enum → trait → WIT (same shape as Mother's children).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReleaseStrategy {
    /// Rust: safeguards → Cargo.toml → commit → tag
    Cargo,
    /// BYO versioning: print reminder, don't touch files
    External,
    /// No versioning: silent no-op
    None,
}

/// Prepared release — typestate token from preflight.
///
/// Per [[compiler-enforced-safety]]: `preflight()` returns this token,
/// `execute()` consumes it. Can't release without preflight — the
/// compiler enforces the protocol.
pub struct PreparedRelease {
    pub(crate) strategy: ReleaseStrategy,
    pub(crate) bump: BumpType,
    pub(crate) old_version: Option<String>,
    pub(crate) new_version: Option<String>,
}

// ============================================================================
// Public Interface
// ============================================================================

impl ReleaseStrategy {
    /// Detect release strategy from project configuration and files.
    ///
    /// Resolution chain:
    /// 1. Explicit config: `.patina/config.toml` `[versioning]` section
    /// 2. Auto-detect from project files (Cargo.toml, package.json, etc.)
    /// 3. Fork detection: Cargo.toml + upstream.owned=false → None
    pub fn from_project(project_path: &Path) -> Self {
        internal::detect_strategy(project_path)
    }

    /// Run preflight checks and return a prepared release token.
    ///
    /// `spec_path` is the file being released — it's expected to be dirty
    /// (we just updated its status) and will be included in the release commit.
    ///
    /// For `Cargo`: runs safeguard checks (clean tree, not behind remote,
    /// tag available, index fresh).
    /// For `External`: verifies version file exists.
    /// For `None`: no-op, returns token immediately.
    pub fn preflight(&self, bump: BumpType, spec_path: &str) -> Result<PreparedRelease> {
        internal::preflight(*self, bump, spec_path)
    }
}

impl PreparedRelease {
    /// Execute the release. Consumes self — can only be called once.
    ///
    /// For `Cargo`: updates Cargo.toml, commits, tags.
    /// For `External`: prints bump recommendation.
    /// For `None`: silent no-op.
    ///
    /// When `archive_dir` is provided, the spec directory is `git rm -r`'d
    /// and folded into the release commit (no separate archive commit needed).
    pub fn execute(self, title: &str, spec_path: &str, archive_dir: Option<&str>) -> Result<()> {
        internal::execute_release(self, title, spec_path, archive_dir)
    }
}
