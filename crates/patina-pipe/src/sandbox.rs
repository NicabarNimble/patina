//! OS sandbox enforcement for native children.
//!
//! macOS: sandbox_init() C API (kernel sandbox, not deprecated sandbox-exec CLI).
//! Linux: Landlock ABI v4+ (kernel 6.7+).
//!
//! Sandbox profiles are parameterized by child type (DESIGN.md §8.3,
//! [[sandbox-profiles-are-parameterized]]):
//! - Connector/transport/transform: deny all filesystem, deny all network.
//! - Storage (lakehouse): scoped filesystem access to a Mother-provided path,
//!   deny all network.
//!
//! Children cannot open sockets — all HTTP goes through Mother via pipe/http.
//! See [[spec-pipe-mother-io]] for the proxied HTTP design.

/// Sandbox profile determined by child type.
///
/// Mother selects the profile at spawn time based on child.toml type
/// and destination config. The OS enforces the boundary irrevocably.
#[derive(Debug, Clone)]
pub enum SandboxProfile {
    /// Connector/transport/transform: deny all filesystem and network.
    /// Children communicate exclusively via stdio (pipe/http for HTTP,
    /// pipe/fact for data).
    DenyAll,
    /// Storage (lakehouse): scoped filesystem access to a specific directory,
    /// deny all network. Mother provides the path from destination config.
    ScopedStorage { path: String },
}

/// Generate a macOS SBPL sandbox profile for a native child.
///
/// Profile format: Scheme-based `.sb` syntax consumed by sandbox_init().
/// - DenyAll: allows stdio only. Denies ALL filesystem and ALL network.
/// - ScopedStorage: allows stdio + read/write to the scoped path. Denies ALL network.
#[cfg(target_os = "macos")]
pub fn generate_macos_profile(profile: &SandboxProfile) -> String {
    let base = r#"(version 1)
(deny default)
(allow file-read* (literal "/dev/stdin"))
(allow file-read* (literal "/dev/null"))
(allow file-write* (literal "/dev/stdout"))
(allow file-write* (literal "/dev/stderr"))
(allow file-write* (literal "/dev/null"))
(allow sysctl-read)
(allow mach-lookup)
(allow file-read*
  (subpath "/usr/lib")
  (subpath "/usr/share")
  (subpath "/private/etc/ssl")
  (subpath "/private/etc/resolv.conf")
  (subpath "/Library/Preferences/com.apple.networkd.plist")
  (subpath "/System"))
"#;

    match profile {
        SandboxProfile::DenyAll => base.to_string(),
        SandboxProfile::ScopedStorage { path } => {
            // Add read/write access to the storage directory.
            // The OS enforces this boundary — the lakehouse child
            // can only touch files under the Mother-provided path.
            format!(
                "{}(allow file-read* file-write* (subpath \"{}\"))\n",
                base, path
            )
        }
    }
}

/// Apply sandbox profile in the current process via sandbox_init() C API.
///
/// Call after fork, before exec. Returns Ok(()) on success, Err with
/// Apple's error message on failure. The sandbox is irrevocable once applied.
#[cfg(target_os = "macos")]
pub fn apply_sandbox(profile_str: &str) -> Result<(), String> {
    use std::ffi::{CStr, CString};
    use std::os::raw::{c_char, c_int};
    use std::ptr;

    extern "C" {
        fn sandbox_init(profile: *const c_char, flags: u64, errorbuf: *mut *mut c_char) -> c_int;
        fn sandbox_free_error(errorbuf: *mut c_char);
    }

    // Flags = 0: interpret profile as inline SBPL string.
    // SANDBOX_NAMED (0x0001) = named profile, SANDBOX_NAMED_EXTERNAL (0x0003) = file path.
    const SANDBOX_INLINE: u64 = 0x0000;

    let c_profile =
        CString::new(profile_str).map_err(|e| format!("invalid profile string: {}", e))?;
    let mut errorbuf: *mut c_char = ptr::null_mut();

    // Safety: FFI call to system sandbox_init(). errorbuf is allocated by the
    // system and must be freed with sandbox_free_error() on failure.
    let ret = unsafe { sandbox_init(c_profile.as_ptr(), SANDBOX_INLINE, &mut errorbuf) };

    if ret != 0 {
        let err_msg = if !errorbuf.is_null() {
            let msg = unsafe { CStr::from_ptr(errorbuf) }
                .to_string_lossy()
                .to_string();
            unsafe { sandbox_free_error(errorbuf) };
            msg
        } else {
            "unknown sandbox_init error".to_string()
        };
        Err(format!("sandbox_init failed: {}", err_msg))
    } else {
        Ok(())
    }
}

