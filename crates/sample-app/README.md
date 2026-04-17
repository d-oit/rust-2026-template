# Sample Application

A comprehensive sample application demonstrating the rust-2026-template features.

## Features

- **Async Runtime**: Uses tokio for async operations
- **Error Handling**: Uses thiserror for custom errors and anyhow for context
- **Serialization**: Uses serde for JSON serialization/deserialization
- **Logging**: Uses tracing for structured logging
- **CLI**: Uses clap for command-line argument parsing

## Usage

```bash
# Run with default settings
cargo run -p sample-app

# Run with custom item count
cargo run -p sample-app -- --count 5

# Run with verbose logging
cargo run -p sample-app -- --verbose

# Run with config file
cargo run -p sample-app -- --config config.json
```

## Testing

```bash
# Run all tests
cargo test -p sample-app

# Run with output
cargo test -p sample-app -- --nocapture
```

## Building

```bash
# Debug build
cargo build -p sample-app

# Release build
cargo build -p sample-app --release
```