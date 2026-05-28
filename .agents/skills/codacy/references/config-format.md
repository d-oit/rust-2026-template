# Codacy Configuration Format

Codacy can be configured via a `.codacy.yml` or `.codacy.yaml` file in the repository root.

## Basic Structure

```yaml
---
engines:
  duplication:
    exclude_paths:
      - "**/tests/**"
    config:
      languages:
        - "rust"
  metric:
    exclude_paths:
      - "**/benches/**"
exclude_paths:
  - "target/**"
  - ".git/**"
  - "dist/**"
