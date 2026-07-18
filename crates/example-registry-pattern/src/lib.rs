//! # Registry / Plugin Dispatch Pattern
//!
//! Demonstrates a `Registry<dyn Handler>` that routes named operations to
//! modular, independently testable handler implementations — without a
//! central `match` tree.
//!
//! ## When to use
//! - CLI tools where subcommands grow over time
//! - Plugin architectures where handlers are registered at startup
//! - Any system where the set of operations is open-ended
//!
//! ## Trade-offs vs `match`
//! | | `match` | Registry |
//! |---|---|---|
//! | Compile-time exhaustiveness | ✅ | ❌ |
//! | Runtime extensibility | ❌ | ✅ |
//! | Independent handler tests | harder | easy |
//! | Adding a new operation | edit central file | add new struct |

#![forbid(unsafe_code)]

use std::collections::HashMap;

/// Errors that can occur during command dispatch.
#[derive(Debug, thiserror::Error)]
pub enum DispatchError {
    /// The command was not found in the registry.
    #[error("unknown command: {0}")]
    Unknown(String),
    /// Invalid command input or identifier.
    #[error("invalid input: {0}")]
    InvalidInput(String),
    /// The handler returned an error.
    #[error("handler error: {0}")]
    Handler(String),
}

/// Implement this trait for each command/operation.
pub trait Handler: Send + Sync {
    /// Handle a command and return the result.
    fn handle(&self, input: &str) -> Result<String, DispatchError>;
}

/// Owns a map of command names → boxed handlers.
#[derive(Default)]
pub struct Registry {
    handlers: HashMap<&'static str, Box<dyn Handler>>,
}

impl Registry {
    /// Register a handler for the given command name.
    pub fn register(&mut self, name: &'static str, handler: Box<dyn Handler>) {
        self.handlers.insert(name, handler);
    }

    /// Dispatch a command to its registered handler.
    pub fn dispatch(&self, command: &str, input: &str) -> Result<String, DispatchError> {
        // Security: Validate command identifier to prevent log injection and resource exhaustion.
        const MAX_COMMAND_LEN: usize = 64;
        if command.len() > MAX_COMMAND_LEN {
            return Err(DispatchError::InvalidInput(
                "Command name too long".to_owned(),
            ));
        }

        for c in command.chars() {
            if c.is_control()
                || matches!(
                    c,
                    '\u{200b}'..='\u{200f}' // Zero-width space and Bidi controls
                        | '\u{2028}' // Line separator
                        | '\u{2029}' // Paragraph separator
                        | '\u{202a}'..='\u{202e}' // Bidi embedding/override
                        | '\u{2060}'..='\u{2064}' // Word joiner and invisible formatters
                        | '\u{2066}'..='\u{2069}' // Bidi isolate controls
                )
            {
                return Err(DispatchError::InvalidInput(
                    "Command name contains control or Bidi characters".to_owned(),
                ));
            }
        }

        self.handlers
            .get(command)
            .ok_or_else(|| DispatchError::Unknown(command.to_owned()))?
            .handle(input)
    }
}

// ── Example handlers ──────────────────────────────────────────────────────────

/// Handler that echoes the input back.
pub struct EchoHandler;
impl Handler for EchoHandler {
    fn handle(&self, input: &str) -> Result<String, DispatchError> {
        Ok(input.to_owned())
    }
}

/// Handler that reverses the input.
pub struct ReverseHandler;
impl Handler for ReverseHandler {
    fn handle(&self, input: &str) -> Result<String, DispatchError> {
        Ok(input.chars().rev().collect())
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, missing_docs)]

    use super::*;

    fn build_registry() -> Registry {
        let mut r = Registry::default();
        r.register("echo", Box::new(EchoHandler));
        r.register("reverse", Box::new(ReverseHandler));
        r
    }

    #[test]
    fn echo() {
        assert_eq!(build_registry().dispatch("echo", "hello").unwrap(), "hello");
    }

    #[test]
    fn reverse() {
        assert_eq!(build_registry().dispatch("reverse", "abc").unwrap(), "cba");
    }

    #[test]
    fn unknown_command() {
        assert!(matches!(
            build_registry().dispatch("nope", ""),
            Err(DispatchError::Unknown(_))
        ));
    }

    #[test]
    fn test_command_too_long() {
        let name = "a".repeat(65);
        let res = build_registry().dispatch(&name, "");
        assert!(matches!(res, Err(DispatchError::InvalidInput(_))));
        let err = res.unwrap_err().to_string();
        assert!(err.contains("Command name too long"));
    }

    #[test]
    fn test_command_control_chars() {
        let name = "cmd\nname";
        let res = build_registry().dispatch(name, "");
        assert!(matches!(res, Err(DispatchError::InvalidInput(_))));
        let err = res.unwrap_err().to_string();
        assert!(err.contains("contains control or Bidi characters"));
    }

    #[test]
    fn test_command_bidi_chars() {
        let name = "cmd\u{202a}name";
        let res = build_registry().dispatch(name, "");
        assert!(matches!(res, Err(DispatchError::InvalidInput(_))));
        let err = res.unwrap_err().to_string();
        assert!(err.contains("contains control or Bidi characters"));
    }
}
