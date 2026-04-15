//! # example-crate
//!
//! This is an example crate in the `rust-2026-template` workspace.
//! Replace this with your actual crate implementation.
//!
//! ## Usage
//!
//! Add this to your workspace member and start building!

#![forbid(unsafe_code)]
#![deny(missing_docs)]
#![warn(clippy::all, clippy::pedantic, clippy::nursery)]

/// Returns a greeting string.
///
/// # Examples
///
/// ```
/// let greeting = example_crate::greet("world");
/// assert_eq!(greeting, "Hello, world!");
/// ```
#[must_use]
pub fn greet(name: &str) -> String {
    let mut s = String::with_capacity(7 + name.len() + 1);
    s.push_str("Hello, ");
    s.push_str(name);
    s.push('!');
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_greet() {
        assert_eq!(greet("world"), "Hello, world!");
    }

    #[test]
    fn test_greet_empty() {
        assert_eq!(greet(""), "Hello, !");
    }
}
