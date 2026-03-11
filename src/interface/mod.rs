mod internal;

use anyhow::{bail, Result};
use std::path::{Path, PathBuf};

use crate::session::InterfaceKind;

pub use internal::bootstrap::{
    ensure_adapter_bootstrap, ensure_adapter_projection, BootstrapResult, ProjectionMode,
};
pub use internal::checkin::{
    check_in, CheckInResult, InterfaceCapabilities, InterfaceCheckIn, LaunchPolicy,
};
pub use internal::tmux::{
    check_tmux_version, derive_interface_session_name, derive_session_name, launch_adapter_cli,
    resolve_tmux_decision, OffReason, TmuxDecision,
};

#[derive(Debug, Clone)]
pub struct AdapterDetection {
    pub detected: bool,
    pub display_name: String,
    pub version: Option<String>,
}

#[derive(Debug, Clone)]
pub struct LaunchRequest {
    pub project_root: PathBuf,
    pub tmux_decision: TmuxDecision,
    pub session_name: String,
    pub env: Vec<(String, String)>,
}

pub trait AiAdapter {
    fn name(&self) -> &'static str;
    fn display_name(&self) -> &'static str;
    fn interface_kind(&self) -> InterfaceKind;

    fn detect(&self) -> Result<AdapterDetection> {
        let info = crate::adapters::launch::get(self.name())?;
        Ok(AdapterDetection {
            detected: info.detected,
            display_name: info.display,
            version: info.version,
        })
    }

    fn bootstrap(&self, project_root: &Path) -> Result<BootstrapResult> {
        ensure_adapter_bootstrap(self.name(), project_root)
    }

    fn context_file(&self, project_root: &Path) -> PathBuf;

    fn launch(&self, request: LaunchRequest) -> Result<()> {
        launch_adapter_cli(
            self.name(),
            &request.project_root,
            &request.tmux_decision,
            &request.session_name,
            &request.env,
        )
    }

    fn capabilities(&self) -> InterfaceCapabilities {
        InterfaceCapabilities::default()
    }
}

pub fn adapter(name: &str) -> Result<Box<dyn AiAdapter>> {
    match name {
        "opencode" => Ok(Box::new(OpenCodeInterface)),
        "gemini" => Ok(Box::new(GeminiInterface)),
        other => bail!("Unsupported ai adapter: {}", other),
    }
}

struct OpenCodeInterface;

impl AiAdapter for OpenCodeInterface {
    fn name(&self) -> &'static str {
        "opencode"
    }

    fn display_name(&self) -> &'static str {
        "OpenCode"
    }

    fn interface_kind(&self) -> InterfaceKind {
        InterfaceKind::OpenCode
    }

    fn context_file(&self, project_root: &Path) -> PathBuf {
        project_root.join("AGENTS.md")
    }
}

struct GeminiInterface;

impl AiAdapter for GeminiInterface {
    fn name(&self) -> &'static str {
        "gemini"
    }

    fn display_name(&self) -> &'static str {
        "Gemini CLI"
    }

    fn interface_kind(&self) -> InterfaceKind {
        InterfaceKind::Gemini
    }

    fn context_file(&self, project_root: &Path) -> PathBuf {
        project_root.join("AGENTS.md")
    }
}
