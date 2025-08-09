// Dependable Rust: Black-box boundary for init command
// This is the ONLY public interface - everything else is hidden

use anyhow::Result;

/// Execute the init command
/// This is the only public function - all implementation is hidden
pub fn execute(name: String, llm: String, design: String, dev: Option<String>) -> Result<()> {
    implementation::execute_impl(name, llm, design, dev)
}

// Everything below here is private
mod implementation;
