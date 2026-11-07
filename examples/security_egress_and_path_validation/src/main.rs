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

pub mod path_validator;

use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

pub use path_validator::{PathValidationError, UntrustedPathValidator};

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
/// and deprecated IPv4-compatible IPv6 addresses (`::x.x.x.x`) into their canonical
/// `IpAddr::V4` representation.
#[must_use]
pub fn normalize_ip(ip: IpAddr) -> IpAddr {
    let canonical = ip.to_canonical();
    if let IpAddr::V6(v6) = canonical {
        // Detect deprecated IPv4-compatible IPv6 addresses (::x.x.x.x for x.x.x.x != 0.0.0.0 and != 0.0.0.1)
        let segments = v6.segments();
        if segments[0] == 0
            && segments[1] == 0
            && segments[2] == 0
            && segments[3] == 0
            && segments[4] == 0
            && segments[5] == 0
            && !(segments[6] == 0 && (segments[7] == 0 || segments[7] == 1))
        {
            let octets = v6.octets();
            return IpAddr::V4(Ipv4Addr::new(
                octets[12], octets[13], octets[14], octets[15],
            ));
        }
    }
    canonical
}

/// Checks if a normalized `IpAddr` is considered non-routable, loopback, unspecified, link-local,
/// or an IPv6 transition mechanism tunneling to a restricted IPv4 address.
#[must_use]
pub fn is_restricted_ip(ip: IpAddr) -> bool {
    let normalized = normalize_ip(ip);
    match normalized {
        IpAddr::V4(v4) => is_restricted_ipv4(&v4),
        IpAddr::V6(v6) => {
            v6.is_loopback()            // ::1
            || v6.is_unspecified()      // ::
            || v6.is_multicast()        // ff00::/8
            || is_v6_link_local(&v6)     // fe80::/10
            || is_v6_unique_local(&v6)   // fc00::/7
            || is_v6_documentation(&v6) // 2001:db8::/32 (RFC 3849)
            || is_v6_discard(&v6)       // 100::/64 Discard-only prefix (RFC 6666)
            || is_v6_transition_restricted(&v6) // 6to4 (2002::/16) or Teredo (2001:0::/32) tunneling restricted IPv4
        }
    }
}

/// Helper checking if an IPv4 address is restricted/non-routable/private.
fn is_restricted_ipv4(v4: &Ipv4Addr) -> bool {
    v4.is_loopback()            // 127.0.0.0/8
    || v4.is_unspecified()      // 0.0.0.0
    || v4.is_link_local()        // 169.254.0.0/16
    || v4.is_private()           // 10.0.0.0/8, 172.16.0.0/12, 192.168.0.0/16
    || v4.is_broadcast()         // 255.255.255.255
    || v4.is_documentation()     // 192.0.2.0/24, 198.51.100.0/24, 203.0.113.0/24
    || v4.is_multicast()         // 224.0.0.0/4
    || is_v4_shared(v4)          // 100.64.0.0/10 Carrier-Grade NAT (RFC 6598)
    || is_v4_benchmarking(v4)    // 198.18.0.0/15 Network interconnect benchmark (RFC 2544)
    || is_v4_reserved(v4) // 240.0.0.0/4 Reserved for future use (RFC 1112)
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

/// Helper checking if an IPv4 address is in the shared/CGNAT address space (`100.64.0.0/10`).
fn is_v4_shared(v4: &std::net::Ipv4Addr) -> bool {
    let octets = v4.octets();
    octets[0] == 100 && (octets[1] & 0xc0) == 64
}

/// Helper checking if an IPv6 address is in the documentation range (`2001:db8::/32`).
fn is_v6_documentation(v6: &Ipv6Addr) -> bool {
    let segments = v6.segments();
    segments[0] == 0x2001 && segments[1] == 0x0db8
}

/// Helper checking if an IPv6 address is in the discard-only prefix (`100::/64`, RFC 6666).
fn is_v6_discard(v6: &Ipv6Addr) -> bool {
    let segments = v6.segments();
    segments[0] == 0x0100 && segments[1] == 0 && segments[2] == 0 && segments[3] == 0
}

/// Helper checking if an IPv4 address is in the benchmarking range (`198.18.0.0/15`, RFC 2544).
fn is_v4_benchmarking(v4: &Ipv4Addr) -> bool {
    let octets = v4.octets();
    octets[0] == 198 && (octets[1] & 0xfe) == 18
}

/// Helper checking if an IPv4 address is in the reserved/future use range (`240.0.0.0/4`, RFC 1112).
fn is_v4_reserved(v4: &Ipv4Addr) -> bool {
    let octets = v4.octets();
    (octets[0] & 0xf0) == 240
}

/// Checks whether an IPv6 address uses 6to4 (`2002::/16`) or Teredo (`2001:0::/32`) tunneling
/// whose embedded destination IPv4 address is in a restricted range.
fn is_v6_transition_restricted(v6: &Ipv6Addr) -> bool {
    let segments = v6.segments();

    // 6to4 tunneling (2002::/16, RFC 3056): segments[1] and segments[2] encode the IPv4 address
    if segments[0] == 0x2002 {
        let v4 = Ipv4Addr::new(
            (segments[1] >> 8) as u8,
            segments[1] as u8,
            (segments[2] >> 8) as u8,
            segments[2] as u8,
        );
        return is_restricted_ipv4(&v4);
    }

    // Teredo tunneling (2001:0000::/32, RFC 4380): segments[6] and segments[7] contain the client's public IPv4 address XORed with 0xFFFF
    if segments[0] == 0x2001 && segments[1] == 0 {
        let v4 = Ipv4Addr::new(
            (!segments[6] >> 8) as u8,
            (!segments[6]) as u8,
            (!segments[7] >> 8) as u8,
            (!segments[7]) as u8,
        );
        return is_restricted_ipv4(&v4);
    }

    false
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
            // Strip optional trailing dot for canonical FQDN comparison
            let host_norm = host.trim_end_matches('.').to_lowercase();
            let is_allowed = self.allowed_domains.iter().any(|domain| {
                let domain_norm = domain.trim_end_matches('.').to_lowercase();
                if host_norm == domain_norm {
                    return true;
                }
                // Zero-allocation subdomain match replacing format!(".{domain_norm}")
                host_norm
                    .strip_suffix(&domain_norm)
                    .is_some_and(|prefix| prefix.ends_with('.'))
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
mod tests;
