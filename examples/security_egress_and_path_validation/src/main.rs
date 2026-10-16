#![allow(missing_docs)]

//! Security Egress and Untrusted Path Validation Example.
//!
//! Demonstrates security boundary enforcement for HTTP egress requests and path validation:
//! 1. Normalizing IP representations (IPv4, IPv4-mapped IPv6, loopback, unspecified, link-local)
//!    before evaluating egress policy rules.
//! 2. Refusing or re-validating HTTP redirects so upstream redirects cannot bypass SSRF filters.
//! 3. Rejecting parent-directory traversal (`..`), absolute paths, and control/bidi characters
//!    in user-controlled filesystem paths.
//! 4. Validating effective configuration values before initiating network or filesystem side effects.
//! 5. Testing both allowed and denied cases at boundary conditions.
//!
//! # Security Limitation Notice
//!
//! **URL parsing and static IP checks alone DO NOT provide complete SSRF protection.**
//! - **DNS Rebinding:** A domain (e.g. `rebind.evil.com`) can resolve to a valid public IP during
//!   policy evaluation, but re-resolve to an internal IP (`127.0.0.1`, `169.254.169.254`) when the
//!   HTTP client opens the TCP socket.
//! - **TOCTOU Gap:** Any delay between hostname resolution and connection allows DNS cache expiration.
//! - **Complete SSRF Defense:** Complete mitigation requires **connection-time IP validation**
//!   (e.g., custom socket connectors or IP pinning) that verifies the target socket address immediately
//!   before sending bytes, in combination with explicit redirect controls.

use std::net::{IpAddr, Ipv6Addr};
use std::path::{Component, Path, PathBuf};

/// Errors encountered during egress policy evaluation.
#[derive(Debug, PartialEq, Eq, thiserror::Error)]
pub enum EgressError {
    /// URL scheme is not supported or prohibited.
    #[error("unsupported scheme: {0} (only HTTPS or explicitly allowed schemes permitted)")]
    UnsupportedScheme(String),

    /// Host is missing from the target URL.
    #[error("missing host in URL")]
    MissingHost,

    /// IP address is denied by policy (e.g. loopback, private, or link-local).
    #[error("denied IP address: {0}")]
    DeniedIp(IpAddr),

    /// Domain is not on the allowed destination list.
    #[error("domain not allowed: {0}")]
    DomainNotAllowed(String),

    /// Redirect was refused by policy.
    #[error("redirect refused: {0}")]
    RedirectRefused(String),

    /// Maximum redirect hop count exceeded.
    #[error("exceeded maximum redirect hops ({0})")]
    MaxRedirectsExceeded(usize),

    /// General URL parse error.
    #[error("invalid URL format: {0}")]
    InvalidUrl(String),
}

/// Policy for handling HTTP redirects during egress requests.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RedirectPolicy {
    /// Refuse all HTTP redirects.
    Refuse,
    /// Follow redirects only after re-validating the location URL against egress policy up to N hops.
    Revalidate {
        /// Maximum number of redirect hops allowed.
        max_hops: usize,
    },
}

/// Normalizes an `IpAddr` by converting IPv4-mapped IPv6 addresses (`::ffff:x.x.x.x`)
/// into their canonical `IpAddr::V4` representation.
#[must_use]
pub fn normalize_ip(ip: IpAddr) -> IpAddr {
    match ip {
        IpAddr::V4(v4) => IpAddr::V4(v4),
        IpAddr::V6(v6) => {
            if let Some(v4) = v6.to_ipv4_mapped() {
                IpAddr::V4(v4)
            } else {
                IpAddr::V6(v6)
            }
        }
    }
}

/// Checks if a normalized `IpAddr` is considered non-routable, loopback, unspecified, or link-local.
#[must_use]
pub fn is_restricted_ip(ip: IpAddr) -> bool {
    let normalized = normalize_ip(ip);
    match normalized {
        IpAddr::V4(v4) => {
            v4.is_loopback()            // 127.0.0.0/8
            || v4.is_unspecified()      // 0.0.0.0
            || v4.is_link_local()        // 169.254.0.0/16
            || v4.is_private()           // 10.0.0.0/8, 172.16.0.0/12, 192.168.0.0/16
            || v4.is_broadcast()         // 255.255.255.255
            || v4.is_documentation() // 192.0.2.0/24, 198.51.100.0/24, 203.0.113.0/24
        }
        IpAddr::V6(v6) => {
            v6.is_loopback()            // ::1
            || v6.is_unspecified()      // ::
            || is_v6_link_local(&v6)     // fe80::/10
            || is_v6_unique_local(&v6) // fc00::/7
        }
    }
}

