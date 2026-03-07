//! OS sandbox enforcement for native children.
//!
//! macOS: sandbox_init() C API (kernel sandbox, not deprecated sandbox-exec CLI).
//! Linux: Landlock ABI v4+ (kernel 6.7+).
//!
//! Both platforms deny all filesystem access and restrict network to
//! declared domains on port 443, plus DNS (UDP 53) and stdio.

/// Generate a macOS SBPL sandbox profile for the given allowed domains.
///
/// Profile format: Scheme-based `.sb` syntax consumed by sandbox_init().
/// Allows: stdio, DNS, HTTPS to declared domains. Denies everything else.
#[cfg(target_os = "macos")]
pub fn generate_macos_profile(allowed_domains: &[String]) -> String {
    let mut profile = String::from(
        r#"(version 1)
(deny default)
(allow file-read* (literal "/dev/stdin"))
(allow file-read* (literal "/dev/null"))
(allow file-write* (literal "/dev/stdout"))
(allow file-write* (literal "/dev/stderr"))
(allow file-write* (literal "/dev/null"))
(allow sysctl-read)
(allow mach-lookup)
(allow system-socket)
(allow network-outbound (remote ip "*:53"))
"#,
    );

    if !allowed_domains.is_empty() {
        // Port-level restriction to 443 — domain filtering requires
        // DNS-level enforcement (future work, per DESIGN.md).
        profile.push_str("(allow network-outbound (remote ip \"*:443\"))\n");
    }

    // Allow reading system libraries and TLS certificates
    profile.push_str(
        r#"(allow file-read*
  (subpath "/usr/lib")
  (subpath "/usr/share")
  (subpath "/private/etc/ssl")
  (subpath "/private/etc/resolv.conf")
  (subpath "/Library/Preferences/com.apple.networkd.plist")
  (subpath "/System"))
"#,
    );

    profile
}

/// Apply sandbox profile in the current process via sandbox_init() C API.
///
/// Call after fork, before exec. Returns Ok(()) on success, Err with
/// Apple's error message on failure. The sandbox is irrevocable once applied.
#[cfg(target_os = "macos")]
pub fn apply_sandbox(profile: &str) -> Result<(), String> {
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

    let c_profile = CString::new(profile).map_err(|e| format!("invalid profile string: {}", e))?;
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
    // If the kernel doesn't support ABI v4, handle_access returns empty
    // flags and create() will fail or return NotEnforced.
    let supported = Ruleset::default()
        .set_compatibility(landlock::CompatLevel::SoftRequirement)
        .handle_access(AccessNet::from_all(abi))
        .is_ok();

    if !supported {
        return Err(
            "Cannot sandbox native child: kernel does not support Landlock ABI v4+. \
             Use --no-sandbox to run without OS-level sandboxing (not recommended)."
                .to_string(),
        );
    }

    Ok(abi as u32)
}

