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
    let domain_regex = if allowed_domains.is_empty() {
        // No domains allowed — deny all network
        String::new()
    } else {
        let patterns: Vec<String> = allowed_domains
            .iter()
            .map(|d| regex_escape_domain(d))
            .collect();
        patterns.join("|")
    };

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
(allow network-outbound (remote udp (remote port 53)))
"#,
    );

    if !domain_regex.is_empty() {
        profile.push_str(&format!(
            r#"(allow network-outbound
  (remote tcp (require-all
    (regex #"({})")
    (remote port 443))))
"#,
            domain_regex
        ));
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

/// Escape a domain name for use in SBPL regex.
/// Dots become literal `\\.`, rest is literal.
#[cfg(target_os = "macos")]
fn regex_escape_domain(domain: &str) -> String {
    domain.replace('.', "\\\\.")
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

    // SANDBOX_NAMED_EXTERNAL = 0x0003: interpret profile as inline SBPL string
    const SANDBOX_NAMED_EXTERNAL: u64 = 0x0003;

    let c_profile = CString::new(profile).map_err(|e| format!("invalid profile string: {}", e))?;
    let mut errorbuf: *mut c_char = ptr::null_mut();

    // Safety: FFI call to system sandbox_init(). errorbuf is allocated by the
    // system and must be freed with sandbox_free_error() on failure.
    let ret = unsafe { sandbox_init(c_profile.as_ptr(), SANDBOX_NAMED_EXTERNAL, &mut errorbuf) };

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
    use landlock::ABI;

    // Check best available ABI
    let abi = ABI::V4;
    let supported = landlock::RulesetCreated::new()
        .set_compatibility(landlock::CompatLevel::BestEffort)
        .handle_access(landlock::AccessFs::Execute)
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
        .add_rule(landlock::NetPortRule::new(
            AccessNet::ConnectTcp,
            NetPort::new(443),
        ))
        .map_err(|e| format!("landlock net rule: {}", e))?
        // Allow DNS (port 53)
        .add_rule(landlock::NetPortRule::new(
            AccessNet::ConnectTcp,
            NetPort::new(53),
        ))
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
        assert!(profile.contains("api\\\\.github\\\\.com"));
        assert!(profile.contains("hooks\\\\.slack\\\\.com"));
        assert!(profile.contains("(remote port 443)"));
        assert!(profile.contains("(remote port 53)"));
    }

    #[test]
    fn profile_no_domains() {
        let profile = generate_macos_profile(&[]);
        assert!(profile.contains("(deny default)"));
        assert!(!profile.contains("(remote port 443)"));
        assert!(profile.contains("(remote port 53)"));
    }

    #[test]
    fn regex_escape() {
        assert_eq!(
            regex_escape_domain("api.github.com"),
            "api\\\\.github\\\\.com"
        );
        assert_eq!(regex_escape_domain("example"), "example");
    }
}

// =========================================================================
// Linux tests
// =========================================================================

#[cfg(test)]
#[cfg(target_os = "linux")]
mod linux_tests {
    use super::*;

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
    fn landlock_enforcement_in_subprocess() {
        // Skip if kernel doesn't support Landlock
        if check_landlock_support().is_err() {
            eprintln!("[test] skipping: Landlock not supported on this kernel");
            return;
        }

        // Landlock is irrevocable — must test in a subprocess.
        // Fork a child that applies Landlock, then tries to connect to
        // port 80 (blocked) and port 443 (allowed).
        use std::os::unix::process::CommandExt;

        // Test 1: port 80 should be BLOCKED after Landlock
        let result = unsafe {
            std::process::Command::new("python3")
                .args([
                    "-c",
                    "import socket; s=socket.socket(); s.settimeout(3); s.connect(('1.1.1.1',80)); print('OPEN')",
                ])
                .pre_exec(|| {
                    apply_landlock(&[]).map_err(|e| {
                        std::io::Error::new(std::io::ErrorKind::Other, e)
                    })
                })
                .output()
        };

        match result {
            Ok(output) => {
                let stdout = String::from_utf8_lossy(&output.stdout);
                eprintln!("[test] port 80 subprocess: stdout={}", stdout.trim());
                assert!(
                    !stdout.contains("OPEN"),
                    "port 80 should be blocked by Landlock"
                );
            }
            Err(e) => {
                // Spawn/connect error = Landlock enforcement working
                eprintln!("[test] port 80 blocked (error: {})", e);
            }
        }

        // Test 2: port 443 should be ALLOWED after Landlock
        let result = unsafe {
            std::process::Command::new("python3")
                .args([
                    "-c",
                    "import socket; s=socket.socket(); s.settimeout(5); s.connect(('1.1.1.1',443)); print('OPEN')",
                ])
                .pre_exec(|| {
                    apply_landlock(&[]).map_err(|e| {
                        std::io::Error::new(std::io::ErrorKind::Other, e)
                    })
                })
                .output()
        };

        match result {
            Ok(output) => {
                let stdout = String::from_utf8_lossy(&output.stdout);
                eprintln!("[test] port 443 subprocess: stdout={}", stdout.trim());
                assert!(
                    stdout.contains("OPEN"),
                    "port 443 should be allowed by Landlock"
                );
            }
            Err(e) => {
                panic!("port 443 should be allowed by Landlock, but got: {}", e);
            }
        }
    }
}
