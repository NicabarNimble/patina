//! Tool installer module for Patina init command
//!
//! This module provides information about installable developer tools.
//! The actual installation functionality was temporarily removed during
//! the dependable-rust refactor (commit 6d9eca3) and will be restored
//! when the --install-tools flag is implemented.

/// Tool information for installation suggestions
#[derive(Debug)]
pub struct Tool {
    pub name: &'static str,
}

/// Get list of tools that Patina can help install
///
/// These are optional tools that enhance the Patina experience:
/// - docker: Container runtime for development environments
/// - go: Required for Dagger pipelines
/// - dagger: CI/CD pipeline engine
/// - gh: GitHub CLI for PR workflows
/// - jq: JSON processing for scripting
pub fn get_available_tools() -> Vec<Tool> {
    vec![
        Tool { name: "docker" },
        Tool { name: "go" },
        Tool { name: "dagger" },
        Tool { name: "gh" },
        Tool { name: "jq" },
    ]
}
