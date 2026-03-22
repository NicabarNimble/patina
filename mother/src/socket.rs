use anyhow::{bail, Context, Result};
use std::os::unix::fs::{FileTypeExt, PermissionsExt};
use std::os::unix::net::UnixListener;
use std::path::Path;

pub fn setup_unix_listener(run_dir: &Path, socket_path: &Path) -> Result<UnixListener> {
    ensure_run_dir(run_dir)?;
    cleanup_stale_socket(socket_path)?;

    let listener = UnixListener::bind(socket_path)
        .with_context(|| format!("binding socket {}", socket_path.display()))?;

    std::fs::set_permissions(socket_path, std::fs::Permissions::from_mode(0o600))
        .with_context(|| format!("setting permissions on {}", socket_path.display()))?;

    Ok(listener)
}

pub fn cleanup_socket(socket_path: &Path) {
    if socket_path.exists() {
        let _ = std::fs::remove_file(socket_path);
    }
}

fn ensure_run_dir(run_dir: &Path) -> Result<()> {
    if !run_dir.exists() {
        std::fs::create_dir_all(run_dir)
            .with_context(|| format!("creating runtime directory {}", run_dir.display()))?;
        std::fs::set_permissions(run_dir, std::fs::Permissions::from_mode(0o700))
            .with_context(|| format!("setting permissions on {}", run_dir.display()))?;
    } else {
        let meta = std::fs::metadata(run_dir)
            .with_context(|| format!("reading metadata for {}", run_dir.display()))?;
        let mode = meta.permissions().mode() & 0o777;
        if mode & 0o077 != 0 {
            bail!(
                "Refusing to start: {} has permissions {:o} (group/world accessible).\n  Fix with: chmod 700 {}",
                run_dir.display(),
                mode,
                run_dir.display()
            );
        }
    }
    Ok(())
}

fn cleanup_stale_socket(socket_path: &Path) -> Result<()> {
    if !socket_path.exists() {
        return Ok(());
    }

    let meta = std::fs::symlink_metadata(socket_path)
        .with_context(|| format!("reading metadata for {}", socket_path.display()))?;

    if !meta.file_type().is_socket() {
        bail!(
            "Refusing to start: {} exists but is not a socket.\n  Remove manually if safe: rm {}",
            socket_path.display(),
            socket_path.display()
        );
    }

    use std::os::unix::fs::MetadataExt;
    let file_uid = meta.uid();
    let my_uid = unsafe { libc::getuid() };
    if file_uid != my_uid {
        bail!(
            "Refusing to start: {} is owned by uid {} (you are {}).\n  This may indicate a security issue.",
            socket_path.display(),
            file_uid,
            my_uid
        );
    }

    std::fs::remove_file(socket_path)
        .with_context(|| format!("removing stale socket {}", socket_path.display()))?;
    Ok(())
}
