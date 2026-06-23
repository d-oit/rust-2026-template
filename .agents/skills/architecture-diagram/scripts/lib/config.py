"""Theme and configuration constants for architecture diagram rendering."""

VIEWBOX_W = 1200
VIEWBOX_MARGIN = 30
CENTER_X = VIEWBOX_W // 2
RIGHT_EDGE = VIEWBOX_W - VIEWBOX_MARGIN
GAP_X = 40
GAP_Y = 30

THEME = {
    "colors": {
        "apps":      {"bg": "#fffbeb", "border": "#fcd34d", "text": "#92400e", "accent": "#f59e0b", "grad": ["#fffbeb", "#fef3c7", "#fde68a"]},
        "core":      {"bg": "#eff6ff", "border": "#93c5fd", "text": "#1e40af", "accent": "#3b82f6", "grad": ["#eff6ff", "#dbeafe", "#bfdbfe"]},
        "templates": {"bg": "#faf5ff", "border": "#d8b4fe", "text": "#6b21a8", "accent": "#a855f7", "grad": ["#faf5ff", "#f3e8ff", "#e9d5ff"]},
        "other":     {"bg": "#f0fdf4", "border": "#86efac", "text": "#166534", "accent": "#22c55e", "grad": ["#f0fdf4", "#dcfce7", "#bbf7d0"]},
        "pipeline":  {"bg": "#f0fdfa", "border": "#5eead4", "text": "#0f766e", "accent": "#14b8a6", "grad": ["#f0fdfa", "#ccfbf1", "#99f6e4"]},
        "interface": {"bg": "#f8fafc", "border": "#cbd5e1", "text": "#334155", "accent": "#64748b", "grad": ["#f8fafc", "#f1f5f9", "#e2e8f0"]},
        "rose":      {"bg": "#fff1f2", "border": "#fda4af", "text": "#9f1239", "accent": "#f43f5e", "grad": ["#fff1f2", "#ffe4e6", "#fecdd3"]},
        "teal":      {"bg": "#f0fdfa", "border": "#5eead4", "text": "#0f766e", "accent": "#14b8a6", "grad": ["#f0fdfa", "#ccfbf1", "#99f6e4"]},
        "blue":      {"bg": "#eff6ff", "border": "#93c5fd", "text": "#1e40af", "accent": "#3b82f6", "grad": ["#eff6ff", "#dbeafe", "#bfdbfe"]},
        "green":     {"bg": "#f0fdf4", "border": "#86efac", "text": "#166534", "accent": "#22c55e", "grad": ["#f0fdf4", "#dcfce7", "#bbf7d0"]},
        "divider":   "#e2e8f0",
        "arrow":     "#94a3b8",
        "bg":        "#ffffff",
        "muted":     "#f8fafc",
        "surface":   "#ffffff",
    },
    "font": "'Inter', -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif",
    "radius": 16,
}

DEFAULT_CONFIG = {
    "title": "SYSTEM ARCHITECTURE",
    "project_name": "RUST 2026 WORKSPACE",
    "author": "MAINTAINER",
    "pipeline_stages": [
        {"name": "ANALYZE", "color": "teal", "desc": "lint · clippy", "timing": "Every push · ~2 min"},
        {"name": "VALIDATE", "color": "blue", "desc": "test · nextest", "timing": "Every push · ~5 min"},
        {"name": "HARDEN", "color": "rose", "desc": "audit · deny", "timing": "Weekly + pre-release"},
        {"name": "DEPLOY", "color": "green", "desc": "release · publish", "timing": "Tag-triggered · ~4 min"},
    ],
}

DEFAULT_DESCRIPTIONS = {
    "benchmarks": "Criterion benchmarks for workspace crates",
    "hello-world-example": "Minimal hello world example crate",
}