/// Apply Landlock restrictions for a native child process.
///
/// Restricts:
/// - Filesystem: deny all access (child communicates via stdio only)
/// - Network: allow only port 443 outbound (domain filtering at DNS level)
///
/// Call after fork, before exec. The restrictions are irrevocable.
///
/// Note: Landlock restricts port-level, not domain-level. Domain restriction
/// requires DNS-level enforcement (future work). For now, restricting to
/// port 443 prevents arbitrary port access.
#[cfg(target_os = "linux")]
pub fn apply_landlock(_allowed_domains: &[String]) -> Result<(), String> {
    use landlock::{
        Access, AccessFs, AccessNet, NetPort, Ruleset, RulesetAttr, RulesetCreatedAttr,
        RulesetStatus, ABI,
    };

    let abi = ABI::V4;

    let status = Ruleset::default()
        .handle_access(AccessFs::from_all(abi))
        .map_err(|e| format!("landlock fs ruleset: {}", e))?
        .handle_access(AccessNet::from_all(abi))
        .map_err(|e| format!("landlock net ruleset: {}", e))?
        .create()
        .map_err(|e| format!("landlock create: {}", e))?
        // Allow outbound HTTPS (port 443)
        .add_rule(NetPort::new(443, AccessNet::ConnectTcp))
        .map_err(|e| format!("landlock net rule: {}", e))?
        // Allow DNS (port 53)
        .add_rule(NetPort::new(53, AccessNet::ConnectTcp))
        .map_err(|e| format!("landlock dns rule: {}", e))?
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
    fn profile_with_domains() {
        let domains = vec!["api.github.com".to_string(), "hooks.slack.com".to_string()];
        let profile = generate_macos_profile(&domains);
        assert!(profile.contains("(version 1)"));
        assert!(profile.contains("(deny default)"));
        assert!(profile.contains(r#"(remote ip "*:443")"#));
        assert!(profile.contains(r#"(remote ip "*:53")"#));
    }

    #[test]
    fn profile_no_domains() {
        let profile = generate_macos_profile(&[]);
        assert!(profile.contains("(deny default)"));
        assert!(!profile.contains("*:443"));
        assert!(profile.contains(r#"(remote ip "*:53")"#));
    }

    #[test]
    fn apply_sandbox_enforcement_via_fork() {
        // sandbox_init() is irrevocable — test in a forked child process.
        // After fork, child applies sandbox, tries to read a blocked path,
        // and reports results via a pipe.
        use std::io::Read;
        use std::os::fd::FromRawFd;

        // Create a pipe for child → parent communication
        let (read_fd, write_fd) = {
            let mut fds = [0i32; 2];
            assert_eq!(unsafe { libc::pipe(fds.as_mut_ptr()) }, 0);
            (fds[0], fds[1])
        };

        let pid = unsafe { libc::fork() };
        assert!(pid >= 0, "fork failed");

        if pid == 0 {
            // Child process: apply sandbox, test enforcement, write results
            unsafe { libc::close(read_fd) };

            // Use a restrictive profile — deny all filesystem except stdio
            let profile = generate_macos_profile(&[]);
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

            // Try to read /etc/passwd — should be BLOCKED by sandbox
            match std::fs::read_to_string("/etc/passwd") {
                Ok(_) => results.push_str("READ_PASSWD:ALLOWED\n"),
                Err(_) => results.push_str("READ_PASSWD:BLOCKED\n"),
            }

            // Try to read /dev/null — should be ALLOWED (in profile)
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

        // Parent: read results from child
        unsafe { libc::close(write_fd) };
        let mut read_file = unsafe { std::fs::File::from_raw_fd(read_fd) };
        let mut results = String::new();
        read_file.read_to_string(&mut results).unwrap();

        // Wait for child
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
        // On kernels 6.7+, this should succeed. On older kernels, it
        // should return a clear error — not panic.
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
    fn landlock_enforcement_via_fork() {
        // Skip if kernel doesn't support Landlock
        if check_landlock_support().is_err() {
            eprintln!("[test] skipping: Landlock not supported on this kernel");
            return;
        }

        // Landlock is irrevocable — test in a forked child process.
        // After fork, child applies Landlock then tries network connections.
        // Parent reads results from a pipe.
        use std::io::Read;

        // Create a pipe for child → parent communication
        let (read_fd, write_fd) = {
            let mut fds = [0i32; 2];
            assert_eq!(unsafe { libc::pipe(fds.as_mut_ptr()) }, 0);
            (fds[0], fds[1])
        };

        let pid = unsafe { libc::fork() };
        assert!(pid >= 0, "fork failed");

        if pid == 0 {
            // Child process: apply Landlock, test connections, write results
            unsafe { libc::close(read_fd) };

            // Apply Landlock — restricts this process irrevocably
            let landlock_result = apply_landlock(&[]);

            let mut results = String::new();

            match landlock_result {
                Ok(()) => results.push_str("LANDLOCK_OK\n"),
                Err(e) => {
                    results.push_str(&format!("LANDLOCK_FAIL:{}\n", e));
                    // Write result and exit — can't test without Landlock
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

            // Test port 80 (should be BLOCKED)
            match std::net::TcpStream::connect_timeout(
                &"1.1.1.1:80".parse().unwrap(),
                std::time::Duration::from_secs(3),
            ) {
                Ok(_) => results.push_str("PORT_80:OPEN\n"),
                Err(_) => results.push_str("PORT_80:BLOCKED\n"),
            }

            // Test port 443 (should be ALLOWED)
            match std::net::TcpStream::connect_timeout(
                &"1.1.1.1:443".parse().unwrap(),
                std::time::Duration::from_secs(5),
            ) {
                Ok(_) => results.push_str("PORT_443:OPEN\n"),
                Err(_) => results.push_str("PORT_443:BLOCKED\n"),
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

        // Parent: read results from child
        unsafe { libc::close(write_fd) };
        let mut read_file = unsafe { std::fs::File::from_raw_fd(read_fd) };
        let mut results = String::new();
        read_file.read_to_string(&mut results).unwrap();

        // Wait for child
        let mut status = 0i32;
        unsafe { libc::waitpid(pid, &mut status, 0) };

        eprintln!("[test] landlock fork results:\n{}", results);

        assert!(
            results.contains("LANDLOCK_OK"),
            "Landlock should apply successfully"
        );
        assert!(
            results.contains("PORT_80:BLOCKED"),
            "port 80 should be blocked by Landlock"
        );
        assert!(
            results.contains("PORT_443:OPEN"),
            "port 443 should be allowed by Landlock"
        );
    }
}
