//! Launcher functionality for interfaces
//!
//! Provides CLI detection, MCP configuration, and bootstrap generation
//! for launching AI interfaces. This complements the init-time functionality
//! in the interface runtime modules.
//!
//! # Example
//!
//! ```no_run
//! use patina::interface::launch;
//!
//! fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // List available interfaces
//!     let interfaces = launch::list()?;
//!     for f in &interfaces {
//!         println!("{}: {} (detected: {})", f.name, f.display, f.detected);
//!     }
//!
//!     // Get specific interface
//!     let claude = launch::get("claude")?;
//!     Ok(())
//! }
//! ```

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::interface::assets as interface_assets;
use crate::workspace;
use crate::Environment;

/// Available interface names
pub const INTERFACES: &[&str] = &["claude", "gemini", "opencode"];
/// Markers for Patina-managed section in bootstrap files
const MARKER_START: &str = "<!-- PATINA:START -->";
const MARKER_END: &str = "<!-- PATINA:END -->";
pub const CANONICAL_AGENTS_FILE: &str = "AGENTS.md";

// =============================================================================
// Types
// =============================================================================

/// Interface identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InterfaceKind {
    Claude,
    Gemini,
    OpenCode,
}

impl InterfaceKind {
    pub fn name(&self) -> &'static str {
        match self {
            InterfaceKind::Claude => "claude",
            InterfaceKind::Gemini => "gemini",
            InterfaceKind::OpenCode => "opencode",
        }
    }

    pub fn display(&self) -> &'static str {
        match self {
            InterfaceKind::Claude => "Claude Code",
            InterfaceKind::Gemini => "Gemini CLI",
            InterfaceKind::OpenCode => "OpenCode",
        }
    }

    pub fn from_name(name: &str) -> Option<Self> {
        match name.to_lowercase().as_str() {
            "claude" => Some(InterfaceKind::Claude),
            "gemini" => Some(InterfaceKind::Gemini),
            "opencode" => Some(InterfaceKind::OpenCode),
            _ => None,
        }
    }

    pub fn bootstrap_file(&self) -> &'static str {
        match self {
            InterfaceKind::Claude => "CLAUDE.md",
            InterfaceKind::Gemini => "GEMINI.md",
            InterfaceKind::OpenCode => CANONICAL_AGENTS_FILE,
        }
    }

    pub fn vendor_bootstrap_file(&self) -> Option<&'static str> {
        match self {
            InterfaceKind::Claude => Some("CLAUDE.md"),
            InterfaceKind::Gemini => Some("GEMINI.md"),
            InterfaceKind::OpenCode => None,
        }
    }

    pub fn detect_commands(&self) -> &'static [&'static str] {
        match self {
            InterfaceKind::Claude => &["claude --version"],
            InterfaceKind::Gemini => &["gemini --version"],
            InterfaceKind::OpenCode => &["opencode --version"],
        }
    }
}

/// Runtime interface info with detection status
#[derive(Debug, Clone)]
pub struct InterfaceInfo {
    pub name: String,
    pub display: String,
    pub detected: bool,
    pub version: Option<String>,
    pub mcp: Option<McpConfig>,
}

/// MCP configuration for an interface
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpConfig {
    pub config_path: String,
    pub config_format: String,
    #[serde(default)]
    pub config_template: Option<String>,
}

// =============================================================================
// Public API
// =============================================================================

/// List all available interfaces with detection status
pub fn list() -> Result<Vec<InterfaceInfo>> {
    let mut interfaces = Vec::new();

    for name in INTERFACES {
        if let Ok(info) = get(name) {
            interfaces.push(info);
        }
    }

    Ok(interfaces)
}

/// Get info for a specific interface
pub fn get(name: &str) -> Result<InterfaceInfo> {
    let interface = InterfaceKind::from_name(name)
        .ok_or_else(|| anyhow::anyhow!("Unknown interface: {}", name))?;

    let (detected, version) = detect_cli(&interface);

    Ok(InterfaceInfo {
        name: interface.name().to_string(),
        display: interface.display().to_string(),
        detected,
        version,
        mcp: get_mcp_config(&interface),
    })
}

