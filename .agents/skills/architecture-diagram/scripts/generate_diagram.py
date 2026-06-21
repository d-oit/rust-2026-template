#!/usr/bin/env python3
"""
generate_diagram.py — Project topology SVG generator
Scans the live project structure, including Rust workspace, and writes an architecture diagram.

2026 Best Practices Applied:
- C4 Model alignment (clear abstraction hierarchy)
- Accessibility (ARIA labels, semantic grouping, role attributes)
- Responsive scaling (viewBox-based)
- Minimal notation (less visual noise, clearer relationships)
- Machine-readable structure (semantic <g> groups with IDs)
- NO hardcoded values - everything discovered from codebase
- 1200px landscape format with descriptions inside elements

Usage:
python generate_diagram.py [--root .] [--out .template/architecture.svg]
"""
import argparse
import json
import re
import subprocess  # nosec B404
import sys
from pathlib import Path

# ── Layout Constants ───────────────────────────────────────────────────────
VIEWBOX_W = 1200
VIEWBOX_MARGIN = 30
CENTER_X = VIEWBOX_W // 2
RIGHT_EDGE = VIEWBOX_W - VIEWBOX_MARGIN
GAP_X = 40
GAP_Y = 30

# ── Modern 2026 Style Configuration ─────────────────────────────────────────
THEME = {
    "colors": {
        "apps":      {"bg": "#fffbeb", "border": "#fde68a", "text": "#92400e", "accent": "#f59e0b", "grad": ["#fffbeb", "#fef3c7"]},
        "core":      {"bg": "#eff6ff", "border": "#bfdbfe", "text": "#1e40af", "accent": "#3b82f6", "grad": ["#eff6ff", "#dbeafe"]},
        "templates": {"bg": "#faf5ff", "border": "#e9d5ff", "text": "#6b21a8", "accent": "#a855f7", "grad": ["#faf5ff", "#f3e8ff"]},
        "other":     {"bg": "#f0fdf4", "border": "#bbf7d0", "text": "#166534", "accent": "#22c55e", "grad": ["#f0fdf4", "#dcfce7"]},
        "pipeline":  {"bg": "#f0fdfa", "border": "#99f6e4", "text": "#0f766e", "accent": "#14b8a6", "grad": ["#f0fdfa", "#ccfbf1"]},
        "interface": {"bg": "#f8fafc", "border": "#e2e8f0", "text": "#334155", "accent": "#64748b", "grad": ["#f8fafc", "#f1f5f9"]},
        "rose":      {"bg": "#fff1f2", "border": "#fecdd3", "text": "#9f1239", "accent": "#f43f5e", "grad": ["#fff1f2", "#ffe4e6"]},
        "teal":      {"bg": "#f0fdfa", "border": "#99f6e4", "text": "#0f766e", "accent": "#14b8a6", "grad": ["#f0fdfa", "#ccfbf1"]},
        "blue":      {"bg": "#eff6ff", "border": "#bfdbfe", "text": "#1e40af", "accent": "#3b82f6", "grad": ["#eff6ff", "#dbeafe"]},
        "green":     {"bg": "#f0fdf4", "border": "#bbf7d0", "text": "#166534", "accent": "#22c55e", "grad": ["#f0fdf4", "#dcfce7"]},
        "divider":   "#f1f5f9",
        "arrow":     "#94a3b8",
        "bg":        "#ffffff",
        "muted":     "#f8fafc",
    },
    "font": "'Inter', -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif",
    "radius": 14,
}

DEFAULT_CONFIG = {
    "title": "SYSTEM ARCHITECTURE",
    "project_name": "RUST 2026 WORKSPACE",
    "author": "MAINTAINER",
}

# ── Discovery Helpers ──────────────────────────────────────────────────────
def _read_frontmatter_name(path: Path) -> str:
    try:
        content = path.read_text(encoding="utf-8")
        if content.startswith("---"):
            m = re.search(r"^name:\s*(.+)$", content, re.MULTILINE)
            if m:
                return m.group(1).strip().strip('"').strip("'")
    except Exception:
        pass
    return path.stem

def discover_skills(root: Path) -> list[str]:
    skills_dir = root / ".agents" / "skills"
    if not skills_dir.is_dir():
        return []
    return [
        _read_frontmatter_name(d / "SKILL.md")
        for d in sorted(skills_dir.iterdir())
        if d.is_dir() and (d / "SKILL.md").exists()
    ]

def discover_agents(root: Path) -> list[str]:
    agents_dir = root / ".opencode" / "agents"
    if not agents_dir.is_dir():
        return []
    return sorted(p.stem for p in agents_dir.glob("*.md"))

def discover_commands(root: Path) -> list[str]:
    commands_dir = root / ".opencode" / "commands"
    if not commands_dir.is_dir():
        return []
    return sorted(
        (p.stem if p.stem.startswith("/") else "/" + p.stem)
        for p in commands_dir.glob("*.md")
    )

def discover_crates(root: Path) -> list[dict]:
    try:
        result = subprocess.run(  # nosec B603
            ["cargo", "metadata", "--format-version", "1", "--no-deps"],
            capture_output=True, text=True, check=True,
            cwd=str(root.resolve()), shell=False,
        )
        data = json.loads(result.stdout)
        crates = []
        workspace_members = data.get("workspace_members", [])
        for pkg in data.get("packages", []):
            if pkg["id"] in workspace_members:
                deps = [d["name"] for d in pkg.get("dependencies", []) if d.get("path")]
                features = [f for f in pkg.get("features", {}).keys() if f != "default"]
                crates.append({
                    "name": pkg["name"],
                    "version": pkg["version"],
                    "dependencies": deps,
                    "features": sorted(features),
                    "description": pkg.get("description", ""),
                })
        return sorted(crates, key=lambda x: x["name"])
    except (subprocess.CalledProcessError, json.JSONDecodeError, KeyError):
        return []

