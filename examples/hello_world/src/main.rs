//! Hello World example demonstrating the example-crate.

use example_crate::greet;

/// Main entry point.
fn main() {
    let message = greet("World");
    println!("{message}");
}
