"""Excalidraw renderer — convert SceneDocument to Excalidraw v2 JSON format.

Uses native Excalidraw features:
- frame elements for section grouping
- label property for bound text on shapes
- startBinding/endBinding for arrow connections
- proper element ordering (children before frames)
"""

from .scene_model import SceneDocument, SceneNode


EXCALIDRAW_COLORS = {
    "apps":      {"bg": "#fef3c7", "stroke": "#d97706"},
    "core":      {"bg": "#dbeafe", "stroke": "#2563eb"},
    "templates": {"bg": "#f3e8ff", "stroke": "#9333ea"},
    "other":     {"bg": "#dcfce7", "stroke": "#16a34a"},
    "pipeline":  {"bg": "#ccfbf1", "stroke": "#0d9488"},
    "interface": {"bg": "#f1f5f9", "stroke": "#64748b"},
    "rose":      {"bg": "#ffe4e6", "stroke": "#e11d48"},
    "teal":      {"bg": "#ccfbf1", "stroke": "#0d9488"},
    "blue":      {"bg": "#dbeafe", "stroke": "#2563eb"},
    "green":     {"bg": "#dcfce7", "stroke": "#16a34a"},
}


def _seed(id_str: str) -> int:
    h = 0
    for c in id_str:
        h = (h * 31 + ord(c)) & 0xFFFFFFFF
    return h


def _rect(el_id: str, x: float, y: float, w: float, h: float,
          color_key: str = "interface", label: str | None = None,
          label_font_size: int = 14, label_align: str = "center",
          label_valign: str = "middle", label_color: str | None = None,
          stroke_width: int = 2, opacity: int = 100,
          frame_id: str | None = None) -> dict:
    colors = EXCALIDRAW_COLORS.get(color_key, EXCALIDRAW_COLORS["interface"])
    el = {
        "id": el_id,
        "type": "rectangle",
        "x": x, "y": y, "width": w, "height": h,
        "angle": 0,
        "strokeColor": colors["stroke"],
        "backgroundColor": colors["bg"],
        "fillStyle": "solid",
        "strokeWidth": stroke_width,
        "strokeStyle": "solid",
        "roughness": 0,
        "opacity": opacity,
        "groupIds": [],
        "frameId": frame_id,
        "index": None,
        "roundness": {"type": 3},
        "seed": _seed(el_id),
        "version": 1, "versionNonce": _seed(el_id + "_v"),
        "isDeleted": False,
        "boundElements": None,
        "updated": 1, "link": None, "locked": False,
    }
    if label:
        el["label"] = {
            "text": label,
            "fontSize": label_font_size,
            "fontFamily": 2,
            "textAlign": label_align,
            "verticalAlign": label_valign,
            "strokeColor": label_color or colors["stroke"],
        }
    return el


def _text(el_id: str, x: float, y: float, text: str,
          font_size: int = 16, color: str = "#1e1e1e",
          align: str = "left", frame_id: str | None = None) -> dict:
    return {
        "id": el_id,
        "type": "text",
        "x": x, "y": y,
        "width": len(text) * font_size * 0.6,
        "height": font_size * 1.25,
        "angle": 0,
        "strokeColor": color,
        "backgroundColor": "transparent",
        "fillStyle": "solid",
        "strokeWidth": 2,
        "strokeStyle": "solid",
        "roughness": 0,
        "opacity": 100,
        "groupIds": [],
        "frameId": frame_id,
        "index": None,
        "roundness": None,
        "seed": _seed(el_id),
        "version": 1, "versionNonce": _seed(el_id + "_v"),
        "isDeleted": False,
        "boundElements": None,
        "updated": 1, "link": None, "locked": False,
        "text": text,
        "fontSize": font_size,
        "fontFamily": 2,
        "textAlign": align,
        "verticalAlign": "top",
        "containerId": None,
        "originalText": text,
        "lineHeight": 1.25,
    }


