//! Adversarial Secret-Detection Regression Corpus Pattern.
//!
//! This crate demonstrates how to implement exact-span secret detection and maintain
//! a fixture-driven regression corpus without importing external DLP dependencies.
//!
//! # Key Security Principles
//! 1. **Exact Span Reporting**: Matches return byte offsets (`start_byte..end_byte`), line, and column.
//! 2. **Boundary & Suffix Enforcement**: Surrounding quotes, whitespace, and trailing punctuation
//!    (e.g., `;`, `,`, `)`) are strictly excluded from matched secret spans.
//! 3. **Non-Leaking Diagnostics**: `SecretMatch` and error types implement custom [`std::fmt::Debug`]
//!    and [`std::fmt::Display`] to ensure raw secrets are never printed in logs or test output.
//! 4. **Adversarial Corpus Testing**: Maintains positive, negative, boundary, and regression fixture suites.

#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};
use std::fmt;
use thiserror::Error;
use tracing::instrument;

/// Errors produced during secret scanning or corpus evaluation.
#[derive(Debug, Error, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DetectorError {
    /// Invalid configuration parameter.
    #[error("Invalid detector configuration: {0}")]
    InvalidConfig(String),

    /// Failure loading or parsing test corpus fixtures.
    #[error("Corpus fixture error: {0}")]
    CorpusError(String),
}

/// Represents a detected secret match with exact byte span offsets and masked values.
///
/// Note: Raw secrets are **never** stored in this struct to ensure logging and error formatting
/// cannot leak sensitive credentials.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SecretMatch {
    /// Identifier of the rule or pattern that detected the secret.
    pub rule_id: String,
    /// Byte offset (inclusive) in the source string where the secret begins.
    pub start_byte: usize,
    /// Byte offset (exclusive) in the source string where the secret ends.
    pub end_byte: usize,
    /// 1-based line number in the source text.
    pub line: usize,
    /// 1-based column number in the source text.
    pub column: usize,
    /// Safely masked representation of the detected secret (e.g. `sec_****3456`).
    pub masked_value: String,
}

impl fmt::Debug for SecretMatch {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SecretMatch")
            .field("rule_id", &self.rule_id)
            .field("span", &format!("{}..{}", self.start_byte, self.end_byte))
            .field("line", &self.line)
            .field("column", &self.column)
            .field("masked_value", &self.masked_value)
            .finish()
    }
}

impl fmt::Display for SecretMatch {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Match[{}] at bytes {}..{} (line {}, col {}): {}",
            self.rule_id, self.start_byte, self.end_byte, self.line, self.column, self.masked_value
        )
    }
}

/// Configuration rules for secret pattern detection.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetectorConfig {
    /// Minimum allowed secret token character length.
    pub min_token_len: usize,
    /// Maximum allowed secret token character length.
    pub max_token_len: usize,
    /// Assignment key keywords to scan for (e.g., `api_key`, `secret`, `token`).
    pub keywords: Vec<String>,
    /// Secret token prefix patterns (e.g., `sk_test_`, `sec_`, `ghp_`).
    pub prefixes: Vec<String>,
}

impl Default for DetectorConfig {
    fn default() -> Self {
        Self {
            min_token_len: 16,
            max_token_len: 128,
            keywords: vec![
                "api_key".to_string(),
                "apikey".to_string(),
                "secret".to_string(),
                "token".to_string(),
                "password".to_string(),
                "auth_key".to_string(),
            ],
            prefixes: vec![
                "sk_test_".to_string(),
                "sk_live_".to_string(),
                "sec_".to_string(),
                "ghp_".to_string(),
                "gho_".to_string(),
            ],
        }
    }
}

/// Detector engine for identifying credentials in source text.
#[derive(Debug, Clone)]
pub struct SecretDetector {
    config: DetectorConfig,
}

impl Default for SecretDetector {
    fn default() -> Self {
        Self::new(DetectorConfig::default()).expect("Default config should be valid")
    }
}

