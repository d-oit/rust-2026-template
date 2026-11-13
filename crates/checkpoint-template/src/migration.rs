//! Migration utilities for schema evolution.

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
///
/// A registry records which version transitions exist and applies the
/// registered transform for each step of a chain (for example 1 -> 2 -> 3).
/// It deliberately does not decode payloads itself: the stored format is
/// bincode, and only the concrete type knows its own layout. Callers supply
/// the per-step transform, typically a thin wrapper over bincode plus the
/// type's own migration logic.
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
    #[must_use]
    pub const fn new() -> Self {
        Self {
            migrations: Vec::new(),
        }
    }

    /// Register a migration path.
    pub fn register(&mut self, from: u32, to: u32) {
        self.migrations.push((from, to));
    }

    /// Check if migration is possible in one step.
    #[must_use]
    pub fn can_migrate(&self, from: u32, to: u32) -> bool {
        self.migrations.iter().any(|(f, t)| *f == from && *t == to)
    }

    /// Resolve the ordered version chain from `from` to `to`.
    ///
    /// # Errors
    ///
    /// Returns [`MigrationError::UnsupportedVersion`] when no registered path
    /// reaches `to`.
    pub fn path(&self, from: u32, to: u32) -> Result<Vec<(u32, u32)>, MigrationError> {
        if from == to {
            return Ok(Vec::new());
        }
        let mut chain = Vec::new();
        let mut current = from;
        // Bound the walk by the number of registered edges so a malformed
        // registry (or a cycle) cannot loop forever.
        for _ in 0..=self.migrations.len() {
            let Some((_, next)) = self.migrations.iter().find(|(f, _)| *f == current) else {
                return Err(MigrationError::UnsupportedVersion(from));
            };
            chain.push((current, *next));
            current = *next;
            if current == to {
                return Ok(chain);
            }
        }
        Err(MigrationError::UnsupportedVersion(from))
    }

    /// Apply every registered step from `from` to `to`.
    ///
    /// `step` receives the intermediate payload and the version pair for that
    /// step, and returns the payload in the next version's layout.
    ///
    /// # Errors
    ///
    /// Returns [`MigrationError::UnsupportedVersion`] when no path exists, and
    /// propagates any error from `step`.
    pub fn migrate_with<F>(
        &self,
        payload: &[u8],
        from: u32,
        to: u32,
        mut step: F,
    ) -> Result<Vec<u8>, MigrationError>
    where
        F: FnMut(&[u8], u32, u32) -> Result<Vec<u8>, MigrationError>,
    {
        let chain = self.path(from, to)?;
        let mut current = payload.to_vec();
        for (step_from, step_to) in chain {
            current = step(&current, step_from, step_to)?;
        }
        Ok(current)
    }
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::panic,
        missing_docs,
        reason = "test modules assert on known-good fixtures, so panicking macros are the intended failure signal"
    )]

    use super::*;

    #[test]
    fn test_migration_registry() {
        let mut registry = MigrationRegistry::new();
        registry.register(1, 2);

        assert!(registry.can_migrate(1, 2));
        assert!(!registry.can_migrate(2, 3));
    }

    #[test]
    fn test_migration_registry_chains_steps() {
        let mut registry = MigrationRegistry::new();
        registry.register(1, 2);
        registry.register(2, 3);

        let chain = registry.path(1, 3).unwrap();
        assert_eq!(chain, vec![(1, 2), (2, 3)]);
    }

    #[test]
    fn test_migration_registry_unsupported_path() {
        let registry = MigrationRegistry::new();
        assert!(matches!(
            registry.path(1, 2),
            Err(MigrationError::UnsupportedVersion(1))
        ));
    }

    #[test]
    fn test_migration_registry_applies_each_step() {
        let mut registry = MigrationRegistry::new();
        registry.register(1, 2);
        registry.register(2, 3);

        let applied = registry
            .migrate_with(b"a", 1, 3, |payload, from, to| {
                let mut next = payload.to_vec();
                next.push(b'0' + u8::try_from(to).unwrap());
                assert_eq!(to, from + 1);
                Ok(next)
            })
            .unwrap();

        assert_eq!(applied, b"a23");
    }

    #[test]
    fn test_migration_registry_same_version_is_noop() {
        let registry = MigrationRegistry::new();
        let applied = registry
            .migrate_with(b"unchanged", 1, 1, |_, _, _| {
                Err(MigrationError::Corruption("must not run".into()))
            })
            .unwrap();
        assert_eq!(applied, b"unchanged");
    }

    #[test]
    fn test_migration_registry_cycle_terminates() {
        let mut registry = MigrationRegistry::new();
        registry.register(1, 2);
        registry.register(2, 1);

        // A cycle with no path to 3 must error, not hang.
        assert!(matches!(
            registry.path(1, 3),
            Err(MigrationError::UnsupportedVersion(1))
        ));
    }
}