def _arrow(el_id: str, x: float, y: float,
           start_id: str, end_id: str,
           points: list[list[float]] | None = None,
           label: str | None = None,
           stroke_color: str = "#868e96",
           stroke_width: int = 2,
           stroke_style: str = "solid",
           opacity: int = 100,
           elbowed: bool = False) -> dict:
    if points is None:
        points = [[0, 0], [100, 0]]
    el = {
        "id": el_id,
        "type": "arrow",
        "x": x, "y": y,
        "width": abs(points[-1][0]) or 1,
        "height": abs(points[-1][1]) or 1,
        "angle": 0,
        "strokeColor": stroke_color,
        "backgroundColor": "transparent",
        "fillStyle": "solid",
        "strokeWidth": stroke_width,
        "strokeStyle": stroke_style,
        "roughness": 0,
        "opacity": opacity,
        "groupIds": [],
        "frameId": None,
        "index": None,
        "roundness": {"type": 2},
        "seed": _seed(el_id),
        "version": 1, "versionNonce": _seed(el_id + "_v"),
        "isDeleted": False,
        "boundElements": None,
        "updated": 1, "link": None, "locked": False,
        "points": points,
        "startBinding": {"elementId": start_id, "focus": 0, "gap": 5},
        "endBinding": {"elementId": end_id, "focus": 0, "gap": 5},
        "startArrowhead": None,
        "endArrowhead": "arrow",
        "elbowed": elbowed,
    }
    if label:
        el["label"] = {"text": label, "fontSize": 14, "fontFamily": 2}
    return el


def _frame(el_id: str, name: str, children: list[str]) -> dict:
    return {
        "id": el_id,
        "type": "frame",
        "x": 0, "y": 0, "width": 100, "height": 100,
        "angle": 0,
        "strokeColor": "#868e96",
        "backgroundColor": "transparent",
        "fillStyle": "solid",
        "strokeWidth": 1,
        "strokeStyle": "solid",
        "roughness": 0,
        "opacity": 100,
        "groupIds": [],
        "frameId": None,
        "index": None,
        "roundness": None,
        "seed": _seed(el_id),
        "version": 1, "versionNonce": _seed(el_id + "_v"),
        "isDeleted": False,
        "boundElements": None,
        "updated": 1, "link": None, "locked": False,
        "name": name,
        "children": children,
    }


def _build_crate(el_id: str, node: SceneNode, frame_id: str | None = None) -> list[dict]:
    """Build a crate card: rectangle with label + version text."""
    els = []
    desc = node.metadata.get("description", "")
    features = node.metadata.get("features", [])
    ver = node.subtitle or ""

    label_text = node.label
    if ver:
        label_text += f"\n{ver}"
    if desc:
        short_desc = desc[:50] + ("..." if len(desc) > 50 else "")
        label_text += f"\n{short_desc}"

    els.append(_rect(el_id, node.x, node.y, node.w, node.h,
                     color_key=node.color_key, label=label_text,
                     label_font_size=14, label_align="left",
                     label_valign="top", frame_id=frame_id))

    for fi, feat in enumerate(features):
        fid = f"{el_id}_feat_{fi}"
        fw = len(feat) * 7 + 16
        fx = node.x + 10 + fi * (fw + 6)
        fy = node.y + node.h - 24
        if fx + fw > node.x + node.w - 10:
            fx = node.x + 10
            fy += 20
        els.append(_rect(fid, fx, fy, fw, 18,
                         color_key=node.color_key,
                         label=feat, label_font_size=10,
                         stroke_width=1, frame_id=frame_id))

    return els


def _build_pipeline_stage(el_id: str, node: SceneNode, frame_id: str | None = None) -> dict:
    """Build a pipeline stage: rectangle with label + timing."""
    label = node.label
    if node.subtitle:
        label += f"\n{node.subtitle}"
    timing = node.metadata.get("timing", "")
    if timing:
        label += f"\n{timing}"
    return _rect(el_id, node.x, node.y, node.w, node.h,
                 color_key=node.color_key, label=label,
                 label_font_size=14, frame_id=frame_id)


def _build_error_type(el_id: str, node: SceneNode, frame_id: str | None = None) -> dict:
    """Build an error type card."""
    label = node.label
    if node.subtitle:
        label += f"\n{node.subtitle}"
    crate = node.metadata.get("crate", "")
    if crate:
        label += f"\n({crate})"
    return _rect(el_id, node.x, node.y, node.w, node.h,
                 color_key="rose", label=label,
                 label_font_size=12, label_align="left",
                 frame_id=frame_id)


