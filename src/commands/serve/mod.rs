//! Mother daemon for Patina
//!
//! Provides HTTP server for:
//! - Container queries to Mac mother
//! - Hot model caching (E5 embeddings)
//! - Cross-project knowledge access
//!
//! Design: Blocking HTTP microserver (no async/tokio)
//!
//! Transport model:
//! - Default: Unix domain socket at ~/.patina/run/serve.sock
//! - Opt-in: TCP at --host/--port (bearer token required)

mod internal;
pub(crate) mod microserver;

use anyhow::{bail, Context, Result};
use std::os::unix::fs::{FileTypeExt, PermissionsExt};
use std::path::Path;

use patina::paths;

/// Options for the serve command
pub struct ServeOptions {
    /// Host to bind to (None = UDS only, no TCP)
    pub host: Option<String>,
    /// Port to bind to (default: 50051)
    pub port: u16,
}

impl Default for ServeOptions {
    fn default() -> Self {
        Self {
            host: None,
            port: 50051,
        }
    }
}

/// Ensure the runtime directory exists with correct permissions.
///
/// Creates `~/.patina/run/` with 0o700 if it doesn't exist.
/// Refuses to start if the directory is world/group accessible.
fn ensure_run_dir() -> Result<()> {
    let run_dir = paths::serve::run_dir();

    if !run_dir.exists() {
        std::fs::create_dir_all(&run_dir)
            .with_context(|| format!("Failed to create {}", run_dir.display()))?;
        std::fs::set_permissions(&run_dir, std::fs::Permissions::from_mode(0o700))
            .with_context(|| format!("Failed to set permissions on {}", run_dir.display()))?;
    } else {
        // Verify permissions
        let meta = std::fs::metadata(&run_dir)?;
        let mode = meta.permissions().mode() & 0o777;
        if mode & 0o077 != 0 {
            bail!(
                "Refusing to start: {} has permissions {:o} (group/world accessible).\n  \
                 Fix with: chmod 700 {}",
                run_dir.display(),
                mode,
                run_dir.display()
            );
        }
    }

    Ok(())
}

/// Remove a stale socket file safely.
///
/// Only unlinks if the path is a socket AND owned by the current user.
/// Refuses to remove non-socket files or files owned by other users.
fn cleanup_stale_socket(socket_path: &Path) -> Result<()> {
    if !socket_path.exists() {
        return Ok(());
    }

    let meta = std::fs::symlink_metadata(socket_path)
        .with_context(|| format!("Failed to stat {}", socket_path.display()))?;

    // Must be a socket (not a regular file, symlink, etc)
    if !meta.file_type().is_socket() {
        bail!(
            "Refusing to start: {} exists but is not a socket.\n  \
             Remove manually if safe: rm {}",
            socket_path.display(),
            socket_path.display()
        );
    }

    // Must be owned by current user
    use std::os::unix::fs::MetadataExt;
    let file_uid = meta.uid();
    let my_uid = unsafe { libc::getuid() };
    if file_uid != my_uid {
        bail!(
            "Refusing to start: {} is owned by uid {} (you are {}).\n  \
             This may indicate a security issue.",
            socket_path.display(),
            file_uid,
            my_uid
        );
    }

    std::fs::remove_file(socket_path)
        .with_context(|| format!("Failed to remove stale socket {}", socket_path.display()))?;

    Ok(())
}

/// Set up the Unix domain socket for serving.
///
/// 1. Ensure ~/.patina/run/ exists with 0o700
/// 2. Clean up stale socket (safe unlink)
/// 3. Bind UnixListener
/// 4. Set socket to 0o600
pub fn setup_unix_listener() -> Result<std::os::unix::net::UnixListener> {
    use std::os::unix::net::UnixListener;

    ensure_run_dir()?;

    let socket_path = paths::serve::socket_path();
    cleanup_stale_socket(&socket_path)?;

    let listener = UnixListener::bind(&socket_path)
        .with_context(|| format!("Failed to bind {}", socket_path.display()))?;

    // Socket permissions: 0o600 (owner read/write only)
    std::fs::set_permissions(&socket_path, std::fs::Permissions::from_mode(0o600))
        .with_context(|| format!("Failed to set permissions on {}", socket_path.display()))?;

    Ok(listener)
}

/// Remove the socket file on clean shutdown.
pub fn cleanup_socket() {
    let socket_path = paths::serve::socket_path();
    if socket_path.exists() {
        let _ = std::fs::remove_file(&socket_path);
    }
}

/// Start the Mother daemon
pub fn execute(options: ServeOptions) -> Result<()> {
    internal::run_server(options)
}
