//! # {{crate_name}}
//!
//! Replace this with your crate's description.
//!
//! ## Features
//!
//! - Feature A
//! - Feature B
//!
//! ## Example
//!
//! ```rust
//! use {{crate_name}}::add;
//!
//! let result = add(2, 3);
//! assert_eq!(result, 5);
//! ```

#![deny(unsafe_code)]
#![warn(
    missing_docs,
    clippy::pedantic,
    clippy::nursery,
    clippy::cargo,
)]
#![allow(
    clippy::module_name_repetitions,
)]

/// Adds two numbers together.
///
/// # Examples
///
/// ```
/// use {{crate_name}}::add;
///
/// assert_eq!(add(2, 3), 5);
/// ```
#[must_use]
pub fn add(left: u64, right: u64) -> u64 {
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