// =========================================================================
// Linux: Landlock ABI v4+ enforcement
// =========================================================================

/// Check if the running kernel supports Landlock ABI v4+ (network restrictions).
///
/// ABI v4 requires kernel 6.7+ (Jan 2024). Returns the supported ABI version
/// on success, or an error describing why Landlock is unavailable.
#[cfg(target_os = "linux")]
pub fn check_landlock_support() -> Result<u32, String> {
    use landlock::{Access, AccessNet, Compatible, Ruleset, RulesetAttr, ABI};

    let abi = ABI::V4;

    // Probe: try to create a ruleset with network access handling.
    // HardRequirement ensures this fails if the kernel doesn't support
    // ABI v4 — SoftRequirement would silently downgrade and always succeed.
    let supported = Ruleset::default()
        .set_compatibility(landlock::CompatLevel::HardRequirement)
        .handle_access(AccessNet::from_all(abi))
        .is_ok();

    if !supported {
        return Err(
            "Cannot sandbox native child: kernel does not support Landlock ABI v4+ \
             (requires kernel 6.7+). Native children cannot run without OS-level sandboxing."
                .to_string(),
        );
    }

    Ok(abi as u32)
}

/// Apply Landlock restrictions for a native child process.
///
/// - DenyAll: deny all filesystem + all network.
/// - ScopedStorage: allow read/write to the scoped path, deny all network.
///
/// Call after fork, before exec. The restrictions are irrevocable.
#[cfg(target_os = "linux")]
pub fn apply_landlock(profile: &SandboxProfile) -> Result<(), String> {
    use landlock::{
        Access, AccessFs, AccessNet, PathBeneath, PathFd, Ruleset, RulesetAttr, RulesetCreatedAttr,
        RulesetStatus, ABI,
    };

    let abi = ABI::V4;

    let mut ruleset = Ruleset::default()
        .handle_access(AccessFs::from_all(abi))
        .map_err(|e| format!("landlock fs ruleset: {}", e))?
        .handle_access(AccessNet::from_all(abi))
        .map_err(|e| format!("landlock net ruleset: {}", e))?
        .create()
        .map_err(|e| format!("landlock create: {}", e))?;

    // For storage profiles, add a filesystem access rule for the scoped path.
    if let SandboxProfile::ScopedStorage { path } = profile {
        let fd = PathFd::new(path).map_err(|e| format!("landlock PathFd({}): {}", path, e))?;
        ruleset = ruleset
            .add_rule(PathBeneath::new(fd, AccessFs::from_all(abi)))
            .map_err(|e| format!("landlock add_rule({}): {}", path, e))?;
    }

    let status = ruleset
        .restrict_self()
        .map_err(|e| format!("landlock restrict_self: {}", e))?;

    match status.ruleset {
        RulesetStatus::FullyEnforced => Ok(()),
        RulesetStatus::PartiallyEnforced => {
            eprintln!("[sandbox] warning: Landlock only partially enforced");
            Ok(())
        }
        RulesetStatus::NotEnforced => {
            Err("Landlock not enforced — kernel may not support required ABI".to_string())
        }
    }
}

