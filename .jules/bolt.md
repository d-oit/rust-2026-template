## 2025-05-15 - [Tokio Runtime and Tracing Metadata in CLI]
**Learning:** Default multi-threaded Tokio runtime and full tracing metadata collection introduce significant overhead in CLI tools that do not require high concurrency. Switching to the `current_thread` flavor and disabling thread IDs/names in tracing reduces startup time and improves throughput for simple I/O or batch tasks.
**Action:** Always prefer `#[tokio::main(flavor = "current_thread")]` for CLI applications and disable redundant tracing metadata unless the tool is explicitly designed for high-concurrency environments.

## 2025-05-15 - [Linker Dependencies in Template Repos]
**Learning:** Template repositories often include aggressive build optimizations like `mold` which may not be present in all development environments (like restricted sandboxes).
**Action:** Verify the presence of specialized tools before assuming their availability in the build configuration, and provide fallbacks or clear documentation for local environment adjustments.

## 2025-05-20 - [Efficient Workspace Scanning]
**Learning:** Recursive scans (like `grep -r`) in a Rust workspace are surprisingly expensive due to the `target/` directory, which often contains tens of thousands of build artifacts. Filtering results with `grep -v` after a full scan is a common anti-pattern that wastes significant I/O.
**Action:** Always use `--exclude-dir=target` (and `.git`) in search commands to prevent the tool from even entering these directories. This can reduce scan times by 70% or more.
