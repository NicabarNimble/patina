use anyhow::Result;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ManagedPathKind {
    File,
    Directory,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ManagedPathSpec {
    pub relative_path: &'static str,
    pub kind: ManagedPathKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InterfaceBundle {
    pub name: &'static str,
    pub display_name: &'static str,
    pub vendor_bootstrap: Option<&'static str>,
    pub managed_paths: &'static [ManagedPathSpec],
    pub tmux_policy: BundleTmuxPolicy,
    pub version: &'static str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BundleTmuxPolicy {
    Auto,
}

const OPENCODE_MANAGED_PATHS: &[ManagedPathSpec] = &[
    ManagedPathSpec {
        relative_path: "AGENTS.md",
        kind: ManagedPathKind::File,
    },
    ManagedPathSpec {
        relative_path: ".opencode",
        kind: ManagedPathKind::Directory,
    },
];

const CLAUDE_MANAGED_PATHS: &[ManagedPathSpec] = &[
    ManagedPathSpec {
        relative_path: "AGENTS.md",
        kind: ManagedPathKind::File,
    },
    ManagedPathSpec {
        relative_path: "CLAUDE.md",
        kind: ManagedPathKind::File,
    },
    ManagedPathSpec {
        relative_path: ".claude",
        kind: ManagedPathKind::Directory,
    },
];

const GEMINI_MANAGED_PATHS: &[ManagedPathSpec] = &[
    ManagedPathSpec {
        relative_path: "AGENTS.md",
        kind: ManagedPathKind::File,
    },
    ManagedPathSpec {
        relative_path: "GEMINI.md",
        kind: ManagedPathKind::File,
    },
    ManagedPathSpec {
        relative_path: ".gemini",
        kind: ManagedPathKind::Directory,
    },
];

const INTERFACE_BUNDLES: &[InterfaceBundle] = &[
    InterfaceBundle {
        name: "claude",
        display_name: "Claude Code",
        vendor_bootstrap: Some("CLAUDE.md"),
        managed_paths: CLAUDE_MANAGED_PATHS,
        tmux_policy: BundleTmuxPolicy::Auto,
        version: env!("CARGO_PKG_VERSION"),
    },
    InterfaceBundle {
        name: "opencode",
        display_name: "OpenCode",
        vendor_bootstrap: None,
        managed_paths: OPENCODE_MANAGED_PATHS,
        tmux_policy: BundleTmuxPolicy::Auto,
        version: env!("CARGO_PKG_VERSION"),
    },
    InterfaceBundle {
        name: "gemini",
        display_name: "Gemini CLI",
        vendor_bootstrap: Some("GEMINI.md"),
        managed_paths: GEMINI_MANAGED_PATHS,
        tmux_policy: BundleTmuxPolicy::Auto,
        version: env!("CARGO_PKG_VERSION"),
    },
];

pub fn interface_bundle_catalog() -> &'static [InterfaceBundle] {
    INTERFACE_BUNDLES
}

pub fn interface_bundle(name: &str) -> Result<&'static InterfaceBundle> {
    let normalized = name.trim().to_ascii_lowercase();
    interface_bundle_catalog()
        .iter()
        .find(|bundle| bundle.name == normalized)
        .ok_or_else(|| {
            anyhow::anyhow!(
                "Unsupported Patina AI interface '{}'. Choose one of: {}.",
                name,
                supported_ai_interfaces().join(", ")
            )
        })
}

pub fn supported_ai_interfaces() -> Vec<&'static str> {
    interface_bundle_catalog()
        .iter()
        .map(|bundle| bundle.name)
        .collect()
}

pub fn is_supported_ai_interface(name: &str) -> bool {
    interface_bundle(name).is_ok()
}

pub fn canonical_interface_name(name: &str) -> Option<&'static str> {
    let normalized = name.trim().to_ascii_lowercase();
    interface_bundle_catalog()
        .iter()
        .find(|bundle| bundle.name == normalized)
        .map(|bundle| bundle.name)
}

pub fn canonicalize_required_interface(name: &str) -> Result<&'static str> {
    canonical_interface_name(name).ok_or_else(|| {
        anyhow::anyhow!(
            "Unsupported Patina AI interface '{}'. Choose one of: {}.",
            name,
            supported_ai_interfaces().join(", ")
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bundle_catalog_lists_supported_interfaces() {
        let names = supported_ai_interfaces();
        assert_eq!(names, vec!["claude", "opencode", "gemini"]);
    }

    #[test]
    fn interface_resolution_is_case_insensitive() {
        assert_eq!(canonical_interface_name("Claude"), Some("claude"));
        assert_eq!(canonical_interface_name("OPENCODE"), Some("opencode"));
        assert_eq!(canonical_interface_name("Gemini"), Some("gemini"));
        assert!(canonical_interface_name("codex").is_none());
    }

    #[test]
    fn bundle_metadata_tracks_vendor_bootstrap_and_managed_paths() {
        let claude = interface_bundle("claude").unwrap();
        assert_eq!(claude.vendor_bootstrap, Some("CLAUDE.md"));
        assert!(claude
            .managed_paths
            .iter()
            .any(|path| path.relative_path == ".claude"));

        let opencode = interface_bundle("opencode").unwrap();
        assert_eq!(opencode.vendor_bootstrap, None);
        assert!(opencode
            .managed_paths
            .iter()
            .any(|path| path.relative_path == "AGENTS.md"));
    }
}
