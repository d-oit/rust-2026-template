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

Usage:
python generate_diagram.py [--root .] [--out .template/architecture.svg]
"""
import argparse
import json
import re
import subprocess  # nosec B404
import sys
from pathlib import Path

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
    "shadow": "0 2px 4px rgba(0,0,0,0.05)"
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
    if not skills_dir.is_dir(): return []
    return [
        _read_frontmatter_name(d / "SKILL.md")
        for d in sorted(skills_dir.iterdir())
        if d.is_dir() and (d / "SKILL.md").exists()
    ]

def discover_agents(root: Path) -> list[str]:
    agents_dir = root / ".opencode" / "agents"
    if not agents_dir.is_dir(): return []
    return sorted(p.stem for p in agents_dir.glob("*.md"))

def discover_commands(root: Path) -> list[str]:
    commands_dir = root / ".opencode" / "commands"
    if not commands_dir.is_dir(): return []
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
    """Discover error enums from codebase."""
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
                # Find error enums
                for match in re.finditer(r'pub enum (\w*Error)\s*\{([^}]+)\}', content, re.DOTALL):
                    enum_name = match.group(1)
                    variants_text = match.group(2)
                    variants = re.findall(r'(\w+)(?:\(|\s|$)', variants_text)
                    variants = [v for v in variants if v[0].isupper() and v not in ('Debug', 'Error', 'Display', 'From')]
                    if variants:
                        error_types.append({
                            "name": enum_name,
                            "variants": variants[:6],  # Limit to 6 variants
                            "crate": crate_dir.name,
                        })
            except Exception:
                continue
    return error_types[:5]  # Limit to 5 error types

def discover_agent_roles(root: Path) -> list[dict]:
    """Discover agent roles from ORCHESTRATION.md."""
    orch_file = root / ".agents" / "ORCHESTRATION.md"
    if not orch_file.exists():
        return []
    try:
        content = orch_file.read_text(encoding="utf-8")
        roles = []
        # Parse markdown table for agent roles
        for match in re.finditer(r'\|\s*\*\*(\w+-\w+)\*\*\s*\|[^|]*\|\s*`([^`]+(?:`[^`]*)*)`\s*\|', content):
            role_name = match.group(1)
            skills_text = match.group(2)
            skills = [s.strip('`') for s in skills_text.split('`, `')]
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
    """Discover handoff protocol files from ORCHESTRATION.md."""
    orch_file = root / ".agents" / "ORCHESTRATION.md"
    if not orch_file.exists():
        return []
    try:
        content = orch_file.read_text(encoding="utf-8")
        items = []
        # Find file references in the handoff protocol section
        for match in re.finditer(r'`([^`]+\.(?:json|jsonl|md))`', content):
            file_path = match.group(1)
            if file_path not in [i["path"] for i in items]:
                items.append({"path": file_path, "desc": "Workflow state"})
        return items[:3]
    except Exception:
        return []

def discover_data_types(root: Path) -> list[dict]:
    """Discover key data types from codebase."""
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
                # Find public structs with doc comments
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

# ── Main SVG Builder ──────────────────────────────────────────────────────
def build_svg(cfg: dict, crates: list[dict], skills: list[str], agents: list[str],
              commands: list[str], labels: dict, root: Path) -> str:
    parts: list[str] = []
    y_cursor = 0
    def push(s: str): parts.append(s)

    # Discover all dynamic content
    error_types = discover_error_types(root)
    agent_roles = discover_agent_roles(root)
    handoff_items = discover_handoff_items(root)
    data_types = discover_data_types(root)

    # ── Gradient Definitions ──
    grad_defs = []
    for key in ["apps", "core", "templates", "other", "pipeline", "interface", "rose", "teal", "blue", "green"]:
        c = THEME["colors"].get(key)
        if c and "grad" in c:
            grad_defs.append(_grad_def(f"grad-{key}", c["grad"]))

    # ── Style Definitions ──
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
        .ts{{font-family:{THEME["font"]};font-size:12px;font-weight:400;fill:#475569}}
        .txs{{font-family:{THEME["font"]};font-size:10px;font-weight:400;fill:#64748b}}
        .tl{{font-family:{THEME["font"]};font-size:20px;font-weight:700;fill:#0f172a;letter-spacing:-0.02em}}
        .arr{{fill:none;stroke:{THEME["colors"]["arrow"]};stroke-width:1.3;opacity:0.65}}
        .arr-dep{{fill:none;stroke:#6366f1;stroke-width:1.2;opacity:0.5;stroke-dasharray:6 3}}
        .card{{filter:url(#shadow)}}
        .card-soft{{filter:url(#soft-shadow)}}
        .section-label{{font-family:{THEME["font"]};font-size:10px;font-weight:700;fill:#94a3b8;letter-spacing:0.08em;text-transform:uppercase}}
        .subtitle{{font-family:{THEME["font"]};font-size:9px;font-weight:500;fill:#94a3b8}}
      </style>
    </defs>"""

    # ── Background ──
    push(f'<rect width="680" height="3000" fill="{THEME["colors"]["bg"]}"/>')

    # ── Header ──
    y_cursor = 50
    push(f'<g id="header" role="banner" aria-label="System architecture header">')
    push(_text(340, y_cursor, cfg["title"], cls="tl", aria="System Architecture Title"))
    y_cursor += 18
    push(_text(340, y_cursor, cfg["project_name"], cls="subtitle", opacity="0.6"))
    y_cursor += 18
    push(f'<line x1="140" y1="{y_cursor}" x2="540" y2="{y_cursor}" stroke="{THEME["colors"]["divider"]}" stroke-width="1"/>')
    push("</g>")

    # ── Legend ──
    y_cursor += 25
    push(f'<g id="legend" role="img" aria-label="Color legend">')
    legend_items = [("apps", "Applications"), ("core", "Core Libraries"), ("templates", "Templates"), ("other", "Examples")]
    legend_x = 21
    for color_key, label in legend_items:
        c = THEME["colors"][color_key]
        push(f'<rect x="{legend_x}" y="{y_cursor}" width="8" height="8" rx="4" fill="{c["accent"]}"/>')
        push(_text(legend_x + 14, y_cursor + 4, label, anchor="start", cls="txs", opacity="0.6"))
        legend_x += len(label) * 6 + 30
    push("</g>")
    y_cursor += 20

    # ── Pipeline ──
    y_cursor += 25
    push(f'<g id="pipeline" role="region" aria-label="CI/CD Pipeline orchestration">')
    push(_text(340, y_cursor, "PIPELINE ORCHESTRATION", cls="section-label"))
    y_cursor += 30
    stages = [("ANALYZE", "teal", "lint · clippy"), ("VALIDATE", "blue", "test · nextest"),
              ("HARDEN", "rose", "audit · deny"), ("DEPLOY", "green", "release · publish")]
    row_h, gap = 48, 18
    bw = min(140, (640 - (len(stages) - 1) * gap) // len(stages))
    x = 21
    for i, (name, color, desc) in enumerate(stages):
        push(f'<g class="card" role="img" aria-label="Pipeline stage: {name} - {desc}">')
        push(_rect(x, y_cursor, bw, row_h, color_key=color))
        push(_text(x + bw // 2, y_cursor + 19, name, cls="th"))
        push(_text(x + bw // 2, y_cursor + 35, desc, cls="txs", opacity="0.5"))
        push("</g>")
        if i > 0:
            push(f'<line x1="{x-gap}" y1="{y_cursor+row_h//2}" x2="{x}" y2="{y_cursor+row_h//2}" class="arr" marker-end="url(#arrow)"/>')
        x += bw + gap
    y_cursor += row_h + 10
    push("</g>")

    # ── Workspace Topology ──
    y_cursor += 50
    push(f'<line x1="20" y1="{y_cursor}" x2="660" y2="{y_cursor}" stroke="{THEME["colors"]["divider"]}" stroke-width="1"/>')
    y_cursor += 25
    push(f'<g id="workspace" role="region" aria-label="Workspace topology with {labels["crates"]} components">')
    push(_text(340, y_cursor, f"WORKSPACE TOPOLOGY · {labels['crates']} COMPONENTS", cls="section-label"))
    y_cursor += 40

    layers = {"apps": [], "core": [], "templates": [], "other": []}
    for crate in crates:
        name = crate["name"]
        if name == cfg.get("project_name", "").lower().replace(" ", "-") or name == "sample-app":
            layers["apps"].append(crate)
        elif "-template" in name: layers["templates"].append(crate)
        elif "example-" in name or name == "hello-world-example": layers["other"].append(crate)
        else: layers["core"].append(crate)

    crate_coords = {}
    col_w, row_h_min = 200, 80
    gap_x, gap_y = 25, 95

    for layer_name in ["apps", "core", "templates", "other"]:
        layer_crates = layers[layer_name]
        if not layer_crates: continue
        layer_labels = {"apps": "APPLICATIONS", "core": "CORE LIBRARIES", "templates": "TEMPLATE CRATES", "other": "EXAMPLES"}
        push(f'<g id="layer-{layer_name}" role="group" aria-label="{layer_labels.get(layer_name, layer_name)}">')
        push(_text(21, y_cursor - 8, layer_labels.get(layer_name, layer_name.upper()), anchor="start", cls="txs", weight="600", opacity="0.35"))
        y_cursor += 12

        for row_idx in range(0, len(layer_crates), 3):
            row_crates = layer_crates[row_idx : row_idx + 3]
            num = len(row_crates)
            start_x = (680 - (num * col_w + (num - 1) * gap_x)) // 2
            max_row_h = 0
            for i, crate in enumerate(row_crates):
                cx = start_x + i * (col_w + gap_x)
                feats = crate.get("features", [])
                box_h = row_h_min + (len(feats) * 18)
                max_row_h = max(max_row_h, box_h)
                crate_coords[crate["name"]] = (cx, y_cursor, box_h)

                push(f'<g id="crate-{crate["name"]}" class="card" role="img" '
                     f'aria-label="Crate: {crate["name"]} v{crate["version"]}">')
                push(_rect(cx, y_cursor, col_w, box_h, color_key=layer_name))
                push(_accent_bar(cx + 8, y_cursor + 8, col_w - 16, color_key=layer_name))
                push(_text(cx + col_w // 2, y_cursor + 28, crate["name"], cls="th"))
                push(_text(cx + col_w // 2, y_cursor + 44, f"v{crate['version']}", cls="txs", opacity="0.45"))
                if feats:
                    feat_y = y_cursor + 58
                    feat_x = cx + 10
                    for f in feats:
                        push(_badge(feat_x, feat_y, f, color_key=layer_name, w=len(f) * 6 + 16))
                        feat_x += len(f) * 6 + 22
                        if feat_x + 50 > cx + col_w:
                            feat_x = cx + 10
                            feat_y += 20
                push("</g>")
            y_cursor += max_row_h + gap_y
        push("</g>")

    # ── Dependency Arrows ──
    push(f'<g id="dependencies" role="img" aria-label="Crate dependency relationships">')
    MARGIN_X = 8
    for crate in crates:
        if crate["name"] in crate_coords:
            p1 = crate_coords[crate["name"]]
            src_name = crate["name"]
            for dep in crate["dependencies"]:
                if dep in crate_coords:
                    p2 = crate_coords[dep]
                    sx, sy = p1[0], p1[1] + p1[2] // 2
                    tx, ty = p2[0], p2[1] + p2[2] // 2
                    control_offset_x = 25
                    push(f'<path d="M {sx} {sy} '
                         f'C {sx - control_offset_x} {sy}, {MARGIN_X + 12} {sy}, {MARGIN_X + 12} {sy} '
                         f'L {MARGIN_X + 12} {ty} '
                         f'C {MARGIN_X + 12} {ty}, {tx - control_offset_x} {ty}, {tx} {ty}" '
                         f'class="arr-dep" marker-end="url(#arrow-dep)" '
                         f'aria-label="Dependency: {src_name} depends on {dep}"/>')
    push("</g>")
    push("</g>")

    # ── Skills & Agents ──
    y_cursor += 20
    push(f'<line x1="20" y1="{y_cursor}" x2="660" y2="{y_cursor}" stroke="{THEME["colors"]["divider"]}" stroke-width="1"/>')
    y_cursor += 30
    side_w, text_h = 310, 22
    skills_h = 65 + max(len(skills), 5) * text_h
    agents_h = 65 + max(len(agents), 5) * text_h
    max_h = max(skills_h, agents_h)

    push(f'<g id="skills" role="region" aria-label="{labels["skills"]} active skills">')
    push(_container(20, y_cursor, side_w, max_h, f"{labels['skills']} ACTIVE SKILLS", ".agents/skills/", aria_label="Active skills list"))
    for i, sk in enumerate(skills):
        push(_text(40, y_cursor + 60 + i * text_h, sk, anchor="start", cls="ts"))
    push("</g>")

    push(f'<g id="agents" role="region" aria-label="{labels["agents"]} cognitive agents">')
    push(_container(350, y_cursor, side_w, max_h, f"{labels['agents']} COGNITIVE AGENTS", ".opencode/agents/", aria_label="Cognitive agents list"))
    if agents:
        for i, ag in enumerate(agents):
            push(_text(370, y_cursor + 60 + i * text_h, ag, anchor="start", cls="ts"))
    else:
        push(_text(370 + side_w // 2, y_cursor + max_h // 2 + 10, "none configured", cls="txs", opacity="0.4"))
    push("</g>")

    # ── Subagent Workflow ──
    y_cursor += max_h + 20
    push(f'<line x1="20" y1="{y_cursor}" x2="660" y2="{y_cursor}" stroke="{THEME["colors"]["divider"]}" stroke-width="1"/>')
    y_cursor += 30
    push(f'<g id="subagent-workflow" role="region" aria-label="Subagent workflow and orchestration">')
    push(_text(340, y_cursor, "SUBAGENT WORKFLOW · MULTI-AGENT ORCHESTRATION", cls="section-label"))
    y_cursor += 35

    # Agent roles (dynamic)
    if agent_roles:
        role_w, role_h, role_gap = 305, 40, 10
        for i, role in enumerate(agent_roles):
            rx = 21 if i % 2 == 0 else 354
            ry = y_cursor + (i // 2) * (role_h + role_gap)
            push(f'<g class="card" role="img" aria-label="Agent role: {role["name"]}">')
            push(_rect(rx, ry, role_w, role_h, rx=10, color_key=role["color"]))
            push(_text(rx + 12, ry + 15, role["name"], anchor="start", cls="th", weight="600"))
            push(_text(rx + 12, ry + 30, " · ".join(role["skills"]), anchor="start", cls="txs", opacity="0.6"))
            push("</g>")
        y_cursor += ((len(agent_roles) + 1) // 2) * (role_h + role_gap) + 15

    # Handoff protocol (dynamic)
    if handoff_items:
        push(_text(340, y_cursor, "HANDOFF PROTOCOL", cls="section-label"))
        y_cursor += 25
        for i, item in enumerate(handoff_items):
            cx = 21 + i * 218
            push(f'<g class="card" role="img" aria-label="Handoff: {item["path"]}">')
            push(_rect(cx, y_cursor, 210, 35, rx=8, color_key="interface"))
            push(_text(cx + 105, y_cursor + 13, item["path"], cls="th", weight="500"))
            push(_text(cx + 105, y_cursor + 27, item["desc"], cls="txs", opacity="0.5"))
            push("</g>")
        y_cursor += 55
    push("</g>")

    # ── Error Handling ──
    y_cursor += 20
    push(f'<line x1="20" y1="{y_cursor}" x2="660" y2="{y_cursor}" stroke="{THEME["colors"]["divider"]}" stroke-width="1"/>')
    y_cursor += 30
    push(f'<g id="error-handling" role="region" aria-label="Error handling patterns">')
    push(_text(340, y_cursor, "ERROR HANDLING PATTERNS", cls="section-label"))
    y_cursor += 30

    # Error strategies (dynamic from crates)
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

    strat_w = 305
    push(f'<g class="card" role="img" aria-label="Error strategy: thiserror">')
    push(_rect(21, y_cursor, strat_w, 45, rx=10, color_key="core"))
    push(_text(36, y_cursor + 16, "thiserror", anchor="start", cls="th", weight="600"))
    push(_text(36, y_cursor + 32, f"Library error types — {thiserror_count} crates using derive macro", anchor="start", cls="txs", opacity="0.6"))
    push("</g>")
    push(f'<g class="card" role="img" aria-label="Error strategy: anyhow">')
    push(_rect(354, y_cursor, strat_w, 45, rx=10, color_key="apps"))
    push(_text(369, y_cursor + 16, "anyhow", anchor="start", cls="th", weight="600"))
    push(_text(369, y_cursor + 32, "Application errors — Context-rich error propagation", anchor="start", cls="txs", opacity="0.6"))
    push("</g>")
    y_cursor += 65

    # Error types (dynamic)
    if error_types:
        push(_text(340, y_cursor, "CRATE ERROR TYPES", cls="section-label"))
        y_cursor += 25
        for i, err in enumerate(error_types):
            cy = y_cursor + i * 46
            push(f'<g class="card-soft" role="img" aria-label="Error type: {err["name"]} from {err["crate"]}">')
            push(_rect(21, cy, 638, 40, rx=8, color_key="rose"))
            push(_text(36, cy + 14, err["name"], anchor="start", cls="th", weight="600"))
            push(_text(36, cy + 30, " · ".join(err["variants"]), anchor="start", cls="txs", opacity="0.6"))
            push(_text(644, cy + 14, err["crate"], anchor="end", cls="txs", opacity="0.4"))
            push("</g>")
        y_cursor += len(error_types) * 46 + 15
    push("</g>")

    # ── Data Flow ──
    y_cursor += 20
    push(f'<line x1="20" y1="{y_cursor}" x2="660" y2="{y_cursor}" stroke="{THEME["colors"]["divider"]}" stroke-width="1"/>')
    y_cursor += 30
    push(f'<g id="data-flow" role="region" aria-label="Data flow patterns">')
    push(_text(340, y_cursor, "DATA FLOW · COMPONENT INTERACTIONS", cls="section-label"))
    y_cursor += 30

    # Data types (dynamic)
    if data_types:
        push(_text(340, y_cursor, "KEY DATA TYPES", cls="section-label"))
        y_cursor += 25
        for i, dt in enumerate(data_types):
            dx = 21 if i % 2 == 0 else 354
            dy = y_cursor + (i // 2) * 43
            safe_name = _esc(dt["name"])
            push(f'<g class="card-soft" role="img" aria-label="Data type: {safe_name}">')
            push(_rect(dx, dy, 305, 35, rx=8, color_key="interface"))
            push(_text(dx + 12, dy + 13, dt["name"], anchor="start", cls="th", weight="500"))
            push(_text(dx + 12, dy + 27, f"{dt['desc']} — {dt['crate']}", anchor="start", cls="txs", opacity="0.5"))
            push("</g>")
        y_cursor += ((len(data_types) + 1) // 2) * 43 + 15
    push("</g>")

    # ── Commands ──
    y_cursor += 20
    push(f'<line x1="20" y1="{y_cursor}" x2="660" y2="{y_cursor}" stroke="{THEME["colors"]["divider"]}" stroke-width="1"/>')
    y_cursor += 30
    push(f'<g id="commands" role="region" aria-label="Interface protocols and commands">')
    push(_text(340, y_cursor, "INTERFACE PROTOCOLS", cls="section-label"))
    y_cursor += 35
    for i, cmd in enumerate(commands[:14]):
        cx, cy = (20 if i % 2 == 0 else 345), y_cursor + (i // 2) * 38
        push(f'<g class="card-soft" role="img" aria-label="Command: {cmd}">')
        push(_rect(cx, cy, 310, 30, rx=15, color_key="interface"))
        push(_text(cx + 155, cy + 15, cmd, cls="ts", weight="500"))
        push("</g>")
    push("</g>")

    y_cursor += ((len(commands[:14]) + 1) // 2) * 38 + 60
    push(f'<line x1="140" y1="{y_cursor}" x2="540" y2="{y_cursor}" stroke="{THEME["colors"]["divider"]}" stroke-width="1"/>')
    y_cursor += 20
    footer = f"{cfg['project_name']} · {cfg['author']} · 2026 EDITION"
    push(_text(340, y_cursor, footer, cls="txs", weight="700", opacity="0.25"))

    # ── Trim background rect ──
    svg_height = y_cursor + 50
    parts[0] = f'<rect width="680" height="{svg_height}" fill="{THEME["colors"]["bg"]}"/>'

    title = cfg["title"]
    project = cfg["project_name"]
    return (f'<svg width="100%" viewBox="0 0 680 {svg_height}" xmlns="http://www.w3.org/2000/svg" '
            f'role="img" aria-label="{title} - {project}">\n{DEFS}\n'
            + "\n".join(parts) + "\n</svg>")

def main():
    parser = argparse.ArgumentParser(description="Generate Project Topology SVG")
    parser.add_argument("--root", default=".", help="Workspace root")
    parser.add_argument("--out", default=".template/architecture.svg", help="Output path")
    args = parser.parse_args()
    root, out = Path(args.root).resolve(), Path(args.out)
    if not out.is_absolute(): out = root / out
    cfg = dict(DEFAULT_CONFIG)
    cfg_file = root / "docs" / "diagram-config.json"
    if cfg_file.exists():
        try:
            with open(cfg_file, encoding="utf-8") as f: cfg.update(json.load(f))
        except Exception: pass
    crates = discover_crates(root)
    skills, agents, commands = discover_skills(root), discover_agents(root), discover_commands(root)
    labels = {"crates": len(crates), "skills": len(skills), "agents": len(agents), "commands": len(commands)}
    d_crates = crates or [{"name": "(no workspace)", "version": "0.0.0", "dependencies": [], "features": []}]
    svg = build_svg(cfg, d_crates, skills, agents, commands, labels, root)
    out.parent.mkdir(parents=True, exist_ok=True)
    out.write_text(svg, encoding="utf-8")
    print(f"Written: {out}")

if __name__ == "__main__": main()
