"""Excalidraw renderer — convert SceneDocument to Excalidraw v2 JSON format.

Uses native Excalidraw features:
- frame elements for section grouping
- label property for bound text on shapes
- startBinding/endBinding for arrow connections
- proper element ordering (children before frames)
"""

from .scene_model import SceneDocument, SceneNode


EXCALIDRAW_COLORS = {
    "apps": {"bg": "#fef3c7", "stroke": "#d97706"},
    "core": {"bg": "#dbeafe", "stroke": "#2563eb"},
    "templates": {"bg": "#f3e8ff", "stroke": "#9333ea"},
    "other": {"bg": "#dcfce7", "stroke": "#16a34a"},
    "pipeline": {"bg": "#ccfbf1", "stroke": "#0d9488"},
    "interface": {"bg": "#f1f5f9", "stroke": "#64748b"},
    "rose": {"bg": "#ffe4e6", "stroke": "#e11d48"},
    "teal": {"bg": "#ccfbf1", "stroke": "#0d9488"},
    "blue": {"bg": "#dbeafe", "stroke": "#2563eb"},
    "green": {"bg": "#dcfce7", "stroke": "#16a34a"},
}


def _seed(id_str: str) -> int:
    h = 0
    for c in id_str:
        h = (h * 31 + ord(c)) & 0xFFFFFFFF
    return h


