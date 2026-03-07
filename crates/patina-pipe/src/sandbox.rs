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

#[cfg(test)]
mod tests {
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
        // No network-outbound TCP rule when no domains
        assert!(!profile.contains("(remote port 443)"));
        // DNS still allowed
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
