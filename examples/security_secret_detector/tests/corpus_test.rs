//! Integration test suite for adversarial secret-detection corpus validation.

#![forbid(unsafe_code)]

use security_secret_detector::{DetectorError, SecretDetector, redact_text};
use serde::Deserialize;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Deserialize)]
struct PositiveFixture {
    id: String,
    input: String,
    expected_matches: Vec<ExpectedMatch>,
}

#[derive(Debug, Deserialize)]
struct ExpectedMatch {
    rule_id: String,
    start_byte: usize,
    end_byte: usize,
    line: usize,
    column: usize,
    masked_value: String,
}

#[derive(Debug, Deserialize)]
struct NegativeFixture {
    id: String,
    input: String,
}

#[derive(Debug, Deserialize)]
struct BoundaryFixture {
    id: String,
    input: String,
    should_match: bool,
    expected_span: Option<(usize, usize)>,
}

#[derive(Debug, Deserialize)]
struct RegressionFixture {
    id: String,
    description: String,
    input: String,
    expected_matches: Vec<ExpectedMatch>,
}

fn fixture_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("fixtures")
        .join(name)
}

#[test]
fn test_positive_corpus_exact_spans() {
    let path = fixture_path("positive.json");
    let content = fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("Failed to read fixture {}: {e}", path.display()));
    let fixtures: Vec<PositiveFixture> = serde_json::from_str(&content)
        .unwrap_or_else(|e| panic!("Failed to parse fixture JSON {}: {e}", path.display()));

    let detector = SecretDetector::default();

    for fix in fixtures {
        let matches = detector.scan(&fix.input);
        assert_eq!(
            matches.len(),
            fix.expected_matches.len(),
            "Fixture '{}' match count mismatch",
            fix.id
        );

        for (actual, expected) in matches.iter().zip(fix.expected_matches.iter()) {
            assert_eq!(
                actual.rule_id, expected.rule_id,
                "Fixture '{}' rule_id mismatch",
                fix.id
            );
            assert_eq!(
                actual.start_byte, expected.start_byte,
                "Fixture '{}' start_byte mismatch",
                fix.id
            );
            assert_eq!(
                actual.end_byte, expected.end_byte,
                "Fixture '{}' end_byte mismatch",
                fix.id
            );
            assert_eq!(
                actual.line, expected.line,
                "Fixture '{}' line mismatch",
                fix.id
            );
            assert_eq!(
                actual.column, expected.column,
                "Fixture '{}' column mismatch",
                fix.id
            );
            assert_eq!(
                actual.masked_value, expected.masked_value,
                "Fixture '{}' masked_value mismatch",
                fix.id
            );

            // Assert exact slice extraction from source text
            let extracted_span = &fix.input[actual.start_byte..actual.end_byte];
            assert!(
                !extracted_span.starts_with('"') && !extracted_span.starts_with('\''),
                "Extracted span in fixture '{}' should not include leading quotes: {extracted_span}",
                fix.id
            );
            assert!(
                !extracted_span.ends_with('"')
                    && !extracted_span.ends_with('\'')
                    && !extracted_span.ends_with(';')
                    && !extracted_span.ends_with(','),
                "Extracted span in fixture '{}' should not include trailing quotes/delimiters: {extracted_span}",
                fix.id
            );
        }
    }
}

#[test]
fn test_negative_corpus_no_overmatching() {
    let path = fixture_path("negative.json");
    let content = fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("Failed to read fixture {}: {e}", path.display()));
    let fixtures: Vec<NegativeFixture> = serde_json::from_str(&content)
        .unwrap_or_else(|e| panic!("Failed to parse fixture JSON {}: {e}", path.display()));

    let detector = SecretDetector::default();

    for fix in fixtures {
        let matches = detector.scan(&fix.input);
        let fix_id = &fix.id;
        assert!(
            matches.is_empty(),
            "Negative fixture '{fix_id}' produced unexpected match(es): {matches:?}"
        );
    }
}

