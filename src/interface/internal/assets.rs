use std::fs;
use std::path::{Path, PathBuf};

const AGENTS_TEMPLATE: &str = include_str!("../../../resources/interfaces/code/AGENTS.md");
const CLAUDE_TEMPLATE: &str = include_str!("../../../resources/interfaces/code/CLAUDE.md");
const GEMINI_TEMPLATE: &str = include_str!("../../../resources/interfaces/code/GEMINI.md");

fn resource_root() -> PathBuf {
    if let Some(root) = std::env::var_os("PATINA_RESOURCE_ROOT") {
        return PathBuf::from(root);
    }
    Path::new(env!("CARGO_MANIFEST_DIR")).join("resources")
}

fn load_template(relative: &str, embedded: &str) -> String {
    let disk_path = resource_root().join(relative);
    fs::read_to_string(&disk_path).unwrap_or_else(|_| embedded.to_string())
}

pub fn canonical_agents_template() -> String {
    load_template("interfaces/code/AGENTS.md", AGENTS_TEMPLATE)
}

pub fn claude_shim_template() -> String {
    load_template("interfaces/code/CLAUDE.md", CLAUDE_TEMPLATE)
}

pub fn gemini_shim_template() -> String {
    load_template("interfaces/code/GEMINI.md", GEMINI_TEMPLATE)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embedded_templates_load() {
        assert!(canonical_agents_template().contains("{{platform}}"));
        assert!(claude_shim_template().contains("{{display_name}}"));
        assert!(gemini_shim_template().contains("{{display_name}}"));
    }
}
