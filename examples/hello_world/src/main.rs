//! Hello World example demonstrating the example-crate.

use anyhow::Result;
use example_crate::greet;

/// Main entry point.
fn main() -> Result<()> {
    let message = greet("World");
    println!("{message}");
    Ok(())
}
