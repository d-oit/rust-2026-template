# sample-app

Binary crate demonstrating `rust-2026-template` patterns: async runtime, CLI, structured logging, serialization, and error handling.

## Dependencies

| Crate | Purpose |
|---|---|
| `tokio` | Async runtime (`current_thread` flavor) |
| `clap` | CLI argument parsing with `derive` feature |
| `serde` + `serde_json` | JSON config serialization |
| `tracing` + `tracing-subscriber` | Structured logging |
| `thiserror` | Typed error enum (`AppError`) |
| `anyhow` | Error context propagation |

## Usage

```bash
# Run with defaults (processes 10 items)
cargo run -p sample-app

# Custom item count
cargo run -p sample-app -- --count 5

# Verbose (DEBUG) logging
cargo run -p sample-app -- --verbose

# Load config from JSON file
cargo run -p sample-app -- --config config.json
```

### Config file format (`config.json`)

```json
{
  "app_name": "my-app",
  "log_level": "debug",
  "max_items": 50
}
```

Unknown fields are rejected (`#[serde(deny_unknown_fields)]`). Config files over 1 MB are rejected.

## CLI Flags

| Flag | Short | Default | Description |
|---|---|---|---|
| `--count` | `-c` | `10` | Number of items to process |
| `--verbose` | `-v` | off | Enable DEBUG logging |
| `--config` | — | none | Path to JSON config file |

## Testing

```bash
# Run all tests
cargo nextest run -p sample-app

# Run with log output
RUST_LOG=debug cargo nextest run -p sample-app -- --nocapture
```

## Building

```bash
cargo build -p sample-app
cargo build -p sample-app --release
```