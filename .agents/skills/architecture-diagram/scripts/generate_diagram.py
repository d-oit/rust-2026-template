#!/usr/bin/env python3
"""
generate_diagram.py — Project architecture SVG generator
Scans the live project structure, including Rust workspace, and writes an architecture diagram.
Usage:
python generate_diagram.py [--root .] [--out docs/architecture.svg]
"""
import argparse
import json
import re
import subprocess
import sys
from pathlib import Path

# ── Default config ─────────────────────────────────────────────────────────
DEFAULT_CONFIG = {
    "title": "Project Architecture",
    "project_name": "Rust 2026 Template",
    "author": "maintainer",
    "pipeline_stages": [
        {"name": "lint", "color": "teal"},
        {"name": "test", "color": "blue"},
        {"name": "audit", "color": "rose"},
        {"name": "release", "color": "green"},
    ],
}

# ── Color palette (modern 2026: high-fidelity soft pastels) ────────────────
COLORS = {
    "teal": {"fill": "#f0fdf9", "stroke": "#14b8a6", "text": "#0f766e"},
    "blue": {"fill": "#eff6ff", "stroke": "#3b82f6", "text": "#1d4ed8"},
    "purple": {"fill": "#f5f3ff", "stroke": "#8b5cf6", "text": "#6d28d9"},
    "green": {"fill": "#f0fdf4", "stroke": "#22c55e", "text": "#15803d"},
    "gray": {"fill": "#f8fafc", "stroke": "#64748b", "text": "#334155"},
    "rose": {"fill": "#fff1f2", "stroke": "#f43f5e", "text": "#be123c"},
    "amber": {"fill": "#fffbeb", "stroke": "#f59e0b", "text": "#b45309"},
    "slate": {"fill": "#f1f5f9", "stroke": "#475569", "text": "#1e293b"},
}

# ── Discovery helpers ──────────────────────────────────────────────────────
def _read_frontmatter_name(path: Path) -> str:
    """Extract `name:` from YAML frontmatter, or fall back to stem."""
    try:
        content = path.read_text(encoding="utf-8")
        if content.startswith("---"):
            block = content.split("---", 2)[1]
            m = re.search(r"^name:\s*(.+)$", block, re.MULTILINE)
            if m:
                return m.group(1).strip().strip('"').strip("'")
    except Exception as e:
        print(f" [info] frontmatter parse skipped for {path.name}: {e}", file=sys.stderr)
    return path.stem

def discover_skills(root: Path) -> list[str]:
    skills_dir = root / ".agents" / "skills"
    if not skills_dir.is_dir():
        return []
    names = []
    for skill_dir in sorted(skills_dir.iterdir()):
        skill_md = skill_dir / "SKILL.md"
        if skill_dir.is_dir() and skill_md.exists():
            names.append(_read_frontmatter_name(skill_md))
    return names

def discover_agents(root: Path) -> list[str]:
    agents_dir = root / ".opencode" / "agents"
    if not agents_dir.is_dir():
        return []
    return sorted(p.stem for p in agents_dir.glob("*.md"))

def discover_commands(root: Path) -> list[str]:
    commands_dir = root / ".opencode" / "commands"
    if not commands_dir.is_dir():
        return []
    names = []
    for p in sorted(commands_dir.glob("*.md")):
        stem = p.stem
        names.append(stem if stem.startswith("/") else "/" + stem)
    return names

def discover_crates(root: Path) -> list[dict]:
    """Uses cargo metadata to find workspace crates, dependencies, and features."""
    try:
        # Secure subprocess call using a static list of literals
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
                # Find internal dependencies
                deps = []
                for dep in pkg.get("dependencies", []):
                    if dep.get("path"): # It's a path dependency (likely internal)
                        deps.append(dep["name"])

                # Extract features (excluding default)
                features = [f for f in pkg.get("features", {}).keys() if f != "default"]

                crates.append({
                    "name": pkg["name"],
                    "version": pkg["version"],
                    "dependencies": deps,
                    "features": sorted(features),
                    "description": pkg.get("description", ""),
                })
        return sorted(crates, key=lambda x: x["name"])
    except (subprocess.CalledProcessError, json.JSONDecodeError, KeyError) as e:
        print(f" [warn] failed to discover crates: {e}")
        return []

# ── SVG helpers ───────────────────────────────────────────────────────────
def _esc(s: str) -> str:
    return s.replace("&", "&amp;").replace("<", "&lt;").replace(">", "&gt;")

