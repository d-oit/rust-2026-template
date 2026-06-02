<!-- AUTO-GENERATED — edit src/lib.rs in the crate, not this file -->

# example-registry-pattern ![License](https://img.shields.io/crates/l/example-registry-pattern) [![example-registry-pattern on crates.io](https://img.shields.io/crates/v/example-registry-pattern)](https://crates.io/crates/example-registry-pattern) [![example-registry-pattern on docs.rs](https://docs.rs/example-registry-pattern/badge.svg)](https://docs.rs/example-registry-pattern)

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
