use anyhow::Result;

/// Legacy shim for plugin-list command surface.
pub fn execute_list() -> Result<()> {
    super::child::execute_list()
}