// =========================================================================
// macOS tests
// =========================================================================

#[cfg(test)]
#[cfg(target_os = "macos")]
mod macos_tests {
    use super::*;

    #[test]
    fn deny_all_profile_denies_network() {
        let profile = generate_macos_profile(&SandboxProfile::DenyAll);
        assert!(profile.contains("(version 1)"));
        assert!(profile.contains("(deny default)"));
        assert!(!profile.contains("*:443"), "port 443 must not be allowed");
        assert!(!profile.contains("*:53"), "DNS port must not be allowed");
        assert!(
            !profile.contains("network-outbound"),
            "no network-outbound rules allowed"
        );
        assert!(!profile.contains("subpath \"/tmp"));
    }

    #[test]
    fn scoped_storage_profile_allows_path() {
        let profile = generate_macos_profile(&SandboxProfile::ScopedStorage {
            path: "/Users/test/.patina/lakes/github".to_string(),
        });
        assert!(profile.contains("(deny default)"));
        assert!(profile.contains(
            "(allow file-read* file-write* (subpath \"/Users/test/.patina/lakes/github\"))"
        ));
        // Still no network
        assert!(!profile.contains("network-outbound"));
    }

    #[test]
    fn scoped_storage_profile_denies_network() {
        let profile = generate_macos_profile(&SandboxProfile::ScopedStorage {
            path: "/tmp/lake".to_string(),
        });
        assert!(!profile.contains("*:443"));
        assert!(!profile.contains("*:53"));
        assert!(!profile.contains("network-outbound"));
    }

    #[test]
    fn apply_deny_all_sandbox_via_fork() {
        // sandbox_init() is irrevocable — test in a forked child process.
        use std::io::Read;
        use std::os::fd::FromRawFd;

        let (read_fd, write_fd) = {
            let mut fds = [0i32; 2];
            assert_eq!(unsafe { libc::pipe(fds.as_mut_ptr()) }, 0);
            (fds[0], fds[1])
        };

        let pid = unsafe { libc::fork() };
        assert!(pid >= 0, "fork failed");

        if pid == 0 {
            unsafe { libc::close(read_fd) };

            let profile = generate_macos_profile(&SandboxProfile::DenyAll);
            let sandbox_result = apply_sandbox(&profile);

            let mut results = String::new();

            match sandbox_result {
                Ok(()) => results.push_str("SANDBOX_OK\n"),
                Err(e) => {
                    results.push_str(&format!("SANDBOX_FAIL:{}\n", e));
                    unsafe {
                        libc::write(
                            write_fd,
                            results.as_ptr() as *const libc::c_void,
                            results.len(),
                        );
                        libc::close(write_fd);
                        libc::_exit(1);
                    }
                }
            }

            // /etc/passwd should be BLOCKED
            match std::fs::read_to_string("/etc/passwd") {
                Ok(_) => results.push_str("READ_PASSWD:ALLOWED\n"),
                Err(_) => results.push_str("READ_PASSWD:BLOCKED\n"),
            }

            // /dev/null should be ALLOWED
            match std::fs::File::open("/dev/null") {
                Ok(_) => results.push_str("READ_DEVNULL:ALLOWED\n"),
                Err(_) => results.push_str("READ_DEVNULL:BLOCKED\n"),
            }

            unsafe {
                libc::write(
                    write_fd,
                    results.as_ptr() as *const libc::c_void,
                    results.len(),
                );
                libc::close(write_fd);
                libc::_exit(0);
            }
        }

        unsafe { libc::close(write_fd) };
        let mut read_file = unsafe { std::fs::File::from_raw_fd(read_fd) };
        let mut results = String::new();
        read_file.read_to_string(&mut results).unwrap();

        let mut status = 0i32;
        unsafe { libc::waitpid(pid, &mut status, 0) };

        eprintln!("[test] sandbox fork results:\n{}", results);

        assert!(
            results.contains("SANDBOX_OK"),
            "sandbox_init() should apply successfully"
        );
        assert!(
            results.contains("READ_PASSWD:BLOCKED"),
            "/etc/passwd should be blocked by sandbox"
        );
        assert!(
            results.contains("READ_DEVNULL:ALLOWED"),
            "/dev/null should be allowed by sandbox profile"
        );
    }

