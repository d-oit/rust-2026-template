# Fuzz workspace dependencies

This directory is an independent Cargo workspace because the root workspace
excludes it and cargo-fuzz manages its targets separately. Its dependency graph
includes the fuzz runtime and may differ from the root application graph.

`fuzz/Cargo.lock` is tracked because the fuzz harness is an executable. When
changing `fuzz/Cargo.toml` or its path dependencies, update the lockfile
intentionally with:

```sh
cargo generate-lockfile --manifest-path fuzz/Cargo.toml
```

Review the lockfile diff, then run `scripts/check-fuzz-deps.sh`. The check uses
`cargo metadata --locked` before auditing the fuzz lockfile with cargo-audit and
running cargo-deny against the repository's `deny.toml`. A missing or stale
lockfile fails without being regenerated. Projects generated without the fuzz
capability do not need to run this check.

The local `sample-app` dependency carries its package version to satisfy the
repository's wildcard-dependency ban. If the root workspace version changes,
keep that dependency version aligned with `sample-app`.

`libfuzzer-sys` uses the NCSA license. `deny.toml` permits it as a crate-specific
exception rather than adding it to the global license allowlist used by every
workspace member.
