<!-- AUTO-GENERATED — edit src/lib.rs in the crate, not this file -->

# example-registry-pattern ![License: MIT](https://img.shields.io/badge/license-MIT-blue) [![example-registry-pattern on crates.io](https://img.shields.io/crates/v/example-registry-pattern)](https://crates.io/crates/example-registry-pattern) [![example-registry-pattern on docs.rs](https://docs.rs/example-registry-pattern/badge.svg)](https://docs.rs/example-registry-pattern) [![Source Code Repository](https://img.shields.io/badge/Code-On%20GitHub-blue?logo=GitHub)](https://github.com/your-org/your-repo) ![Rust Version: 1.88.0](https://img.shields.io/badge/rustc-1.88.0-orange.svg)

## Registry / Plugin Dispatch Pattern

Demonstrates a `Registry<dyn Handler>` that routes named operations to
modular, independently testable handler implementations — without a
central `match` tree.

### When to use

* CLI tools where subcommands grow over time
* Plugin architectures where handlers are registered at startup
* Any system where the set of operations is open-ended

### Trade-offs vs `match`

||`match`|Registry|
|--|-------|--------|
|Compile-time exhaustiveness|✅|❌|
|Runtime extensibility|❌|✅|
|Independent handler tests|harder|easy|
|Adding a new operation|edit central file|add new struct|
