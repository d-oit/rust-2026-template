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
    /// The command identifier is malformed (too long or contains forbidden characters).
    #[error("invalid command: {0}")]
    Invalid(String),
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
        // Security: Validate the command identifier before it can reach a log/error path.
        // The length cap is a *byte* budget (`str::len`), which bounds memory/resource use
        // even for multi-byte UTF-8 identifiers, and forbids control characters plus the
        // Unicode line/paragraph separators that `char::is_control` does not cover.
        const MAX_COMMAND_LEN: usize = 64;
        if command.len() > MAX_COMMAND_LEN {
            return Err(DispatchError::Invalid(format!(
                "command identifier exceeds {MAX_COMMAND_LEN} bytes"
            )));
        }

        if command
            .chars()
            .any(|c| c.is_control() || matches!(c, '\u{2028}' | '\u{2029}'))
        {
            return Err(DispatchError::Invalid(
                "command identifier contains control or line-separator characters".to_string(),
            ));
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
        let result = build_registry().dispatch(&name, "");
        assert!(
            matches!(result, Err(DispatchError::Invalid(msg)) if msg.contains("exceeds 64 bytes"))
        );
    }

    #[test]
    fn test_command_at_max_length_passes_validation() {
        // The byte budget is inclusive: exactly 64 bytes must pass validation and
        // reach the registry lookup (Unknown) rather than being rejected (Invalid).
        let name = "a".repeat(64);
        assert_eq!(name.len(), 64);
        assert!(matches!(
            build_registry().dispatch(&name, ""),
            Err(DispatchError::Unknown(_))
        ));
    }

    #[test]
    fn test_command_multibyte_byte_budget() {
        // 40 multi-byte chars exceed the 64-*byte* budget even though they are < 64 chars.
        let name = "🦀".repeat(40);
        assert!(name.chars().count() < 64);
        let result = build_registry().dispatch(&name, "");
        assert!(matches!(result, Err(DispatchError::Invalid(_))));
    }

    #[test]
    fn test_command_contains_control_chars() {
        for bad in ["command\nname", "command\rname", "command\tname"] {
            let result = build_registry().dispatch(bad, "");
            assert!(matches!(result, Err(DispatchError::Invalid(msg)) if msg.contains("control")));
        }
    }

    #[test]
    fn test_command_unicode_line_separator_rejected() {
        // U+2028 LINE SEPARATOR is not a control char; log-injection hardening must catch it.
        let result = build_registry().dispatch("command\u{2028}name", "");
        assert!(matches!(result, Err(DispatchError::Invalid(_))));
    }
}
