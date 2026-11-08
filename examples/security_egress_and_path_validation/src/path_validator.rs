//! Untrusted filesystem path validation module.

use std::path::{Component, Path, PathBuf};

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

        let bytes = raw_path.as_bytes();
        let mut i = 0;

        // SWAR (SIMD Within A Register) fast-path: check 8-byte chunks simultaneously for printable ASCII values (0x20..=0x7E).
        // Printable ASCII contains no control, zero-width, or Bidi override characters.
        while i + 8 <= bytes.len() {
            let mut arr = [0u8; 8];
            arr.copy_from_slice(&bytes[i..i + 8]);
            let chunk = u64::from_ne_bytes(arr);
            let has_low =
                (chunk.wrapping_sub(0x2020_2020_2020_2020) & !chunk) & 0x8080_8080_8080_8080;
            let has_high = chunk & 0x8080_8080_8080_8080;
            let y = chunk ^ 0x7F7F_7F7F_7F7F_7F7F;
            let zero_check = (y.wrapping_sub(0x0101_0101_0101_0101) & !y) & 0x8080_8080_8080_8080;
            if (has_low | has_high | zero_check) != 0 {
                break;
            }
            i += 8;
        }

        while i < bytes.len() {
            let b = bytes[i];
            if !(0x20..=0x7E).contains(&b) {
                break;
            }
            i += 1;
        }

        if i < bytes.len() {
            // Slow path: Check for control, zero-width, and Bidi control characters across full Unicode
            for ch in raw_path[i..].chars() {
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
