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
def build_svg(cfg: dict, crates: list[dict], skills: list[str], agents: list[str], commands: list[str], labels: dict) -> str:
    parts: list[str] = []
    y_cursor = 0
    def push(s: str): parts.append(s)

    # ── Gradient Definitions ──
    grad_defs = []
    for key in ["apps", "core", "templates", "other", "pipeline", "interface", "rose", "teal", "blue", "green"]:
        c = THEME["colors"].get(key)
        if c and "grad" in c:
            grad_defs.append(_grad_def(f"grad-{key}", c["grad"]))

    # ── Style Definitions (2026 Best Practice: Semantic, Accessible) ──
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
        .legend-dot{{display:inline-block;width:8px;height:8px;border-radius:50%;margin-right:6px}}
      </style>
    </defs>"""

    # ── Background ──
    push(f'<rect width="680" height="3000" fill="{THEME["colors"]["bg"]}"/>')

    # ── Header (C4 Level 0: System Context) ──
    y_cursor = 50
    push(f'<g id="header" role="banner" aria-label="System architecture header">')
    push(_text(340, y_cursor, cfg["title"], cls="tl", aria="System Architecture Title"))
    y_cursor += 18
    push(_text(340, y_cursor, cfg["project_name"], cls="subtitle", opacity="0.6"))
    y_cursor += 18
    push(f'<line x1="140" y1="{y_cursor}" x2="540" y2="{y_cursor}" stroke="{THEME["colors"]["divider"]}" stroke-width="1"/>')
    push("</g>")

    # ── Legend (2026 Best Practice: Visual key for color coding) ──
    y_cursor += 25
    push(f'<g id="legend" role="img" aria-label="Color legend">')
    legend_items = [
        ("apps", "Applications"),
        ("core", "Core Libraries"),
        ("templates", "Templates"),
        ("other", "Examples"),
    ]
    legend_x = 21
    for color_key, label in legend_items:
        c = THEME["colors"][color_key]
        push(f'<rect x="{legend_x}" y="{y_cursor}" width="8" height="8" rx="4" fill="{c["accent"]}"/>')
        push(_text(legend_x + 14, y_cursor + 4, label, anchor="start", cls="txs", opacity="0.6"))
        legend_x += len(label) * 6 + 30
    push("</g>")
    y_cursor += 20

    # ── CI/CD Pipeline (C4 Level 1: Container) ──
    y_cursor += 25
    push(f'<g id="pipeline" role="region" aria-label="CI/CD Pipeline orchestration">')
    push(_text(340, y_cursor, "PIPELINE ORCHESTRATION", cls="section-label"))
    y_cursor += 30
    stages = [
        ("ANALYZE", "teal", "lint · clippy"),
        ("VALIDATE", "blue", "test · nextest"),
        ("HARDEN", "rose", "audit · deny"),
        ("DEPLOY", "green", "release · publish"),
    ]
    row_h = 48
    gap = 18
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

    # ── Workspace Topology (C4 Level 2: Components) ──
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

                crate_name = crate["name"]
                crate_version = crate["version"]
                push(f'<g id="crate-{crate_name}" class="card" role="img" '
                     f'aria-label="Crate: {crate_name} v{crate_version}">')
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

    # ── Dependency Arrows (routed through left margin to avoid card overlap) ──
    push(f'<g id="dependencies" role="img" aria-label="Crate dependency relationships">')
    MARGIN_X = 8
    for crate in crates:
        if crate["name"] in crate_coords:
            p1 = crate_coords[crate["name"]]
            src_name = crate["name"]
            for dep in crate["dependencies"]:
                if dep in crate_coords:
                    p2 = crate_coords[dep]
                    # Source: left edge, vertical center
                    sx, sy = p1[0], p1[1] + p1[2] // 2
                    # Target: left edge, vertical center
                    tx, ty = p2[0], p2[1] + p2[2] // 2
                    # Route through left margin with smooth curves
                    # Use orthogonal routing: source → left → down → right → target
                    control_offset_x = 25
                    push(f'<path d="M {sx} {sy} '
                         f'C {sx - control_offset_x} {sy}, {MARGIN_X + 12} {sy}, {MARGIN_X + 12} {sy} '
                         f'L {MARGIN_X + 12} {ty} '
                         f'C {MARGIN_X + 12} {ty}, {tx - control_offset_x} {ty}, {tx} {ty}" '
                         f'class="arr-dep" marker-end="url(#arrow-dep)" '
                         f'aria-label="Dependency: {src_name} depends on {dep}"/>')
    push("</g>")
    push("</g>")

    # ── Skills & Agents (C4 Level 3: Code/Patterns) ──
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

    # ── Subagent Workflow (Multi-Agent Orchestration) ──
    y_cursor += 20
    push(f'<line x1="20" y1="{y_cursor}" x2="660" y2="{y_cursor}" stroke="{THEME["colors"]["divider"]}" stroke-width="1"/>')
    y_cursor += 30
    push(f'<g id="subagent-workflow" role="region" aria-label="Subagent workflow and orchestration">')
    push(_text(340, y_cursor, "SUBAGENT WORKFLOW · MULTI-AGENT ORCHESTRATION", cls="section-label"))
    y_cursor += 35

    # Agent roles and their skills
    agent_roles = [
        ("code-agent", "build-rust · lint-rust · test-rust · anti-ai-slop", "teal"),
        ("release-agent", "release-rust · crates-io-name-check", "green"),
        ("quality-agent", "skill-evaluator · codacy · dora-report · metrics-reporter", "blue"),
        ("meta-agent", "skill-creator · skill-evaluator", "templates"),
    ]

    # Draw agent roles as horizontal cards
    role_w = 305
    role_h = 40
    role_gap = 10

    for i, (role_name, skills, color_key) in enumerate(agent_roles):
        rx = 21 if i % 2 == 0 else 354
        ry = y_cursor + (i // 2) * (role_h + role_gap)
        push(f'<g class="card" role="img" aria-label="Agent role: {role_name}">')
        push(_rect(rx, ry, role_w, role_h, rx=10, color_key=color_key))
        push(_text(rx + 12, ry + 15, role_name, anchor="start", cls="th", weight="600"))
        push(_text(rx + 12, ry + 30, skills, anchor="start", cls="txs", opacity="0.6"))
        push("</g>")

    y_cursor += 2 * (role_h + role_gap) + 15

    # Skill dependency chains
    push(_text(340, y_cursor, "SKILL DEPENDENCY CHAINS", cls="section-label"))
    y_cursor += 25

    chains = [
        ("build → lint → test → release", "Primary pipeline"),
        ("anti-ai-slop / privacy-first / codacy → lint → test", "Quality gates"),
        ("skill-creator → skill-evaluator", "Meta workflow"),
    ]

    chain_w = 638
    chain_h = 28

    for i, (chain, desc) in enumerate(chains):
        cy = y_cursor + i * (chain_h + 6)
        push(f'<g class="card-soft" role="img" aria-label="Skill chain: {chain}">')
        push(_rect(21, cy, chain_w, chain_h, rx=8, color_key="interface"))
        push(_text(36, cy + chain_h // 2, chain, anchor="start", cls="ts", weight="500"))
        push(_text(644, cy + chain_h // 2, desc, anchor="end", cls="txs", opacity="0.5"))
        push("</g>")

    y_cursor += len(chains) * (chain_h + 6) + 15

    # Handoff protocol
    push(_text(340, y_cursor, "HANDOFF PROTOCOL", cls="section-label"))
    y_cursor += 25

    handoff_items = [
        ("workflow-state.json", "Live state tracking"),
        (".agents/events/", "Immutable event files"),
        ("metrics.jsonl", "Aggregated metrics"),
    ]

    for i, (file_path, desc) in enumerate(handoff_items):
        cx = 21 + i * 218
        push(f'<g class="card" role="img" aria-label="Handoff: {file_path}">')
        push(_rect(cx, y_cursor, 210, 35, rx=8, color_key="interface"))
        push(_text(cx + 105, y_cursor + 13, file_path, cls="th", weight="500"))
        push(_text(cx + 105, y_cursor + 27, desc, cls="txs", opacity="0.5"))
        push("</g>")

    y_cursor += 55
    push("</g>")

    # ── Error Handling Patterns ──
    y_cursor += 20
    push(f'<line x1="20" y1="{y_cursor}" x2="660" y2="{y_cursor}" stroke="{THEME["colors"]["divider"]}" stroke-width="1"/>')
    y_cursor += 30
    push(f'<g id="error-handling" role="region" aria-label="Error handling patterns and error types">')
    push(_text(340, y_cursor, "ERROR HANDLING PATTERNS", cls="section-label"))
    y_cursor += 30

    # Error strategy cards
    error_strategies = [
        ("thiserror", "Library error types", "Derive macro for error enums", "core"),
        ("anyhow", "Application errors", "Context-rich error propagation", "apps"),
    ]

    strat_w = 305
    strat_h = 45

    for i, (name, title, desc, color_key) in enumerate(error_strategies):
        rx = 21 if i % 2 == 0 else 354
        push(f'<g class="card" role="img" aria-label="Error strategy: {name}">')
        push(_rect(rx, y_cursor, strat_w, strat_h, rx=10, color_key=color_key))
        push(_text(rx + 15, y_cursor + 16, name, anchor="start", cls="th", weight="600"))
        push(_text(rx + 15, y_cursor + 32, f"{title} — {desc}", anchor="start", cls="txs", opacity="0.6"))
        push("</g>")

    y_cursor += strat_h + 20

    # Crate-specific error types with variants
    push(_text(340, y_cursor, "CRATE ERROR TYPES & VARIANTS", cls="section-label"))
    y_cursor += 25

    error_types = [
        ("CheckpointError", ["Serialization", "Io", "Migration", "VersionMismatch", "Storage"], "checkpoint-template"),
        ("StorageError", ["NotFound", "Serialization", "Connection", "Backend", "Poisoned"], "hybrid-storage-template"),
        ("ServerError", ["ToolNotFound", "Tool", "Init"], "mcp-server-template"),
        ("ActorError", ["Panic", "Timeout", "MailboxFull", "State"], "actor-runtime-template"),
        ("DispatchError", ["Unknown", "Handler"], "example-registry-pattern"),
    ]

    err_w = 638
    err_h = 40

    for i, (err_name, variants, crate_name) in enumerate(error_types):
        cy = y_cursor + i * (err_h + 6)
        push(f'<g class="card-soft" role="img" aria-label="Error type: {err_name} from {crate_name}">')
        push(_rect(21, cy, err_w, err_h, rx=8, color_key="rose"))
        push(_text(36, cy + 14, err_name, anchor="start", cls="th", weight="600"))
        push(_text(36, cy + 30, " · ".join(variants), anchor="start", cls="txs", opacity="0.6"))
        push(_text(644, cy + 14, crate_name, anchor="end", cls="txs", opacity="0.4"))
        push("</g>")

    y_cursor += len(error_types) * (err_h + 6) + 15

    # Error conversion patterns
    push(_text(340, y_cursor, "ERROR CONVERSION PATTERNS", cls="section-label"))
    y_cursor += 25

    conversion_patterns = [
        ("#[from]", "Auto-convert inner error types", "Storage(#[from] storage::StorageError)"),
        ("#[source]", "Preserve error chain for display", "Io(#[source] std::io::Error)"),
        ("From impl", "Manual conversion for complex types", "impl From<ToolError> for ServerError"),
    ]

    conv_w = 638
    conv_h = 35

    for i, (pattern, desc, example) in enumerate(conversion_patterns):
        cy = y_cursor + i * (conv_h + 6)
        push(f'<g class="card-soft" role="img" aria-label="Conversion pattern: {pattern}">')
        push(_rect(21, cy, conv_w, conv_h, rx=8, color_key="interface"))
        push(_text(36, cy + 13, pattern, anchor="start", cls="th", weight="600"))
        push(_text(36, cy + 27, f"{desc} — {example}", anchor="start", cls="txs", opacity="0.55"))
        push("</g>")

    y_cursor += len(conversion_patterns) * (conv_h + 6) + 15

    # Error flow pattern
    push(_text(340, y_cursor, "ERROR PROPAGATION FLOW", cls="section-label"))
    y_cursor += 25

    flow_items = [
        ("Library Layer", "thiserror enums", "templates"),
        ("Application Layer", "anyhow::Result + context", "apps"),
        ("CLI Boundary", "User-friendly messages", "interface"),
    ]

    flow_w = 195
    flow_h = 35

    for i, (layer, desc, color_key) in enumerate(flow_items):
        fx = 21 + i * (flow_w + 20)
        push(f'<g class="card" role="img" aria-label="Error flow: {layer}">')
        push(_rect(fx, y_cursor, flow_w, flow_h, rx=8, color_key=color_key))
        push(_text(fx + flow_w // 2, y_cursor + 14, layer, cls="th", weight="500"))
        push(_text(fx + flow_w // 2, y_cursor + 28, desc, cls="txs", opacity="0.5"))
        push("</g>")
        # Draw arrow between flow items
        if i < len(flow_items) - 1:
            arrow_x = fx + flow_w + 5
            push(f'<line x1="{arrow_x}" y1="{y_cursor + flow_h // 2}" x2="{arrow_x + 10}" y2="{y_cursor + flow_h // 2}" class="arr" marker-end="url(#arrow)"/>')

    y_cursor += flow_h + 15
    push("</g>")

    # ── Data Flow Between Components ──
    y_cursor += 20
    push(f'<line x1="20" y1="{y_cursor}" x2="660" y2="{y_cursor}" stroke="{THEME["colors"]["divider"]}" stroke-width="1"/>')
    y_cursor += 30
    push(f'<g id="data-flow" role="region" aria-label="Data flow patterns between components">')
    push(_text(340, y_cursor, "DATA FLOW · COMPONENT INTERACTIONS", cls="section-label"))
    y_cursor += 30

    # Data flow patterns
    flow_patterns = [
        {
            "name": "Storage Pattern",
            "flow": ["Application", "HybridStorage", "Backend Trait", "Memory / SQLite"],
            "color": "core",
            "desc": "Key-value CRUD with swappable backends",
        },
        {
            "name": "MCP Server Pattern",
            "flow": ["Application", "McpServer", "Tool Trait", "Echo / Calc Tool"],
            "color": "templates",
            "desc": "Dynamic tool registry with lifecycle hooks",
        },
        {
            "name": "Actor Pattern",
            "flow": ["Supervisor", "Mailbox", "Actor", "State Transitions"],
            "color": "apps",
            "desc": "Message-driven with restart strategies",
        },
        {
            "name": "Checkpoint Pattern",
            "flow": ["Application", "CheckpointManager", "Storable Trait", "FileStorage"],
            "color": "other",
            "desc": "Atomic save/load with version migration",
        },
        {
            "name": "Registry Pattern",
            "flow": ["Dispatcher", "Registry HashMap", "Handler Trait", "Echo / Reverse"],
            "color": "teal",
            "desc": "Command dispatch with dynamic registration",
        },
    ]

    flow_box_w = 638
    flow_box_h = 55
    node_w = 135
    node_h = 25
    node_gap = 18

    for i, pattern in enumerate(flow_patterns):
        py = y_cursor + i * (flow_box_h + 10)
        color_key = pattern["color"]

        # Draw container box
        push(f'<g class="card-soft" role="img" aria-label="Data flow: {pattern["name"]}">')
        push(_rect(21, py, flow_box_w, flow_box_h, rx=10, color_key="interface"))

        # Pattern name and description
        push(_text(36, py + 14, pattern["name"], anchor="start", cls="th", weight="600"))
        push(_text(36, py + 30, pattern["desc"], anchor="start", cls="txs", opacity="0.5"))

        # Draw flow nodes
        flow = pattern["flow"]
        total_flow_w = len(flow) * node_w + (len(flow) - 1) * node_gap
        start_x = 36
        node_y = py + 12

        for j, node in enumerate(flow):
            nx = start_x + j * (node_w + node_gap)
            # Draw node
            push(f'<rect x="{nx}" y="{node_y}" width="{node_w}" height="{node_h}" rx="6" '
                 f'fill="{THEME["colors"][color_key]["bg"]}" stroke="{THEME["colors"][color_key]["border"]}" stroke-width="1"/>')
            push(_text(nx + node_w // 2, node_y + node_h // 2, node, cls="txs", weight="500"))
            # Draw arrow to next node
            if j < len(flow) - 1:
                arrow_x = nx + node_w + 2
                push(f'<line x1="{arrow_x}" y1="{node_y + node_h // 2}" x2="{arrow_x + node_gap - 4}" y2="{node_y + node_h // 2}" '
                     f'stroke="{THEME["colors"]["arrow"]}" stroke-width="1.2" opacity="0.5" marker-end="url(#arrow)"/>')

        push("</g>")

    y_cursor += len(flow_patterns) * (flow_box_h + 10) + 10

    # Key data types
    push(_text(340, y_cursor, "KEY DATA TYPES", cls="section-label"))
    y_cursor += 25

    data_types = [
        ("ToolRequest / ToolResponse", "MCP message protocol", "mcp-server-template"),
        ("StateTransition<T>", "Init · Update · Reset", "actor-runtime-template"),
        ("CheckpointHeader", "Version · Timestamp · App", "checkpoint-template"),
        ("Storable trait", "Serialize + Deserialize + Version", "checkpoint-template"),
    ]

    dt_w = 305
    dt_h = 35

    for i, (type_name, desc, crate_name) in enumerate(data_types):
        dx = 21 if i % 2 == 0 else 354
        dy = y_cursor + (i // 2) * (dt_h + 8)
        safe_name = _esc(type_name)
        push(f'<g class="card-soft" role="img" aria-label="Data type: {safe_name}">')
        push(_rect(dx, dy, dt_w, dt_h, rx=8, color_key="interface"))
        push(_text(dx + 12, dy + 13, type_name, anchor="start", cls="th", weight="500"))
        push(_text(dx + 12, dy + 27, f"{desc} — {crate_name}", anchor="start", cls="txs", opacity="0.5"))
        push("</g>")

    y_cursor += 2 * (dt_h + 8) + 15
    push("</g>")

    # ── Slash Commands (Interface Protocols) ──
    y_cursor += max_h + 50
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

    # ── Trim background rect to actual content ──
    svg_height = y_cursor + 50
    parts[0] = f'<rect width="680" height="{svg_height}" fill="{THEME["colors"]["bg"]}"/>'

    # ── 2026 Best Practice: Semantic SVG with ARIA ──
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
    svg = build_svg(cfg, d_crates, skills, agents, commands, labels)
    out.parent.mkdir(parents=True, exist_ok=True)
    out.write_text(svg, encoding="utf-8")
    print(f"Written: {out}")

if __name__ == "__main__": main()
