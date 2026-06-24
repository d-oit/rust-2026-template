"""SVG renderer — convert SceneDocument to standalone SVG string (legacy mode)."""

from .config import THEME, VIEWBOX_W, VIEWBOX_MARGIN, CENTER_X, RIGHT_EDGE, GAP_X
from .scene_model import SceneDocument, SceneNode


def wrap_text(text: str, max_chars: int = 50) -> list[str]:
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


def _esc(s: str) -> str:
    return s.replace("&", "&amp;").replace("<", "&lt;").replace(">", "&gt;")


def _grad_def(name: str, colors: list[str]) -> str:
    if len(colors) >= 3:
        return (f'<linearGradient id="{name}" x1="0" y1="0" x2="0" y2="1">'
                f'<stop offset="0%" stop-color="{colors[0]}"/>'
                f'<stop offset="50%" stop-color="{colors[1]}"/>'
                f'<stop offset="100%" stop-color="{colors[2]}"/>'
                f'</linearGradient>')
    return (f'<linearGradient id="{name}" x1="0" y1="0" x2="0" y2="1">'
            f'<stop offset="0%" stop-color="{colors[0]}"/>'
            f'<stop offset="100%" stop-color="{colors[-1]}"/>'
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
    return f'<rect x="{x}" y="{y}" width="{w}" height="4" rx="2" fill="{c["accent"]}" opacity="0.9"/>'


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
    return (f'<rect x="{x}" y="{y}" width="{bw}" height="20" rx="10" '
            f'fill="{c["bg"]}" stroke="{c["border"]}" stroke-width="1"/>'
            f'<text class="badge" x="{x + bw // 2}" y="{y + 10}" '
            f'text-anchor="middle" dominant-baseline="central" '
            f'fill="{c["text"]}">{_esc(text_content)}</text>')


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


def _section_divider(y: int) -> str:
    return f'<line x1="{VIEWBOX_MARGIN}" y1="{y}" x2="{RIGHT_EDGE}" y2="{y}" stroke="{THEME["colors"]["divider"]}" stroke-width="1"/>'


def _crate_card(x, y, crate_node: SceneNode, card_w=340) -> tuple[str, int]:
    meta = crate_node.metadata
    desc = meta.get("description", "")
    features = meta.get("features", [])
    deps_count = meta.get("deps_count", 0)
    desc_lines = wrap_text(desc, max_chars=max(int(card_w / 7), 30))
    desc_h = len(desc_lines) * 16 if desc_lines else 0
    feat_h = 26 if features else 0
    card_h = 70 + desc_h + feat_h + 10
    color_key = crate_node.color_key
    parts = [
        f'<g id="crate-{crate_node.label}" class="card" role="img" '
        f'aria-label="Crate: {crate_node.label} {crate_node.subtitle or ""}">',
        f'<title>{_esc(f"{crate_node.label} {crate_node.subtitle}: {desc}" if desc else f"{crate_node.label} {crate_node.subtitle}")}</title>',
        _rect(x, y, card_w, card_h, color_key=color_key),
        _accent_bar(x + 8, y + 8, card_w - 16, color_key=color_key),
        _text(x + 12, y + 28, crate_node.label, anchor="start", cls="th"),
    ]
    if crate_node.subtitle:
        parts.append(_text(x + card_w - 12, y + 28, crate_node.subtitle, anchor="end", cls="txs", opacity="0.55"))
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


def scene_to_svg(scene: SceneDocument, crate_coords: dict, layers: dict, no_graphviz: bool = False) -> str:
    """Convert SceneDocument to standalone SVG string."""
    parts: list[str] = []

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
        <feDropShadow dx="0" dy="2" stdDeviation="4" flood-opacity="0.1"/>
        <feDropShadow dx="0" dy="1" stdDeviation="2" flood-opacity="0.05"/>
      </filter>
      <filter id="soft-shadow" x="-10%" y="-10%" width="120%" height="130%">
        <feDropShadow dx="0" dy="4" stdDeviation="8" flood-opacity="0.08"/>
      </filter>
      <filter id="glow" x="-20%" y="-20%" width="140%" height="140%">
        <feGaussianBlur stdDeviation="3" result="blur"/>
        <feComposite in="SourceGraphic" in2="blur" operator="over"/>
      </filter>
      <style>
        @import url('https://fonts.googleapis.com/css2?family=Inter:wght@400;500;600;700;800&amp;display=swap');
        .th{{font-family:{THEME["font"]};font-size:13px;font-weight:700;fill:#0f172a;letter-spacing:-0.01em}}
        .ts{{font-family:{THEME["font"]};font-size:12.5px;font-weight:500;fill:#334155;letter-spacing:0.005em}}
        .txs{{font-family:{THEME["font"]};font-size:11px;font-weight:500;fill:#475569;letter-spacing:0.01em}}
        .tl{{font-family:{THEME["font"]};font-size:28px;font-weight:800;fill:#0f172a;letter-spacing:-0.03em}}
        .arr{{fill:none;stroke:{THEME["colors"]["arrow"]};stroke-width:1.5;opacity:0.6}}
        .arr-dep{{fill:none;stroke:#6366f1;stroke-width:1.5;opacity:0.5;stroke-dasharray:8 4}}
        .card{{filter:url(#shadow)}}
        .card-soft{{filter:url(#soft-shadow)}}
        .section-label{{font-family:{THEME["font"]};font-size:11px;font-weight:800;fill:#64748b;letter-spacing:0.1em;text-transform:uppercase}}
        .subtitle{{font-family:{THEME["font"]};font-size:12px;font-weight:600;fill:#94a3b8;letter-spacing:0.05em}}
        .badge{{font-family:{THEME["font"]};font-size:10px;font-weight:600;letter-spacing:0.02em}}
      </style>
    </defs>"""

    parts.append(f'<rect width="{VIEWBOX_W}" height="{scene.height}" fill="{THEME["colors"]["bg"]}"/>')
    parts.append(DEFS)

    node_map = {n.id: n for n in scene.all_nodes}

    # ── Header ──
    y_cursor = 50.0
    parts.append(f'<g id="header" role="banner" aria-label="System architecture header">')
    parts.append(_text(CENTER_X, y_cursor, scene.title, cls="tl", aria="System Architecture Title"))
    y_cursor += 20
    parts.append(_text(CENTER_X, y_cursor, scene.project_name, cls="subtitle", opacity="0.6"))
    y_cursor += 20
    parts.append(f'<line x1="{CENTER_X - 200}" y1="{y_cursor}" x2="{CENTER_X + 200}" y2="{y_cursor}" stroke="{THEME["colors"]["divider"]}" stroke-width="1"/>')
    parts.append("</g>")

    # ── Legend ──
    y_cursor += 30
    parts.append(f'<g id="legend" role="img" aria-label="Color legend">')
    legend_items = [
        ("apps", "Applications", "Binary crates and CLI tools"),
        ("core", "Core Libraries", "Shared library crates"),
        ("templates", "Templates", "Reusable architectural patterns"),
        ("other", "Examples", "Learning references and demos"),
    ]
    legend_x = VIEWBOX_MARGIN
    for color_key, label, desc in legend_items:
        c = THEME["colors"][color_key]
        parts.append(f'<rect x="{legend_x}" y="{y_cursor}" width="10" height="10" rx="5" fill="{c["accent"]}"/>')
        parts.append(_text(legend_x + 16, y_cursor + 5, label, anchor="start", cls="th", weight="600"))
        parts.append(_text(legend_x + 16, y_cursor + 20, desc, anchor="start", cls="txs", opacity="0.5"))
        legend_x += 280
    parts.append("</g>")
    y_cursor += 40

    # ── Pipeline ──
    y_cursor += 20
    parts.append(f'<g id="pipeline" role="region" aria-label="CI/CD Pipeline orchestration">')
    parts.append(_text(CENTER_X, y_cursor, "PIPELINE ORCHESTRATION", cls="section-label"))
    y_cursor += 30
    pipeline_nodes = [n for n in scene.all_nodes if n.kind == "pipeline_stage"]
    num_stages = len(pipeline_nodes)
    if num_stages:
        row_h, gap = 60, 20
        bw = (VIEWBOX_W - 2 * VIEWBOX_MARGIN - (num_stages - 1) * gap) // num_stages
        for i, pn in enumerate(pipeline_nodes):
            x = pn.x
            parts.append(f'<g class="card" role="img" aria-label="Pipeline stage: {pn.label} - {pn.subtitle or ""}">')
            parts.append(_rect(x, y_cursor, bw, row_h, color_key=pn.color_key))
            parts.append(_text(x + bw // 2, y_cursor + 20, pn.label, cls="th"))
            if pn.subtitle:
                parts.append(_text(x + bw // 2, y_cursor + 36, pn.subtitle, cls="txs", opacity="0.6"))
            if pn.metadata.get("timing"):
                parts.append(_text(x + bw // 2, y_cursor + 50, pn.metadata["timing"], cls="txs", opacity="0.55"))
            parts.append("</g>")
            if i > 0:
                parts.append(f'<line x1="{x - gap}" y1="{y_cursor + row_h // 2}" x2="{x}" y2="{y_cursor + row_h // 2}" class="arr" marker-end="url(#arrow)"/>')
        y_cursor += row_h + 10
    parts.append("</g>")

    # ── Workspace Topology ──
    y_cursor += 40
    parts.append(_section_divider(y_cursor))
    y_cursor += 25
    parts.append(f'<g id="workspace" role="region" aria-label="Workspace topology with {scene.labels.get("crates", 0)} components">')
    parts.append(_text(CENTER_X, y_cursor, f"WORKSPACE TOPOLOGY · {scene.labels.get('crates', 0)} COMPONENTS", cls="section-label"))
    y_cursor += 40

    # Dependencies
    parts.append(f'<g id="dependencies" role="img" aria-label="Crate dependency relationships">')
    for edge in scene.all_edges:
        if edge.style == "dependency":
            src_node = node_map.get(edge.source_id)
            tgt_node = node_map.get(edge.target_id)
            if src_node and tgt_node and src_node.label in crate_coords and tgt_node.label in crate_coords:
                p1 = crate_coords[src_node.label]
                p2 = crate_coords[tgt_node.label]
                sx, sy = p1[0] + p1[2] // 2, p1[1] + p1[3]
                tx, ty = p2[0] + p2[2] // 2, p2[1]
                mid_y = (sy + ty) // 2
                if abs(sx - tx) < 10:
                    path_d = f'M {sx} {sy} L {tx} {ty}'
                else:
                    path_d = f'M {sx} {sy} L {sx} {mid_y} L {tx} {mid_y} L {tx} {ty}'
                parts.append(f'<path d="{path_d}" '
                             f'class="arr-dep" marker-end="url(#arrow-dep)" '
                             f'aria-label="Dependency: {src_node.label} depends on {tgt_node.label}"/>')
    parts.append("</g>")

    # Crate cards
    layer_labels_map = {"apps": "APPLICATIONS", "core": "CORE LIBRARIES", "templates": "TEMPLATE CRATES", "other": "EXAMPLES"}
    for layer_name in ["apps", "core", "templates", "other"]:
        layer_crates_nodes = [n for n in scene.all_nodes if n.kind == "crate" and n.color_key == layer_name]
        if not layer_crates_nodes:
            continue
        parts.append(f'<g id="layer-{layer_name}" role="group" aria-label="{layer_labels_map.get(layer_name, layer_name)}">')
        parts.append(_text(VIEWBOX_MARGIN, y_cursor, layer_labels_map.get(layer_name, layer_name.upper()), anchor="start", cls="ts", weight="600", opacity="0.7"))
        y_cursor += 18
        for cn in layer_crates_nodes:
            if cn.label in crate_coords:
                cx, cy, cw, ch = crate_coords[cn.label]
                card_content, _ = _crate_card(cx, cy, cn, card_w=cw)
                parts.append(card_content)
        parts.append("</g>")
    parts.append("</g>")

    # ── Skills & Agents ──
    if crate_coords:
        max_crate_bottom = max(cy + ch for cx, cy, cw, ch in crate_coords.values())
        y_cursor = max_crate_bottom + 30
    else:
        y_cursor += 30
    parts.append(_section_divider(y_cursor))
    y_cursor += 30
    panel_w = (VIEWBOX_W - 2 * VIEWBOX_MARGIN - GAP_X) // 2
    text_h = 22
    skill_nodes = [n for n in scene.all_nodes if n.kind == "skill"]
    agent_nodes = [n for n in scene.all_nodes if n.kind == "agent" and not n.id.startswith("role_")]
    skills_h = 70 + max(len(skill_nodes), 5) * text_h
    agents_h = 70 + max(len(agent_nodes), 5) * text_h
    max_h = max(skills_h, agents_h)

    parts.append(f'<g id="skills" role="region" aria-label="{scene.labels.get("skills", 0)} active skills">')
    parts.append(_container(VIEWBOX_MARGIN, y_cursor, panel_w, max_h, f"{scene.labels.get('skills', 0)} ACTIVE SKILLS", ".agents/skills/", aria_label="Active skills list"))
    for i, sk in enumerate(skill_nodes):
        parts.append(_text(VIEWBOX_MARGIN + 20, y_cursor + 60 + i * text_h, sk.label, anchor="start", cls="ts"))
    parts.append("</g>")

    parts.append(f'<g id="agents" role="region" aria-label="{scene.labels.get("agents", 0)} cognitive agents">')
    parts.append(_container(VIEWBOX_MARGIN + panel_w + GAP_X, y_cursor, panel_w, max_h, f"{scene.labels.get('agents', 0)} COGNITIVE AGENTS", ".opencode/agents/", aria_label="Cognitive agents list"))
    if agent_nodes:
        for i, ag in enumerate(agent_nodes):
            parts.append(_text(VIEWBOX_MARGIN + panel_w + GAP_X + 20, y_cursor + 60 + i * text_h, ag.label, anchor="start", cls="ts"))
    else:
        parts.append(_text(VIEWBOX_MARGIN + panel_w + GAP_X + panel_w // 2, y_cursor + max_h // 2 + 10, "none configured", cls="txs", opacity="0.55"))
    parts.append("</g>")

    # ── Subagent Workflow ──
    y_cursor += max_h + 30
    parts.append(_section_divider(y_cursor))
    y_cursor += 30
    parts.append(f'<g id="subagent-workflow" role="region" aria-label="Subagent workflow and orchestration">')
    parts.append(_text(CENTER_X, y_cursor, "SUBAGENT WORKFLOW · MULTI-AGENT ORCHESTRATION", cls="section-label"))
    y_cursor += 35

    role_nodes = [n for n in scene.all_nodes if n.id.startswith("role_")]
    if role_nodes:
        role_w = panel_w
        for i, rn in enumerate(role_nodes):
            rx = VIEWBOX_MARGIN if i % 2 == 0 else VIEWBOX_MARGIN + panel_w + GAP_X
            ry = y_cursor + (i // 2) * 57
            parts.append(f'<g class="card" role="img" aria-label="Agent role: {rn.label}">')
            parts.append(_rect(rx, ry, role_w, 45, rx=10, color_key=rn.color_key))
            parts.append(_text(rx + 15, ry + 18, rn.label, anchor="start", cls="th", weight="600"))
            if rn.subtitle:
                parts.append(_text(rx + 15, ry + 35, rn.subtitle, anchor="start", cls="txs", opacity="0.6"))
            parts.append("</g>")
        y_cursor += ((len(role_nodes) + 1) // 2) * 57 + 15

    handoff_nodes = [n for n in scene.all_nodes if n.id.startswith("handoff_")]
    if handoff_nodes:
        parts.append(_text(CENTER_X, y_cursor, "HANDOFF PROTOCOL", cls="section-label"))
        y_cursor += 25
        handoff_w = (VIEWBOX_W - 2 * VIEWBOX_MARGIN - (len(handoff_nodes) - 1) * GAP_X) // len(handoff_nodes)
        for i, hn in enumerate(handoff_nodes):
            cx = VIEWBOX_MARGIN + i * (handoff_w + GAP_X)
            parts.append(f'<g class="card" role="img" aria-label="Handoff: {hn.label}">')
            parts.append(_rect(cx, y_cursor, handoff_w, 40, rx=8, color_key="interface"))
            parts.append(_text(cx + handoff_w // 2, y_cursor + 15, hn.label, cls="th", weight="500"))
            if hn.subtitle:
                parts.append(_text(cx + handoff_w // 2, y_cursor + 30, hn.subtitle, cls="txs", opacity="0.5"))
            parts.append("</g>")
        y_cursor += 60
    parts.append("</g>")

    # ── Error Handling ──
    y_cursor += 20
    parts.append(_section_divider(y_cursor))
    y_cursor += 30
    parts.append(f'<g id="error-handling" role="region" aria-label="Error handling patterns">')
    parts.append(_text(CENTER_X, y_cursor, "ERROR HANDLING PATTERNS", cls="section-label"))
    y_cursor += 30

    strategy_nodes = [n for n in scene.all_nodes if n.kind == "strategy"]
    for sn in strategy_nodes:
        parts.append(f'<g class="card" role="img" aria-label="Error strategy: {sn.label}">')
        parts.append(_rect(sn.x, y_cursor, panel_w, 50, rx=10, color_key=sn.color_key))
        parts.append(_text(sn.x + 15, y_cursor + 18, sn.label, anchor="start", cls="th", weight="600"))
        if sn.subtitle:
            parts.append(_text(sn.x + 15, y_cursor + 35, sn.subtitle, anchor="start", cls="txs", opacity="0.6"))
        parts.append("</g>")
    if strategy_nodes:
        y_cursor += 70

    error_nodes = [n for n in scene.all_nodes if n.kind == "error_type"]
    if error_nodes:
        parts.append(_text(CENTER_X, y_cursor, "CRATE ERROR TYPES", cls="section-label"))
        y_cursor += 25
        for en in error_nodes:
            parts.append(f'<g class="card-soft" role="img" aria-label="Error type: {en.label} from {en.metadata.get("crate", "")}">')
            parts.append(_rect(VIEWBOX_MARGIN, y_cursor, VIEWBOX_W - 2 * VIEWBOX_MARGIN, 40, rx=8, color_key="rose"))
            parts.append(_text(VIEWBOX_MARGIN + 15, y_cursor + 14, en.label, anchor="start", cls="th", weight="600"))
            if en.subtitle:
                parts.append(_text(VIEWBOX_MARGIN + 15, y_cursor + 30, en.subtitle, anchor="start", cls="txs", opacity="0.6"))
            if en.metadata.get("crate"):
                parts.append(_text(RIGHT_EDGE - 15, y_cursor + 14, en.metadata["crate"], anchor="end", cls="txs", opacity="0.55"))
            parts.append("</g>")
            y_cursor += 46
        y_cursor += 15
    parts.append("</g>")

    # ── Data Flow ──
    y_cursor += 20
    parts.append(_section_divider(y_cursor))
    y_cursor += 30
    parts.append(f'<g id="data-flow" role="region" aria-label="Data flow patterns">')
    parts.append(_text(CENTER_X, y_cursor, "DATA FLOW · COMPONENT INTERACTIONS", cls="section-label"))
    y_cursor += 30

    data_nodes = [n for n in scene.all_nodes if n.kind == "data_type"]
    if data_nodes:
        parts.append(_text(CENTER_X, y_cursor, "KEY DATA TYPES", cls="section-label"))
        y_cursor += 25
        for i, dn in enumerate(data_nodes):
            dx = VIEWBOX_MARGIN if i % 2 == 0 else VIEWBOX_MARGIN + panel_w + GAP_X
            dy = y_cursor + (i // 2) * 48
            parts.append(f'<g class="card-soft" role="img" aria-label="Data type: {_esc(dn.label)}">')
            parts.append(_rect(dx, dy, panel_w, 40, rx=8, color_key="interface"))
            parts.append(_text(dx + 15, dy + 15, dn.label, anchor="start", cls="th", weight="500"))
            if dn.subtitle:
                parts.append(_text(dx + 15, dy + 30, dn.subtitle, anchor="start", cls="txs", opacity="0.5"))
            parts.append("</g>")
        y_cursor += ((len(data_nodes) + 1) // 2) * 48 + 15
    parts.append("</g>")

    # ── Commands ──
    y_cursor += 20
    parts.append(_section_divider(y_cursor))
    y_cursor += 30
    parts.append(f'<g id="commands" role="region" aria-label="Interface protocols and commands">')
    parts.append(_text(CENTER_X, y_cursor, "INTERFACE PROTOCOLS", cls="section-label"))
    y_cursor += 35
    cmd_nodes = [n for n in scene.all_nodes if n.kind == "command"]
    cmd_cols = 3
    cmd_w = (VIEWBOX_W - 2 * VIEWBOX_MARGIN - (cmd_cols - 1) * GAP_X) // cmd_cols
    for cn in cmd_nodes[:18]:
        col = cmd_nodes.index(cn) % cmd_cols
        row = cmd_nodes.index(cn) // cmd_cols
        cx = VIEWBOX_MARGIN + col * (cmd_w + GAP_X)
        cy = y_cursor + row * 38
        parts.append(f'<g class="card-soft" role="img" aria-label="Command: {cn.label}">')
        parts.append(_rect(cx, cy, cmd_w, 30, rx=15, color_key="interface"))
        parts.append(_text(cx + cmd_w // 2, cy + 15, cn.label, cls="ts", weight="500"))
        parts.append("</g>")
    parts.append("</g>")

    y_cursor += ((min(len(cmd_nodes), 18) + cmd_cols - 1) // cmd_cols) * 38 + 60
    parts.append(f'<line x1="{CENTER_X - 200}" y1="{y_cursor}" x2="{CENTER_X + 200}" y2="{y_cursor}" stroke="{THEME["colors"]["divider"]}" stroke-width="1"/>')
    y_cursor += 20
    footer = f"{scene.project_name} · {scene.author} · 2026 EDITION"
    parts.append(_text(CENTER_X, y_cursor, footer, cls="txs", weight="700", opacity="0.35"))

    svg_height = y_cursor + 50

    return (f'<svg width="100%" viewBox="0 0 {VIEWBOX_W} {svg_height}" xmlns="http://www.w3.org/2000/svg" '
            f'role="img" aria-label="{scene.title} - {scene.project_name}">\n'
            + "\n".join(parts) + "\n</svg>")
