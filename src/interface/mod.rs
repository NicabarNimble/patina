mod internal;
pub mod launch;
pub mod runtime;

use anyhow::Result;
use std::path::{Path, PathBuf};

use crate::session::InterfaceKind;

pub use internal::assets;
pub use internal::bootstrap::{
    bundle_deployment_status, ensure_bundle_bootstrap, ensure_bundle_projection,
    ensure_interface_bootstrap, ensure_interface_projection, BootstrapResult,
    BundleDeploymentStatus, ProjectionMode,
};
pub use internal::bundle::{
    builtin_registry_toml, interface_bundle, interface_bundle_catalog, is_supported_ai_interface,
    supported_ai_interfaces, BundleTmuxPolicy, InterfaceBundle,
};
pub use internal::checkin::{CheckInResult, InterfaceCapabilities};
pub use internal::launcher::{
    derive_interface_session_name, launch_interface_cli, teardown_interface_tmux_lane,
};
pub use internal::surface::{
    ensure_ai_project_config, ensure_ai_surface, prepare_ai_bundle, resolve_preferred_ai_interface,
    set_project_default_interface, AiProjectConfigResult, AiSurfaceRequest, AiSurfaceResult,
    PreparedInterface,
};
pub use runtime::claude::CLAUDE_INTERFACE_VERSION;
pub use runtime::gemini::GEMINI_INTERFACE_VERSION;
pub use runtime::templates;

#[derive(Debug, Clone)]
pub struct InterfaceDetection {
    pub detected: bool,
    pub display_name: String,
    pub version: Option<String>,
}

#[derive(Debug, Clone)]
pub struct LaunchRequest {
    pub project_root: PathBuf,
    pub env: Vec<(String, String)>,
    pub tmux_mode: TmuxLaunchMode,
    pub tmux_session_name: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TmuxLaunchMode {
    Auto,
    Force,
    Off,
}

pub trait AiInterface {
    fn name(&self) -> &'static str;
    fn display_name(&self) -> &'static str;
    fn interface_kind(&self) -> InterfaceKind;

    fn detect(&self) -> Result<InterfaceDetection> {
        let info = crate::interface::launch::get(self.name())?;
        Ok(InterfaceDetection {
            detected: info.detected,
            display_name: info.display,
            version: info.version,
        })
    }

    fn bootstrap(&self, project_root: &Path) -> Result<BootstrapResult> {
        ensure_bundle_bootstrap(self.name(), project_root)
    }

    fn context_file(&self, project_root: &Path) -> PathBuf;

    fn launch(&self, request: LaunchRequest) -> Result<()> {
        launch_interface_cli(
            self.name(),
            &request.project_root,
            request.tmux_mode,
            request.tmux_session_name.as_deref(),
            &request.env,
        )
    }

    fn capabilities(&self) -> InterfaceCapabilities {
        InterfaceCapabilities::default()
    }
}

pub fn interface(name: &str) -> Result<Box<dyn AiInterface>> {
    let bundle = interface_bundle(name)?;
    Ok(Box::new(RegistryBackedInterface { bundle }))
}

struct RegistryBackedInterface {
    bundle: &'static InterfaceBundle,
}

impl AiInterface for RegistryBackedInterface {
    fn name(&self) -> &'static str {
        self.bundle.name.as_str()
    }

    fn display_name(&self) -> &'static str {
        self.bundle.display_name.as_str()
    }

    fn interface_kind(&self) -> InterfaceKind {
        InterfaceKind::from_interface_name(&self.bundle.name)
    }

    fn context_file(&self, project_root: &Path) -> PathBuf {
        project_root.join("AGENTS.md")
    }
}
