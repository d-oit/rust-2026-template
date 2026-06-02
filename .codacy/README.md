# Codacy Configuration Directory

This directory contains configuration files for Codacy static analysis and related agent review skills.

## Structure

- `codacy.yml`: Main configuration for Codacy analysis (previously `.codacy.yml` at root).
- `plugins.json`: (Optional) Custom plugin configurations.
- `patterns.json`: (Optional) Custom pattern definitions.

## Agent Integration

Agent skills (e.g., `codacy` skill in `.agents/skills/`) may use this directory to store tuned linting profiles and triage rules specifically for Rust projects.
