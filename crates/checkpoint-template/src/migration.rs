//! Migration utilities for schema evolution.

use serde_json::Value;
use thiserror::Error;

/// Migration error types.
#[derive(Debug, Error)]
pub enum MigrationError {
    /// Unknown field in migration.
    #[error("Unknown field during migration: {0}")]
    UnknownField(String),

    /// Data corruption detected.
    #[error("Data corruption during migration: {0}")]
    Corruption(String),

    /// Version not supported.
    #[error("Version {0} not supported for migration")]
    UnsupportedVersion(u32),
}

/// Migration registry for handling schema changes.
pub struct MigrationRegistry {
    migrations: Vec<(u32, u32)>,
}

impl Default for MigrationRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl MigrationRegistry {
    /// Create a new migration registry.
    pub const fn new() -> Self {
        Self {
            migrations: Vec::new(),
        }
    }

    /// Register a migration path.
    pub fn register(&mut self, from: u32, to: u32) {
        self.migrations.push((from, to));
    }

    /// Check if migration is possible.
    pub fn can_migrate(&self, from: u32, to: u32) -> bool {
        self.migrations.iter().any(|(f, t)| *f == from && *t == to)
    }

    /// Apply migration to data.
    pub fn migrate(&self, data: Value, from: u32, to: u32) -> Result<Value, MigrationError> {
        if !self.can_migrate(from, to) {
            return Err(MigrationError::UnsupportedVersion(from));
        }

        // Identity migration (pass through)
        // Override in concrete implementations
        Ok(data)
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, missing_docs)]

    use super::*;

    #[test]
    fn test_migration_registry() {
        let mut registry = MigrationRegistry::new();
        registry.register(1, 2);

        assert!(registry.can_migrate(1, 2));
        assert!(!registry.can_migrate(2, 3));
    }

    #[test]
    fn test_migration_data() {
        let mut registry = MigrationRegistry::new();
        registry.register(1, 2);

        let data = serde_json::json!({"value": 42});
        let result = registry.migrate(data.clone(), 1, 2).unwrap();
        assert_eq!(result, data);
    }
}
