use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

static PID_PATH: OnceLock<PathBuf> = OnceLock::new();
static SOCKET_PATH: OnceLock<PathBuf> = OnceLock::new();

pub fn write_pid_file(pid_path: &Path) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;

    let pid = std::process::id();

    std::fs::write(pid_path, pid.to_string())
        .with_context(|| format!("writing PID file {}", pid_path.display()))?;

    std::fs::set_permissions(pid_path, std::fs::Permissions::from_mode(0o600))
        .with_context(|| format!("setting permissions on {}", pid_path.display()))?;

    Ok(())
}

pub fn register_signal_handlers(pid_path: PathBuf, socket_path: PathBuf) {
    let _ = PID_PATH.set(pid_path);
    let _ = SOCKET_PATH.set(socket_path);

    unsafe {
        libc::signal(
            libc::SIGINT,
            sigint_handler as *const () as libc::sighandler_t,
        );
        libc::signal(
            libc::SIGTERM,
            sigint_handler as *const () as libc::sighandler_t,
        );
    }
}

extern "C" fn sigint_handler(_: libc::c_int) {
    if let Some(pid_path) = PID_PATH.get() {
        let _ = std::fs::remove_file(pid_path);
    }
    if let Some(socket_path) = SOCKET_PATH.get() {
        let _ = std::fs::remove_file(socket_path);
    }
    std::process::exit(0);
}
