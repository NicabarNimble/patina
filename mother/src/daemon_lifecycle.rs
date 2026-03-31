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

fn process_is_alive(pid: i32) -> bool {
    let rc = unsafe { libc::kill(pid, 0) };
    if rc == 0 {
        return true;
    }

    let errno = std::io::Error::last_os_error()
        .raw_os_error()
        .unwrap_or_default();
    errno == libc::EPERM
}

fn remove_file_if_exists(path: &Path) -> Result<()> {
    match std::fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error.into()),
    }
}

pub fn reconcile_pid_state(pid_path: &Path, socket_path: &Path) -> Result<()> {
    if !pid_path.exists() {
        return Ok(());
    }

    let pid_text = std::fs::read_to_string(pid_path)
        .with_context(|| format!("reading PID file {}", pid_path.display()))?;
    let pid = pid_text
        .trim()
        .parse::<i32>()
        .with_context(|| format!("parsing PID from {}", pid_path.display()))?;

    if process_is_alive(pid) {
        anyhow::bail!(
            "Mother daemon already running (pid {}) — stop it first with `patina mother stop`",
            pid
        );
    }

    tracing::warn!(
        pid = pid,
        "stale pid file detected; cleaning up runtime files"
    );
    remove_file_if_exists(pid_path)
        .with_context(|| format!("removing stale PID file {}", pid_path.display()))?;
    remove_file_if_exists(socket_path)
        .with_context(|| format!("removing stale socket {}", socket_path.display()))?;
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stale_pid_cleans_pid_and_socket_files() {
        let temp = tempfile::tempdir().unwrap();
        let pid_path = temp.path().join("mother.pid");
        let socket_path = temp.path().join("serve.sock");
        std::fs::write(&pid_path, "999999").unwrap();
        std::fs::write(&socket_path, "stale").unwrap();

        reconcile_pid_state(&pid_path, &socket_path).unwrap();

        assert!(!pid_path.exists());
        assert!(!socket_path.exists());
    }

    #[test]
    fn live_pid_refuses_startup() {
        let temp = tempfile::tempdir().unwrap();
        let pid_path = temp.path().join("mother.pid");
        let socket_path = temp.path().join("serve.sock");
        std::fs::write(&pid_path, std::process::id().to_string()).unwrap();

        let error = reconcile_pid_state(&pid_path, &socket_path).unwrap_err();
        assert!(error.to_string().contains("already running"));
        assert!(pid_path.exists());
    }
}