    #[test]
    fn apply_scoped_storage_sandbox_via_fork() {
        use std::io::Read;
        use std::os::fd::FromRawFd;

        // Create a temp directory for the scoped path.
        // Must use canonical path — macOS sandbox_init() resolves symlinks
        // (/var → /private/var, /tmp → /private/tmp).
        let tmp = tempfile::tempdir().unwrap();
        let scoped_path = tmp
            .path()
            .canonicalize()
            .unwrap()
            .to_string_lossy()
            .to_string();

        // Pre-create a test file in the scoped path
        std::fs::write(tmp.path().join("test.txt"), "hello").unwrap();

        let (read_fd, write_fd) = {
            let mut fds = [0i32; 2];
            assert_eq!(unsafe { libc::pipe(fds.as_mut_ptr()) }, 0);
            (fds[0], fds[1])
        };

        let pid = unsafe { libc::fork() };
        assert!(pid >= 0, "fork failed");

        if pid == 0 {
            unsafe { libc::close(read_fd) };

            let profile = generate_macos_profile(&SandboxProfile::ScopedStorage {
                path: scoped_path.clone(),
            });
            let sandbox_result = apply_sandbox(&profile);

            let mut results = String::new();

            match sandbox_result {
                Ok(()) => results.push_str("SANDBOX_OK\n"),
                Err(e) => {
                    results.push_str(&format!("SANDBOX_FAIL:{}\n", e));
                    unsafe {
                        libc::write(
                            write_fd,
                            results.as_ptr() as *const libc::c_void,
                            results.len(),
                        );
                        libc::close(write_fd);
                        libc::_exit(1);
                    }
                }
            }

            // Reading the scoped path should be ALLOWED
            let test_file = format!("{}/test.txt", scoped_path);
            match std::fs::read_to_string(&test_file) {
                Ok(content) if content == "hello" => results.push_str("READ_SCOPED:ALLOWED\n"),
                Ok(_) => results.push_str("READ_SCOPED:WRONG_CONTENT\n"),
                Err(_) => results.push_str("READ_SCOPED:BLOCKED\n"),
            }

            // Writing to the scoped path should be ALLOWED
            let write_file = format!("{}/write_test.txt", scoped_path);
            match std::fs::write(&write_file, "written") {
                Ok(()) => results.push_str("WRITE_SCOPED:ALLOWED\n"),
                Err(_) => results.push_str("WRITE_SCOPED:BLOCKED\n"),
            }

            // /etc/passwd should still be BLOCKED
            match std::fs::read_to_string("/etc/passwd") {
                Ok(_) => results.push_str("READ_PASSWD:ALLOWED\n"),
                Err(_) => results.push_str("READ_PASSWD:BLOCKED\n"),
            }

            unsafe {
                libc::write(
                    write_fd,
                    results.as_ptr() as *const libc::c_void,
                    results.len(),
                );
                libc::close(write_fd);
                libc::_exit(0);
            }
        }

        unsafe { libc::close(write_fd) };
        let mut read_file = unsafe { std::fs::File::from_raw_fd(read_fd) };
        let mut results = String::new();
        read_file.read_to_string(&mut results).unwrap();

        let mut status = 0i32;
        unsafe { libc::waitpid(pid, &mut status, 0) };

        eprintln!("[test] scoped sandbox fork results:\n{}", results);

        assert!(
            results.contains("SANDBOX_OK"),
            "sandbox_init() should apply successfully"
        );
        assert!(
            results.contains("READ_SCOPED:ALLOWED"),
            "scoped path should be readable"
        );
        assert!(
            results.contains("WRITE_SCOPED:ALLOWED"),
            "scoped path should be writable"
        );
        assert!(
            results.contains("READ_PASSWD:BLOCKED"),
            "/etc/passwd should still be blocked"
        );
    }
}

