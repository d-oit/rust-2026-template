# Actor Runtime Template

A template crate demonstrating the actor model pattern using message passing for concurrent, fault-tolerant systems.

## Usage

```rust
use actor_runtime_template::{Actor, ActorMessage, ActorState, Lifecycle, RestartStrategy};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct MyState {
    counter: u32,
}

impl ActorState for MyState {}

#[tokio::main]
async fn main() {
    let (tx, rx) = tokio::sync::mpsc::channel(64);
    
    // Spawn actor
    tokio::spawn(async move {
        let actor = Actor::new(MyState { counter: 0 });
        let _ = actor.run(rx).await;
    });
    
    // Send message
    tx.send(ActorMessage::Process("increment".to_string())).await.unwrap();
}
```

## Architecture

- `Actor` - Core actor type with message handling
- `ActorMessage` - Message enum with Init, Process, GetState, Stop variants
- `ActorState` - Trait for validating state transitions
- `RestartStrategy` - Policy for handling actor failures