def _build_data_type(el_id: str, node: SceneNode, frame_id: str | None = None) -> dict:
    """Build a data type card."""
    label = node.label
    if node.subtitle:
        label += f"\n{node.subtitle}"
    return _rect(el_id, node.x, node.y, node.w, node.h,
                 color_key="interface", label=label,
                 label_font_size=12, label_align="left",
                 frame_id=frame_id)


def scene_to_excalidraw(scene: SceneDocument) -> dict:
    """Convert SceneDocument to Excalidraw v2 JSON using native features."""
    elements: list[dict] = []
    node_map = {n.id: n for n in scene.all_nodes}
    arrow_bindings: dict[str, list[str]] = {}

    # ── Header (standalone text) ──
    for section in scene.sections:
        if section.id != "header":
            continue
        for node in section.nodes:
            font_size = 28 if node.metadata.get("cls") == "tl" else 14
            elements.append(_text(
                f"el_{node.id}", node.x - node.w / 2, node.y,
                node.label, font_size=font_size, color="#1e1e1e",
            ))

    # ── Sections with frames ──
    for section in scene.sections:
        if section.id in ("header", "legend"):
            continue

        section_nodes = [n for n in section.nodes if n.id in node_map]
        if not section_nodes:
            continue

        frame_id = f"frame_{section.id}"
        child_ids = []

        # Build child elements first (required by Excalidraw frame ordering)
        for node in section_nodes:
            el_id = f"el_{node.id}"

            if node.kind == "crate":
                children = _build_crate(el_id, node, frame_id=frame_id)
                elements.extend(children)
                child_ids.append(el_id)
                arrow_bindings[node.id] = [el_id]

            elif node.kind == "pipeline_stage":
                elements.append(_build_pipeline_stage(el_id, node, frame_id=frame_id))
                child_ids.append(el_id)
                arrow_bindings[node.id] = [el_id]

            elif node.kind == "error_type":
                elements.append(_build_error_type(el_id, node, frame_id=frame_id))
                child_ids.append(el_id)

            elif node.kind == "data_type":
                elements.append(_build_data_type(el_id, node, frame_id=frame_id))
                child_ids.append(el_id)

            elif node.kind in ("skill", "agent", "command"):
                elements.append(_rect(el_id, node.x, node.y, node.w or 200, node.h or 24,
                                      color_key="interface", label=node.label,
                                      label_font_size=12, label_align="left",
                                      frame_id=frame_id))
                child_ids.append(el_id)

            elif node.kind == "strategy":
                label = node.label
                if node.subtitle:
                    label += f"\n{node.subtitle}"
                elements.append(_rect(el_id, node.x, node.y, node.w, node.h,
                                      color_key=node.color_key, label=label,
                                      label_font_size=12, label_align="left",
                                      frame_id=frame_id))
                child_ids.append(el_id)

            elif node.kind == "section_label":
                label = node.label
                if node.subtitle:
                    label += f"\n{node.subtitle}"
                elements.append(_text(f"el_{node.id}", node.x, node.y,
                                      label, font_size=14, color="#495057",
                                      frame_id=frame_id))
                child_ids.append(el_id)

        # Build frame after children
        if child_ids:
            elements.append(_frame(frame_id, section.title.upper(), child_ids))

    # ── Arrows with bindings ──
    for edge in scene.all_edges:
        source = node_map.get(edge.source_id)
        target = node_map.get(edge.target_id)
        if not source or not target:
            continue

        src_el_id = f"el_{source.id}"
        tgt_el_id = f"el_{target.id}"

        if edge.style == "horizontal":
            sx = source.x + source.w
            sy = source.y + source.h / 2
            tx = target.x
            ty = target.y + target.h / 2
            points = [[0, 0], [tx - sx, ty - sy]]
            stroke_color = "#868e96"
        else:
            sx = source.x + source.w / 2
            sy = source.y + source.h
            tx = target.x + target.w / 2
            ty = target.y
            points = [[0, 0], [tx - sx, ty - sy]]
            stroke_color = "#868e96"

        elements.append(_arrow(
            f"el_{edge.id}", sx, sy,
            start_id=src_el_id, end_id=tgt_el_id,
            points=points,
            stroke_color=stroke_color,
            stroke_width=2,
        ))

    return {
        "type": "excalidraw",
        "version": 2,
        "source": "https://excalidraw.com",
        "elements": elements,
        "appState": {
            "gridSize": None,
            "viewBackgroundColor": "#ffffff",
        },
        "files": {},
    }