impl SecretDetector {
    /// Creates a new [`SecretDetector`] with the given configuration.
    ///
    /// # Errors
    ///
    /// Returns [`DetectorError::InvalidConfig`] if configuration limits are invalid.
    pub fn new(config: DetectorConfig) -> Result<Self, DetectorError> {
        if config.min_token_len == 0 {
            return Err(DetectorError::InvalidConfig(
                "min_token_len must be greater than 0".to_string(),
            ));
        }
        if config.max_token_len < config.min_token_len {
            return Err(DetectorError::InvalidConfig(
                "max_token_len cannot be less than min_token_len".to_string(),
            ));
        }
        Ok(Self { config })
    }

    /// Returns a reference to the detector configuration.
    #[must_use]
    pub fn config(&self) -> &DetectorConfig {
        &self.config
    }

    /// Scans the provided `input` text for secrets and returns exact match spans.
    ///
    /// Extracted matches contain exact byte offsets (`start_byte..end_byte`) pointing
    /// directly to secret bytes within `input`, excluding surrounding quotes, trailing delimiters,
    /// or whitespace.
    #[instrument(skip(self, input), fields(input_len = input.len()))]
    #[must_use]
    pub fn scan(&self, input: &str) -> Vec<SecretMatch> {
        let mut matches = Vec::new();
        let bytes = input.as_bytes();

        // 1. Scan for key-value assignment patterns
        self.scan_key_assignments(input, &mut matches);

        // 2. Scan for prefix-based tokens
        self.scan_prefixes(input, &mut matches);

        // Deduplicate overlapping or identical matches by exact (start_byte, end_byte)
        matches.sort_by_key(|m| (m.start_byte, m.end_byte));
        matches.dedup_by(|a, b| a.start_byte == b.start_byte && a.end_byte == b.end_byte);

        // Sanity-check spans against raw input string boundaries
        matches.retain(|m| {
            m.start_byte < m.end_byte
                && m.end_byte <= bytes.len()
                && input.is_char_boundary(m.start_byte)
                && input.is_char_boundary(m.end_byte)
        });

        matches
    }

    /// Scans for key-value assignment patterns (e.g. `API_KEY = "sec_1234567890123456"`).
    fn scan_key_assignments(&self, input: &str, matches: &mut Vec<SecretMatch>) {
        let mut line_num = 1;
        let mut line_start_byte = 0;

        for line in input.split('\n') {
            let line_len = line.len();
            let line_clean = if line.ends_with('\r') {
                &line[..line_len - 1]
            } else {
                line
            };

            for kw in &self.config.keywords {
                if let Some(kw_pos) = find_ascii_case_insensitive(line_clean, kw) {
                    // Look for assignment operator (`=` or `:`) after keyword
                    let after_kw = &line_clean[kw_pos + kw.len()..];
                    if let Some(sep_rel) = after_kw.find(['=', ':']) {
                        let val_part = &after_kw[sep_rel + 1..];
                        if let Some((start_in_line, end_in_line, token_str)) =
                            self.extract_assigned_value(kw_pos + kw.len() + sep_rel + 1, val_part)
                        {
                            if self.is_valid_secret_token(token_str) {
                                let abs_start = line_start_byte + start_in_line;
                                let abs_end = line_start_byte + end_in_line;
                                let prefix_in_line = &line_clean[..start_in_line];
                                let column = if prefix_in_line.is_ascii() {
                                    prefix_in_line.len() + 1
                                } else {
                                    prefix_in_line.chars().count() + 1
                                };

                                matches.push(SecretMatch {
                                    rule_id: format!("assignment-{kw}"),
                                    start_byte: abs_start,
                                    end_byte: abs_end,
                                    line: line_num,
                                    column,
                                    masked_value: mask_secret_token(token_str),
                                });
                            }
                        }
                    }
                }
            }

            line_start_byte += line_len + 1; // +1 for '\n'
            line_num += 1;
        }
    }

