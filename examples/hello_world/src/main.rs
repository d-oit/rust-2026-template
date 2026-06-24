use anyhow::Result;
use example_crate::greet;

fn main() -> Result<()> {
    let message = greet("World");
    println!("{message}");
    Ok(())
}
