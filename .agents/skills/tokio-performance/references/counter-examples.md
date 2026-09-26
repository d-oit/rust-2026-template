# Anti-Patterns & Counter-Examples

Worked examples of the four Tokio misapplications that cost the most, each with
the symptom, the cost, and the correct alternative. Read this when a design
proposes `spawn_blocking`, `yield_now`, a second runtime, or a lock held across
`.await` — before accepting it.

## Counter-Example 1: When `spawn_blocking` is NOT the Answer

**Problem:** Wrapping short CPU tasks (e.g. quick string parsing, fast JSON field extraction, small hash calculation) in `spawn_blocking`.

```rust
// BAD: Cargo-culting spawn_blocking for a 2-microsecond JSON parse
let user: User = tokio::task::spawn_blocking(move || {
    serde_json::from_str(&payload)
}).await??;

// GOOD: Direct execution on Tokio worker thread avoids thread pool queue overhead
let user: User = serde_json::from_str(&payload)?;
```

## Counter-Example 2: When `yield_now` is NOT the Answer

**Problem:** Inserting `yield_now()` manually inside tight processing loops instead of structuring work with backpressure or batching.

```rust
// BAD: Blindly scattering yield_now in every iteration
for item in items {
    process_item(item);
    tokio::task::yield_now().await;
}

// GOOD: Process in batches or leverage channel backpressure and Tokio budget
for chunk in items.chunks(100) {
    process_batch(chunk);
    // Tokio automatically manages cooperative budget for async operations,
    // or explicit yield only if heavy CPU batching risks worker thread starvation.
}
```

## Counter-Example 3: When `RwLock` is NOT the Answer

**Problem:** Wrapping read-heavy config or tool registries in `Arc<tokio::sync::RwLock<Config>>` causing reader lock contention and async overhead on hot paths.

```rust
// BAD: Heavy async RwLock overhead for read-heavy state lookups or tool dispatch
let registry = server.tools.read().await;
let tool = registry.get("echo");

// GOOD: Copy-On-Write snapshot pattern (e.g. std::sync::RwLock<Arc<Registry>>)
// Dispatches take a sub-nanosecond Arc snapshot and drop the lock guard immediately:
let registry = {
    let guard = server.tools.read().unwrap_or_else(|e| e.into_inner());
    Arc::clone(&*guard)
};
// Registry lookup, validation, and async handle execution run without holding any lock guard:
let tool = registry.get("echo")?;
tool.handle(req).await?;
```

## Counter-Example 4: When a Second Tokio Runtime is NOT the Answer

**Problem:** Creating a multi-runtime setup (e.g. `Builder::new_multi_thread().build()`) to "isolate" background work.

```rust
// BAD: Spinning up a complete second Tokio runtime inside an async context
let secondary_rt = tokio::runtime::Runtime::new().unwrap();
secondary_rt.block_on(async { ... });

// GOOD: Use bounded tasks, Semaphore, or dedicated std::thread pool for isolated work
let permit = semaphore.acquire_owned().await?;
tokio::spawn(async move {
    let _permit = permit;
    perform_background_work().await;
});
```

## Counter-Example 5: Lock Held Across `.await` Point

**Problem:** Holding a `std::sync::MutexGuard` or `tokio::sync::MutexGuard` across an external `.await` point.

```rust
// BAD: Lock held across async HTTP call blocks all other accessors
let mut guard = state.lock().unwrap();
let data = fetch_remote_data(&guard.url).await?; // Deadlock/contention risk!
guard.last_result = data;

// GOOD: Read state, drop lock, await I/O, re-acquire lock for update
let url = {
    let guard = state.lock().unwrap();
    guard.url.clone()
};
let data = fetch_remote_data(&url).await?;
{
    let mut guard = state.lock().unwrap();
    guard.last_result = data;
}
```