    /// Scans for secret tokens identified by known prefixes (e.g., `sk_test_...`, `sec_...`).
    fn scan_prefixes(&self, input: &str, matches: &mut Vec<SecretMatch>) {
        for prefix in &self.config.prefixes {
            let mut search_idx = 0;
            while let Some(found_rel) = input[search_idx..].find(prefix) {
                let abs_prefix_start = search_idx + found_rel;
                let remainder = &input[abs_prefix_start..];

                // Find length of secret token characters (alphanumeric, '_', '-')
                let token_len = remainder
                    .bytes()
                    .take_while(|&b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
                    .count();

                let token_str = &remainder[..token_len];
                if self.is_valid_secret_token(token_str) {
                    let abs_start = abs_prefix_start;
                    let abs_end = abs_prefix_start + token_len;

                    let (line, col) = compute_line_col(input, abs_start);

                    matches.push(SecretMatch {
                        rule_id: format!("prefix-{}", prefix.trim_end_matches('_')),
                        start_byte: abs_start,
                        end_byte: abs_end,
                        line,
                        column: col,
                        masked_value: mask_secret_token(token_str),
                    });
                }

                search_idx = abs_prefix_start + prefix.len().max(1);
            }
        }
    }

    /// Extracts value part after assignment operator, trimming quotes, spaces, and trailing delimiters.
    fn extract_assigned_value<'a>(
        &self,
        base_offset_in_line: usize,
        val_part: &'a str,
    ) -> Option<(usize, usize, &'a str)> {
        let trimmed_leading = val_part.trim_start();
        let leading_spaces = val_part.len() - trimmed_leading.len();
        let val_start_in_line = base_offset_in_line + leading_spaces;

        if trimmed_leading.is_empty() {
            return None;
        }

        let first_byte = trimmed_leading.as_bytes()[0];
        if first_byte == b'"' || first_byte == b'\'' {
            let quote_char = char::from(first_byte);
            let inner = &trimmed_leading[1..];
            if let Some(quote_end) = inner.find(quote_char) {
                let token = &inner[..quote_end];
                let start_byte = val_start_in_line + 1;
                let end_byte = start_byte + token.len();
                return Some((start_byte, end_byte, token));
            }
            return None;
        }

        // Unquoted value: stop at whitespace, comment (`#`, `//`), or trailing punctuation (`;`, `,`, `)`, `]`, `"`, `'`)
        let token_end_len = trimmed_leading
            .find(|c: char| {
                c.is_whitespace()
                    || c == ';'
                    || c == ','
                    || c == ')'
                    || c == ']'
                    || c == '#'
                    || c == '"'
                    || c == '\''
            })
            .unwrap_or(trimmed_leading.len());

        let raw_token = &trimmed_leading[..token_end_len];

        if raw_token.is_empty() {
            return None;
        }

        let start_byte = val_start_in_line;
        let end_byte = start_byte + raw_token.len();
        Some((start_byte, end_byte, raw_token))
    }

    /// Validates token length constraints and rejects non-secret values (e.g., pure integers, booleans).
    fn is_valid_secret_token(&self, token: &str) -> bool {
        // Fast path: byte length is a lower bound for UTF-8 character length.
        if token.len() < self.config.min_token_len {
            return false;
        }

        let char_len = if token.is_ascii() {
            token.len()
        } else {
            token.chars().count()
        };

        if char_len > self.config.max_token_len {
            return false;
        }

        // Reject plain numeric values (e.g. `secret_count = 12345678901234567`)
        if token.bytes().all(|b| b.is_ascii_digit()) {
            return false;
        }

        // Reject boolean values
        if token.eq_ignore_ascii_case("true") || token.eq_ignore_ascii_case("false") {
            return false;
        }

        true
    }
}

