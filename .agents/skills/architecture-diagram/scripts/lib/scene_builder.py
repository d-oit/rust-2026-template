"""Scene builder — convert discovery outputs and layout coordinates into SceneDocument."""

from pathlib import Path

from .config import (
    DEFAULT_CONFIG, DEFAULT_DESCRIPTIONS, VIEWBOX_W, VIEWBOX_MARGIN,
    CENTER_X, RIGHT_EDGE, GAP_X,
)
from .discovery import (
    discover_error_types, discover_agent_roles,
    discover_handoff_items, discover_data_types,
)
from .scene_model import SceneNode, SceneEdge, SceneSection, SceneDocument


def build_scene(
    cfg: dict,
    crates: list[dict],
    skills: list[str],
    agents: list[str],
    commands: list[str],
    labels: dict,
    root: Path,
    crate_coords: dict,
    layers: dict,
    no_graphviz: bool = False,
) -> SceneDocument:
    """Build a SceneDocument from discovery data and layout coordinates."""
    scene = SceneDocument(
        title=cfg["title"],
        project_name=cfg["project_name"],
        author=cfg.get("author", DEFAULT_CONFIG["author"]),
        labels=labels,
    )

    error_types = discover_error_types(root)
    agent_roles = discover_agent_roles(root)
    handoff_items = discover_handoff_items(root)
    data_types = discover_data_types(root)

    y_cursor = 50.0

    # ── Header ──
    header_nodes = [
        SceneNode(id="header_title", kind="section_label", x=CENTER_X, y=y_cursor, w=400, h=35,
                  label=cfg["title"], color_key="interface", metadata={"cls": "tl", "anchor": "center"}),
        SceneNode(id="header_subtitle", kind="section_label", x=CENTER_X, y=y_cursor + 25, w=400, h=18,
                  label=cfg["project_name"], color_key="interface", metadata={"cls": "subtitle", "anchor": "center"}),
    ]
    scene.sections.append(SceneSection(id="header", title="Header", nodes=header_nodes))
    scene.all_nodes.extend(header_nodes)
    y_cursor += 70

    # ── Legend ──
    legend_items = [
        ("apps", "Applications", "Binary crates and CLI tools"),
        ("core", "Core Libraries", "Shared library crates"),
        ("templates", "Templates", "Reusable architectural patterns"),
        ("other", "Examples", "Learning references and demos"),
    ]
    legend_x = float(VIEWBOX_MARGIN)
    legend_nodes = []
    for color_key, label, desc in legend_items:
        legend_nodes.append(SceneNode(
            id=f"legend_{color_key}", kind="section_label",
            x=legend_x, y=y_cursor, w=260, h=25,
            label=label, subtitle=desc, color_key=color_key,
        ))
        legend_x += 280
    scene.sections.append(SceneSection(id="legend", title="Legend", nodes=legend_nodes))
    scene.all_nodes.extend(legend_nodes)
    y_cursor += 45

    # ── Pipeline ──
    y_cursor += 20
    pipeline_nodes = []
    pipeline_edges = []
    stages = cfg.get("pipeline_stages", DEFAULT_CONFIG["pipeline_stages"])
    num_stages = len(stages)
    row_h, gap = 60, 20
    bw = (VIEWBOX_W - 2 * VIEWBOX_MARGIN - (num_stages - 1) * gap) // num_stages
    px = float(VIEWBOX_MARGIN)
    for i, stage in enumerate(stages):
        node = SceneNode(
            id=f"pipeline_{stage['name']}", kind="pipeline_stage",
            x=px, y=y_cursor, w=bw, h=row_h,
            label=stage["name"], subtitle=stage.get("desc", ""),
            color_key=stage.get("color", "teal"),
            metadata={"timing": stage.get("timing", ""), "cls": "th"},
        )
        pipeline_nodes.append(node)
        if i > 0:
            pipeline_edges.append(SceneEdge(
                id=f"pipeline_edge_{stage['name']}",
                source_id=f"pipeline_{stages[i-1]['name']}",
                target_id=f"pipeline_{stage['name']}",
                style="pipeline",
            ))
        px += bw + gap
    y_cursor += row_h + 10
    scene.sections.append(SceneSection(id="pipeline", title="Pipeline", nodes=pipeline_nodes, edges=pipeline_edges))
    scene.all_nodes.extend(pipeline_nodes)
    scene.all_edges.extend(pipeline_edges)

    # ── Workspace Topology ──
    y_cursor += 40
    workspace_nodes = []
    workspace_edges = []

    layer_labels = {"apps": "APPLICATIONS", "core": "CORE LIBRARIES", "templates": "TEMPLATE CRATES", "other": "EXAMPLES"}
    for layer_name in ["apps", "core", "templates", "other"]:
        layer_crates = layers.get(layer_name, [])
        if not layer_crates:
            continue
        for crate in layer_crates:
            if crate["name"] in crate_coords:
                cx, cy, cw, ch = crate_coords[crate["name"]]
                desc = crate.get("description") or DEFAULT_DESCRIPTIONS.get(crate["name"], "")
                workspace_nodes.append(SceneNode(
                    id=f"crate_{crate['name']}", kind="crate",
                    x=cx, y=cy, w=cw, h=ch,
                    label=crate["name"],
                    subtitle=f"v{crate['version']}",
                    color_key=layer_name,
                    metadata={
                        "version": crate["version"],
                        "description": desc,
                        "features": crate.get("features", []),
                        "deps_count": len(crate.get("dependencies", [])),
                        "layer": layer_labels.get(layer_name, layer_name),
                    },
                ))

    for crate in crates:
        if crate["name"] in crate_coords:
            for dep in crate["dependencies"]:
                if dep in crate_coords:
                    workspace_edges.append(SceneEdge(
                        id=f"dep_{crate['name']}_{dep}",
                        source_id=f"crate_{crate['name']}",
                        target_id=f"crate_{dep}",
                        style="dependency",
                    ))

    if crate_coords:
        max_crate_bottom = max(cy + ch for cx, cy, cw, ch in crate_coords.values())
        y_cursor = max_crate_bottom + 30
    else:
        y_cursor += 30

    scene.sections.append(SceneSection(id="workspace", title="Workspace Topology", nodes=workspace_nodes, edges=workspace_edges))
    scene.all_nodes.extend(workspace_nodes)
    scene.all_edges.extend(workspace_edges)

    # ── Skills & Agents ──
    panel_w = (VIEWBOX_W - 2 * VIEWBOX_MARGIN - GAP_X) // 2
    text_h = 22
    skills_h = 70 + max(len(skills), 5) * text_h
    agents_h = 70 + max(len(agents), 5) * text_h
    max_h = max(skills_h, agents_h)

    skill_nodes = []
    for i, sk in enumerate(skills):
        skill_nodes.append(SceneNode(
            id=f"skill_{sk}", kind="skill",
            x=float(VIEWBOX_MARGIN + 20), y=y_cursor + 60 + i * text_h,
            w=panel_w - 40, h=text_h,
            label=sk, color_key="core",
        ))

    agent_nodes = []
    if agents:
        for i, ag in enumerate(agents):
            agent_nodes.append(SceneNode(
                id=f"agent_{ag}", kind="agent",
                x=float(VIEWBOX_MARGIN + panel_w + GAP_X + 20), y=y_cursor + 60 + i * text_h,
                w=panel_w - 40, h=text_h,
                label=ag, color_key="interface",
            ))

    scene.sections.append(SceneSection(id="skills_agents", title="Skills & Agents",
                                       nodes=skill_nodes + agent_nodes))
    scene.all_nodes.extend(skill_nodes)
    scene.all_nodes.extend(agent_nodes)
    y_cursor += max_h + 30

    # ── Subagent Workflow ──
    y_cursor += 30
    workflow_nodes = []
    role_w, role_h, role_gap = panel_w, 45, 12
    for i, role in enumerate(agent_roles):
        rx = VIEWBOX_MARGIN if i % 2 == 0 else VIEWBOX_MARGIN + panel_w + GAP_X
        ry = y_cursor + (i // 2) * (role_h + role_gap)
        workflow_nodes.append(SceneNode(
            id=f"role_{role['name']}", kind="agent",
            x=float(rx), y=ry, w=role_w, h=role_h,
            label=role["name"], subtitle=" · ".join(role["skills"]),
            color_key=role["color"],
        ))
    y_cursor += ((len(agent_roles) + 1) // 2) * (role_h + role_gap) + 15 if agent_roles else 0

    handoff_nodes = []
    if handoff_items:
        handoff_w = (VIEWBOX_W - 2 * VIEWBOX_MARGIN - (len(handoff_items) - 1) * GAP_X) // len(handoff_items)
        for i, item in enumerate(handoff_items):
            cx = VIEWBOX_MARGIN + i * (handoff_w + GAP_X)
            handoff_nodes.append(SceneNode(
                id=f"handoff_{i}", kind="section_label",
                x=float(cx), y=y_cursor, w=handoff_w, h=40,
                label=item["path"], subtitle=item["desc"],
                color_key="interface",
            ))
        y_cursor += 60

    scene.sections.append(SceneSection(id="subagent_workflow", title="Subagent Workflow",
                                       nodes=workflow_nodes + handoff_nodes))
    scene.all_nodes.extend(workflow_nodes)
    scene.all_nodes.extend(handoff_nodes)

    # ── Error Handling ──
    y_cursor += 20
    error_nodes = []

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

    error_nodes.append(SceneNode(
        id="strategy_thiserror", kind="strategy",
        x=float(VIEWBOX_MARGIN), y=y_cursor, w=panel_w, h=50,
        label="thiserror",
        subtitle=f"Library error types — {thiserror_count} crates using derive macro",
        color_key="core",
    ))
    error_nodes.append(SceneNode(
        id="strategy_anyhow", kind="strategy",
        x=float(VIEWBOX_MARGIN + panel_w + GAP_X), y=y_cursor, w=panel_w, h=50,
        label="anyhow",
        subtitle="Application errors — Context-rich error propagation",
        color_key="apps",
    ))
    y_cursor += 70

    for err in error_types:
        error_nodes.append(SceneNode(
            id=f"error_{err['name']}", kind="error_type",
            x=float(VIEWBOX_MARGIN), y=y_cursor, w=VIEWBOX_W - 2 * VIEWBOX_MARGIN, h=40,
            label=err["name"], subtitle=" · ".join(err["variants"]),
            color_key="rose",
            metadata={"crate": err["crate"]},
        ))
        y_cursor += 46

    scene.sections.append(SceneSection(id="error_handling", title="Error Handling", nodes=error_nodes))
    scene.all_nodes.extend(error_nodes)

    # ── Data Flow ──
    y_cursor += 20
    data_nodes = []
    for i, dt in enumerate(data_types):
        dx = VIEWBOX_MARGIN if i % 2 == 0 else VIEWBOX_MARGIN + panel_w + GAP_X
        dy = y_cursor + (i // 2) * 48
        data_nodes.append(SceneNode(
            id=f"data_{dt['name']}", kind="data_type",
            x=float(dx), y=dy, w=panel_w, h=40,
            label=dt["name"], subtitle=f"{dt['desc']} — {dt['crate']}",
            color_key="interface",
        ))
    y_cursor += ((len(data_types) + 1) // 2) * 48 + 15 if data_types else 0

    scene.sections.append(SceneSection(id="data_flow", title="Data Flow", nodes=data_nodes))
    scene.all_nodes.extend(data_nodes)

    # ── Commands ──
    y_cursor += 20
    cmd_nodes = []
    cmd_cols = 3
    cmd_w = (VIEWBOX_W - 2 * VIEWBOX_MARGIN - (cmd_cols - 1) * GAP_X) // cmd_cols
    for i, cmd in enumerate(commands[:18]):
        col = i % cmd_cols
        row = i // cmd_cols
        cx = VIEWBOX_MARGIN + col * (cmd_w + GAP_X)
        cy = y_cursor + row * 38
        cmd_nodes.append(SceneNode(
            id=f"cmd_{cmd}", kind="command",
            x=float(cx), y=cy, w=cmd_w, h=30,
            label=cmd, color_key="interface",
        ))
    y_cursor += ((len(commands[:18]) + cmd_cols - 1) // cmd_cols) * 38 + 60

    scene.sections.append(SceneSection(id="commands", title="Commands", nodes=cmd_nodes))
    scene.all_nodes.extend(cmd_nodes)

    scene.height = y_cursor + 50
    return scene