/// Check if an interface CLI is available
pub fn is_available(name: &str) -> bool {
    get(name).map(|f| f.detected).unwrap_or(false)
}

/// Get the default interface name from global config
pub fn default_interface_name() -> Result<String> {
    let config = workspace::config()?;
    Ok(config.interface.default)
}

pub fn default_name() -> Result<String> {
    default_interface_name()
}

/// Set the default interface
pub fn set_default_interface(name: &str) -> Result<()> {
    let _ = InterfaceKind::from_name(name)
        .ok_or_else(|| anyhow::anyhow!("Unknown interface: {}", name))?;

    let mut config = workspace::config()?;
    config.interface.default = name.to_string();
    workspace::save_config(&config)?;

    Ok(())
}

pub fn set_default(name: &str) -> Result<()> {
    set_default_interface(name)
}

/// Generate bootstrap file for a project
///
/// Uses marker-based updates to preserve user content after Patina has taken ownership:
/// - If file doesn't exist: create a clean shell with a managed Patina section
/// - If file exists with markers: replace only content between markers
/// - If file exists without markers: replace with a clean shell (takeover should
///   already have been backed up by setup reconciliation)
pub fn canonical_agents_path(project_path: &Path) -> PathBuf {
    project_path.join(CANONICAL_AGENTS_FILE)
}

pub fn vendor_bootstrap_path(name: &str, project_path: &Path) -> Result<Option<PathBuf>> {
    let interface = InterfaceKind::from_name(name)
        .ok_or_else(|| anyhow::anyhow!("Unknown interface: {}", name))?;
    Ok(interface
        .vendor_bootstrap_file()
        .map(|file| project_path.join(file)))
}

pub fn generate_bootstrap(name: &str, project_path: &Path, force_rewrite: bool) -> Result<()> {
    let interface = InterfaceKind::from_name(name)
        .ok_or_else(|| anyhow::anyhow!("Unknown interface: {}", name))?;

    write_canonical_agents(project_path, force_rewrite)?;
    write_vendor_bootstrap(&interface, project_path, force_rewrite)?;

    Ok(())
}

/// Check if MCP is configured for an interface (patina server present in config)
pub fn is_mcp_configured(name: &str) -> Result<bool> {
    let info = get(name)?;

    let mcp = match info.mcp.as_ref() {
        Some(m) => m,
        None => return Ok(true), // No MCP config needed for this interface
    };

    let config_path = PathBuf::from(shellexpand::tilde(&mcp.config_path).as_ref());

    if !config_path.exists() {
        return Ok(false);
    }

    // Read config and check for patina server
    let content = fs::read_to_string(&config_path).unwrap_or_default();

    // Check if "patina" server is configured
    // For JSON configs, look for "patina" in mcpServers
    Ok(content.contains("\"patina\"") && content.contains("mcpServers"))
}

/// Truthful MCP availability for Patina AI interface projection.
///
/// This is intentionally conservative. If Patina cannot find a known interface
/// config with a configured Patina MCP server, projection should assume MCP is
/// unavailable and teach the JSON CLI fallback instead.
pub fn interface_mcp_available(name: &str) -> Result<bool> {
    let interface = InterfaceKind::from_name(name)
        .ok_or_else(|| anyhow::anyhow!("Unknown interface: {}", name))?;

    if let Some(config) = get_mcp_config(&interface) {
        return config_contains_patina_server(&config.config_path);
    }

    let candidates = match interface {
        InterfaceKind::Gemini => &[
            "~/.gemini/settings.json",
            "~/.config/gemini/settings.json",
            "~/.config/google-gemini/settings.json",
        ][..],
        InterfaceKind::OpenCode => &[
            "~/.config/opencode/config.json",
            "~/.config/opencode/opencode.json",
            "~/.config/opencode/config.toml",
        ][..],
        InterfaceKind::Claude => &[][..],
    };

    for path in candidates {
        if config_contains_patina_server(path)? {
            return Ok(true);
        }
    }

    Ok(false)
}

