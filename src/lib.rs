//! A production-ready Rust workspace template with modern tooling, CI/CD,
//! and AI agent integration.
//!
//! ## Overview
//!
//! ![Architecture](.template/architecture.svg)
//!
//! This template is designed for Rust developers who want to start new projects with best practices baked in. It provides a modular workspace structure, comprehensive quality gates, and built-in support for AI-assisted development.
//!
//! ## Features
//!
//! - **Rust 2024 Edition:** Leverages the latest language features and idioms with an MSRV of 1.88.
//! - **Workspace Layout:** Clean separation of concerns with a `crates/` directory for internal libraries and applications.
//! - **Security First:** Pre-configured supply chain audits, secret scanning, and hardened configuration patterns.
//! - **Performance Optimized:** Optimized dev profiles with reduced debug artifacts and disk space savings.
//! - **AI-Native:** First-class support for AI coding agents with specialized skills and canonical instruction sets. Includes `llms.txt` for machine-readable project context.
//!
//! ## Example
//!
//! ```rust,no_run
//! # use rust_2026_template::add;
//!
//! let result = add(2, 3);
//! assert_eq!(result, 5);
//! ```

// For library crates where docs.rs is the primary surface, you can instead
// make README.md the source of truth and include it directly:
//
// #![doc = include_str!("../README.md")]
//
// This is stable since Rust 1.54. The README renders verbatim on docs.rs.
// Use this OR cargo-sync-readme — not both.

/// Adds two numbers together.
///
/// # Examples
///
/// ```
/// # use rust_2026_template::add;
/// assert_eq!(add(2, 3), 5);
/// ```
#[must_use]
pub const fn add(left: u64, right: u64) -> u64 {
    left + right
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add() {
        assert_eq!(add(2, 2), 4);
    }

    #[test]
    fn test_add_zero() {
        assert_eq!(add(0, 0), 0);
        assert_eq!(add(5, 0), 5);
    }
}