def _rect(x, y, w, h, rx=6, fill="#F1EFE8", stroke="#5F5E5A", sw=1.0, dash=False) -> str:
    dash_attr = ' stroke-dasharray="4 3"' if dash else ""
    return (f'<rect x="{x}" y="{y}" width="{w}" height="{h}" rx="{rx}" '
            f'fill="{fill}" stroke="{stroke}" stroke-width="{sw}"{dash_attr}/>')

def _text(x, y, content, anchor="middle", cls="ts", dy=None, opacity=None) -> str:
    dy_attr = f' dy="{dy}"' if dy else ""
    op_attr = f' opacity="{opacity}"' if opacity else ""
    return (f'<text class="{cls}" x="{x}" y="{y}" '
            f'text-anchor="{anchor}" dominant-baseline="central"{dy_attr}{op_attr}>'
            f'{_esc(content)}</text>')

def _line(x1, y1, x2, y2, cls="arr", arrow=True) -> str:
    marker = ' marker-end="url(#arrow)"' if arrow else ""
    return f'<line x1="{x1}" y1="{y1}" x2="{x2}" y2="{y2}" class="{cls}"{marker}/>'

def _section_line(y) -> str:
    return (f'<line x1="20" y1="{y}" x2="660" y2="{y}" '
            'stroke="#cbd5e1" stroke-width="0.5" opacity="0.3"/>')

