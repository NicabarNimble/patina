//! SDK-adjacent diagnostics for Patina child packages.
//!
//! This crate validates child package surfaces without linking into the main
//! `patina` binary. The first supported stage is local development: manifest
//! shape, WIT resolution, and WIT toy imports compared with `[needs].toys`.

use std::path::{Path, PathBuf};

use anyhow::Result;

mod component;
mod dev_config;
mod manifest;
mod release;
pub mod report;
mod wit;

pub use dev_config::{
    check_children_dev_components, check_children_dev_config,
    check_children_dev_config_with_options, check_children_dev_release_candidates,
    children_dev_config_path, load_children_dev_config, parse_children_dev_config, ChildDevConfig,
    ChildDevReport, ChildrenDevCheckOptions, ChildrenDevConfig, ChildrenDevReport,
    CHILDREN_DEV_CONFIG_RELATIVE_PATH, PATINA_DEV_COMPONENTS_RELATIVE_PATH,
    PATINA_DEV_RELATIVE_PATH, PATINA_DEV_RELEASES_RELATIVE_PATH,
};
pub use report::{
    DiagnosticFinding, DiagnosticPhase, DiagnosticReport, DiagnosticSeverity, DiagnosticStage,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CheckOptions {
    pub stage: DiagnosticStage,
    pub component_path: Option<PathBuf>,
    pub release_path: Option<PathBuf>,
    pub release_tag: Option<String>,
}

impl CheckOptions {
    pub fn component_built(component_path: impl Into<PathBuf>) -> Self {
        Self {
            stage: DiagnosticStage::ComponentBuilt,
            component_path: Some(component_path.into()),
            release_path: None,
            release_tag: None,
        }
    }

    pub fn release_candidate(
        component_path: impl Into<PathBuf>,
        release_path: impl Into<PathBuf>,
    ) -> Self {
        Self {
            stage: DiagnosticStage::ReleaseCandidate,
            component_path: Some(component_path.into()),
            release_path: Some(release_path.into()),
            release_tag: None,
        }
    }

    pub fn with_release_tag(mut self, release_tag: impl Into<String>) -> Self {
        self.release_tag = Some(release_tag.into());
        self
    }
}

impl Default for CheckOptions {
    fn default() -> Self {
        Self {
            stage: DiagnosticStage::LocalDev,
            component_path: None,
            release_path: None,
            release_tag: None,
        }
    }
}

pub fn check_current_package() -> DiagnosticReport {
    check_local_dev(std::env::current_dir().expect("current directory is available"))
}

pub fn check_local_dev(root: impl AsRef<Path>) -> DiagnosticReport {
    check_package(root, CheckOptions::default())
}

pub fn check_component_built(
    root: impl AsRef<Path>,
    component_path: impl Into<PathBuf>,
) -> DiagnosticReport {
    check_package(root, CheckOptions::component_built(component_path))
}

pub fn check_release_candidate(
    root: impl AsRef<Path>,
    component_path: impl Into<PathBuf>,
    release_path: impl Into<PathBuf>,
) -> DiagnosticReport {
    check_package(
        root,
        CheckOptions::release_candidate(component_path, release_path),
    )
}

pub fn check_release_candidate_with_tag(
    root: impl AsRef<Path>,
    component_path: impl Into<PathBuf>,
    release_path: impl Into<PathBuf>,
    release_tag: impl Into<String>,
) -> DiagnosticReport {
    check_package(
        root,
        CheckOptions::release_candidate(component_path, release_path).with_release_tag(release_tag),
    )
}

pub fn check_package(root: impl AsRef<Path>, options: CheckOptions) -> DiagnosticReport {
    let root = root.as_ref();
    let package_root = root.to_path_buf();
    let mut report = DiagnosticReport::new(package_root, options.stage);

    match run_checks(root, &options) {
        Ok(findings) => report.extend(findings),
        Err(error) => report.push(DiagnosticFinding::error(
            DiagnosticPhase::Manifest,
            "PTN-DIAGNOSTIC-000",
            Some(root.to_path_buf()),
            "child diagnostics could not complete",
            Some(format!("{error:#}")),
        )),
    }

    report
}

fn run_checks(root: &Path, options: &CheckOptions) -> Result<Vec<DiagnosticFinding>> {
    let (manifest, mut findings) = manifest::check_manifest(root)?;
    let declared_toys = manifest
        .as_ref()
        .map(|manifest| manifest.declared_toys.clone())
        .unwrap_or_default();
    let (wit, wit_findings) = wit::inspect_wit(root, &declared_toys)?;
    findings.extend(wit_findings);

    if matches!(
        options.stage,
        DiagnosticStage::ComponentBuilt | DiagnosticStage::ReleaseCandidate
    ) {
        findings.extend(component::check_component(
            root,
            options.component_path.as_deref(),
            wit.as_ref(),
            &declared_toys,
        )?);
    }

    if options.stage == DiagnosticStage::ReleaseCandidate {
        findings.extend(release::check_release(
            root,
            options.release_path.as_deref(),
            manifest.as_ref(),
            options.component_path.as_deref(),
            options.release_tag.as_deref(),
        )?);
    }

    Ok(findings)
}