/// Helper checking if an IPv6 address is in the unicast link-local range (`fe80::/10`).
fn is_v6_link_local(v6: &Ipv6Addr) -> bool {
    let segments = v6.segments();
    (segments[0] & 0xffc0) == 0xfe80
}

/// Helper checking if an IPv6 address is in the unique local address range (`fc00::/7`).
fn is_v6_unique_local(v6: &Ipv6Addr) -> bool {
    let segments = v6.segments();
    (segments[0] & 0xfe00) == 0xfc00
}

/// Evaluates whether an IP address is permitted under safe egress rules.
///
/// # Errors
///
/// Returns [`EgressError::DeniedIp`] if the address is loopback, unspecified,
/// link-local, or private.
pub fn validate_egress_ip(ip: IpAddr) -> Result<IpAddr, EgressError> {
    let normalized = normalize_ip(ip);
    if is_restricted_ip(normalized) {
        Err(EgressError::DeniedIp(normalized))
    } else {
        Ok(normalized)
    }
}

/// Safe Egress Filter configuration.
#[derive(Debug, Clone)]
pub struct EgressFilter {
    /// List of explicitly allowed hostnames/domains (if empty, any public host is permitted).
    pub allowed_domains: Vec<String>,
    /// Configured HTTP redirect policy.
    pub redirect_policy: RedirectPolicy,
    /// Whether HTTPS is strictly enforced.
    pub require_https: bool,
}

impl EgressFilter {
    /// Creates a default strict egress filter requiring HTTPS and refusing redirects.
    #[must_use]
    pub fn strict() -> Self {
        Self {
            allowed_domains: Vec::new(),
            redirect_policy: RedirectPolicy::Refuse,
            require_https: true,
        }
    }

    /// Validates a initial request target host and IP.
    ///
    /// # Errors
    ///
    /// Returns [`EgressError`] if the target fails protocol, host, IP, or domain checks.
    pub fn validate_request(
        &self,
        scheme: &str,
        host: &str,
        ip: Option<IpAddr>,
    ) -> Result<(), EgressError> {
        if self.require_https && !scheme.eq_ignore_ascii_case("https") {
            return Err(EgressError::UnsupportedScheme(scheme.to_string()));
        }

        if host.trim().is_empty() {
            return Err(EgressError::MissingHost);
        }

        // Domain allowlist check if configured
        if !self.allowed_domains.is_empty() {
            let host_lower = host.to_lowercase();
            let is_allowed = self.allowed_domains.iter().any(|domain| {
                let domain_lower = domain.to_lowercase();
                host_lower == domain_lower || host_lower.ends_with(&format!(".{domain_lower}"))
            });
            if !is_allowed {
                return Err(EgressError::DomainNotAllowed(host.to_string()));
            }
        }

        // IP address validation if resolved or explicitly passed
        if let Some(addr) = ip {
            validate_egress_ip(addr)?;
        }

        Ok(())
    }

    /// Evaluates a redirect response location header.
    ///
    /// # Errors
    ///
    /// Returns [`EgressError`] if redirects are refused, hop count is exceeded,
    /// or redirect location fails validation.
    pub fn validate_redirect(
        &self,
        current_hop: usize,
        location_scheme: &str,
        location_host: &str,
        location_ip: Option<IpAddr>,
    ) -> Result<(), EgressError> {
        match self.redirect_policy {
            RedirectPolicy::Refuse => Err(EgressError::RedirectRefused(format!(
                "{location_scheme}://{location_host}"
            ))),
            RedirectPolicy::Revalidate { max_hops } => {
                if current_hop >= max_hops {
                    return Err(EgressError::MaxRedirectsExceeded(max_hops));
                }
                // Re-validate the redirect target against all egress policy rules
                self.validate_request(location_scheme, location_host, location_ip)
            }
        }
    }
}

/// Errors encountered during path validation.
#[derive(Debug, PartialEq, Eq, thiserror::Error)]
pub enum PathValidationError {
    /// Path contains parent directory traversal (`..`).
    #[error("path traversal detected ('..')")]
    ParentTraversalDetected,

    /// Absolute path provided when a relative path was required.
    #[error("absolute path not allowed: {0}")]
    AbsolutePathNotAllowed(String),

