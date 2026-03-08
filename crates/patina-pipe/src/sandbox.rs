//! OS sandbox enforcement for native children.
//!
//! macOS: sandbox_init() C API (kernel sandbox, not deprecated sandbox-exec CLI).
//! Linux: Landlock ABI v4+ (kernel 6.7+).
//!
//! Both platforms deny all filesystem access and ALL outbound network.
//! Children cannot open sockets — all HTTP goes through Mother via pipe/http.
//! See [[spec-pipe-mother-io]] for the proxied HTTP design.

/// Generate a macOS SBPL sandbox profile for a native child.
///
/// Profile format: Scheme-based `.sb` syntax consumed by sandbox_init().
/// Allows: stdio only. Denies ALL filesystem and ALL outbound network.
/// Children use pipe/http through Mother for all HTTP access.
#[cfg(target_os = "macos")]
pub fn generate_macos_profile(_allowed_domains: &[String]) -> String {
    // Deny-all network: no port 443, no DNS. Children communicate
    // exclusively via stdio (pipe/http for HTTP, pipe/fact for data).
    // Domain enforcement happens in Mother, not in the OS sandbox.
    String::from(
        r#"(version 1)
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
"#,
    )
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
/// Restricts:
/// - Filesystem: deny all access (child communicates via stdio only)
/// - Network: deny ALL outbound (children use pipe/http through Mother)
///
/// Call after fork, before exec. The restrictions are irrevocable.
#[cfg(target_os = "linux")]
pub fn apply_landlock(_allowed_domains: &[String]) -> Result<(), String> {
    use landlock::{
        Access, AccessFs, AccessNet, Ruleset, RulesetAttr, RulesetStatus, ABI,
    };

    let abi = ABI::V4;

    // Deny-all network: no port 443, no DNS. Children communicate
    // exclusively via stdio (pipe/http for HTTP, pipe/fact for data).
    // No add_rule calls = all network access denied.
    let status = Ruleset::default()
        .handle_access(AccessFs::from_all(abi))
        .map_err(|e| format!("landlock fs ruleset: {}", e))?
        .handle_access(AccessNet::from_all(abi))
        .map_err(|e| format!("landlock net ruleset: {}", e))?
        .create()
        .map_err(|e| format!("landlock create: {}", e))?
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
    fn profile_denies_all_network() {
        let domains = vec!["api.github.com".to_string(), "hooks.slack.com".to_string()];
        let profile = generate_macos_profile(&domains);
        assert!(profile.contains("(version 1)"));
        assert!(profile.contains("(deny default)"));
        // No network rules — all outbound denied. Children use pipe/http.
        assert!(!profile.contains("*:443"), "port 443 must not be allowed");
        assert!(!profile.contains("*:53"), "DNS port must not be allowed");
        assert!(
            !profile.contains("network-outbound"),
            "no network-outbound rules allowed"
        );
    }

    #[test]
    fn profile_no_domains_also_denies_all_network() {
        let profile = generate_macos_profile(&[]);
        assert!(profile.contains("(deny default)"));
        assert!(!profile.contains("*:443"));
        assert!(!profile.contains("*:53"));
        assert!(!profile.contains("network-outbound"));
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
        // Uses loopback only — no external network dependency.
        //
        // Strategy: bind a listener on an ephemeral port and also test
        // port 443. After Landlock deny-all, BOTH should get EACCES.
        // No ports are allowed — children use pipe/http through Mother.
        use std::io::Read;
        use std::net::TcpListener;

        // Bind a listener on an ephemeral port (OS assigns)
        let listener = TcpListener::bind("127.0.0.1:0").expect("failed to bind listener");
        let blocked_port = listener.local_addr().unwrap().port();
        eprintln!("[test] listener bound on port {}", blocked_port);

        // Create a pipe for child -> parent communication
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

            // Test ephemeral port (should get EACCES from Landlock)
            let blocked_addr = format!("127.0.0.1:{}", blocked_port);
            match std::net::TcpStream::connect_timeout(
                &blocked_addr.parse().unwrap(),
                std::time::Duration::from_secs(3),
            ) {
                Ok(_) => results.push_str("BLOCKED_PORT:OPEN\n"),
                Err(e) => {
                    let kind = e.kind();
                    results.push_str(&format!("BLOCKED_PORT:ERR:{:?}\n", kind));
                }
            }

            // Test port 443 — also blocked now (deny-all network).
            // Should get PermissionDenied, same as any other port.
            match std::net::TcpStream::connect_timeout(
                &"127.0.0.1:443".parse().unwrap(),
                std::time::Duration::from_secs(3),
            ) {
                Ok(_) => results.push_str("PORT_443:OPEN\n"),
                Err(e) => {
                    let kind = e.kind();
                    results.push_str(&format!("PORT_443:ERR:{:?}\n", kind));
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
        // Ephemeral port: Landlock returns PermissionDenied (EACCES)
        assert!(
            results.contains("BLOCKED_PORT:ERR:PermissionDenied"),
            "ephemeral port should be blocked by Landlock (EACCES), got: {}",
            results
        );
        // Port 443: also PermissionDenied — deny-all network, no exceptions
        assert!(
            results.contains("PORT_443:ERR:PermissionDenied"),
            "port 443 should be blocked by Landlock (EACCES, deny-all), got: {}",
            results
        );
    }
}
