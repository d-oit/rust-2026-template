# Faster Builds

This guide provides strategies to improve Rust compilation times. While the template includes sensible defaults, some highly effective optimizations are platform-specific or depend on your development workflow.

## Universal Guidance

### 1. Use `cargo check`
For quick validation of your code, use `cargo check` instead of `cargo build`. It skips the code generation phase, which is often the most time-consuming part of the build process.

### 2. Use the Opt-in `fast-dev` Profile
The template includes a `fast-dev` profile optimized for compilation speed. It inherits from the standard `dev` profile but sets `panic = "abort"` to reduce the amount of work the compiler needs to do by removing the need for stack unwinding information.

To use it:

```bash
cargo build --profile fast-dev
```

### 3. Identify Bottlenecks with `--timings`
If you're wondering why your build is slow, use the `--timings` flag:

```bash
cargo build --timings
```

This generates an HTML report in `target/cargo-timings/` showing which crates took the longest to compile and how much parallelism was achieved.

---

## Platform-Specific Optimizations

These optimizations are not enabled by default as they require external tools or specific OS configurations.

### Linux: `mold` Linker
The `mold` linker is significantly faster than the default `ld` or `gold` linkers.

**Setup:**
1. Install `mold` (e.g., `sudo apt install mold`).
2. Add the following to your `.cargo/config.toml`:

```toml
[target.x86_64-unknown-linux-gnu]
linker = "clang"
rustflags = ["-C", "link-arg=-fuse-ld=mold"]

[target.aarch64-unknown-linux-gnu]
linker = "clang"
rustflags = ["-C", "link-arg=-fuse-ld=mold"]
```

### macOS: Split Debug Info
On macOS, the linker (`ld64`) can spend a lot of time packaging debug information into `.dSYM` bundles. You can skip this by using "unpacked" debug info.

Add the following to your `.cargo/config.toml`:

```toml
[target.aarch64-apple-darwin]
split-debuginfo = "unpacked"

[target.x86_64-apple-darwin]
split-debuginfo = "unpacked"
```

### Windows: Dev Drive
If you are on Windows 11, using a **Dev Drive** (ReFS) can significantly improve I/O performance for Rust builds.

1. Create a Dev Drive in Windows Settings.
2. Move your project or your `CARGO_HOME` to the Dev Drive.
3. Ensure your antivirus (e.g., Microsoft Defender) is in "performance mode" for the Dev Drive.

---

## Recommended Configuration (Opt-in)

If you have the prerequisites installed for your platform, we recommend adding these blocks to your local or project-level configuration.

### For `.cargo/config.toml`

```toml
# Linux (requires mold and clang)
[target.x86_64-unknown-linux-gnu]
linker = "clang"
rustflags = ["-C", "link-arg=-fuse-ld=mold"]

[target.aarch64-unknown-linux-gnu]
linker = "clang"
rustflags = ["-C", "link-arg=-fuse-ld=mold"]

# macOS (split-debuginfo = "unpacked")
[target.aarch64-apple-darwin]
split-debuginfo = "unpacked"

[target.x86_64-apple-darwin]
split-debuginfo = "unpacked"

# Windows (if you prefer rust-lld)
# [target.x86_64-pc-windows-msvc]
# linker = "rust-lld"
```

### For `Cargo.toml`

The `fast-dev` profile is already included in the workspace `Cargo.toml`. You can customize it further if needed:

```toml
[profile.fast-dev]
inherits = "dev"
panic = "abort"
# Add more speed-oriented overrides here
```