def _container(x, y, w, h, label, sublabel=None) -> str:
    c = COLORS["gray"]
    parts = [
        '<g class="card">',
        _rect(x, y, w, h, rx=10, fill="none", stroke=c["stroke"], sw=1, dash=True),
        _text(x + w // 2, y + 16, label, cls="th"),
    ]
    if sublabel:
        parts.append(_text(x + w // 2, y + 32, sublabel, cls="ts"))
    parts.append("</g>")
    return "\n".join(parts)

def _pill(x, y, w, h, label, color="gray") -> str:
    c = COLORS.get(color, COLORS["gray"])
    return "\n".join([
        '<g class="card">',
        _rect(x, y, w, h, rx=h // 2, fill=c["fill"], stroke=c["stroke"], sw=1),
        _text(x + w // 2, y + h // 2, label, cls="ts"),
        "</g>",
    ])

# ── Main SVG builder ──────────────────────────────────────────────────────
DEFS = """<defs>
  <marker id="arrow" viewBox="0 0 10 10" refX="8" refY="5"
    markerWidth="6" markerHeight="6" orient="auto-start-reverse">
    <path d="M2 1L8 5L2 9" fill="none" stroke="#64748b"
      stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/>
  </marker>
  <filter id="shadow" x="-5%" y="-5%" width="110%" height="115%">
    <feDropShadow dx="0" dy="2" stdDeviation="3" flood-opacity="0.12"/>
  </filter>
  <style>
    @import url('https://fonts.googleapis.com/css2?family=Inter:wght@400;500;600;700&display=swap');
    .th{font-family:'Inter',system-ui,sans-serif;font-size:13px;font-weight:600;fill:#0f172a;letter-spacing:-0.01em}
    .ts{font-family:'Inter',system-ui,sans-serif;font-size:11px;font-weight:400;fill:#475569}
    .txs{font-family:'Inter',system-ui,sans-serif;font-size:9px;font-weight:400;fill:#64748b}
    .tl{font-family:'Inter',system-ui,sans-serif;font-size:18px;font-weight:700;fill:#0f172a;letter-spacing:-0.02em}
    .arr{fill:none;stroke:#94a3b8;stroke-width:1.2;opacity:0.6}
    .leader{fill:none;stroke:#cbd5e1;stroke-width:0.5;stroke-dasharray:3 3}
    .card{filter:url(#shadow)}
  </style>
</defs>"""

def build_svg(cfg: dict, crates: list[dict], skills: list[str], agents: list[str], commands: list[str], labels: dict) -> str:
    parts: list[str] = []
    y = 0

    def push(s: str):
        parts.append(s)

    # ── Title ───────────────────────────────────────────────────────────
    y = 35
    push(_text(340, y, cfg["title"], cls="tl"))
    y += 15
    push(f'<line x1="100" y1="{y}" x2="580" y2="{y}" stroke="#e2e8f0" stroke-width="1" opacity="0.6"/>')

    # ── Pipeline stages ───────────────────────────────────────────────────
    y += 20
    push(f'<text class="txs" x="21" y="{y}" opacity="0.8" font-weight="600" fill="#94a3b8">CI/CD PIPELINE</text>')
    y += 15
    stages = cfg.get("pipeline_stages", [])
    if stages:
        row_h = 38
        gap = 16
        bw = min(130, (640 - (len(stages) - 1) * gap) // len(stages))
        x = 21
        prev_right = None
        for s in stages:
            c = COLORS.get(s.get("color", "teal"), COLORS["teal"])
            push('<g class="card">')
            push(_rect(x, y, bw, row_h, rx=8, fill=c["fill"], stroke=c["stroke"], sw=1.5))
            push(_text(x + bw // 2, y + row_h // 2, s["name"], cls="th"))
            push("</g>")
            if prev_right is not None:
                push(_line(prev_right, y + row_h // 2, prev_right + gap, y + row_h // 2))
            prev_right = x + bw
            x += bw + gap
        y += row_h

    # ── Section divider ──────────────────────────────────────────────────
    y += 25
    push(_section_line(y))
    section_top = y + 15

    # ── Crate Dependencies (Improved Graph!) ──────────────────────────────
    push(_text(340, section_top + 10, f"RUST WORKSPACE: {labels['crates']} CRATES", cls="txs", opacity="0.8"))

    layers = {
        "apps": [],
        "core": [],
        "templates": [],
        "other": []
    }

    for crate in crates:
        name = crate["name"]
        if name == cfg.get("project_name", "") or name == "sample-app":
            layers["apps"].append(crate)
        elif "-template" in name:
            layers["templates"].append(crate)
        elif "example-" in name or name == "hello-world-example":
            layers["other"].append(crate)
        else:
            layers["core"].append(crate)

    current_y = section_top + 45
    crate_coords = {}
    col_w = 190
    row_h_base = 65
    gap_x = 25
    gap_y = 65 # Increased for modern spacing

    for layer_name in ["apps", "core", "templates", "other"]:
        layer_crates = layers[layer_name]
        if not layer_crates: continue

        max_per_row = 3
        for row_idx in range(0, len(layer_crates), max_per_row):
            row_crates = layer_crates[row_idx : row_idx + max_per_row]
            num_in_row = len(row_crates)
            total_row_w = num_in_row * col_w + (num_in_row - 1) * gap_x
            start_x = (680 - total_row_w) // 2

            max_row_box_h = 0
            for i, crate in enumerate(row_crates):
                cx = start_x + i * (col_w + gap_x)

                features = crate.get("features", [])
                box_h = row_h_base + (len(features) * 14) # Increased spacing to avoid overlap
                max_row_box_h = max(max_row_box_h, box_h)

                crate_coords[crate["name"]] = (cx, current_y, box_h)

                color = "blue"
                if layer_name == "apps": color = "amber"
                if layer_name == "templates": color = "purple"

                push('<g class="card">')
                push(_rect(cx, current_y, col_w, box_h, rx=10, fill=COLORS[color]["fill"], stroke=COLORS[color]["stroke"], sw=1.5))
                push(_text(cx + col_w // 2, current_y + 20, crate["name"], cls="th"))
                push(_text(cx + col_w // 2, current_y + 38, f"v{crate['version']}", cls="txs", opacity="0.7"))

                if features:
                    push(_text(cx + col_w // 2, current_y + 52, "features:", cls="txs", opacity="0.5"))
                    for j, feat in enumerate(features):
                        push(_text(cx + col_w // 2, current_y + 66 + j * 14, feat, cls="txs"))
                push("</g>")

            current_y += max_row_box_h + gap_y

    # Draw Dependency Arrows with Intelligent Routing
    for crate in crates:
        if crate["name"] in crate_coords:
            p1 = crate_coords[crate["name"]]
            for dep in crate["dependencies"]:
                if dep in crate_coords:
                    p2 = crate_coords[dep]

                    x1 = p1[0] + col_w // 2
                    y1 = p1[1] + p1[2]
                    x2 = p2[0] + col_w // 2
                    y2 = p2[1]

                    # Modern curved routing to avoid crossing labels
                    if abs(x1 - x2) < 5: # Vertically aligned
                        if y1 < y2: # Downward
                             push(_line(x1, y1, x2, y2, cls="arr"))
                        else: # Upward (side loop)
                             push(f'<path d="M {x1} {y1} C {x1+col_w//2+15} {y1+20}, {x2+col_w//2+15} {y2-20}, {x2} {y2}" class="arr" marker-end="url(#arrow)"/>')
                    else:
                        # Smooth S-curve
                        dx = x2 - x1
                        dy = y2 - y1
                        ctrl_y_offset = min(35, abs(dy) // 2)
                        if y1 < y2:
                            push(f'<path d="M {x1} {y1} C {x1} {y1+ctrl_y_offset}, {x2} {y2-ctrl_y_offset}, {x2} {y2}" class="arr" marker-end="url(#arrow)"/>')
                        else:
                            # Avoid box overlap by looping wide
                            offset = 60 if dx > 0 else -60
                            push(f'<path d="M {x1} {y1} C {x1+offset} {y1+25}, {x2+offset} {y2-25}, {x2} {y2}" class="arr" marker-end="url(#arrow)"/>')

    y = current_y + 5
    push(_section_line(y))
    section_top = y + 20

    # ── Skills | Agents ───────────────────────────────────────────────────
    col_w_side = 310
    skills_x = 20
    agents_x = skills_x + col_w_side + 20
    row_h_text = 18
    max_rows = max(len(skills), len(agents), 5)
    col_h = 45 + max_rows * row_h_text + 10

    push(_container(skills_x, section_top, col_w_side, col_h, f"{labels['skills']} SKILLS", ".agents/skills/"))
    for i, sk in enumerate(skills):
        push(f'<text class="ts" x="{skills_x+16}" y="{section_top+52+i*row_h_text}">{_esc(sk)}</text>')

    push(_container(agents_x, section_top, col_w_side, col_h, f"{labels['agents']} AGENTS", ".opencode/agents/"))
    for i, ag in enumerate(agents):
        push(f'<text class="ts" x="{agents_x+16}" y="{section_top+52+i*row_h_text}">{_esc(ag)}</text>')

    y = section_top + col_h + 15
    push(_section_line(y))
    y += 15

    # ── Commands ─────────────────────────────────────────────────────────
    push(_text(340, y + 10, f"{labels['commands']} SLASH COMMANDS", cls="txs", opacity="0.8"))
    push(_text(340, y + 26, ".opencode/commands/", cls="txs", opacity="0.5"))
    y += 45

    show_cmds = commands[:14]
    extra = len(commands) - len(show_cmds)
    pill_w = 310
    pill_h = 24
    pill_gap = 8
    col2_x = 345

    for i, cmd in enumerate(show_cmds):
        col = i % 2
        row = i // 2
        cx = 20 if col == 0 else col2_x
        cy = y + row * (pill_h + pill_gap)
        push(_pill(cx, cy, pill_w, pill_h, cmd, color="slate"))

    y += ((len(show_cmds) + 1) // 2) * (pill_h + pill_gap) + 10
    if extra > 0:
        push(_text(340, y + 6, f"+ {extra} more commands", cls="ts", opacity="0.6"))
        y += 20

    # ── Footer ───────────────────────────────────────────────────────────
    y += 30
    footer = f'{cfg.get("project_name", "Project")} · {cfg.get("author", "maintainer")}'
    push(f'<text class="txs" x="{340}" y="{y}" text-anchor="middle" opacity="0.4" font-weight="600">{_esc(footer.upper())}</text>')
    y += 35

    viewbox_h = y
    svg = (f'<svg width="100%" viewBox="0 0 680 {viewbox_h}" xmlns="http://www.w3.org/2000/svg">\n'
           f'{DEFS}\n' +
           "\n".join(parts) +
           "\n</svg>")
    return svg

def main():
    parser = argparse.ArgumentParser(description="Generate architecture SVG")
    parser.add_argument("--root", default=".", help="Project root directory")
    parser.add_argument("--out", default="docs/architecture.svg", help="Output SVG path")
    args = parser.parse_args()

    root = Path(args.root).resolve()
    out_path = Path(args.out) if Path(args.out).is_absolute() else root / args.out

    # Load config
    cfg_path = root / "docs" / "diagram-config.json"
    cfg = dict(DEFAULT_CONFIG)
    if cfg_path.exists():
        try:
            with open(cfg_path, encoding="utf-8") as f:
                cfg.update(json.load(f))
        except (json.JSONDecodeError, OSError) as e:
            print(f" [warn] failed to load diagram-config.json: {e}", file=sys.stderr)

    # Discover
    crates = discover_crates(root)
    skills = discover_skills(root)
    agents = discover_agents(root)
    commands = discover_commands(root)

    print(f"Found: {len(crates)} crates, {len(skills)} skills, {len(agents)} agents, {len(commands)} commands")

    # Fallbacks for display
    display_crates = crates if crates else [{"name": "(no crates found)", "version": "0.0.0", "dependencies": [], "features": []}]
    display_skills = skills if skills else ["(no skills found)"]
    display_agents = agents if agents else ["(no agents found)"]
    display_commands = commands if commands else ["/no-commands"]

    # Use actual counts for labels
    labels = {
        "crates": len(crates),
        "skills": len(skills),
        "agents": len(agents),
        "commands": len(commands)
    }

    svg = build_svg(cfg, display_crates, display_skills, display_agents, display_commands, labels)

    out_path.parent.mkdir(parents=True, exist_ok=True)
    out_path.write_text(svg, encoding="utf-8")
    print(f"Written: {out_path}")
    return 0

if __name__ == "__main__":
    main()
