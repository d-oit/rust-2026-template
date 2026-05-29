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
//! ```rust,no_run
//! // Replace `{{crate_name}}` with your actual crate name
//! // use {{crate_name}}::add;
//! # use rust_2026_template::add;
//!
//! let result = add(2, 3);
//! assert_eq!(result, 5);
//! ```

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
