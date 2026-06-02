# Configuration Profiles

This directory contains environment-specific or profile-based configuration files.

## Profile Pattern

A common pattern for multi-environment Rust applications is to load a base configuration and then override it with profile-specific settings.

- `default.json`: Base configuration shared across all environments.
- `production.json`: Overrides for production environment.
- `local.json`: Local developer overrides (ignored by git, see `.gitignore`).

## Usage

Applications can use the `config` crate or similar to merge these JSON files at runtime based on an environment variable (e.g., `APP_PROFILE=production`).
