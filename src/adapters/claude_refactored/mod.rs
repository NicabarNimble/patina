//! Claude Adapter Module - Black Box Implementation
//! 
//! This module provides Claude-specific LLM integration following Dependable Rust principles.
//! The public interface is kept minimal (<150 lines) while implementation details are hidden.
//!
//! Owner: TBD
//! Public API changes require owner approval.

use crate::adapters::LLMAdapter;

// Re-export only what's necessary
pub use self::versioning::CLAUDE_ADAPTER_VERSION;

// Hide implementation modules
mod implementation;
mod templates;
mod versioning;

/// Creates a new Claude adapter instance
/// 
/// This is the only way to create a ClaudeAdapter, ensuring the implementation
/// stays hidden behind the trait boundary.
pub fn create() -> Box<dyn LLMAdapter> {
    Box::new(implementation::ClaudeImpl::new())
}

/// Claude-specific capability information
#[derive(Debug, Clone)]
pub struct ClaudeCapability {
    pub version: &'static str,
    pub has_mcp_support: bool,
    pub has_session_commands: bool,
}

/// Get Claude adapter capabilities without instantiating
pub fn capability() -> ClaudeCapability {
    ClaudeCapability {
        version: CLAUDE_ADAPTER_VERSION,
        has_mcp_support: true,
        has_session_commands: true,
    }
}

// That's it! The entire public API in <50 lines.
// All the complexity is hidden in the implementation module.