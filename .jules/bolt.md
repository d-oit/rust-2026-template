## 2025-05-15 - [Tokio Runtime and Tracing Metadata in CLI]
**Learning:** Default multi-threaded Tokio runtime and full tracing metadata collection introduce significant overhead in CLI tools that do not require high concurrency. Switching to the `current_thread` flavor and disabling thread IDs/names in tracing reduces startup time and improves throughput for simple I/O or batch tasks.
**Action:** Always prefer `#[tokio::main(flavor = "current_thread")]` for CLI applications and disable redundant tracing metadata unless the tool is explicitly designed for high-concurrency environments.

## 2025-05-15 - [Linker Dependencies in Template Repos]
**Learning:** Template repositories often include aggressive build optimizations like `mold` which may not be present in all development environments (like restricted sandboxes).
**Action:** Verify the presence of specialized tools before assuming their availability in the build configuration, and provide fallbacks or clear documentation for local environment adjustments.

## 2026-04-14 - Manual String Construction vs format! Macro
**Learning:** In Rust, replacing the `format!` macro with manual string construction using `String::with_capacity` and `push_str` can yield significant performance gains (up to 2.6x) for simple concatenations by avoiding the overhead of the formatting machinery and reducing allocations to exactly one.
**Action:** Prefer pre-allocating strings with exact capacity for simple concatenations in performance-critical hot loops. Always use `std::hint::black_box` when benchmarking to ensure the compiler doesn't optimize away the function call.