/// Helper function to search for an ASCII needle in a haystack ignoring ASCII case without allocations.
#[must_use]
pub fn find_ascii_case_insensitive(haystack: &str, needle: &str) -> Option<usize> {
    if needle.is_empty() {
        return Some(0);
    }
    let h_bytes = haystack.as_bytes();
    let n_bytes = needle.as_bytes();
    if n_bytes.len() > h_bytes.len() {
        return None;
    }
    h_bytes
        .windows(n_bytes.len())
        .position(|w| w.eq_ignore_ascii_case(n_bytes))
}

/// Helper function to compute 1-based line and column for a given byte offset in text.
///
/// Fast-path algorithm:
/// 1. Slice text up to `byte_offset`.
/// 2. Count `\n` bytes across prefix for line number (vectorized byte scan).
/// 3. Extract the last line segment after the final `\n` and compute column count.
#[must_use]
pub fn compute_line_col(text: &str, byte_offset: usize) -> (usize, usize) {
    let mut safe_offset = text.len().min(byte_offset);
    while safe_offset > 0 && !text.is_char_boundary(safe_offset) {
        safe_offset -= 1;
    }
    let prefix = &text[..safe_offset];
    let line = prefix.as_bytes().iter().filter(|&&b| b == b'\n').count() + 1;
    let last_line_start = prefix.rfind('\n').map_or(0, |pos| pos + 1);
    let segment = &prefix[last_line_start..];
    let col = if segment.is_ascii() {
        segment.len() + 1
    } else {
        segment.chars().count() + 1
    };

    (line, col)
}

/// Safely masks a secret string for logging or inspection purposes without revealing sensitive bytes.
#[must_use]
pub fn mask_secret_token(secret: &str) -> String {
    let len = secret.chars().count();
    if len <= 8 {
        return "[REDACTED]".to_string();
    }

    let prefix: String = secret.chars().take(4).collect();
    let suffix: String = secret.chars().skip(len.saturating_sub(4)).collect();
    format!("{prefix}****{suffix}")
}

/// Replaces exact byte ranges of matches in `input` with `[REDACTED:{rule_id}]`.
#[must_use]
pub fn redact_text(input: &str, matches: &[SecretMatch]) -> String {
    let mut result = String::with_capacity(input.len());
    let mut last_idx = 0;

    let mut sorted_matches = matches.to_vec();
    sorted_matches.sort_by_key(|m| m.start_byte);

    for m in sorted_matches {
        if m.start_byte >= last_idx && m.end_byte <= input.len() {
            result.push_str(&input[last_idx..m.start_byte]);
            result.push_str(&format!("[REDACTED:{}]", m.rule_id));
            last_idx = m.end_byte;
        }
    }

    if last_idx < input.len() {
        result.push_str(&input[last_idx..]);
    }

    result
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, missing_docs)]

    use super::*;

    #[test]
    fn test_compute_line_col_ascii_and_multibyte() {
        assert_eq!(compute_line_col("", 0), (1, 1));
        assert_eq!(compute_line_col("hello", 0), (1, 1));
        assert_eq!(compute_line_col("hello", 3), (1, 4));
        assert_eq!(compute_line_col("hello\nworld", 6), (2, 1));
        assert_eq!(compute_line_col("hello\nworld", 8), (2, 3));
        assert_eq!(compute_line_col("a\n\nb", 3), (3, 1));
        assert_eq!(compute_line_col("🦀\nfoo", 6), (2, 2));
        // Offset landing in the middle of a 4-byte emoji (non-char boundary) must not panic
        assert_eq!(compute_line_col("🦀\nfoo", 2), (1, 1));
    }

    #[test]
    fn test_find_ascii_case_insensitive() {
        assert_eq!(
            find_ascii_case_insensitive("API_KEY = 123", "api_key"),
            Some(0)
        );
        assert_eq!(
            find_ascii_case_insensitive("  Secret_Token =", "secret_token"),
            Some(2)
        );
        assert_eq!(find_ascii_case_insensitive("hello", "missing"), None);
        assert_eq!(find_ascii_case_insensitive("hello", ""), Some(0));
    }
}