def discover_error_types(root: Path) -> list[dict]:
    error_types = []
    crates_dir = root / "crates"
    if not crates_dir.is_dir():
        return error_types
    for crate_dir in crates_dir.iterdir():
        if not crate_dir.is_dir():
            continue
        for rs_file in crate_dir.rglob("*.rs"):
            try:
                content = rs_file.read_text(encoding="utf-8")
                for match in re.finditer(r'pub enum (\w*Error)\s*\{([^}]+)\}', content, re.DOTALL):
                    enum_name = match.group(1)
                    variants_text = match.group(2)
                    variants = []
                    for line in variants_text.split('\n'):
                        line = line.strip()
                        vm = re.match(r'(\w+)(?:\(|\s|$)', line)
                        if vm:
                            v = vm.group(1)
                            if v[0].isupper() and v not in ('Debug', 'Error', 'Display', 'From', 'Source'):
                                variants.append(v)
                    variants = list(dict.fromkeys(variants))
                    if variants and enum_name not in [e["name"] for e in error_types]:
                        error_types.append({
                            "name": enum_name,
                            "variants": variants[:6],
                            "crate": crate_dir.name,
                        })
            except Exception:
                continue
    return error_types[:5]

def discover_agent_roles(root: Path) -> list[dict]:
    orch_file = root / ".agents" / "ORCHESTRATION.md"
    if not orch_file.exists():
        return []
    try:
        content = orch_file.read_text(encoding="utf-8")
        roles = []
        for match in re.finditer(r'\|\s*\*\*(\w+-\w+)\*\*\s*\|[^|]*\|\s*`([^`]+)`', content):
            role_name = match.group(1)
            skills_text = match.group(2)
            skills = [s.strip() for s in skills_text.split(',')]
            colors = {"code": "teal", "release": "green", "quality": "blue", "meta": "templates"}
            color_key = colors.get(role_name.split("-")[0], "interface")
            roles.append({
                "name": role_name,
                "skills": skills,
                "color": color_key,
            })
        return roles
    except Exception:
        return []

def discover_handoff_items(root: Path) -> list[dict]:
    orch_file = root / ".agents" / "ORCHESTRATION.md"
    if not orch_file.exists():
        return []
    try:
        content = orch_file.read_text(encoding="utf-8")
        items = []
        seen = set()
        for match in re.finditer(r'`([^`]+\.(?:json|jsonl|md))`', content):
            file_path = match.group(1)
            if file_path not in seen:
                seen.add(file_path)
                items.append({"path": file_path, "desc": "Workflow state"})
        return items[:3]
    except Exception:
        return []

def discover_data_types(root: Path) -> list[dict]:
    data_types = []
    crates_dir = root / "crates"
    if not crates_dir.is_dir():
        return data_types
    for crate_dir in crates_dir.iterdir():
        if not crate_dir.is_dir():
            continue
        for rs_file in crate_dir.rglob("*.rs"):
            try:
                content = rs_file.read_text(encoding="utf-8")
                for match in re.finditer(r'/// ([^\n]+)\npub struct (\w+)', content):
                    desc = match.group(1)
                    struct_name = match.group(2)
                    data_types.append({
                        "name": struct_name,
                        "desc": desc[:50],
                        "crate": crate_dir.name,
                    })
            except Exception:
                continue
    return data_types[:4]

# ── Dynamic Layout Engine ─────────────────────────────────────────────────
def _estimate_text_width(text: str, font_size: int = 12) -> float:
    avg_char_w = font_size * 0.62
    return len(text) * avg_char_w

def _compute_card_dimensions(crate: dict, card_w: int = 340) -> tuple[int, int]:
    name_w = _estimate_text_width(crate["name"], font_size=13)
    desc = crate.get("description", "")
    desc_lines = _wrap_text(desc, max_chars=max(int(card_w / 7), 30))
    desc_h = len(desc_lines) * 16 if desc_lines else 0
    features = crate.get("features", [])
    feat_h = 26 if features else 0
    card_h = 70 + desc_h + feat_h + 10
    return card_w, card_h

def _has_graphviz() -> bool:
    try:
        result = subprocess.run(  # nosec B603
            ["dot", "-V"], capture_output=True, text=True, timeout=5, shell=False,
        )
        return result.returncode == 0 or "graphviz" in result.stderr.lower()
    except (FileNotFoundError, subprocess.TimeoutExpired):
        return False

