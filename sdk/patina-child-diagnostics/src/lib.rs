//! SDK-adjacent diagnostics for Patina child packages.
//!
//! This crate validates child package surfaces without linking into the main
//! `patina` binary. The first supported stage is local development: manifest
//! shape, WIT resolution, and WIT toy imports compared with `[needs].toys`.

use std::path::Path;

use anyhow::Result;

mod dev_config;
mod manifest;
pub mod report;
mod wit;

pub use dev_config::{
    check_children_dev_config, children_dev_config_path, load_children_dev_config,
    parse_children_dev_config, ChildDevConfig, ChildDevReport, ChildrenDevConfig,
    ChildrenDevReport, CHILDREN_DEV_CONFIG_RELATIVE_PATH, PATINA_DEV_COMPONENTS_RELATIVE_PATH,
    PATINA_DEV_RELATIVE_PATH,
};
pub use report::{
    DiagnosticFinding, DiagnosticPhase, DiagnosticReport, DiagnosticSeverity, DiagnosticStage,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CheckOptions {
    pub stage: DiagnosticStage,
}

impl Default for CheckOptions {
    fn default() -> Self {
        Self {
            stage: DiagnosticStage::LocalDev,
        }
    }
}

pub fn check_current_package() -> DiagnosticReport {
    check_local_dev(std::env::current_dir().expect("current directory is available"))
}

pub fn check_local_dev(root: impl AsRef<Path>) -> DiagnosticReport {
    check_package(root, CheckOptions::default())
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

fn run_checks(root: &Path, _options: &CheckOptions) -> Result<Vec<DiagnosticFinding>> {
    let (manifest, mut findings) = manifest::check_manifest(root)?;
    let declared_toys = manifest
        .as_ref()
        .map(|manifest| manifest.declared_toys.clone())
        .unwrap_or_default();
    findings.extend(wit::check_wit(root, &declared_toys)?);
    Ok(findings)
}
