# Benchmarks

Runtime and workload regression harnesses for the template workspace. Everything here runs
locally with no privileges, no external services, and no network.

## Layout

| Path | Purpose |
|---|---|
| `benches/tokio_runtime_bench.rs` | Tokio execution strategy (direct vs `spawn_blocking`) and cooperative-yield telemetry |
| `benches/tokio_runtime_pressure_bench.rs` | Bounded vs unbounded fan-out, registry contention, service-queue latency |
| `benches/tokio_common/mod.rs` | Shared constants, runtimes, CPU unit and percentile reporting for both targets |
| `tests/tokio_runtime_behavior.rs` | Deterministic runtime invariants (backpressure, starvation, isolation, permit bounds, registry updates) |
| `benches/mcp_server_bench.rs` | MCP tool-dispatch contention |
| `benches/end_to_end.rs`, `benches/memory_usage.rs`, `benches/sanitization_bench.rs`, `benches/checkpoint_bench.rs` | Application-level microbenchmarks |
| `events/YYYY/MM/DD/<sha>.jsonl` | Machine-readable telemetry, one record per benchmark per run |
| `history.jsonl` | Aggregated trend input consumed by `scripts/compare-benchmarks.sh` |

## Running

```bash
cargo bench -p benchmarks --bench tokio_runtime --bench tokio_runtime_pressure -- --quick
cargo bench -p benchmarks --bench tokio_runtime --bench tokio_runtime_pressure
cargo nextest run -p benchmarks                              # deterministic invariants
```

CI runs the full suite in the `Benchmarks` job and publishes the parsed results through
`scripts/parse_bench.py` into `benchmarks/events/**`.

## Two Kinds of Evidence

- **Deterministic invariants** (`tests/`) assert capacity, ordering, thread identity,
  permit bounds and deadlock-freedom. They never assert wall-clock numbers, so they are
  safe as gates on a shared runner.
- **Informational telemetry** (`benches/`) measures observable behaviour and records
  `p50/p95/p99/max` per workload. Percentiles are printed in bencher format, so each one
  becomes its own row in the event stream (`…/p99`, `…/max`) with no schema change.

`scripts/parse_bench.py` merges two sources by benchmark name: those harness-emitted
percentile rows, and criterion's own `target/criterion/**/new/estimates.json` medians
(criterion's bencher output carries no benchmark id, so its rows can only come from the
JSON artifacts it always writes). The Benchmarks job fails if the merged run records zero
rows, so a silent telemetry outage cannot pass as a green job again.

## Interpreting Results

- Compare percentiles, not averages: tail movement outside run-to-run noise is the signal.
- Compare like with like — same machine class, toolchain, and workload size. Laptop and
  shared-runner numbers contextualise a change, they do not decide it.
- The event stream is a time series, not a threshold. No CI gate fails on a timing delta.
- Justify an architectural change with a measured symptom: degraded `p99` as fan-out grows
  justifies `spawn_blocking`; a starved peer in the behavior test justifies `yield_now()`
  or batching at that loop; growing queue depth justifies a bounded channel.
- See `.agents/skills/tokio-performance/SKILL.md` for the decision matrix these numbers feed.

## Adding a Workload

1. Add the benchmark function to the relevant bench file and register it in the
   `criterion_group!` for that target.
2. If the workload has a behavioural invariant worth gating, add it to `tests/` — with no
   timing threshold.
3. If it produces a tail worth tracking, run an instrumented sampling pass and emit
   percentiles through the existing bencher-format helper.
4. Run `cargo clippy -p benchmarks --all-targets --all-features -- -D warnings` and
   `cargo nextest run -p benchmarks` before opening a PR.
