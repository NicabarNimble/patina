use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::report::{
    DiagnosticFinding, DiagnosticPhase, DiagnosticReport, DiagnosticSeverity, DiagnosticStage,
};
use crate::{check_package, CheckOptions};

pub const CHILDREN_DEV_CONFIG_RELATIVE_PATH: &str = ".patina/children-dev.toml";
pub const PATINA_DEV_RELATIVE_PATH: &str = ".patina/dev";
pub const PATINA_DEV_COMPONENTS_RELATIVE_PATH: &str = ".patina/dev/components";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ChildrenDevConfig {
    #[serde(default)]
    pub children: BTreeMap<String, ChildDevConfig>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ChildDevConfig {
    pub root: PathBuf,
    #[serde(default)]
    pub component: Option<PathBuf>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChildrenDevCheckOptions {
    pub stage: DiagnosticStage,
}

impl Default for ChildrenDevCheckOptions {
    fn default() -> Self {
        Self {
            stage: DiagnosticStage::LocalDev,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChildrenDevReport {
    pub repo_root: PathBuf,
    pub config_path: PathBuf,
    pub stage: DiagnosticStage,
    pub findings: Vec<DiagnosticFinding>,
    pub children: Vec<ChildDevReport>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChildDevReport {
    pub name: String,
    pub root: PathBuf,
    pub component: Option<PathBuf>,
    pub report: DiagnosticReport,
}

impl ChildrenDevReport {
    pub fn new(repo_root: PathBuf, config_path: PathBuf, stage: DiagnosticStage) -> Self {
        Self {
            repo_root,
            config_path,
            stage,
            findings: Vec::new(),
            children: Vec::new(),
        }
    }

    pub fn push(&mut self, finding: DiagnosticFinding) {
        self.findings.push(finding);
    }

    pub fn has_errors(&self) -> bool {
        self.findings
            .iter()
            .any(|finding| finding.severity == DiagnosticSeverity::Error)
            || self.children.iter().any(|child| child.report.has_errors())
    }

    pub fn is_ok(&self) -> bool {
        !self.has_errors()
    }

    pub fn assert_ok(&self) {
        if self.is_ok() {
            return;
        }

        panic!(
            "Patina children-dev diagnostics failed:\n{}",
            self.render_text()
        );
    }

    pub fn render_text(&self) -> String {
        let mut out = String::new();
        for finding in &self.findings {
            render_finding(&mut out, finding);
        }

        for child in &self.children {
            let rendered = child.report.render_text();
            if rendered.is_empty() {
                continue;
            }
            out.push_str(&format!("child[{}]:\n", child.name));
            out.push_str(&indent(&rendered));
        }

        out
    }
}

pub fn children_dev_config_path(repo_root: impl AsRef<Path>) -> PathBuf {
    repo_root.as_ref().join(CHILDREN_DEV_CONFIG_RELATIVE_PATH)
}

pub fn parse_children_dev_config(content: &str) -> Result<ChildrenDevConfig, toml::de::Error> {
    toml::from_str(content)
}

pub fn load_children_dev_config(
    repo_root: impl AsRef<Path>,
) -> Result<ChildrenDevConfig, std::io::Error> {
    let content = std::fs::read_to_string(children_dev_config_path(repo_root))?;
    parse_children_dev_config(&content)
        .map_err(|error| std::io::Error::new(std::io::ErrorKind::InvalidData, error))
}

pub fn check_children_dev_config(repo_root: impl AsRef<Path>) -> ChildrenDevReport {
    check_children_dev_config_with_options(repo_root, ChildrenDevCheckOptions::default())
}

pub fn check_children_dev_config_with_options(
    repo_root: impl AsRef<Path>,
    options: ChildrenDevCheckOptions,
) -> ChildrenDevReport {
    let repo_root = repo_root.as_ref().to_path_buf();
    let config_path = children_dev_config_path(&repo_root);
    let mut report = ChildrenDevReport::new(repo_root.clone(), config_path.clone(), options.stage);

    let content = match std::fs::read_to_string(&config_path) {
        Ok(content) => content,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            report.push(DiagnosticFinding::error(
                DiagnosticPhase::Manifest,
                "PTN-DEV-001",
                Some(config_path),
                "repo is missing .patina/children-dev.toml",
                Some(
                    "add .patina/children-dev.toml with [children.<name>] root entries for each child package"
                        .to_string(),
                ),
            ));
            return report;
        }
        Err(error) => {
            report.push(DiagnosticFinding::error(
                DiagnosticPhase::Manifest,
                "PTN-DEV-002",
                Some(config_path),
                "could not read .patina/children-dev.toml",
                Some(format!("fix file permissions or path access: {error}")),
            ));
            return report;
        }
    };

    let config = match parse_children_dev_config(&content) {
        Ok(config) => config,
        Err(error) => {
            report.push(DiagnosticFinding::error(
                DiagnosticPhase::Manifest,
                "PTN-DEV-003",
                Some(config_path),
                ".patina/children-dev.toml could not be parsed",
                Some(format!(
                    "fix TOML syntax and use [children.<name>] root/component entries: {error}"
                )),
            ));
            return report;
        }
    };

    if config.children.is_empty() {
        report.push(DiagnosticFinding::error(
            DiagnosticPhase::Manifest,
            "PTN-DEV-004",
            Some(config_path),
            ".patina/children-dev.toml declares no children",
            Some("add at least one [children.<name>] table with a root path".to_string()),
        ));
        return report;
    }

    for (name, child) in config.children {
        let child_root = resolve_repo_path(&repo_root, &child.root);
        let component = child
            .component
            .as_ref()
            .map(|component| resolve_repo_path(&repo_root, component));

        if !child_root.exists() {
            report.push(DiagnosticFinding::error(
                DiagnosticPhase::Manifest,
                "PTN-DEV-005",
                Some(child_root),
                format!("children-dev entry `{name}` root does not exist"),
                Some("set root to a child package directory containing child.toml".to_string()),
            ));
            continue;
        }

        if !child_root.is_dir() {
            report.push(DiagnosticFinding::error(
                DiagnosticPhase::Manifest,
                "PTN-DEV-006",
                Some(child_root),
                format!("children-dev entry `{name}` root is not a directory"),
                Some("set root to a child package directory containing child.toml".to_string()),
            ));
            continue;
        }

        if let Some(component) = &component {
            let patina_dev_root = repo_root.join(PATINA_DEV_RELATIVE_PATH);
            if !component.starts_with(&patina_dev_root) {
                report.push(DiagnosticFinding::warning(
                    DiagnosticPhase::Component,
                    "PTN-DEV-007",
                    Some(component.clone()),
                    format!(
                        "children-dev entry `{name}` component is outside repo-local .patina/dev/"
                    ),
                    Some(
                        "prefer .patina/dev/components/<child>.wasm so generated SDK artifacts are language-neutral and easy to clean"
                            .to_string(),
                    ),
                ));
            }
        }

        let child_report = check_package(
            &child_root,
            CheckOptions {
                stage: options.stage,
                component_path: component.clone(),
            },
        );
        report.children.push(ChildDevReport {
            name,
            root: child_root,
            component,
            report: child_report,
        });
    }

    report
}

pub fn check_children_dev_components(repo_root: impl AsRef<Path>) -> ChildrenDevReport {
    check_children_dev_config_with_options(
        repo_root,
        ChildrenDevCheckOptions {
            stage: DiagnosticStage::ComponentBuilt,
        },
    )
}

fn resolve_repo_path(repo_root: &Path, path: &Path) -> PathBuf {
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        repo_root.join(path)
    }
}

fn render_finding(out: &mut String, finding: &DiagnosticFinding) {
    let severity = match finding.severity {
        DiagnosticSeverity::Error => "error",
        DiagnosticSeverity::Warning => "warning",
        DiagnosticSeverity::Note => "note",
    };
    out.push_str(&format!(
        "{severity}[{}]: {}\n",
        finding.code, finding.message
    ));
    if let Some(location) = &finding.location {
        out.push_str(&format!("  --> {}\n", location.to_string_lossy()));
    }
    if let Some(help) = &finding.help {
        out.push_str(&format!("  help: {help}\n"));
    }
}

fn indent(text: &str) -> String {
    text.lines()
        .map(|line| format!("  {line}\n"))
        .collect::<String>()
}