/// Configure MCP for an interface (update its config file)
pub fn configure_mcp(name: &str) -> Result<()> {
    anyhow::bail!(
        "MCP registration is retired in this runtime (interface: {}). Use daemon-first CLI flows: `patina mother start`, `patina context`, `patina scry`, `patina assay`.",
        name
    )
}

/// Detect CLI version for an interface
pub fn detect_version(name: &str) -> Option<String> {
    let interface = InterfaceKind::from_name(name)?;
    let (_, version) = detect_cli(&interface);
    version
}

/// Select an interface from available options.
///
/// Returns the chosen interface name.
///
/// Behavior:
/// - 0 available: Error with installation instructions
/// - 1 available: Returns it (no prompt)
/// - 2+ available: Prompts user to choose
///
/// If `preference` matches an available interface, it becomes the default selection.
/// This is used to honor the global config default without forcing it.
pub fn select_interface(available: &[InterfaceInfo], preference: Option<&str>) -> Result<String> {
    use std::io::{self, Write};

    match available.len() {
        0 => {
            anyhow::bail!(
                "No AI interfaces detected on this system.\n\
                 Install one of: {}",
                INTERFACES.join(", ")
            );
        }
        1 => {
            // Single interface - use it without prompting
            Ok(available[0].name.clone())
        }
        _ => {
            // Multiple interfaces - prompt user to choose
            println!("\n📱 Available interfaces:");

            // Find which index should be default (1-based for display)
            let default_idx = preference
                .and_then(|pref| available.iter().position(|a| a.name == pref))
                .map(|i| i + 1)
                .unwrap_or(1);

            for (i, interface) in available.iter().enumerate() {
                let num = i + 1;
                let default_marker = if num == default_idx { " (default)" } else { "" };
                println!("  [{}] {}{}", num, interface.display, default_marker);
            }

            print!("\nSelect interface [{}]: ", default_idx);
            io::stdout().flush()?;

            let mut input = String::new();
            io::stdin().read_line(&mut input)?;

            let choice = input.trim();
            let idx = if choice.is_empty() {
                default_idx
            } else {
                choice.parse::<usize>().unwrap_or(default_idx)
            };

            // Validate and return
            if idx >= 1 && idx <= available.len() {
                Ok(available[idx - 1].name.clone())
            } else {
                // Invalid input, use default
                Ok(available[default_idx - 1].name.clone())
            }
        }
    }
}

// =============================================================================
// Internal
// =============================================================================

/// Detect if CLI is installed and get version
fn detect_cli(interface: &InterfaceKind) -> (bool, Option<String>) {
    for cmd in interface.detect_commands() {
        if let Some(version) = try_command(cmd) {
            return (true, Some(version));
        }
    }
    (false, None)
}

/// Try running a command and capture version output
fn try_command(cmd: &str) -> Option<String> {
    let parts: Vec<&str> = cmd.split_whitespace().collect();
    if parts.is_empty() {
        return None;
    }

    let output = Command::new(parts[0]).args(&parts[1..]).output().ok()?;

    if output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        // Return first non-empty line as version
        stdout
            .lines()
            .chain(stderr.lines())
            .find(|l| !l.trim().is_empty())
            .map(|s| s.trim().to_string())
    } else {
        None
    }
}

/// Get MCP config for an interface
fn get_mcp_config(interface: &InterfaceKind) -> Option<McpConfig> {
    match interface {
        InterfaceKind::Claude => None,
        InterfaceKind::Gemini => None,   // TBD
        InterfaceKind::OpenCode => None, // TBD
    }
}

