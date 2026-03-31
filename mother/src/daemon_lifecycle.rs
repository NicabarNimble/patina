use anyhow::{Context, Result};
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};

static SHUTDOWN_REQUESTED: AtomicBool = AtomicBool::new(false);

pub fn write_pid_file(pid_path: &Path) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;

    let pid = std::process::id();

    std::fs::write(pid_path, pid.to_string())
        .with_context(|| format!("writing PID file {}", pid_path.display()))?;

    std::fs::set_permissions(pid_path, std::fs::Permissions::from_mode(0o600))
        .with_context(|| format!("setting permissions on {}", pid_path.display()))?;

    Ok(())
}

pub fn register_signal_handlers() {
    SHUTDOWN_REQUESTED.store(false, Ordering::SeqCst);

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

pub fn shutdown_flag() -> &'static AtomicBool {
    &SHUTDOWN_REQUESTED
}

pub fn shutdown_requested() -> bool {
    SHUTDOWN_REQUESTED.load(Ordering::Relaxed)
}

extern "C" fn sigint_handler(_: libc::c_int) {
    SHUTDOWN_REQUESTED.store(true, Ordering::Relaxed);
}
