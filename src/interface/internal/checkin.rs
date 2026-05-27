use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct InterfaceCapabilities {
    pub bootstrap: bool,
    pub durable_sessions: bool,
}

impl Default for InterfaceCapabilities {
    fn default() -> Self {
        Self {
            bootstrap: true,
            durable_sessions: true,
        }
    }
}

#[derive(Debug, Clone)]
pub struct CheckInResult {
    pub voice_uid: Option<String>,
    pub session_runtime_id: String,
    pub session_file_id: String,
    pub artifact_path: PathBuf,
    pub attached_existing: bool,
}
