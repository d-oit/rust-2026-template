#!/usr/bin/env python3
"""
generate_diagram.py — Project topology SVG generator
Scans the live project structure, including Rust workspace, and writes an architecture diagram.

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
# Professional, high-fidelity palette inspired by modern UI/UX design trends
THEME = {
    "colors": {
        "apps": {"bg": "#fffbeb", "border": "#fde68a", "text": "#92400e"},      # Amber
        "core": {"bg": "#eff6ff", "border": "#bfdbfe", "text": "#1e40af"},      # Blue
        "templates": {"bg": "#faf5ff", "border": "#e9d5ff", "text": "#6b21a8"}, # Purple
        "other": {"bg": "#f0fdf4", "border": "#bbf7d0", "text": "#166534"},    # Green
        "pipeline": {"bg": "#f0fdfa", "border": "#99f6e4", "text": "#0f766e"}, # Teal
        "interface": {"bg": "#f8fafc", "border": "#e2e8f0", "text": "#334155"},# Slate
        "divider": "#f1f5f9",
        "arrow": "#94a3b8"
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
    """Extract `name:` from YAML frontmatter, or fall back to stem."""
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
    """Uses cargo metadata to find workspace crates, dependencies, and features."""
    try:
        # Secure subprocess call using a static list of literals for security compliance
        result = subprocess.run(  # nosec B603
            ["cargo", "metadata", "--format-version", "1", "--no-deps"],
            capture_output=True,
            text=True,
            check=True,
            cwd=str(root.resolve()),
            shell=False,
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

# ── SVG Primitives ────────────────────────────────────────────────────────
def _esc(s: str) -> str:
    return s.replace("&", "&amp;").replace("<", "&lt;").replace(">", "&gt;")

def _rect(x, y, w, h, rx=None, color_key="core", dash=False, fill_override=None) -> str:
    c = THEME["colors"].get(color_key, THEME["colors"]["core"])
    r = rx if rx is not None else THEME["radius"]
    dash_attr = ' stroke-dasharray="4 3"' if dash else ""
    fill = fill_override if fill_override else c["bg"]
    return (f'<rect x="{x}" y="{y}" width="{w}" height="{h}" rx="{r}" '
            f'fill="{fill}" stroke="{c["border"]}" stroke-width="1.5"{dash_attr}/>')

def _text(x, y, content, anchor="middle", cls="ts", dy=None, opacity=None, weight=None) -> str:
    dy_attr = f' dy="{dy}"' if dy else ""
    op_attr = f' opacity="{opacity}"' if opacity else ""
    w_attr = f' font-weight="{weight}"' if weight else ""
    return (f'<text class="{cls}" x="{x}" y="{y}" '
            f'text-anchor="{anchor}" dominant-baseline="central"{dy_attr}{op_attr}{w_attr}>'
            f'{_esc(content)}</text>')

def _container(x, y, w, h, label, sublabel=None) -> str:
    parts = [
        '<g class="card">',
        f'<rect x="{x}" y="{y}" width="{w}" height="{h}" rx="{THEME["radius"]}" fill="none" stroke="{THEME["colors"]["interface"]["border"]}" stroke-width="1" stroke-dasharray="4 4"/>',
        _text(x + w // 2, y + 18, label, cls="th", weight="700"),
    ]
    if sublabel:
        parts.append(_text(x + w // 2, y + 36, sublabel, cls="txs", opacity="0.5"))
    parts.append("</g>")
    return "\n".join(parts)

# ── Main SVG Builder ──────────────────────────────────────────────────────
def build_svg(cfg: dict, crates: list[dict], skills: list[str], agents: list[str], commands: list[str], labels: dict) -> str:
    parts: list[str] = []
    y_cursor = 0
    def push(s: str): parts.append(s)

    # ── Style Definitions ──
    DEFS = f"""<defs>
      <marker id="arrow" viewBox="0 0 10 10" refX="8" refY="5" markerWidth="6" markerHeight="6" orient="auto-start-reverse">
        <path d="M2 1L8 5L2 9" fill="none" stroke="{THEME["colors"]["arrow"]}" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/>
      </marker>
      <filter id="shadow" x="-5%" y="-5%" width="110%" height="115%">
        <feDropShadow dx="0" dy="2" stdDeviation="3" flood-opacity="0.1"/>
      </filter>
      <style>
        @import url('https://fonts.googleapis.com/css2?family=Inter:wght@400;500;600;700&amp;display=swap');
        .th{{font-family:{THEME["font"]};font-size:13px;font-weight:600;fill:#0f172a;letter-spacing:-0.01em}}
        .ts{{font-family:{THEME["font"]};font-size:11px;font-weight:400;fill:#475569}}
        .txs{{font-family:{THEME["font"]};font-size:9px;font-weight:400;fill:#64748b}}
        .tl{{font-family:{THEME["font"]};font-size:18px;font-weight:700;fill:#0f172a;letter-spacing:-0.02em}}
        .arr{{fill:none;stroke:{THEME["colors"]["arrow"]};stroke-width:1.2;opacity:0.6}}
        .card{{filter:url(#shadow)}}
      </style>
    </defs>"""

    # ── Header ──
    y_cursor = 50
    push(_text(340, y_cursor, cfg["title"], cls="tl"))
    y_cursor += 20
    push(f'<line x1="120" y1="{y_cursor}" x2="560" y2="{y_cursor}" stroke="{THEME["colors"]["divider"]}" stroke-width="1"/>')

    # ── CI/CD Pipeline ──
    y_cursor += 40
    push(_text(21, y_cursor, "PIPELINE ORCHESTRATION", anchor="start", cls="txs", weight="700", opacity="0.4"))
    y_cursor += 25
    stages = [("ANALYZE", "teal"), ("VALIDATE", "blue"), ("HARDEN", "rose"), ("DEPLOY", "green")]
    row_h = 42
    gap = 16
    bw = min(130, (640 - (len(stages) - 1) * gap) // len(stages))
    x = 21
    for i, (name, color) in enumerate(stages):
        push('<g class="card">')
        push(_rect(x, y_cursor, bw, row_h, color_key=color))
        push(_text(x + bw // 2, y_cursor + row_h // 2, name, cls="th"))
        push("</g>")
        if i > 0:
            push(f'<line x1="{x-gap}" y1="{y_cursor+row_h//2}" x2="{x}" y2="{y_cursor+row_h//2}" class="arr" marker-end="url(#arrow)"/>')
        x += bw + gap
    y_cursor += row_h

    # ── Workspace Topology ──
    y_cursor += 60
    push(f'<line x1="20" y1="{y_cursor}" x2="660" y2="{y_cursor}" stroke="{THEME["colors"]["divider"]}" stroke-width="1"/>')
    y_cursor += 25
    push(_text(340, y_cursor, f"WORKSPACE TOPOLOGY · {labels['crates']} COMPONENTS", cls="txs", weight="700", opacity="0.6"))
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
    col_w, row_h_min = 200, 75
    gap_x, gap_y = 25, 90

    for layer_name in ["apps", "core", "templates", "other"]:
        layer_crates = layers[layer_name]
        if not layer_crates: continue
        for row_idx in range(0, len(layer_crates), 3):
            row_crates = layer_crates[row_idx : row_idx + 3]
            num = len(row_crates)
            start_x = (680 - (num * col_w + (num - 1) * gap_x)) // 2
            max_row_h = 0
            for i, crate in enumerate(row_crates):
                cx = start_x + i * (col_w + gap_x)
                feats = crate.get("features", [])
                box_h = row_h_min + (len(feats) * 16)
                max_row_h = max(max_row_h, box_h)
                crate_coords[crate["name"]] = (cx, y_cursor, box_h)

                push('<g class="card">')
                push(_rect(cx, y_cursor, col_w, box_h, color_key=layer_name))
                push(_text(cx + col_w // 2, y_cursor + 24, crate["name"], cls="th"))
                push(_text(cx + col_w // 2, y_cursor + 44, f"v{crate['version']}", cls="txs", opacity="0.5"))
                if feats:
                    push(_text(cx + col_w // 2, y_cursor + 58, "FEATURES", cls="txs", weight="700", opacity="0.3"))
                    for j, f in enumerate(feats):
                        push(_text(cx + col_w // 2, y_cursor + 74 + j * 16, f, cls="txs"))
                push("</g>")
            y_cursor += max_row_h + gap_y

    # ── Dependency Arrows ──
    for crate in crates:
        if crate["name"] in crate_coords:
            p1 = crate_coords[crate["name"]]
            for dep in crate["dependencies"]:
                if dep in crate_coords:
                    p2 = crate_coords[dep]
                    x1, y1 = p1[0] + col_w // 2, p1[1] + p1[2]
                    x2, y2 = p2[0] + col_w // 2, p2[1]
                    if abs(x1 - x2) < 5:
                        if y1 < y2: push(f'<line x1="{x1}" y1="{y1}" x2="{x2}" y2="{y2}" class="arr" marker-end="url(#arrow)"/>')
                        else: push(f'<path d="M {x1} {y1} C {x1+40} {y1+40}, {x2+40} {y2-40}, {x2} {y2}" class="arr" marker-end="url(#arrow)"/>')
                    else:
                        ctrl_y = min(50, abs(y2 - y1) // 2)
                        push(f'<path d="M {x1} {y1} C {x1} {y1+ctrl_y}, {x2} {y2-ctrl_y}, {x2} {y2}" class="arr" marker-end="url(#arrow)"/>')

    # ── Skills & Agents ──
    y_cursor += 20
    push(f'<line x1="20" y1="{y_cursor}" x2="660" y2="{y_cursor}" stroke="{THEME["colors"]["divider"]}" stroke-width="1"/>')
    y_cursor += 30
    side_w, text_h = 310, 20
    box_h = 65 + max(len(skills), len(agents), 5) * text_h
    push(_container(20, y_cursor, side_w, box_h, f"{labels['skills']} ACTIVE SKILLS", ".agents/skills/"))
    for i, sk in enumerate(skills):
        push(_text(40, y_cursor + 58 + i * text_h, sk, anchor="start", cls="ts"))
    push(_container(350, y_cursor, side_w, box_h, f"{labels['agents']} COGNITIVE AGENTS", ".opencode/agents/"))
    for i, ag in enumerate(agents):
        push(_text(370, y_cursor + 58 + i * text_h, ag, anchor="start", cls="ts"))

    # ── Slash Commands ──
    y_cursor += box_h + 50
    push(f'<line x1="120" y1="{y_cursor}" x2="560" y2="{y_cursor}" stroke="{THEME["colors"]["divider"]}" stroke-width="1"/>')
    y_cursor += 30
    push(_text(340, y_cursor, "INTERFACE PROTOCOLS", cls="txs", weight="700", opacity="0.6"))
    y_cursor += 35
    for i, cmd in enumerate(commands[:14]):
        cx, cy = (20 if i % 2 == 0 else 345), y_cursor + (i // 2) * 38
        push(f'<g class="card">')
        push(_rect(cx, cy, 310, 30, rx=15, color_key="interface"))
        push(_text(cx + 155, cy + 15, cmd, cls="ts", weight="500"))
        push("</g>")

    y_cursor += ((len(commands[:14]) + 1) // 2) * 38 + 55
    footer = f"{cfg['project_name']} · {cfg['author']} · 2026 EDITION"
    push(_text(340, y_cursor, footer, cls="txs", weight="700", opacity="0.3"))

    return f'<svg width="100%" viewBox="0 0 680 {y_cursor+60}" xmlns="http://www.w3.org/2000/svg">\n{DEFS}\n' + "\n".join(parts) + "\n</svg>"

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
    svg = build_svg(cfg, d_crates, skills, agents, commands, labels)
    out.parent.mkdir(parents=True, exist_ok=True)
    out.write_text(svg, encoding="utf-8")
    print(f"Written: {out}")

if __name__ == "__main__": main()
