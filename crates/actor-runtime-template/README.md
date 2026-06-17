# Actor Runtime Template

> Message-passing concurrency using the actor pattern for concurrent, fault-tolerant systems.

## When to use

- Concurrent state machines that need isolation between components
- Event-driven architectures with supervised failure handling
- Systems requiring graceful shutdown and restart policies

## Quick start

```rust,ignore
use actor_runtime_template::{Actor, ActorMessage, ActorState, Lifecycle};
use tokio::sync::mpsc;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct Counter { count: u32 }

impl ActorState for Counter {}

#[tokio::main]
async fn main() {
    let (tx, rx) = mpsc::channel(64);

    tokio::spawn(async move {
        let actor = Actor::new(Counter { count: 0 });
        actor.run(rx).await;
    });

    tx.send(ActorMessage::Process("increment".to_string())).await.unwrap();
    tx.send(ActorMessage::Stop).await.unwrap();
}
```

## Configuration

| Type | Options | Description |
|------|---------|-------------|
| `RestartStrategy::Always` | Default | Always restart on failure |
| `RestartStrategy::Backoff` | `initial`, `max`, `max_retries` | Exponential backoff restart |
| `RestartStrategy::Never` | — | Stop on failure, no restart |
| `RestartStrategy::OnError` | `max_retries` | Restart only on specific errors |

## Architecture

- **`Actor<S>`** — Core actor with mailbox and state management
- **`ActorMessage<S>`** — Message enum: `Init`, `Process`, `GetState`, `Stop`, `Ping`
- **`ActorState`** — Trait for validating state transitions
- **`Supervisor<S>`** — Spawns and supervises child actors with restart policies
- **`RestartStrategy`** — Configurable failure handling (always, backoff, never, on-error)

## Features

- Async message handlers with backpressure (bounded channels)
- Supervision trees with configurable restart policies
- Graceful shutdown via `Stop` message
- Health check via `Ping` message
- Zero external runtime dependencies beyond `tokio`