fn config_contains_patina_server(path: &str) -> Result<bool> {
    let config_path = PathBuf::from(shellexpand::tilde(path).as_ref());
    if !config_path.exists() {
        return Ok(false);
    }
    let content = fs::read_to_string(&config_path)
        .with_context(|| format!("Failed to read {}", config_path.display()))?;
    Ok(content.contains("patina") && content.contains("mcp"))
}

/// Update or append Patina section in existing content
fn update_or_append_section(content: &str, section: &str) -> String {
    if let (Some(start), Some(end)) = (content.find(MARKER_START), content.find(MARKER_END)) {
        if start < end {
            // Replace between markers
            let before = &content[..start];
            let after = &content[end + MARKER_END.len()..];
            return format!("{}{}{}", before, section, after);
        }
    }
    // Append (no markers or malformed)
    format!("{}\n\n{}", content.trim_end(), section)
}

fn managed_shell(title: &str, section: &str) -> String {
    format!(
        "# {title}\n\n\
Add project-specific notes outside the Patina block.\n\n\
{}\n",
        section
    )
}

fn render_template(template: &str, replacements: &[(&str, String)]) -> String {
    let mut rendered = template.to_string();
    for (needle, value) in replacements {
        rendered = rendered.replace(needle, value);
    }
    rendered
}

fn write_canonical_agents(project_path: &Path, force_rewrite: bool) -> Result<()> {
    let agents_path = canonical_agents_path(project_path);
    let environment = Environment::detect()?;
    let section = canonical_agents_section(&environment)?;

    let new_content = if agents_path.exists() && !force_rewrite {
        let content = fs::read_to_string(&agents_path)
            .with_context(|| format!("Failed to read {}", agents_path.display()))?;
        if content.contains(MARKER_START) && content.contains(MARKER_END) {
            update_or_append_section(&content, &section)
        } else {
            return Ok(());
        }
    } else {
        managed_shell("Agent Instructions", &section)
    };

    fs::write(&agents_path, new_content)
        .with_context(|| format!("Failed to write {}", agents_path.display()))?;
    Ok(())
}

fn write_vendor_bootstrap(
    interface: &InterfaceKind,
    project_path: &Path,
    force_rewrite: bool,
) -> Result<()> {
    let Some(bootstrap_path) = interface
        .vendor_bootstrap_file()
        .map(|file| project_path.join(file))
    else {
        return Ok(());
    };
    let section = vendor_shim_section(interface);

    let new_content = if bootstrap_path.exists() && !force_rewrite {
        let content = fs::read_to_string(&bootstrap_path)
            .with_context(|| format!("Failed to read {}", bootstrap_path.display()))?;
        if content.contains(MARKER_START) && content.contains(MARKER_END) {
            update_or_append_section(&content, &section)
        } else {
            managed_shell(&format!("{} Instructions", interface.display()), &section)
        }
    } else {
        managed_shell(&format!("{} Instructions", interface.display()), &section)
    };

    fs::write(&bootstrap_path, new_content)
        .with_context(|| format!("Failed to write {}", bootstrap_path.display()))?;
    Ok(())
}

fn canonical_agents_section(environment: &Environment) -> Result<String> {
    let tools = ["cargo", "git", "docker", "python"];
    let available: Vec<_> = tools
        .iter()
        .filter_map(|&tool| {
            environment
                .tools
                .get(tool)
                .filter(|info| info.available)
                .map(|_| tool)
        })
        .collect();

    let claude_mcp = interface_mcp_available("claude")?;
    let opencode_mcp = interface_mcp_available("opencode")?;
    let gemini_mcp = interface_mcp_available("gemini")?;

    let tools_line = if available.is_empty() {
        String::new()
    } else {
        format!("- **Tools**: {}\n", available.join(", "))
    };

    Ok(render_template(
        &interface_assets::canonical_agents_template(),
        &[
            (
                "{{platform}}",
                format!("{} ({})", environment.os, environment.arch),
            ),
            ("{{directory}}", environment.current_dir.clone()),
            ("{{tools_line}}", tools_line),
            (
                "{{claude_runtime}}",
                runtime_surface_section("Claude Code", "claude", claude_mcp),
            ),
            (
                "{{opencode_runtime}}",
                runtime_surface_section("OpenCode", "opencode", opencode_mcp),
            ),
            (
                "{{gemini_runtime}}",
                runtime_surface_section("Gemini CLI", "gemini", gemini_mcp),
            ),
        ],
    ))
}

