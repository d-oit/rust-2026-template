<!-- AUTO-GENERATED — edit src/lib.rs in the crate, not this file -->

# example-storage-pattern ![License: MIT](https://img.shields.io/badge/license-MIT-blue) [![example-storage-pattern on crates.io](https://img.shields.io/crates/v/example-storage-pattern)](https://crates.io/crates/example-storage-pattern) [![example-storage-pattern on docs.rs](https://docs.rs/example-storage-pattern/badge.svg)](https://docs.rs/example-storage-pattern)

## Trait-Only Storage Pattern

Demonstrates the **trait-only storage layer**: the `Backend` trait lives in
this crate with zero implementations. Concrete backends (`SqliteBackend`,
future `PostgresBackend`) implement the trait behind a feature flag without
touching any consumer crate.

### When to use

* You want to swap storage backends without changing business logic
* You need fast, zero-I/O unit tests via a `MockBackend`
* You publish a library and don’t want to force a storage dependency on users

### Structure

```text
your-types  ←  your-storage-trait  ←  your-sqlite-backend
                                   ←  your-mock-backend (cfg(test))
                    ↑
             your-business-logic (depends only on the trait)
```