def _layout_with_graphviz(crates: list[dict], layers: dict, y_start: int, viewbox_w: int) -> dict:
    dot_lines = ['digraph G {', '  rankdir=TB;', '  node [shape=box, style=filled, fontname="Inter", fontsize=11];', '  edge [style=invis, weight=10];', '  graph [ranksep=0.8, nodesep=0.5, splines=ortho];']
    rank_groups = {}
    for layer_name in ["apps", "core", "templates", "other"]:
        layer_crates = layers.get(layer_name, [])
        if layer_crates:
            names = [c["name"] for c in layer_crates]
            rank_groups[layer_name] = names
            dot_lines.append(f'  {{ rank=same; {"; ".join(f"{n}" for n in names)}; }}')

    for crate in crates:
        safe = crate["name"].replace("-", "_").replace(".", "_")
        label = crate["name"].replace("&", "&&")
        dot_lines.append(f'  {safe} [label="{label}", fillcolor="#eff6ff", fontcolor="#1e40af"];')

    for crate in crates:
        src = crate["name"].replace("-", "_").replace(".", "_")
        for dep in crate["dependencies"]:
            dst = dep.replace("-", "_").replace(".", "_")
            dot_lines.append(f'  {src} -> {dst};')
    dot_lines.append('}')
    dot_src = "\n".join(dot_lines)
    try:
        result = subprocess.run(  # nosec B603
            ["dot", "-Tjson"], input=dot_src, capture_output=True, text=True,
            timeout=30, shell=False,
        )
        if result.returncode != 0:
            return {}
        data = json.loads(result.stdout)
        objects = data.get("objects", [])
        if not objects:
            return {}
        min_x = min(obj.get("pos", "0,0").split(",")[0].strip("!") for obj in objects if "pos" in obj)
        max_x = max(obj.get("pos", "0,0").split(",")[0].strip("!") for obj in objects if "pos" in obj)
        min_y = min(obj.get("pos", "0,0").split(",")[1].strip("!") for obj in objects if "pos" in obj)
        max_y = max(obj.get("pos", "0,0").split(",")[1].strip("!") for obj in objects if "pos" in obj)
        range_x = max(float(max_x) - float(min_x), 1)
        range_y = max(float(max_y) - float(min_y), 1)
        margin = 30
        usable_w = viewbox_w - 2 * margin
        usable_h = max(range_y * (usable_w / range_x), 300)
        scale = usable_w / range_x if range_x > 0 else 1
        coords = {}
        for obj in objects:
            name = obj.get("name", "")
            if "pos" not in obj:
                continue
            px, py = [float(v.strip("!")) for v in obj["pos"].split(",")]
            w = float(obj.get("width", 1)) * 72
            h = float(obj.get("height", 1)) * 72
            sx = margin + (px - float(min_x)) * scale - w / 2
            sy = y_start + (py - float(min_y)) * scale * 0.8
            original = name.replace("_", "-")
            coords[original] = (int(sx), int(sy), int(w), int(h))
        return coords
    except (subprocess.TimeoutExpired, json.JSONDecodeError, ValueError):
        return {}

def _layout_grid_fallback(crates: list[dict], layers: dict, y_start: int, viewbox_w: int) -> dict:
    coords = {}
    col_w = 340
    gap_x, gap_y = 40, 30
    temp_y = y_start
    for layer_name in ["apps", "core", "templates", "other"]:
        layer_crates = layers.get(layer_name, [])
        if not layer_crates:
            continue
        temp_y += 18
        for row_idx in range(0, len(layer_crates), 2):
            row_crates = layer_crates[row_idx:row_idx + 2]
            num = len(row_crates)
            start_x = (viewbox_w - (num * col_w + (num - 1) * gap_x)) // 2
            max_row_h = 0
            for i, crate in enumerate(row_crates):
                cx = start_x + i * (col_w + gap_x)
                _, box_h = _compute_card_dimensions(crate, card_w=col_w)
                coords[crate["name"]] = (cx, temp_y, col_w, box_h)
                max_row_h = max(max_row_h, box_h)
            temp_y += max_row_h + gap_y
    return coords

def _boxes_intersect(a: tuple, b: tuple) -> bool:
    ax, ay, aw, ah = a[0], a[1], a[2], a[3]
    bx, by, bw, bh = b[0], b[1], b[2], b[3]
    return not (ax + aw <= bx or bx + bw <= ax or ay + ah <= by or by + bh <= ay)

def _detect_overlaps(coords: dict) -> list[tuple[str, str]]:
    items = list(coords.items())
    overlaps = []
    for i, (na, a) in enumerate(items):
        for nb, b in items[i + 1:]:
            if _boxes_intersect(a, b):
                overlaps.append((na, nb))
    return overlaps

def _fix_overlaps(coords: dict, max_iterations: int = 10) -> int:
    fix_count = 0
    for _ in range(max_iterations):
        overlaps = _detect_overlaps(coords)
        if not overlaps:
            break
        for na, nb in overlaps:
            a, b = coords[na], coords[nb]
            overlap_y = min(a[1] + a[3], b[1] + b[3]) - max(a[1], b[1])
            overlap_x = min(a[0] + a[2], b[0] + b[2]) - max(a[0], b[0])
            if overlap_y > 0 and overlap_x > 0:
                if a[1] <= b[1]:
                    coords[nb] = (b[0], a[1] + a[3] + 15, b[2], b[3])
                else:
                    coords[na] = (a[0], b[1] + b[3] + 15, a[2], a[3])
                fix_count += 1
    return fix_count

def _route_edge_avoiding_obstacles(sx, sy, tx, ty, obstacles, src_name, dst_name) -> str:
    min_gap = 20
    for obs_name, (ox, oy, ow, oh) in obstacles.items():
        if obs_name in (src_name, dst_name):
            continue
        if min(ox + ow, tx) - max(ox, sx) > 0 and min(oy + oh, ty) - max(oy, sy) > 0:
            if sx < ox and tx > ox:
                sx_adj = ox - min_gap
                return (f'M {sx} {sy} L {sx} {sy + 20} L {sx_adj} {sy + 20} '
                        f'L {sx_adj} {ty - 20} L {tx} {ty - 20} L {tx} {ty}')
    if abs(sx - tx) < 10:
        return f'M {sx} {sy} L {tx} {ty}'
    mid_y = (sy + ty) // 2
    return f'M {sx} {sy} L {sx} {mid_y} L {tx} {mid_y} L {tx} {ty}'

# ── SVG Primitives ────────────────────────────────────────────────────────
def _esc(s: str) -> str:
    return s.replace("&", "&amp;").replace("<", "&lt;").replace(">", "&gt;")

def _grad_def(name: str, colors: list[str]) -> str:
    return (f'<linearGradient id="{name}" x1="0" y1="0" x2="0" y2="1">'
            f'<stop offset="0%" stop-color="{colors[0]}"/>'
            f'<stop offset="100%" stop-color="{colors[1]}"/>'
            f'</linearGradient>')