fn runtime_surface_section(display: &str, adapter_name: &str, mcp_available: bool) -> String {
    let header = format!("### {display}\n\n");
    let wrapper_root = format!(".{adapter_name}/bin");
    let body = if mcp_available {
        format!(
            "Patina MCP is configured for this runtime.\n\n\
Use MCP directly for Patina workflow:\n\
- Discovery: `context`, `scry`, `assay`\n\
- Specs: `spec.next`, `spec.list`, `spec.show`, `spec.check`\n\n\
Session lifecycle stays on the bundled native wrapper path even when MCP is configured:\n\
- Sessions: `{wrapper_root}/session-start.sh \"<title>\"`\n\
- Sessions: `{wrapper_root}/session-update.sh`\n\
- Sessions: `{wrapper_root}/session-note.sh \"<note>\"`\n\
- Sessions: `{wrapper_root}/session-end.sh`\n"
        )
    } else {
        format!(
            "Patina MCP is not configured for this runtime. Do not assume `context`, `scry`, `assay`, `session.*`, or `spec.*` MCP tools exist here.\n\n\
Use CLI/native fallbacks instead:\n\
- Discovery: `patina context`, `patina scry`, `patina assay`\n\
- Specs: `patina spec show <id>`, `patina spec check <id> --json`, `patina spec next`\n\
- Sessions: `{wrapper_root}/session-start.sh \"<title>\"`\n\
- Sessions: `{wrapper_root}/session-update.sh`\n\
- Sessions: `{wrapper_root}/session-note.sh \"<note>\"`\n\
- Sessions: `{wrapper_root}/session-end.sh`\n"
        )
    };

    format!("{header}{body}")
}