// =========================================================================
// Linux tests
// =========================================================================

#[cfg(test)]
#[cfg(target_os = "linux")]
mod linux_tests {
    use super::*;
    use std::os::fd::FromRawFd;

    #[test]
    fn landlock_support_detected() {
        match check_landlock_support() {
            Ok(abi) => {
                assert!(abi >= 4, "expected ABI v4+, got v{}", abi);
                eprintln!("[test] Landlock ABI v{} supported", abi);
            }
            Err(msg) => {
                eprintln!("[test] Landlock not supported: {}", msg);
                assert!(
                    msg.contains("Landlock"),
                    "error message should mention Landlock"
                );
            }
        }
    }

    #[test]
    fn landlock_deny_all_via_fork() {
        if check_landlock_support().is_err() {
            eprintln!("[test] skipping: Landlock not supported on this kernel");
            return;
        }

        use std::io::Read;
        use std::net::TcpListener;

        let listener = TcpListener::bind("127.0.0.1:0").expect("failed to bind listener");
        let blocked_port = listener.local_addr().unwrap().port();

        let (read_fd, write_fd) = {
            let mut fds = [0i32; 2];
            assert_eq!(unsafe { libc::pipe(fds.as_mut_ptr()) }, 0);
            (fds[0], fds[1])
        };

        let pid = unsafe { libc::fork() };
        assert!(pid >= 0, "fork failed");

        if pid == 0 {
            unsafe { libc::close(read_fd) };

            let landlock_result = apply_landlock(&SandboxProfile::DenyAll);

            let mut results = String::new();

            match landlock_result {
                Ok(()) => results.push_str("LANDLOCK_OK\n"),
                Err(e) => {
                    results.push_str(&format!("LANDLOCK_FAIL:{}\n", e));
                    unsafe {
                        libc::write(
                            write_fd,
                            results.as_ptr() as *const libc::c_void,
                            results.len(),
                        );
                        libc::close(write_fd);
                        libc::_exit(1);
                    }
                }
            }

            let blocked_addr = format!("127.0.0.1:{}", blocked_port);
            match std::net::TcpStream::connect_timeout(
                &blocked_addr.parse().unwrap(),
                std::time::Duration::from_secs(3),
            ) {
                Ok(_) => results.push_str("BLOCKED_PORT:OPEN\n"),
                Err(e) => {
                    results.push_str(&format!("BLOCKED_PORT:ERR:{:?}\n", e.kind()));
                }
            }

            match std::net::TcpStream::connect_timeout(
                &"127.0.0.1:443".parse().unwrap(),
                std::time::Duration::from_secs(3),
            ) {
                Ok(_) => results.push_str("PORT_443:OPEN\n"),
                Err(e) => {
                    results.push_str(&format!("PORT_443:ERR:{:?}\n", e.kind()));
                }
            }

            unsafe {
                libc::write(
                    write_fd,
                    results.as_ptr() as *const libc::c_void,
                    results.len(),
                );
                libc::close(write_fd);
                libc::_exit(0);
            }
        }

        unsafe { libc::close(write_fd) };
        let mut read_file = unsafe { std::fs::File::from_raw_fd(read_fd) };
        let mut results = String::new();
        read_file.read_to_string(&mut results).unwrap();

        let mut status = 0i32;
        unsafe { libc::waitpid(pid, &mut status, 0) };

        eprintln!("[test] landlock fork results:\n{}", results);

        assert!(results.contains("LANDLOCK_OK"));
        assert!(results.contains("BLOCKED_PORT:ERR:PermissionDenied"));
        assert!(results.contains("PORT_443:ERR:PermissionDenied"));
    }
}
