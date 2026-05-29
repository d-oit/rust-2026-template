//! # example-crate
//!
//! This is an example crate in the `rust-2026-template` workspace.
//! Replace this with your actual crate implementation.
//!
//! ## Usage
//!
//! Add this to your workspace member and start building!

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
    // Bolt: Pre-allocate String to avoid reallocations and bypass format! macro overhead
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

#[cfg(test)]
mod prop_tests {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        /// Tests that `greet` always starts with "Hello, " and ends with "!" for a wide range of inputs.
        #[test]
        fn greet_always_starts_with_hello_and_ends_with_bang(name in "[a-zA-Z0-9 ]{0,50}") {
            let result = greet(&name);
            assert!(result.starts_with("Hello, "));
            assert!(result.ends_with('!'));
        }

        /// Tests that the length of the resulting string is correctly calculated based on the input name length.
        #[test]
        fn greet_length_is_correct(name in ".*") {
            let result = greet(&name);
            assert_eq!(result.len(), 7 + name.len() + 1);
        }
    }
}
