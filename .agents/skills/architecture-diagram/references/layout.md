# Architecture Diagram — Layout Reference

## Layout Modes

The generator supports two layout modes:

### 1. Graphviz Auto-Layout (default)

Uses `dot -Tjson` to compute node positions by dependency hierarchy.

```bash
python scripts/generate_diagram.py --root . --out .template/architecture.svg
```

- Requires Graphviz installed (`dot -V` to verify)
- Nodes placed by dependency hierarchy (top-down)
- Edges routed as cubic Bezier curves
- Falls back to grid if Graphviz unavailable

### 2. Grid Layout (fallback)

Enhanced grid with obstacle-avoiding edge routing.

```bash
python scripts/generate_diagram.py --root . --out .template/architecture.svg --no-graphviz
```

- No external dependencies
- 2-column grid with 340px card width
- Edges route around obstacle cards using waypoints

## SVG Output Format

- **ViewBox:** 1200px wide, dynamic height (~3000px)
- **Font:** Inter (Google Fonts import), fallback to system fonts
- **Colors:** 10-color gradient palette (apps, core, templates, other, pipeline, interface, rose, teal, blue, green)
- **Accessibility:** ARIA labels, semantic `<g>` groups, `role` attributes
- **Shadows:** Drop shadows on cards (`feDropShadow`)

## Text Readability Specs

| Class | Font Size | Weight | Min Opacity | Use |
|-------|-----------|--------|-------------|-----|
| `.tl` | 24px | 700 | 0.6 | Title |
| `.th` | 13px | 600 | — | Card headers |
| `.ts` | 12.5px | 500 | 0.7 | Body text |
| `.txs` | 11px | 500 | 0.5 | Secondary text |
| `.section-label` | 11px | 700 | — | Section headers |

## Overlap Detection

Post-layout validation checks all crate card bounding boxes. If collisions are detected, cards are pushed apart by 15px until clear. Max 10 iterations.

## Dependency Arrows

- **Color:** `#6366f1` (indigo), 60% opacity, dashed
- **Routing:** Cubic Bezier (Graphviz mode) or obstacle-avoiding waypoints (grid mode)
- **Markers:** Arrow markers at target end
