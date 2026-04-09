## 2025-05-15 - [Tokio Runtime and Tracing Metadata in CLI]
**Learning:** Default multi-threaded Tokio runtime and full tracing metadata collection introduce significant overhead in CLI tools that do not require high concurrency. Switching to the `current_thread` flavor and disabling thread IDs/names in tracing reduces startup time and improves throughput for simple I/O or batch tasks.
**Action:** Always prefer `#[tokio::main(flavor = "current_thread")]` for CLI applications and disable redundant tracing metadata unless the tool is explicitly designed for high-concurrency environments.

## 2025-05-15 - [Linker Dependencies in Template Repos]
**Learning:** Template repositories often include aggressive build optimizations like `mold` which may not be present in all development environments (like restricted sandboxes).
**Action:** Verify the presence of specialized tools before assuming their availability in the build configuration, and provide fallbacks or clear documentation for local environment adjustments.

## 2026-04-09 - [Efficient String Formatting and Config Loading]
**Learning:** In performance-critical hot loops, manual digit extraction and direct `String::push` can significantly outperform `write!` macros by bypassing runtime format parsing and trait dispatch. For file I/O, `BufReader` with `serde_json::from_reader` is superior to reading the entire file into a string, as it reduces peak memory usage and avoids an intermediate allocation.
**Action:** Use streaming parsers for configuration and manual formatting for high-frequency string generation when constraints are known.