    /// Path contains invalid control, zero-width, or Bidi override characters.
    #[error("path contains invalid or forbidden control characters")]
    InvalidCharacters,

    /// Target path escapes the designated base directory root.
    #[error("target path escapes base directory: {0}")]
    EscapesBaseDirectory(String),

    /// Path is empty or contains only whitespace.
    #[error("empty or blank path")]
    EmptyPath,
}

/// Validates user-controlled relative paths to prevent traversal and injection attacks.
pub struct UntrustedPathValidator;

impl UntrustedPathValidator {
    /// Validates an untrusted relative path input string.
    ///
    /// # Errors
    ///
    /// Returns [`PathValidationError`] if the input contains control characters,
    /// parent-directory traversal (`..`), or absolute path components.
    pub fn validate_relative_path(raw_path: &str) -> Result<PathBuf, PathValidationError> {
        if raw_path.trim().is_empty() {
            return Err(PathValidationError::EmptyPath);
        }

        // Check for control, zero-width, and Bidi control characters
        for ch in raw_path.chars() {
            if ch.is_control()
                || ('\u{200b}'..='\u{200f}').contains(&ch)
                || ('\u{202a}'..='\u{202e}').contains(&ch)
                || ('\u{2060}'..='\u{2064}').contains(&ch)
                || ('\u{2066}'..='\u{2069}').contains(&ch)
                || ch == '\u{2028}'
                || ch == '\u{2029}'
            {
                return Err(PathValidationError::InvalidCharacters);
            }
        }

        let path = Path::new(raw_path);

        if path.is_absolute() {
            return Err(PathValidationError::AbsolutePathNotAllowed(
                raw_path.to_string(),
            ));
        }

        for component in path.components() {
            match component {
                Component::ParentDir => return Err(PathValidationError::ParentTraversalDetected),
                Component::Prefix(_) | Component::RootDir => {
                    return Err(PathValidationError::AbsolutePathNotAllowed(
                        raw_path.to_string(),
                    ));
                }
                Component::CurDir | Component::Normal(_) => {}
            }
        }

        Ok(path.to_path_buf())
    }

    /// Safely resolves a relative subpath within an allowed base directory.
    ///
    /// # Errors
    ///
    /// Returns [`PathValidationError`] if path validation fails or if the resulting
    /// path escapes `base_dir`.
    pub fn resolve_under_base(
        base_dir: &Path,
        raw_path: &str,
    ) -> Result<PathBuf, PathValidationError> {
        let valid_relative = Self::validate_relative_path(raw_path)?;
        let combined = base_dir.join(&valid_relative);

        // Verify component-by-component containment
        if !combined.starts_with(base_dir) {
            return Err(PathValidationError::EscapesBaseDirectory(
                raw_path.to_string(),
            ));
        }

        Ok(combined)
    }
}

/// Errors during service configuration validation.
#[derive(Debug, PartialEq, Eq, thiserror::Error)]
pub enum ConfigValidationError {
    /// Storage root path is unsafe or invalid.
    #[error("invalid storage root: {0}")]
    InvalidStorageRoot(String),

    /// Egress timeout duration is invalid or exceeds safe boundary.
    #[error("timeout ({0}s) must be between 1 and 30 seconds")]
    InvalidTimeout(u64),

    /// Max redirect limit is invalid.
    #[error("max_redirects ({0}) exceeds maximum safe limit of 5")]
    InvalidMaxRedirects(usize),
}

/// Effective service configuration requiring validation before side effects.
#[derive(Debug, Clone)]
pub struct EffectiveConfig {
    /// Base directory for local file storage.
    pub storage_root: String,
    /// HTTP egress request timeout in seconds.
    pub timeout_seconds: u64,
    /// Egress filter settings.
    pub egress_filter: EgressFilter,
}