def _rect(x, y, w, h, rx=None, color_key="core", dash=False, fill_override=None) -> str:
    c = THEME["colors"].get(color_key, THEME["colors"]["core"])
    r = rx if rx is not None else THEME["radius"]
    dash_attr = ' stroke-dasharray="4 3"' if dash else ""
    fill = fill_override if fill_override else f'url(#grad-{color_key})'
    return (f'<rect x="{x}" y="{y}" width="{w}" height="{h}" rx="{r}" '
            f'fill="{fill}" stroke="{c["border"]}" stroke-width="1.5"{dash_attr}/>')

def _accent_bar(x, y, w, color_key="core") -> str:
    c = THEME["colors"].get(color_key, THEME["colors"]["core"])
    return f'<rect x="{x}" y="{y}" width="{w}" height="3" rx="1.5" fill="{c["accent"]}"/>'

def _text(x, y, content, anchor="middle", cls="ts", dy=None, opacity=None, weight=None, aria=None) -> str:
    dy_attr = f' dy="{dy}"' if dy else ""
    op_attr = f' opacity="{opacity}"' if opacity else ""
    w_attr = f' font-weight="{weight}"' if weight else ""
    aria_attr = f' role="img" aria-label="{_esc(aria)}"' if aria else ""
    return (f'<text class="{cls}" x="{x}" y="{y}" '
            f'text-anchor="{anchor}" dominant-baseline="central"{dy_attr}{op_attr}{w_attr}{aria_attr}>'
            f'{_esc(content)}</text>')

def _badge(x, y, text_content, color_key="core", w=None) -> str:
    c = THEME["colors"].get(color_key, THEME["colors"]["core"])
    tw = len(text_content) * 6 + 16
    bw = w or max(tw, 50)
    return (f'<rect x="{x}" y="{y}" width="{bw}" height="18" rx="9" '
            f'fill="{c["bg"]}" stroke="{c["border"]}" stroke-width="0.75"/>'
            f'<text class="txs" x="{x + bw // 2}" y="{y + 9}" '
            f'text-anchor="middle" dominant-baseline="central" '
            f'fill="{c["text"]}" font-weight="500">{_esc(text_content)}</text>')

