<!-- AUTO-GENERATED — edit src/lib.rs in the crate, not this file -->

# Safe CLI Dry-Run Pattern for Costly External Side Effects

Demonstrates a safe `--dry-run` pattern for CLI tools that trigger paid APIs, billable cloud operations, or irreversible remote mutations (inspired by tools like `do-music`).

## Motivation

CLI applications often trigger remote operations that carry financial cost, consume API quota, or mutate external infrastructure. Users need a reliable way to validate input parameters and preview execution scope before invoking expensive side effects.

## Core Design Principles

1. **Shared Validation & Planning**: The exact same parsing and validation logic (`Plan::build`) is executed for both dry-run and live runs. Invalid inputs produce identical errors regardless of the `--dry-run` flag.
2. **Reliable Scope & Cost Reporting**: Calculate concrete, deterministic metrics (e.g., track counts, duration, fixed credit formulas). Avoid fabricating uncertain cost estimates.
3. **Explicit & Transparent Output**: Mark dry-run output clearly (e.g., `[DRY-RUN]`, `[WOULD EXECUTE]`, `SKIPPED paid API call`) so users know what would happen without claiming execution occurred.
4. **Zero Outbound Side Effects**: Abstract external operations behind traits or client wrappers so dry-run guarantees zero network requests and zero state mutations.
5. **Optionality**: The pattern lives in `examples/cli_dry_run` as an optional example and does not add overhead to projects that do not have costly side effects.

## Architecture

```
                       ┌─────────────────────────┐
                       │  CLI Arguments Parsing  │
                       └────────────┬────────────┘
                                    │
                                    ▼
                       ┌─────────────────────────┐
                       │   Shared Validation &   │
                       │     Plan Building       │
                       └────────────┬────────────┘
                                    │
                         ┌──────────┴──────────┐
                         │ Validated Plan      │
                         └──────────┬──────────┘
                                    │
                 ┌──────────────────┴──────────────────┐
                 │                                     │
                 ▼                                     ▼
      [--dry-run = true]                     [--dry-run = false]
┌───────────────────────────────┐     ┌───────────────────────────────┐
│ Preview planned actions       │     │ Execute planned actions       │
│ Report scope & costs          │     │ Invoke remote paid API        │
│ List skipped remote calls     │     │ Mutate external state         │
│ Zero outbound requests        │     │ Return execution results      │
└───────────────────────────────┘     └───────────────────────────────┘
```

## Example Usage

```bash
# Preview execution plan without triggering paid APIs
cargo run -p cli_dry_run -- --input album.json --tracks 5 --quality hd --dry-run

# Output:
# [DRY-RUN] Mode active — inspecting execution plan without side effects.
# [DRY-RUN] Scope: 5 track(s) at 'hd' quality. Estimated credits: 25.
# [DRY-RUN] Actions that WOULD be performed:
#   [WOULD EXECUTE] Step 1: Validate local input manifest: 'album.json'
#   [WOULD EXECUTE] Step 2: Synthesize track #1/5: tier 'hd' (cost: 5 credits) (SKIPPED paid API call)
#   [WOULD EXECUTE] Step 3: Synthesize track #2/5: tier 'hd' (cost: 5 credits) (SKIPPED paid API call)
#   [WOULD EXECUTE] Step 4: Synthesize track #3/5: tier 'hd' (cost: 5 credits) (SKIPPED paid API call)
#   [WOULD EXECUTE] Step 5: Synthesize track #4/5: tier 'hd' (cost: 5 credits) (SKIPPED paid API call)
#   [WOULD EXECUTE] Step 6: Synthesize track #5/5: tier 'hd' (cost: 5 credits) (SKIPPED paid API call)
#   [WOULD EXECUTE] Step 7: Finalize and store output batch for 'album.json'
# [DRY-RUN] Completed inspection: 0 actions performed, 5 paid remote call(s) skipped.
```

## Testing Strategy

To guarantee dry-run safety and prevent regressions, write tests with a mock API client that panics if any remote method is invoked:

```rust
#[test]
fn test_dry_run_makes_no_outbound_request() {
    let args = CliArgs {
        input: "album.json".into(),
        tracks: 3,
        quality: "hd".into(),
        dry_run: true,
    };

    let plan = Plan::build(&args).expect("Plan construction should succeed");
    let client = StrictMockApiClient; // Panics if execute_remote() is called

    let report = run_plan(&plan, &client).expect("Dry run should succeed without network calls");
    assert!(!report.executed);
    assert_eq!(report.actions_performed, 0);
}
```

See the full executable implementation and tests in `examples/cli_dry_run/`.