impl EffectiveConfig {
    /// Validates configuration fields before applying filesystem or network side effects.
    ///
    /// # Errors
    ///
    /// Returns [`ConfigValidationError`] if any field violates boundary safety constraints.
    pub fn validate_before_side_effects(&self) -> Result<(), ConfigValidationError> {
        // Validate storage root path format
        UntrustedPathValidator::validate_relative_path(&self.storage_root)
            .map_err(|e| ConfigValidationError::InvalidStorageRoot(e.to_string()))?;

        // Validate timeout boundaries
        if self.timeout_seconds < 1 || self.timeout_seconds > 30 {
            return Err(ConfigValidationError::InvalidTimeout(self.timeout_seconds));
        }

        // Validate redirect limits if revalidation is enabled
        if let RedirectPolicy::Revalidate { max_hops } = self.egress_filter.redirect_policy {
            if max_hops > 5 {
                return Err(ConfigValidationError::InvalidMaxRedirects(max_hops));
            }
        }

        Ok(())
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Security Egress and Untrusted Path Validation Example ===");

    // 1. IP Normalization and Egress Policy
    let loopback_v4: IpAddr = "127.0.0.1".parse()?;
    let mapped_loopback: IpAddr = "::ffff:127.0.0.1".parse()?;
    let public_v4: IpAddr = "93.184.216.34".parse()?;

    println!(
        "Normalizing ::ffff:127.0.0.1 -> {}",
        normalize_ip(mapped_loopback)
    );
    assert_eq!(normalize_ip(mapped_loopback), loopback_v4);

    let filter = EgressFilter::strict();
    println!(
        "Validating 93.184.216.34 -> {:?}",
        filter.validate_request("https", "example.com", Some(public_v4))
    );
    println!(
        "Validating 127.0.0.1 -> {:?}",
        filter.validate_request("https", "localhost", Some(loopback_v4))
    );

    // 2. Redirect Policy
    let redirect_err = filter.validate_redirect(0, "https", "example.com", Some(public_v4));
    println!("Redirect validation with Refuse policy -> {redirect_err:?}");

    // 3. Path Traversal Prevention
    let safe_path = "uploads/avatar.png";
    let bad_path = "uploads/../../etc/passwd";
    println!(
        "Path '{safe_path}' valid -> {:?}",
        UntrustedPathValidator::validate_relative_path(safe_path).is_ok()
    );
    println!(
        "Path '{bad_path}' valid -> {:?}",
        UntrustedPathValidator::validate_relative_path(bad_path).is_ok()
    );

    // 4. Config Validation
    let config = EffectiveConfig {
        storage_root: "var/data".to_string(),
        timeout_seconds: 10,
        egress_filter: filter,
    };
    config.validate_before_side_effects()?;
    println!("Effective configuration validated successfully before side effects.");

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ip_normalization() {
        let v4_loopback: IpAddr = "127.0.0.1".parse().unwrap();
        let mapped_v4_loopback: IpAddr = "::ffff:127.0.0.1".parse().unwrap();
        assert_eq!(normalize_ip(mapped_v4_loopback), v4_loopback);

        let v4_link_local: IpAddr = "169.254.169.254".parse().unwrap();
        let mapped_link_local: IpAddr = "::ffff:169.254.169.254".parse().unwrap();
        assert_eq!(normalize_ip(mapped_link_local), v4_link_local);

        let v6_loopback: IpAddr = "::1".parse().unwrap();
        assert_eq!(normalize_ip(v6_loopback), v6_loopback);
    }

    #[test]
    fn test_restricted_ip_denied_cases() {
        let denied_ips = [
            "127.0.0.1",          // IPv4 Loopback
            "0.0.0.0",            // IPv4 Unspecified
            "169.254.169.254",    // IPv4 Link-local (AWS metadata)
            "10.0.0.1",           // IPv4 Private
            "172.16.0.1",         // IPv4 Private
            "192.168.1.1",        // IPv4 Private
            "::ffff:127.0.0.1",   // IPv4-mapped IPv6 Loopback
            "::ffff:169.254.1.1", // IPv4-mapped IPv6 Link-local
            "::ffff:10.0.0.5",    // IPv4-mapped IPv6 Private
            "::1",                // IPv6 Loopback
            "::",                 // IPv6 Unspecified
            "fe80::1",            // IPv6 Link-local
            "fc00::1",            // IPv6 Unique Local
        ];

        for ip_str in denied_ips {
            let ip: IpAddr = ip_str.parse().unwrap();
            assert!(
                is_restricted_ip(ip),
                "IP {ip_str} should be detected as restricted"
            );
            assert!(
                matches!(validate_egress_ip(ip), Err(EgressError::DeniedIp(_))),
                "IP {ip_str} should be denied by egress validation"
            );
        }
    }

    #[test]
    fn test_allowed_public_ips() {
        let allowed_ips = [
            "93.184.216.34",                      // example.com public IPv4
            "1.1.1.1",                            // Cloudflare DNS IPv4
            "2606:2800:220:1:248:1893:25c8:1946", // example.com public IPv6
        ];

        for ip_str in allowed_ips {
            let ip: IpAddr = ip_str.parse().unwrap();
            assert!(
                !is_restricted_ip(ip),
                "Public IP {ip_str} should not be restricted"
            );
            assert_eq!(validate_egress_ip(ip), Ok(normalize_ip(ip)));
        }
    }

    #[test]
    fn test_redirect_policy_refuse() {
        let filter = EgressFilter::strict();
        let result = filter.validate_redirect(0, "https", "example.com", None);
        assert!(matches!(result, Err(EgressError::RedirectRefused(_))));
    }

    #[test]
    fn test_redirect_policy_revalidate() {
        let filter = EgressFilter {
            allowed_domains: vec!["example.com".to_string()],
            redirect_policy: RedirectPolicy::Revalidate { max_hops: 2 },
            require_https: true,
        };

        // Allowed hop 0 to valid public destination
        let public_ip: IpAddr = "93.184.216.34".parse().unwrap();
        assert!(
            filter
                .validate_redirect(0, "https", "example.com", Some(public_ip))
                .is_ok()
        );

        // Denied redirect to internal IP (SSRF attempt)
        let internal_ip: IpAddr = "169.254.169.254".parse().unwrap();
        assert!(matches!(
            filter.validate_redirect(1, "https", "example.com", Some(internal_ip)),
            Err(EgressError::DeniedIp(_))
        ));

        // Exceeded max hops
        assert!(matches!(
            filter.validate_redirect(2, "https", "example.com", Some(public_ip)),
            Err(EgressError::MaxRedirectsExceeded(2))
        ));
    }

    #[test]
    fn test_untrusted_path_validation() {
        // Allowed paths
        assert!(UntrustedPathValidator::validate_relative_path("foo/bar.txt").is_ok());
        assert!(UntrustedPathValidator::validate_relative_path("images/user_123.png").is_ok());

        // Denied: Parent directory traversal
        assert_eq!(
            UntrustedPathValidator::validate_relative_path("../etc/passwd"),
            Err(PathValidationError::ParentTraversalDetected)
        );
        assert_eq!(
            UntrustedPathValidator::validate_relative_path("a/b/../../c"),
            Err(PathValidationError::ParentTraversalDetected)
        );

        // Denied: Absolute path
        assert!(matches!(
            UntrustedPathValidator::validate_relative_path("/etc/passwd"),
            Err(PathValidationError::AbsolutePathNotAllowed(_))
        ));

        // Denied: Bidi control override
        assert_eq!(
            UntrustedPathValidator::validate_relative_path("file\u{202e}txt.exe"),
            Err(PathValidationError::InvalidCharacters)
        );

        // Denied: Null byte / control char
        assert_eq!(
            UntrustedPathValidator::validate_relative_path("file\0.txt"),
            Err(PathValidationError::InvalidCharacters)
        );
    }

    #[test]
    fn test_path_resolution_under_base() {
        let base = Path::new("/var/app/storage");
        let safe = UntrustedPathValidator::resolve_under_base(base, "user_data/file.json");
        assert_eq!(
            safe.unwrap(),
            Path::new("/var/app/storage/user_data/file.json")
        );

        let traversal = UntrustedPathValidator::resolve_under_base(base, "../storage/file.json");
        assert_eq!(traversal, Err(PathValidationError::ParentTraversalDetected));
    }

    #[test]
    fn test_effective_config_validation() {
        let filter = EgressFilter::strict();

        // Valid config
        let valid_cfg = EffectiveConfig {
            storage_root: "data/store".to_string(),
            timeout_seconds: 5,
            egress_filter: filter.clone(),
        };
        assert!(valid_cfg.validate_before_side_effects().is_ok());

        // Invalid storage path (parent traversal)
        let invalid_path_cfg = EffectiveConfig {
            storage_root: "../data/store".to_string(),
            timeout_seconds: 5,
            egress_filter: filter.clone(),
        };
        assert!(matches!(
            invalid_path_cfg.validate_before_side_effects(),
            Err(ConfigValidationError::InvalidStorageRoot(_))
        ));

        // Invalid timeout
        let invalid_timeout_cfg = EffectiveConfig {
            storage_root: "data/store".to_string(),
            timeout_seconds: 0,
            egress_filter: filter,
        };
        assert_eq!(
            invalid_timeout_cfg.validate_before_side_effects(),
            Err(ConfigValidationError::InvalidTimeout(0))
        );
    }
}
