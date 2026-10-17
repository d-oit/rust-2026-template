<!-- AUTO-GENERATED — edit src/lib.rs in the crate, not this file -->

# Adversarial Secret-Detection Regression Corpus Pattern

Demonstrates how to validate security-sensitive string parsing and secret detection without importing heavy DLP implementations or external runtime dependencies.

## Motivation

Services processing untrusted inputs, log streams, or user submissions must prevent credential leakage. While full DLP engines bring heavy dependencies and complex rulesets, application developers need exact-boundary testing to verify that parser changes do not introduce secret leaks or false negatives.

Inspired by tools like `do-context-shield`, this pattern provides an adversarial regression corpus and lightweight detector engine supporting exact byte span assertions and non-leaking diagnostics.

## Core Design Principles

1. **Exact Match Spans & Byte Offsets**: Matches return exact byte offsets (`start_byte..end_byte`) pointing directly to secret characters within source text.
2. **Boundary & Suffix Character Rules**: Surrounding quotes (`"`, `'`), trailing delimiters (`;`, `,`, `)`, `]`), and whitespace are strictly excluded from matched secret spans.
3. **Negative Case Filtering**: Test suites include ordinary prose, variable names (`secret_count = 42`), short strings, and URLs to prevent overmatching or noisy false positives.
4. **Non-Leaking Diagnostics & Logging**: `SecretMatch` and error types implement custom `Debug` and `Display` traits that mask secrets (`sec_****3456`) and omit raw credential values from logs, panic output, and test reports.
5. **Fixture-Driven Regression Growth**: Every newly discovered leak or parser edge case is added to a versioned fixture corpus (`fixtures/regression.json`) alongside positive, negative, and boundary tests.
6. **Zero Heavy Dependencies**: The baseline detector uses standard library string operations and lightweight serialization without external DLP runtime overhead.

## Corpus Directory Structure

```
examples/security_secret_detector/
├── Cargo.toml
├── src/
│   ├── lib.rs
│   └── main.rs
├── fixtures/
│   ├── positive.json       # Confirmed secret assignments with exact byte spans
│   ├── negative.json       # Non-secret prose, numeric assignments, short strings
│   ├── boundary.json       # Length thresholds and suffix delimiter edge cases
│   └── regression.json     # Historical leaks and caught regression patterns
└── tests/
    └── corpus_test.rs      # Integration test suite validating fixtures & non-leakage
```

## Fixture Schema Example

```json
[
  {
    "id": "quoted-double-assignment",
    "input": "API_KEY = \"sec_1234567890123456\"",
    "expected_matches": [
      {
        "rule_id": "assignment-api_key",
        "start_byte": 11,
        "end_byte": 31,
        "line": 1,
        "column": 12,
        "masked_value": "sec_****3456"
      }
    ]
  }
]
```

## Testing Strategy

To guarantee that raw secrets are never printed in test output or logs, write assertions verifying stringified representations:

```rust
#[test]
fn test_raw_secrets_never_leaked_in_logs_or_debug() {
    let raw_secret = "sec_SUPER_SENSITIVE_LEAK_TOKEN_999";
    let input = format!("API_KEY = \"{raw_secret}\"");

    let detector = SecretDetector::default();
    let matches = detector.scan(&input);

    let m = &matches[0];
    let debug_str = format!("{m:?}");
    let display_str = format!("{m}");

    // Assert Debug and Display representations mask sensitive bytes
    assert!(!debug_str.contains(raw_secret));
    assert!(!display_str.contains(raw_secret));

    // Assert redacted output replaces raw secret
    let redacted = redact_text(&input, &matches);
    assert!(!redacted.contains(raw_secret));
    assert!(redacted.contains("[REDACTED:assignment-api_key]"));
}
```

See the full executable implementation, fixtures, and tests in `examples/security_secret_detector/`.