#[test]
fn test_boundary_lengths_and_suffixes() {
    let path = fixture_path("boundary.json");
    let content = fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("Failed to read fixture {}: {e}", path.display()));
    let fixtures: Vec<BoundaryFixture> = serde_json::from_str(&content)
        .unwrap_or_else(|e| panic!("Failed to parse fixture JSON {}: {e}", path.display()));

    let detector = SecretDetector::default();

    for fix in fixtures {
        let matches = detector.scan(&fix.input);
        let fix_id = &fix.id;
        if fix.should_match {
            assert!(
                !matches.is_empty(),
                "Boundary fixture '{fix_id}' expected a match but found none"
            );
            let m = &matches[0];
            if let Some((exp_start, exp_end)) = fix.expected_span {
                assert_eq!(
                    (m.start_byte, m.end_byte),
                    (exp_start, exp_end),
                    "Boundary fixture '{fix_id}' exact span mismatch"
                );
            }
        } else {
            assert!(
                matches.is_empty(),
                "Boundary fixture '{fix_id}' expected no match but found: {matches:?}"
            );
        }
    }
}

#[test]
fn test_regression_fixture_caught() {
    let path = fixture_path("regression.json");
    let content = fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("Failed to read fixture {}: {e}", path.display()));
    let fixtures: Vec<RegressionFixture> = serde_json::from_str(&content)
        .unwrap_or_else(|e| panic!("Failed to parse fixture JSON {}: {e}", path.display()));

    let detector = SecretDetector::default();

    for fix in fixtures {
        let matches = detector.scan(&fix.input);
        assert_eq!(
            matches.len(),
            fix.expected_matches.len(),
            "Regression fixture '{}' ({}) match count mismatch",
            fix.id,
            fix.description
        );

        for (actual, expected) in matches.iter().zip(fix.expected_matches.iter()) {
            assert_eq!(
                actual.rule_id, expected.rule_id,
                "Regression fixture '{}' rule_id mismatch",
                fix.id
            );
            assert_eq!(
                actual.start_byte, expected.start_byte,
                "Regression fixture '{}' start_byte mismatch",
                fix.id
            );
            assert_eq!(
                actual.end_byte, expected.end_byte,
                "Regression fixture '{}' end_byte mismatch",
                fix.id
            );
            assert_eq!(
                actual.line, expected.line,
                "Regression fixture '{}' line mismatch",
                fix.id
            );
            assert_eq!(
                actual.column, expected.column,
                "Regression fixture '{}' column mismatch",
                fix.id
            );

            let extracted_secret = &fix.input[actual.start_byte..actual.end_byte];
            assert_eq!(
                extracted_secret, "sec_regression_token_9999",
                "Regression fixture '{}' extracted secret exact value mismatch",
                fix.id
            );
        }
    }
}

#[test]
fn test_raw_secrets_never_leaked_in_logs_or_debug() {
    let raw_secret = "sec_SUPER_SENSITIVE_LEAK_TOKEN_999";
    let input = format!("API_KEY = \"{raw_secret}\"");

    let detector = SecretDetector::default();
    let matches = detector.scan(&input);

    assert_eq!(matches.len(), 1);
    let m = &matches[0];

    // Assert Debug and Display output formats do not leak the raw secret payload
    let debug_str = format!("{m:?}");
    let display_str = format!("{m}");

    assert!(
        !debug_str.contains(raw_secret),
        "SecretMatch Debug representation leaked raw secret string: {debug_str}"
    );
    assert!(
        !display_str.contains(raw_secret),
        "SecretMatch Display representation leaked raw secret string: {display_str}"
    );

    // Assert DetectorError formatting does not leak sensitive information
    let err = DetectorError::CorpusError("Scan error occurred".to_string());
    let err_debug = format!("{err:?}");
    let err_display = format!("{err}");
    assert!(!err_debug.contains(raw_secret));
    assert!(!err_display.contains(raw_secret));

    // Assert redacted text replaces raw secret string with masked marker
    let redacted = redact_text(&input, &matches);
    assert!(
        !redacted.contains(raw_secret),
        "Redacted output leaked raw secret string: {redacted}"
    );
    assert!(redacted.contains("[REDACTED:assignment-api_key]"));
}
