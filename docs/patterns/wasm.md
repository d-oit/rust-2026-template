# Opt-in Rust-to-WASM extension

This guide describes an optional WebAssembly setup for projects that need it. It
does not add a WASM target, dependencies, or CI jobs to this template's default
workspace. Adopt only the pieces your project needs.

## Choose a target

- Use `wasm32-unknown-unknown` for browser-facing modules and JavaScript
  interoperability. It has no operating-system APIs; provide host functionality
  through browser APIs or imported bindings.
- Use a WASI target, such as `wasm32-wasip1`, for command-line or server-side
  modules that need the WASI system interface. Confirm that the intended runtime
  supports the selected WASI version.

These targets have different runtime capabilities and are not interchangeable.

## Install and pin the toolchain

Add the target only in the project that needs it:

```sh
rustup target add wasm32-unknown-unknown
```

For reproducible builds, record the target in that project's toolchain setup or
document it in its build instructions. Pin `wasm-bindgen-cli` to the same
version as the project's `wasm-bindgen` dependency, and commit the dependency
lockfile for an application:

```sh
cargo install wasm-bindgen-cli --version <matching-version> --locked
```

Use the exact version chosen by the project in place of `<matching-version>`.
Do not install the CLI at an unpinned latest version in CI.

## Native and WASM tests

Keep ordinary Rust tests and WASM tests distinct. Native tests exercise the
crate's host-compatible behavior:

```sh
cargo test -p <wasm-crate>
```

For tests that require browser or JavaScript APIs, add `wasm-bindgen-test` as a
development dependency in the WASM crate and mark those tests with
`#[wasm_bindgen_test]`. Use `wasm-pack test --node <crate-directory>` for a
Node.js runner. Use a browser runner, for example
`wasm-pack test --headless --chrome <crate-directory>`, when browser behavior is
part of the contract; install and pin the browser and driver in CI as well.

Do not treat a successful native test run as evidence that the WASM-only tests
ran. Each environment should have an explicit command in the project's test
documentation.

### Reject an empty WASM test suite

A target can compile successfully while running no WASM tests. Make the
WASM-specific test command fail if its runner reports zero tests. For example,
wrap the chosen runner command and inspect its test summary:

```sh
set -o pipefail
output="$(wasm-pack test --node <crate-directory> 2>&1)" || {
    printf '%s\n' "$output"
    exit 1
}
printf '%s\n' "$output"
if ! printf '%s\n' "$output" | grep -Eq 'running [1-9][0-9]* tests?'; then
    echo "WASM test runner did not report any tests" >&2
    exit 1
fi
```

Check this guard against the exact runner/version used by the project; if its
summary format differs, use an equivalent assertion on that runner's test
count. Keep at least one real `wasm-bindgen-test` in the suite so a registered
but empty command cannot satisfy the guard.

## Release optimization and size budgets

Use `wasm-opt` from Binaryen only in a release or size-measurement path, not in
the default edit/test loop. Record the Binaryen version and optimization flags
used for published artifacts. Set a named, project-specific size budget,
measure the final optimized artifact, and fail the release check when that
budget is exceeded. Avoid an unexplained magic threshold.

## Optional adoption boundary

Keep any reusable target setup, test wrapper, or size-check script in an
optional example or extension directory. Move it into the project's normal
`scripts/` or CI workflows only after explicitly adopting and maintaining that
capability. Non-WASM projects should not install additional components or run
additional jobs.