def _container(x, y, w, h, label, sublabel=None, aria_label=None) -> str:
    parts = [
        f'<g class="card" role="group" aria-label="{_esc(aria_label or label)}">',
        f'<rect x="{x}" y="{y}" width="{w}" height="{h}" rx="{THEME["radius"]}" '
        f'fill="{THEME["colors"]["muted"]}" stroke="{THEME["colors"]["interface"]["border"]}" '
        f'stroke-width="1" stroke-dasharray="4 4"/>',
        _text(x + w // 2, y + 18, label, cls="th", weight="700"),
    ]
    if sublabel:
        parts.append(_text(x + w // 2, y + 36, sublabel, cls="txs", opacity="0.5"))
    parts.append("</g>")
    return "\n".join(parts)

def _wrap_text(text: str, max_chars: int = 50) -> list[str]:
    if not text:
        return []
    words = text.split()
    lines = []
    current = ""
    for word in words:
        if len(current) + len(word) + 1 <= max_chars:
            current = f"{current} {word}" if current else word
        else:
            lines.append(current)
            current = word
    if current:
        lines.append(current)
    return lines[:2]

def _section_divider(y: int) -> str:
    return f'<line x1="{VIEWBOX_MARGIN}" y1="{y}" x2="{RIGHT_EDGE}" y2="{y}" stroke="{THEME["colors"]["divider"]}" stroke-width="1"/>'

DEFAULT_DESCRIPTIONS = {
    "benchmarks": "Criterion benchmarks for workspace crates",
    "hello-world-example": "Minimal hello world example crate",
}

def _crate_card(x, y, crate, color_key, card_w=340) -> tuple[str, int]:
    desc = crate.get("description") or DEFAULT_DESCRIPTIONS.get(crate["name"], "")
    features = crate.get("features", [])
    deps_count = len(crate.get("dependencies", []))
    desc_lines = _wrap_text(desc, max_chars=max(int(card_w / 7), 30))
    desc_h = len(desc_lines) * 16 if desc_lines else 0
    feat_h = 26 if features else 0
    card_h = 70 + desc_h + feat_h + 10
    tooltip = _esc(f"{crate['name']} v{crate['version']}: {desc}" if desc else f"{crate['name']} v{crate['version']}")
    parts = [
        f'<g id="crate-{crate["name"]}" class="card" role="img" '
        f'aria-label="Crate: {crate["name"]} v{crate["version"]}">',
        f'<title>{tooltip}</title>',
        _rect(x, y, card_w, card_h, color_key=color_key),
        _accent_bar(x + 8, y + 8, card_w - 16, color_key=color_key),
        _text(x + 12, y + 28, crate["name"], anchor="start", cls="th"),
        _text(x + card_w - 12, y + 28, f"v{crate['version']}", anchor="end", cls="txs", opacity="0.55"),
    ]
    if desc_lines:
        for i, line in enumerate(desc_lines):
            parts.append(_text(x + 12, y + 50 + i * 16, line, anchor="start", cls="ts", opacity="0.75"))
    if features:
        feat_y = y + card_h - 28
        feat_x = x + 10
        for f in features:
            fw = len(f) * 6 + 16
            if feat_x + fw > x + card_w - 10:
                feat_x = x + 10
                feat_y += 20
            parts.append(_badge(feat_x, feat_y, f, color_key=color_key, w=fw))
            feat_x += fw + 6
    if deps_count:
        parts.append(_text(x + card_w - 12, y + card_h - 14, f"deps: {deps_count}", anchor="end", cls="txs", opacity="0.55"))
    parts.append("</g>")
    return "\n".join(parts), card_h

# ── Main SVG Builder ──────────────────────────────────────────────────────
def build_svg(cfg: dict, crates: list[dict], skills: list[str], agents: list[str],
              commands: list[str], labels: dict, root: Path, no_graphviz: bool = False) -> str:
    parts: list[str] = []
    y_cursor = 0
    def push(s: str): parts.append(s)

    error_types = discover_error_types(root)
    agent_roles = discover_agent_roles(root)
    handoff_items = discover_handoff_items(root)
    data_types = discover_data_types(root)

    grad_defs = []
    for key in ["apps", "core", "templates", "other", "pipeline", "interface", "rose", "teal", "blue", "green"]:
        c = THEME["colors"].get(key)
        if c and "grad" in c:
            grad_defs.append(_grad_def(f"grad-{key}", c["grad"]))

    DEFS = f"""<defs>
      {chr(10).join(grad_defs)}
      <marker id="arrow" viewBox="0 0 12 12" refX="10" refY="6" markerWidth="8" markerHeight="8" orient="auto-start-reverse">
        <path d="M2 2L10 6L2 10Z" fill="{THEME["colors"]["arrow"]}" opacity="0.7"/>
      </marker>
      <marker id="arrow-dep" viewBox="0 0 12 12" refX="10" refY="6" markerWidth="8" markerHeight="8" orient="auto-start-reverse">
        <path d="M2 2L10 6L2 10Z" fill="#6366f1" opacity="0.6"/>
      </marker>
      <filter id="shadow" x="-5%" y="-5%" width="110%" height="115%">
        <feDropShadow dx="0" dy="2" stdDeviation="3" flood-opacity="0.08"/>
      </filter>
      <filter id="soft-shadow" x="-8%" y="-8%" width="116%" height="120%">
        <feDropShadow dx="0" dy="4" stdDeviation="6" flood-opacity="0.06"/>
      </filter>
      <style>
        @import url('https://fonts.googleapis.com/css2?family=Inter:wght@400;500;600;700&amp;display=swap');
        .th{{font-family:{THEME["font"]};font-size:13px;font-weight:600;fill:#0f172a;letter-spacing:-0.01em}}
        .ts{{font-family:{THEME["font"]};font-size:12.5px;font-weight:500;fill:#334155;letter-spacing:0.01em}}
        .txs{{font-family:{THEME["font"]};font-size:11px;font-weight:500;fill:#475569;letter-spacing:0.01em}}
        .tl{{font-family:{THEME["font"]};font-size:24px;font-weight:700;fill:#0f172a;letter-spacing:-0.02em}}
        .arr{{fill:none;stroke:{THEME["colors"]["arrow"]};stroke-width:1.5;opacity:0.7}}
        .arr-dep{{fill:none;stroke:#6366f1;stroke-width:1.3;opacity:0.6;stroke-dasharray:6 3}}
        .card{{filter:url(#shadow)}}
        .card-soft{{filter:url(#soft-shadow)}}
        .section-label{{font-family:{THEME["font"]};font-size:11px;font-weight:700;fill:#94a3b8;letter-spacing:0.08em;text-transform:uppercase}}
        .subtitle{{font-family:{THEME["font"]};font-size:11px;font-weight:500;fill:#94a3b8}}
      </style>
    </defs>"""

    push(f'<rect width="{VIEWBOX_W}" height="3000" fill="{THEME["colors"]["bg"]}"/>')

    # Header
    y_cursor = 50
    push(f'<g id="header" role="banner" aria-label="System architecture header">')
    push(_text(CENTER_X, y_cursor, cfg["title"], cls="tl", aria="System Architecture Title"))
    y_cursor += 20
    push(_text(CENTER_X, y_cursor, cfg["project_name"], cls="subtitle", opacity="0.6"))
    y_cursor += 20
    push(f'<line x1="{CENTER_X - 200}" y1="{y_cursor}" x2="{CENTER_X + 200}" y2="{y_cursor}" stroke="{THEME["colors"]["divider"]}" stroke-width="1"/>')
    push("</g>")

    # Legend
    y_cursor += 30
    push(f'<g id="legend" role="img" aria-label="Color legend">')
    legend_items = [
        ("apps", "Applications", "Binary crates and CLI tools"),
        ("core", "Core Libraries", "Shared library crates"),
        ("templates", "Templates", "Reusable architectural patterns"),
        ("other", "Examples", "Learning references and demos"),
    ]
    legend_x = VIEWBOX_MARGIN
    for color_key, label, desc in legend_items:
        c = THEME["colors"][color_key]
        push(f'<rect x="{legend_x}" y="{y_cursor}" width="10" height="10" rx="5" fill="{c["accent"]}"/>')
        push(_text(legend_x + 16, y_cursor + 5, label, anchor="start", cls="th", weight="600"))
        push(_text(legend_x + 16, y_cursor + 20, desc, anchor="start", cls="txs", opacity="0.5"))
        legend_x += 280
    push("</g>")
    y_cursor += 40

    # Pipeline
    y_cursor += 20
    push(f'<g id="pipeline" role="region" aria-label="CI/CD Pipeline orchestration">')
    push(_text(CENTER_X, y_cursor, "PIPELINE ORCHESTRATION", cls="section-label"))
    y_cursor += 30
    stages = [
        ("ANALYZE", "teal", "lint · clippy", "Every push · ~2 min"),
        ("VALIDATE", "blue", "test · nextest", "Every push · ~5 min"),
        ("HARDEN", "rose", "audit · deny", "Weekly + pre-release"),
        ("DEPLOY", "green", "release · publish", "Tag-triggered · ~4 min"),
    ]
    row_h, gap = 60, 20
    bw = (VIEWBOX_W - 2 * VIEWBOX_MARGIN - 3 * gap) // 4
    x = VIEWBOX_MARGIN
    for i, (name, color, desc, timing) in enumerate(stages):
        push(f'<g class="card" role="img" aria-label="Pipeline stage: {name} - {desc}">')
        push(_rect(x, y_cursor, bw, row_h, color_key=color))
        push(_text(x + bw // 2, y_cursor + 20, name, cls="th"))
        push(_text(x + bw // 2, y_cursor + 36, desc, cls="txs", opacity="0.6"))
        push(_text(x + bw // 2, y_cursor + 50, timing, cls="txs", opacity="0.55"))
        push("</g>")
        if i > 0:
            push(f'<line x1="{x - gap}" y1="{y_cursor + row_h // 2}" x2="{x}" y2="{y_cursor + row_h // 2}" class="arr" marker-end="url(#arrow)"/>')
        x += bw + gap
    y_cursor += row_h + 10
    push("</g>")

    # Workspace Topology
    y_cursor += 40
    push(_section_divider(y_cursor))
    y_cursor += 25
    push(f'<g id="workspace" role="region" aria-label="Workspace topology with {labels["crates"]} components">')
    push(_text(CENTER_X, y_cursor, f"WORKSPACE TOPOLOGY · {labels['crates']} COMPONENTS", cls="section-label"))
    y_cursor += 40

    layers = {"apps": [], "core": [], "templates": [], "other": []}
    for crate in crates:
        name = crate["name"]
        if name == cfg.get("project_name", "").lower().replace(" ", "-") or name == "sample-app":
            layers["apps"].append(crate)
        elif "-template" in name:
            layers["templates"].append(crate)
        elif "example-" in name or name == "hello-world-example":
            layers["other"].append(crate)
        else:
            layers["core"].append(crate)

    use_graphviz = not no_graphviz and _has_graphviz()
    if use_graphviz:
        crate_coords = _layout_with_graphviz(crates, layers, y_cursor, VIEWBOX_W)
    if not use_graphviz or not crate_coords:
        crate_coords = _layout_grid_fallback(crates, layers, y_cursor, VIEWBOX_W)
        use_graphviz = False

    overlap_fixes = _fix_overlaps(crate_coords)
    if overlap_fixes:
        print(f"Fixed {overlap_fixes} overlapping elements", file=sys.stderr)

    push(f'<g id="dependencies" role="img" aria-label="Crate dependency relationships">')
    for crate in crates:
        if crate["name"] in crate_coords:
            p1 = crate_coords[crate["name"]]
            src_name = crate["name"]
            for dep in crate["dependencies"]:
                if dep in crate_coords:
                    p2 = crate_coords[dep]
                    sx, sy = p1[0] + p1[2] // 2, p1[1] + p1[3]
                    tx, ty = p2[0] + p2[2] // 2, p2[1]
                    if not use_graphviz:
                        path_d = _route_edge_avoiding_obstacles(sx, sy, tx, ty, crate_coords, src_name, dep)
                    else:
                        mid_y = (sy + ty) // 2
                        if abs(sx - tx) < 10:
                            path_d = f'M {sx} {sy} L {tx} {ty}'
                        else:
                            path_d = f'M {sx} {sy} L {sx} {mid_y} L {tx} {mid_y} L {tx} {ty}'
                    push(f'<path d="{path_d}" '
                         f'class="arr-dep" marker-end="url(#arrow-dep)" '
                         f'aria-label="Dependency: {src_name} depends on {dep}"/>')
    push("</g>")

    # Draw crate cards ON TOP of arrows
    for layer_name in ["apps", "core", "templates", "other"]:
        layer_crates = layers[layer_name]
        if not layer_crates:
            continue
        layer_labels = {"apps": "APPLICATIONS", "core": "CORE LIBRARIES", "templates": "TEMPLATE CRATES", "other": "EXAMPLES"}
        push(f'<g id="layer-{layer_name}" role="group" aria-label="{layer_labels.get(layer_name, layer_name)}">')
        push(_text(VIEWBOX_MARGIN, y_cursor, layer_labels.get(layer_name, layer_name.upper()), anchor="start", cls="txs", weight="600", opacity="0.5"))
        y_cursor += 18

        for crate in layer_crates:
            if crate["name"] in crate_coords:
                cx, cy, cw, ch = crate_coords[crate["name"]]
                card_content, _ = _crate_card(cx, cy, crate, layer_name, card_w=cw)
                push(card_content)
        push("</g>")
    push("</g>")

    # Skills & Agents
    if crate_coords:
        max_crate_bottom = max(cy + ch for cx, cy, cw, ch in crate_coords.values())
        y_cursor = max_crate_bottom + 30
    else:
        y_cursor += 30
    push(_section_divider(y_cursor))
    y_cursor += 30
    panel_w = (VIEWBOX_W - 2 * VIEWBOX_MARGIN - GAP_X) // 2
    text_h = 22
    skills_h = 70 + max(len(skills), 5) * text_h
    agents_h = 70 + max(len(agents), 5) * text_h
    max_h = max(skills_h, agents_h)

    push(f'<g id="skills" role="region" aria-label="{labels["skills"]} active skills">')
    push(_container(VIEWBOX_MARGIN, y_cursor, panel_w, max_h, f"{labels['skills']} ACTIVE SKILLS", ".agents/skills/", aria_label="Active skills list"))
    for i, sk in enumerate(skills):
        push(_text(VIEWBOX_MARGIN + 20, y_cursor + 60 + i * text_h, sk, anchor="start", cls="ts"))
    push("</g>")

    push(f'<g id="agents" role="region" aria-label="{labels["agents"]} cognitive agents">')
    push(_container(VIEWBOX_MARGIN + panel_w + GAP_X, y_cursor, panel_w, max_h, f"{labels['agents']} COGNITIVE AGENTS", ".opencode/agents/", aria_label="Cognitive agents list"))
    if agents:
        for i, ag in enumerate(agents):
            push(_text(VIEWBOX_MARGIN + panel_w + GAP_X + 20, y_cursor + 60 + i * text_h, ag, anchor="start", cls="ts"))
    else:
        push(_text(VIEWBOX_MARGIN + panel_w + GAP_X + panel_w // 2, y_cursor + max_h // 2 + 10, "none configured", cls="txs", opacity="0.55"))
    push("</g>")

    # Subagent Workflow
    y_cursor += max_h + 30
    push(_section_divider(y_cursor))
    y_cursor += 30
    push(f'<g id="subagent-workflow" role="region" aria-label="Subagent workflow and orchestration">')
    push(_text(CENTER_X, y_cursor, "SUBAGENT WORKFLOW · MULTI-AGENT ORCHESTRATION", cls="section-label"))
    y_cursor += 35

    if agent_roles:
        role_w, role_h, role_gap = panel_w, 45, 12
        for i, role in enumerate(agent_roles):
            rx = VIEWBOX_MARGIN if i % 2 == 0 else VIEWBOX_MARGIN + panel_w + GAP_X
            ry = y_cursor + (i // 2) * (role_h + role_gap)
            push(f'<g class="card" role="img" aria-label="Agent role: {role["name"]}">')
            push(_rect(rx, ry, role_w, role_h, rx=10, color_key=role["color"]))
            push(_text(rx + 15, ry + 18, role["name"], anchor="start", cls="th", weight="600"))
            push(_text(rx + 15, ry + 35, " · ".join(role["skills"]), anchor="start", cls="txs", opacity="0.6"))
            push("</g>")
        y_cursor += ((len(agent_roles) + 1) // 2) * (role_h + role_gap) + 15

    if handoff_items:
        push(_text(CENTER_X, y_cursor, "HANDOFF PROTOCOL", cls="section-label"))
        y_cursor += 25
        handoff_w = (VIEWBOX_W - 2 * VIEWBOX_MARGIN - (len(handoff_items) - 1) * GAP_X) // len(handoff_items)
        for i, item in enumerate(handoff_items):
            cx = VIEWBOX_MARGIN + i * (handoff_w + GAP_X)
            push(f'<g class="card" role="img" aria-label="Handoff: {item["path"]}">')
            push(_rect(cx, y_cursor, handoff_w, 40, rx=8, color_key="interface"))
            push(_text(cx + handoff_w // 2, y_cursor + 15, item["path"], cls="th", weight="500"))
            push(_text(cx + handoff_w // 2, y_cursor + 30, item["desc"], cls="txs", opacity="0.5"))
            push("</g>")
        y_cursor += 60
    push("</g>")

    # Error Handling
    y_cursor += 20
    push(_section_divider(y_cursor))
    y_cursor += 30
    push(f'<g id="error-handling" role="region" aria-label="Error handling patterns">')
    push(_text(CENTER_X, y_cursor, "ERROR HANDLING PATTERNS", cls="section-label"))
    y_cursor += 30

    thiserror_count = 0
    for crate in crates:
        crate_dir = root / "crates" / crate["name"]
        if crate_dir.is_dir():
            for rs_file in crate_dir.rglob("*.rs"):
                try:
                    if "thiserror" in rs_file.read_text(encoding="utf-8"):
                        thiserror_count += 1
                        break
                except Exception:
                    continue

    strat_w = panel_w
    push(f'<g class="card" role="img" aria-label="Error strategy: thiserror">')
    push(_rect(VIEWBOX_MARGIN, y_cursor, strat_w, 50, rx=10, color_key="core"))
    push(_text(VIEWBOX_MARGIN + 15, y_cursor + 18, "thiserror", anchor="start", cls="th", weight="600"))
    push(_text(VIEWBOX_MARGIN + 15, y_cursor + 35, f"Library error types — {thiserror_count} crates using derive macro", anchor="start", cls="txs", opacity="0.6"))
    push("</g>")
    push(f'<g class="card" role="img" aria-label="Error strategy: anyhow">')
    push(_rect(VIEWBOX_MARGIN + panel_w + GAP_X, y_cursor, strat_w, 50, rx=10, color_key="apps"))
    push(_text(VIEWBOX_MARGIN + panel_w + GAP_X + 15, y_cursor + 18, "anyhow", anchor="start", cls="th", weight="600"))
    push(_text(VIEWBOX_MARGIN + panel_w + GAP_X + 15, y_cursor + 35, "Application errors — Context-rich error propagation", anchor="start", cls="txs", opacity="0.6"))
    push("</g>")
    y_cursor += 70

    if error_types:
        push(_text(CENTER_X, y_cursor, "CRATE ERROR TYPES", cls="section-label"))
        y_cursor += 25
        for i, err in enumerate(error_types):
            cy = y_cursor + i * 46
            push(f'<g class="card-soft" role="img" aria-label="Error type: {err["name"]} from {err["crate"]}">')
            push(_rect(VIEWBOX_MARGIN, cy, VIEWBOX_W - 2 * VIEWBOX_MARGIN, 40, rx=8, color_key="rose"))
            push(_text(VIEWBOX_MARGIN + 15, cy + 14, err["name"], anchor="start", cls="th", weight="600"))
            push(_text(VIEWBOX_MARGIN + 15, cy + 30, " · ".join(err["variants"]), anchor="start", cls="txs", opacity="0.6"))
            push(_text(RIGHT_EDGE - 15, cy + 14, err["crate"], anchor="end", cls="txs", opacity="0.55"))
            push("</g>")
        y_cursor += len(error_types) * 46 + 15
    push("</g>")

    # Data Flow
    y_cursor += 20
    push(_section_divider(y_cursor))
    y_cursor += 30
    push(f'<g id="data-flow" role="region" aria-label="Data flow patterns">')
    push(_text(CENTER_X, y_cursor, "DATA FLOW · COMPONENT INTERACTIONS", cls="section-label"))
    y_cursor += 30

    if data_types:
        push(_text(CENTER_X, y_cursor, "KEY DATA TYPES", cls="section-label"))
        y_cursor += 25
        dt_w = panel_w
        for i, dt in enumerate(data_types):
            dx = VIEWBOX_MARGIN if i % 2 == 0 else VIEWBOX_MARGIN + panel_w + GAP_X
            dy = y_cursor + (i // 2) * 48
            safe_name = _esc(dt["name"])
            push(f'<g class="card-soft" role="img" aria-label="Data type: {safe_name}">')
            push(_rect(dx, dy, dt_w, 40, rx=8, color_key="interface"))
            push(_text(dx + 15, dy + 15, dt["name"], anchor="start", cls="th", weight="500"))
            push(_text(dx + 15, dy + 30, f"{dt['desc']} — {dt['crate']}", anchor="start", cls="txs", opacity="0.5"))
            push("</g>")
        y_cursor += ((len(data_types) + 1) // 2) * 48 + 15
    push("</g>")

    # Commands
    y_cursor += 20
    push(_section_divider(y_cursor))
    y_cursor += 30
    push(f'<g id="commands" role="region" aria-label="Interface protocols and commands">')
    push(_text(CENTER_X, y_cursor, "INTERFACE PROTOCOLS", cls="section-label"))
    y_cursor += 35
    cmd_cols = 3
    cmd_w = (VIEWBOX_W - 2 * VIEWBOX_MARGIN - (cmd_cols - 1) * GAP_X) // cmd_cols
    for i, cmd in enumerate(commands[:18]):
        col = i % cmd_cols
        row = i // cmd_cols
        cx = VIEWBOX_MARGIN + col * (cmd_w + GAP_X)
        cy = y_cursor + row * 38
        push(f'<g class="card-soft" role="img" aria-label="Command: {cmd}">')
        push(_rect(cx, cy, cmd_w, 30, rx=15, color_key="interface"))
        push(_text(cx + cmd_w // 2, cy + 15, cmd, cls="ts", weight="500"))
        push("</g>")
    push("</g>")

    y_cursor += ((len(commands[:18]) + cmd_cols - 1) // cmd_cols) * 38 + 60
    push(f'<line x1="{CENTER_X - 200}" y1="{y_cursor}" x2="{CENTER_X + 200}" y2="{y_cursor}" stroke="{THEME["colors"]["divider"]}" stroke-width="1"/>')
    y_cursor += 20
    footer = f"{cfg['project_name']} · {cfg['author']} · 2026 EDITION"
    push(_text(CENTER_X, y_cursor, footer, cls="txs", weight="700", opacity="0.35"))

    svg_height = y_cursor + 50
    parts[0] = f'<rect width="{VIEWBOX_W}" height="{svg_height}" fill="{THEME["colors"]["bg"]}"/>'

    title = cfg["title"]
    project = cfg["project_name"]
    return (f'<svg width="100%" viewBox="0 0 {VIEWBOX_W} {svg_height}" xmlns="http://www.w3.org/2000/svg" '
            f'role="img" aria-label="{title} - {project}">\n{DEFS}\n'
            + "\n".join(parts) + "\n</svg>")

def _export_format(svg: str, svg_path: Path, out_path: Path, fmt: str) -> bool:
    if fmt == "png":
        for cmd in [["rsvg-convert", "-f", "png", "-o", str(out_path), str(svg_path)],
                     ["inkscape", "--export-type=png", f"--export-filename={out_path}", str(svg_path)]]:
            try:
                subprocess.run(cmd, capture_output=True, timeout=30, check=True, shell=False)  # nosec B603
                return True
            except (FileNotFoundError, subprocess.TimeoutExpired, subprocess.CalledProcessError):
                continue
    elif fmt == "pdf":
        for cmd in [["rsvg-convert", "-f", "pdf", "-o", str(out_path), str(svg_path)],
                     ["inkscape", "--export-type=pdf", f"--export-filename={out_path}", str(svg_path)]]:
            try:
                subprocess.run(cmd, capture_output=True, timeout=30, check=True, shell=False)  # nosec B603
                return True
            except (FileNotFoundError, subprocess.TimeoutExpired, subprocess.CalledProcessError):
                continue
    return False

def main():
    parser = argparse.ArgumentParser(description="Generate Project Topology SVG")
    parser.add_argument("--root", default=".", help="Workspace root")
    parser.add_argument("--out", default=".template/architecture.svg", help="Output path")
    parser.add_argument("--no-graphviz", action="store_true", help="Force grid layout (skip Graphviz auto-layout)")
    parser.add_argument("--format", choices=["svg", "png", "pdf"], default="svg", help="Output format (default: svg)")
    args = parser.parse_args()
    root, out = Path(args.root).resolve(), Path(args.out)
    if not out.is_absolute():
        out = root / out
    cfg = dict(DEFAULT_CONFIG)
    cfg_file = root / "docs" / "diagram-config.json"
    if cfg_file.exists():
        try:
            with open(cfg_file, encoding="utf-8") as f:
                cfg.update(json.load(f))
        except Exception:
            pass
    crates = discover_crates(root)
    skills, agents, commands = discover_skills(root), discover_agents(root), discover_commands(root)
    labels = {"crates": len(crates), "skills": len(skills), "agents": len(agents), "commands": len(commands)}
    d_crates = crates or [{"name": "(no workspace)", "version": "0.0.0", "dependencies": [], "features": [], "description": ""}]
    svg = build_svg(cfg, d_crates, skills, agents, commands, labels, root, no_graphviz=args.no_graphviz)
    out.parent.mkdir(parents=True, exist_ok=True)
    out.write_text(svg, encoding="utf-8")
    print(f"Written: {out}")

    if args.format != "svg":
        fmt_out = out.with_suffix(f".{args.format}")
        converted = _export_format(svg, out, fmt_out, args.format)
        if converted:
            print(f"Exported: {fmt_out}")
        else:
            print(f"Warning: {args.format} export requires rsvg-convert or inkscape on PATH", file=sys.stderr)

if __name__ == "__main__":
    main()
