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
        "224.0.0.1",          // IPv4 Multicast
        "239.255.255.250",    // IPv4 Multicast (SSDP)
        "100.64.0.1",         // IPv4 CGNAT (RFC 6598)
        "ff02::1",            // IPv6 Multicast
        "2001:db8::1",        // IPv6 Documentation (RFC 3849)
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
fn test_domain_allowlist_trailing_dot_normalization() {
    let filter = EgressFilter {
        allowed_domains: vec!["example.com.".to_string(), "api.service.org".to_string()],
        redirect_policy: RedirectPolicy::Refuse,
        require_https: true,
    };

    let public_ip: IpAddr = "93.184.216.34".parse().unwrap();

    // Exact match without trailing dot
    assert!(
        filter
            .validate_request("https", "example.com", Some(public_ip))
            .is_ok()
    );

    // FQDN with trailing dot matching allowed domain with trailing dot
    assert!(
        filter
            .validate_request("https", "example.com.", Some(public_ip))
            .is_ok()
    );

    // Subdomain matching FQDN allowed domain
    assert!(
        filter
            .validate_request("https", "sub.example.com.", Some(public_ip))
            .is_ok()
    );

    // FQDN with trailing dot matching allowed domain without trailing dot
    assert!(
        filter
            .validate_request("https", "api.service.org.", Some(public_ip))
            .is_ok()
    );

    // Subdomain with trailing dot matching allowed domain without trailing dot
    assert!(
        filter
            .validate_request("https", "v1.api.service.org.", Some(public_ip))
            .is_ok()
    );
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
