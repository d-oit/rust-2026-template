# Schema Directory

This directory is used for storing JSON Schema definitions for configuration validation, API contracts, and other structured data formats used in the project.

## Purpose

- **Validation:** Provide a single source of truth for structured data formats.
- **Contract Testing:** Ensure consistency between different parts of the system or external integrations.
- **Code Generation:** Optionally generate types or validation logic from these schemas.

## Schema Inventory

- `agent-adapters.schema.json` - Schema for agent framework adapters (`.agents/agent-adapters.toml`)
- `ci-telemetry.schema.json` - Schema for CI quality-run telemetry (`.agents/ci/quality-run.json`)
- `config.schema.json` - Schema for application configuration
- `template-profile.schema.json` - Schema for blueprint template profiles (`config/template-profiles/*.toml`)
- `xtask-config.schema.json` - Schema for CI verification tiers configuration (`config/xtask.json`)

## Usage

Store your `.schema.json` files here. For example, `xtask-config.schema.json` is used to validate the CI verification tiers configuration in `config/xtask.json`.