fn vendor_shim_section(interface: &InterfaceKind) -> String {
    let template = match interface {
        InterfaceKind::Claude => interface_assets::claude_shim_template(),
        InterfaceKind::Gemini => interface_assets::gemini_shim_template(),
        InterfaceKind::OpenCode => return String::new(),
    };
    render_template(
        &template,
        &[("{{display_name}}", interface.display().to_string())],
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn with_temp_home<T>(f: impl FnOnce(&TempDir) -> T) -> T {
        let _guard = crate::test_support::env_test_mutex()
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        let temp = TempDir::new().unwrap();
        let old_home = std::env::var_os("HOME");
        unsafe {
            std::env::set_var("HOME", temp.path());
        }
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| f(&temp)));
        match old_home {
            Some(value) => unsafe {
                std::env::set_var("HOME", value);
            },
            None => unsafe {
                std::env::remove_var("HOME");
            },
        }
        match result {
            Ok(value) => value,
            Err(panic) => std::panic::resume_unwind(panic),
        }
    }

    #[test]
    fn test_interface_names() {
        assert_eq!(InterfaceKind::Claude.name(), "claude");
        assert_eq!(InterfaceKind::Gemini.name(), "gemini");
        assert_eq!(InterfaceKind::OpenCode.name(), "opencode");
    }

    #[test]
    fn test_interface_from_name() {
        assert_eq!(
            InterfaceKind::from_name("claude"),
            Some(InterfaceKind::Claude)
        );
        assert_eq!(
            InterfaceKind::from_name("CLAUDE"),
            Some(InterfaceKind::Claude)
        );
        assert_eq!(
            InterfaceKind::from_name("opencode"),
            Some(InterfaceKind::OpenCode)
        );
        assert_eq!(
            InterfaceKind::from_name("OpenCode"),
            Some(InterfaceKind::OpenCode)
        );
        assert_eq!(InterfaceKind::from_name("unknown"), None);
    }

    #[test]
    fn test_bootstrap_files() {
        assert_eq!(InterfaceKind::Claude.bootstrap_file(), "CLAUDE.md");
        assert_eq!(InterfaceKind::Gemini.bootstrap_file(), "GEMINI.md");
        assert_eq!(InterfaceKind::OpenCode.bootstrap_file(), "AGENTS.md");
        assert_eq!(
            InterfaceKind::Claude.vendor_bootstrap_file(),
            Some("CLAUDE.md")
        );
        assert_eq!(
            InterfaceKind::Gemini.vendor_bootstrap_file(),
            Some("GEMINI.md")
        );
        assert_eq!(InterfaceKind::OpenCode.vendor_bootstrap_file(), None);
    }

    #[test]
    fn canonical_agents_section_contains_runtime_specific_truth() {
        with_temp_home(|_| {
            let environment = Environment {
                os: "macos".to_string(),
                arch: "arm64".to_string(),
                home_dir: "/tmp".to_string(),
                current_dir: "/tmp/project".to_string(),
                tools: Default::default(),
                languages: Default::default(),
                env_vars: Default::default(),
            };
            let section = canonical_agents_section(&environment).unwrap();
            assert!(section.contains("Read `layer/` before non-trivial changes."));
            assert!(section.contains("Use `patina --help` to discover command surfaces."));
            assert!(section.contains("### Claude Code"));
            assert!(section.contains("### OpenCode"));
            assert!(section.contains("### Gemini CLI"));
            assert!(section.contains(
                "Do not assume MCP exists unless that runtime section says it is configured."
            ));
            assert!(!section.contains("session.start"));
            assert!(section.contains(".claude/bin/session-start.sh"));
            assert!(section.contains(".opencode/bin/session-start.sh"));
            assert!(section.contains(".gemini/bin/session-start.sh"));
            assert!(section.contains("session-note.sh"));
            assert!(section.contains("patina spec check <id> --json"));
        });
    }

    #[test]
    fn vendor_shim_points_to_root_agents() {
        let section = vendor_shim_section(&InterfaceKind::Gemini);
        assert!(section.contains("Read `AGENTS.md` first."));
        assert!(section.contains("compatibility shim"));
    }

    #[test]
    fn native_interface_mcp_availability_is_conservative_for_opencode() {
        with_temp_home(|temp| {
            let config_dir = temp.path().join(".config/opencode");
            fs::create_dir_all(&config_dir).unwrap();
            fs::write(
                config_dir.join("config.json"),
                r#"{"mcpServers":{"patina":{}}}"#,
            )
            .unwrap();

            assert!(interface_mcp_available("opencode").unwrap());
        });
    }

    #[test]
    fn test_interfaces_list() {
        assert!(INTERFACES.contains(&"claude"));
        assert!(INTERFACES.contains(&"gemini"));
        assert!(INTERFACES.contains(&"opencode"));
        assert_eq!(INTERFACES.len(), 3);
    }

    #[test]
    fn test_select_interface_zero_available() {
        let available: Vec<InterfaceInfo> = vec![];
        let result = select_interface(&available, None);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("No AI interfaces detected"));
    }

    #[test]
    fn test_select_interface_single_available() {
        let available = vec![InterfaceInfo {
            name: "claude".to_string(),
            display: "Claude Code".to_string(),
            detected: true,
            version: Some("1.0".to_string()),
            mcp: None,
        }];
        let result = select_interface(&available, None);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "claude");
    }

    #[test]
    fn test_select_interface_single_ignores_preference() {
        let available = vec![InterfaceInfo {
            name: "gemini".to_string(),
            display: "Gemini CLI".to_string(),
            detected: true,
            version: None,
            mcp: None,
        }];
        // Even with claude preference, returns the only available interface
        let result = select_interface(&available, Some("claude"));
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "gemini");
    }
}
