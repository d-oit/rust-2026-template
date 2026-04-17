# example-crate

Library placeholder in the `rust-2026-template` workspace. Rename this crate and replace its contents with your own implementation.

## Current API

```rust
use example_crate::greet;

fn main() {
    println!("{}", greet("world")); // "Hello, world!"
}
```

### `greet(name: &str) -> String`

Returns `"Hello, {name}!"`. Demonstrates doc tests, `#[must_use]`, and `#[forbid(unsafe_code)]`.

## Renaming This Crate

```bash
# 1. Check crates.io availability
cargo search your-crate-name

# 2. Rename the directory
mv crates/example-crate crates/your-crate-name

# 3. Update Cargo.toml
# Change: name = "example-crate" -> name = "your-crate-name"
```

See `.agents/skills/crates-io-name-check/SKILL.md` for the full name-check workflow.

## Testing

```bash
cargo nextest run -p example-crate
cargo test --doc -p example-crate
```

## License

MIT