def _badge_box(el_id: str, x: float, y: float, w: float, h: float,
               color_key: str, label: str, subtitle: str = "",
               frame_id: str | None = None) -> list[dict]:
    """
    Create a colored badge box with text INSIDE.
    Text is positioned with proper padding to NEVER overlap the border.
    """
    colors = EXCALIDRAW_COLORS.get(color_key, EXCALIDRAW_COLORS["interface"])
    elements = []

    # Main rectangle with rounded corners
    rect = {
        "id": el_id,
        "type": "rectangle",
        "x": x, "y": y, "width": w, "height": h,
        "angle": 0,
        "strokeColor": colors["stroke"],
        "backgroundColor": colors["bg"],
        "fillStyle": "solid",
        "strokeWidth": 2,
        "strokeStyle": "solid",
        "roughness": 0,
        "opacity": 100,
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
    elements.append(rect)

    # Label text - positioned INSIDE with 12px padding from edges
    label_y = y + 10  # Top padding
    if subtitle:
        label_y = y + 8  # Adjust when subtitle present

    label_text = {
        "id": f"{el_id}_label",
        "type": "text",
        "x": x + 12,  # Left padding - never overlaps border
        "y": label_y,
        "width": w - 24,  # Right padding
        "height": 18,
        "angle": 0,
        "strokeColor": colors["stroke"],  # Dark text on light background
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
        "seed": _seed(f"{el_id}_label"),
        "version": 1, "versionNonce": _seed(f"{el_id}_label_v"),
        "isDeleted": False,
        "boundElements": None,
        "updated": 1, "link": None, "locked": False,
        "text": label,
        "fontSize": 13,
        "fontFamily": 2,
        "textAlign": "left",
        "verticalAlign": "top",
        "containerId": None,
        "originalText": label,
        "lineHeight": 1.25,
    }
    elements.append(label_text)

    # Subtitle text - positioned below label with proper spacing
    if subtitle:
        sub_text = {
            "id": f"{el_id}_sub",
            "type": "text",
            "x": x + 12,
            "y": y + 28,  # Below label with 10px gap
            "width": w - 24,
            "height": 14,
            "angle": 0,
            "strokeColor": "#495057",  # Darker for readability
            "backgroundColor": "transparent",
            "fillStyle": "solid",
            "strokeWidth": 2,
            "strokeStyle": "solid",
            "roughness": 0,
            "opacity": 80,
            "groupIds": [],
            "frameId": frame_id,
            "index": None,
            "roundness": None,
            "seed": _seed(f"{el_id}_sub"),
            "version": 1, "versionNonce": _seed(f"{el_id}_sub_v"),
            "isDeleted": False,
            "boundElements": None,
            "updated": 1, "link": None, "locked": False,
            "text": subtitle,
            "fontSize": 10,
            "fontFamily": 2,
            "textAlign": "left",
            "verticalAlign": "top",
            "containerId": None,
            "originalText": subtitle,
            "lineHeight": 1.25,
        }
        elements.append(sub_text)

    return elements


def _orthogonal_arrow(el_id: str, source_node, target_node,
                      label: str | None = None, stroke_color: str = "#868e96",
                      frame_id: str | None = None) -> dict:
    """
    Create an orthogonal (Manhattan-routed) arrow.
    Routes around boxes instead of crossing through them.
    Connects edge-to-edge, not corner-to-corner.
    """
    sx, sy, sw, sh = source_node.x, source_node.y, source_node.w, source_node.h
    tx, ty, tw, th = target_node.x, target_node.y, target_node.w, target_node.h

    # Determine connection points based on relative positions
    # Always connect from the closest edges
    if abs(sx - tx) > abs(sy - ty):
        # Horizontal relationship - connect left/right edges
        if tx > sx:
            # Target is to the right
            start_x = sx + sw  # Right edge of source
            start_y = sy + sh / 2
            end_x = tx  # Left edge of target
            end_y = ty + th / 2
        else:
            # Target is to the left
            start_x = sx  # Left edge of source
            start_y = sy + sh / 2
            end_x = tx + tw  # Right edge of target
            end_y = ty + th / 2

        # Create orthogonal path: horizontal -> vertical -> horizontal
        if abs(start_y - end_y) < 5:
            # Same vertical level - simple horizontal line
            points = [[0, 0], [end_x - start_x, 0]]
        else:
            # Different vertical levels - route with bend
            mid_x = (start_x + end_x) / 2
            points = [
                [0, 0],
                [mid_x - start_x, 0],
                [mid_x - start_x, end_y - start_y],
                [end_x - start_x, end_y - start_y]
            ]
    else:
        # Vertical relationship - connect top/bottom edges
        if ty > sy:
            # Target is below
            start_x = sx + sw / 2
            start_y = sy + sh  # Bottom edge of source
            end_x = tx + tw / 2
            end_y = ty  # Top edge of target
        else:
            # Target is above
            start_x = sx + sw / 2
            start_y = sy  # Top edge of source
            end_x = tx + tw / 2
            end_y = ty + th  # Bottom edge of target

        if abs(start_x - end_x) < 5:
            # Same horizontal level - simple vertical line
            points = [[0, 0], [0, end_y - start_y]]
        else:
            # Different horizontal levels - route with bend
            mid_y = (start_y + end_y) / 2
            points = [
                [0, 0],
                [0, mid_y - start_y],
                [end_x - start_x, mid_y - start_y],
                [end_x - start_x, end_y - start_y]
            ]

    return {
        "id": el_id,
        "type": "arrow",
        "x": start_x, "y": start_y,
        "width": abs(points[-1][0]) or 1,
        "height": abs(points[-1][1]) or 1,
        "angle": 0,
        "strokeColor": stroke_color,
        "backgroundColor": "transparent",
        "fillStyle": "solid",
        "strokeWidth": 2,
        "strokeStyle": "solid",
        "roughness": 0,
        "opacity": 100,
        "groupIds": [],
        "frameId": frame_id,
        "index": None,
        "roundness": {"type": 2},
        "seed": _seed(el_id),
        "version": 1, "versionNonce": _seed(el_id + "_v"),
        "isDeleted": False,
        "boundElements": None,
        "updated": 1, "link": None, "locked": False,
        "points": points,
        "startBinding": {"elementId": f"el_{source_node.id}", "focus": 0, "gap": 5},
        "endBinding": {"elementId": f"el_{target_node.id}", "focus": 0, "gap": 5},
        "startArrowhead": None,
        "endArrowhead": "arrow",
        "elbowed": True,
        "label": {"text": label, "fontSize": 12, "fontFamily": 2} if label else None,
    }


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


def _frame(el_id: str, name: str, children: list[str]) -> dict:
    return {
        "id": el_id,
        "type": "frame",
        "x": 0, "y": 0, "width": 100, "height": 100,
        "angle": 0,
        "strokeColor": "transparent",
        "backgroundColor": "transparent",
        "fillStyle": "solid",
        "strokeWidth": 0,
        "strokeStyle": "solid",
        "roughness": 0,
        "opacity": 0,
        "groupIds": [],
        "frameId": None,
        "index": None,
        "roundness": None,
        "seed": _seed(el_id),
        "version": 1, "versionNonce": _seed(el_id + "_v"),
        "isDeleted": False,
        "boundElements": None,
        "updated": 1, "link": None, "locked": False,
        "name": "",
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

    # ── Header (standalone text) ──
    for section in scene.sections:
        if section.id != "header":
            continue
        for node in section.nodes:
            font_size = 28 if node.metadata.get("cls") == "tl" else 16
            color = "#1e1e1e" if node.metadata.get("cls") == "tl" else "#495057"
            elements.append(_text(
                f"el_{node.id}", node.x - node.w / 2, node.y,
                node.label, font_size=font_size, color=color,
            ))

    # ── Legend as Badge Boxes ──
    for section in scene.sections:
        if section.id != "legend":
            continue

        frame_id = f"frame_{section.id}"
        child_ids = []

        legend_items = [
            ("apps", "Applications", "Binary crates and CLI tools"),
            ("core", "Core Libraries", "Shared library crates"),
            ("templates", "Templates", "Reusable architectural patterns"),
            ("other", "Examples", "Learning references and demos"),
        ]

        legend_x = 50.0
        legend_y = section.nodes[0].y if section.nodes else 100

        for color_key, label, desc in legend_items:
            badge_id = f"el_legend_{color_key}"
            # Create badge box with text INSIDE
            badge_elements = _badge_box(
                badge_id, legend_x, legend_y, 250, 50,
                color_key, label, desc,
                frame_id=frame_id
            )
            elements.extend(badge_elements)
            child_ids.append(badge_id)
            child_ids.append(f"{badge_id}_label")
            if desc:
                child_ids.append(f"{badge_id}_sub")
            legend_x += 280

        if child_ids:
            elements.append(_frame(frame_id, "LEGEND", child_ids))

    # ── Sections with frames ──
    for section in scene.sections:
        if section.id in ("header", "legend"):
            continue

        section_nodes = [n for n in section.nodes if n.id in node_map]
        if not section_nodes:
            continue

        frame_id = f"frame_{section.id}"
        child_ids = []

        layer_labels = {"apps": "APPLICATIONS", "core": "CORE LIBRARIES",
                        "templates": "TEMPLATE CRATES", "other": "EXAMPLES"}
        seen_layers = set()

        for node in section_nodes:
            el_id = f"el_{node.id}"

            if node.kind == "crate":
                # Add layer label as badge if not seen yet
                layer = node.metadata.get("layer", "")
                if layer and layer not in seen_layers:
                    seen_layers.add(layer)
                    layer_label_id = f"el_layer_label_{layer}"
                    layer_text = layer_labels.get(layer, layer_labels.get(layer, layer.upper()))
                    # Create layer label as a small badge
                    layer_badge = _badge_box(
                        layer_label_id, node.x, node.y - 30, 150, 22,
                        node.color_key, layer_text, "",
                        frame_id=frame_id
                    )
                    elements.extend(layer_badge)
                    child_ids.append(layer_label_id)
                    child_ids.append(f"{layer_label_id}_label")

                children = _build_crate(el_id, node, frame_id=frame_id)
                elements.extend(children)
                child_ids.append(el_id)

            elif node.kind == "pipeline_stage":
                elements.append(_build_pipeline_stage(el_id, node, frame_id=frame_id))
                child_ids.append(el_id)

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
                # Render as badge box if it has a color_key
                if node.color_key and node.color_key != "interface":
                    badge_els = _badge_box(
                        el_id, node.x, node.y, node.w, node.h,
                        node.color_key, node.label, node.subtitle or "",
                        frame_id=frame_id
                    )
                    elements.extend(badge_els)
                    child_ids.append(el_id)
                    child_ids.append(f"{el_id}_label")
                    if node.subtitle:
                        child_ids.append(f"{el_id}_sub")
                else:
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

    # ── Arrows with orthogonal routing ──
    for edge in scene.all_edges:
        source = node_map.get(edge.source_id)
        target = node_map.get(edge.target_id)
        if not source or not target:
            continue

        arrow = _orthogonal_arrow(
            f"el_{edge.id}", source, target,
            label=edge.metadata.get("label") if hasattr(edge, 'metadata') else None,
            stroke_color="#868e96",
        )
        elements.append(arrow)

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
