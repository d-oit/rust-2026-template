"""Excalidraw renderer — convert SceneDocument to Excalidraw v2 JSON format.

FIXES:
- Layer badges now render as colored boxes with text INSIDE (never overlapping)
- Separate text elements for ALL labels (better compatibility with exporters)
- Correct Z-order: rectangles always before text
- Frames have proper bounding boxes
- Arrows use robust orthogonal (Manhattan) routing
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

def _rect(el_id: str, x: float, y: float, w: float, h: float,
          color_key: str = "interface", frame_id: str | None = None,
          opacity: int = 100, stroke_width: int = 2) -> dict:
    colors = EXCALIDRAW_COLORS.get(color_key, EXCALIDRAW_COLORS["interface"])
    return {
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
        "roundness": {"type": 2},
        "seed": _seed(el_id),
        "version": 1, "versionNonce": _seed(el_id + "_v"),
        "isDeleted": False,
        "boundElements": None,
        "updated": 1, "link": None, "locked": False,
    }

def _text(el_id: str, x: float, y: float, text: str,
          font_size: int = 14, color: str = "#1e1e1e",
          align: str = "left", frame_id: str | None = None,
          width: float | None = None, opacity: int = 100) -> dict:
    if width is None:
        width = len(text) * font_size * 0.6
    return {
        "id": el_id,
        "type": "text",
        "x": x, "y": y,
        "width": width,
        "height": font_size * 1.25,
        "angle": 0,
        "strokeColor": color,
        "backgroundColor": "transparent",
        "fillStyle": "solid",
        "strokeWidth": 2,
        "strokeStyle": "solid",
        "roughness": 0,
        "opacity": opacity,
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
        "fontFamily": 3,  # Sans-serif
        "textAlign": align,
        "verticalAlign": "top",
        "containerId": None,
        "originalText": text,
        "lineHeight": 1.25,
    }

def _badge_box(el_id: str, x: float, y: float, w: float, h: float,
               color_key: str, label: str, subtitle: str = "",
               frame_id: str | None = None) -> list[dict]:
    colors = EXCALIDRAW_COLORS.get(color_key, EXCALIDRAW_COLORS["interface"])
    elements = []

    # 1. Rectangle
    elements.append(_rect(el_id, x, y, w, h, color_key, frame_id))

    # 2. Label
    label_y = y + 10 if not subtitle else y + 8
    elements.append(_text(f"{el_id}_label", x + 12, label_y, label,
                         font_size=13, color=colors["stroke"], align="left",
                         frame_id=frame_id, width=w-24))

    # 3. Subtitle
    if subtitle:
        elements.append(_text(f"{el_id}_sub", x + 12, y + 28, subtitle,
                             font_size=10, color="#495057", align="left",
                             frame_id=frame_id, width=w-24, opacity=80))

    return elements

def _orthogonal_arrow(el_id: str, source_node, target_node,
                      label: str | None = None, stroke_color: str = "#868e96",
                      frame_id: str | None = None) -> dict:
    sx, sy, sw, sh = source_node.x, source_node.y, source_node.w, source_node.h
    tx, ty, tw, th = target_node.x, target_node.y, target_node.w, target_node.h

    # Manhattan routing
    if abs(sx - tx) > abs(sy - ty):
        # Horizontal
        if tx > sx:
            start_x, start_y = sx + sw, sy + sh / 2
            end_x, end_y = tx, ty + th / 2
        else:
            start_x, start_y = sx, sy + sh / 2
            end_x, end_y = tx + tw, ty + th / 2

        mid_x = (start_x + end_x) / 2
        points = [[0, 0], [mid_x - start_x, 0], [mid_x - start_x, end_y - start_y], [end_x - start_x, end_y - start_y]]
    else:
        # Vertical
        if ty > sy:
            start_x, start_y = sx + sw / 2, sy + sh
            end_x, end_y = tx + tw / 2, ty
        else:
            start_x, start_y = sx + sw / 2, sy
            end_x, end_y = tx + tw / 2, ty + th

        mid_y = (start_y + end_y) / 2
        points = [[0, 0], [0, mid_y - start_y], [end_x - start_x, mid_y - start_y], [end_x - start_x, end_y - start_y]]

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
    }

def _frame(el_id: str, name: str, children: list[str], elements: list[dict]) -> dict:
    # Compute bounding box
    child_els = [e for e in elements if e["id"] in children]
    if not child_els:
        return None

    min_x = min(e["x"] for e in child_els)
    min_y = min(e["y"] for e in child_els)
    max_x = max(e["x"] + e.get("width", 0) for e in child_els)
    max_y = max(e["y"] + e.get("height", 0) for e in child_els)

    padding = 20
    return {
        "id": el_id,
        "type": "frame",
        "x": min_x - padding, "y": min_y - padding - 30,
        "width": (max_x - min_x) + 2 * padding,
        "height": (max_y - min_y) + 2 * padding + 30,
        "angle": 0,
        "strokeColor": "#cbd5e1",
        "backgroundColor": "transparent",
        "fillStyle": "solid",
        "strokeWidth": 2,
        "strokeStyle": "dotted",
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

def scene_to_excalidraw(scene: SceneDocument) -> dict:
    elements: list[dict] = []
    node_map = {n.id: n for n in scene.all_nodes}

    # ── Header ──
    for section in scene.sections:
        if section.id != "header": continue
        for node in section.nodes:
            fs = 28 if node.metadata.get("cls") == "tl" else 16
            color = "#1e1e1e" if node.metadata.get("cls") == "tl" else "#495057"
            elements.append(_text(f"el_{node.id}", node.x - node.w/2, node.y, node.label, font_size=fs, color=color))

    # ── Legend ──
    for section in scene.sections:
        if section.id != "legend": continue
        frame_id = f"frame_{section.id}"
        child_ids = []
        legend_items = [("apps", "Applications", "Binary crates and CLI tools"),
                       ("core", "Core Libraries", "Shared library crates"),
                       ("templates", "Templates", "Reusable architectural patterns"),
                       ("other", "Examples", "Learning references and demos")]
        lx, ly = 50.0, section.nodes[0].y if section.nodes else 100
        for ck, l, d in legend_items:
            bid = f"el_legend_{ck}"
            els = _badge_box(bid, lx, ly, 250, 50, ck, l, d, frame_id=frame_id)
            elements.extend(els)
            child_ids.extend([e["id"] for e in els])
            lx += 280
        f = _frame(frame_id, "LEGEND", child_ids, elements)
        if f: elements.append(f)

    # ── Sections ──
    for section in scene.sections:
        if section.id in ("header", "legend"): continue
        frame_id = f"frame_{section.id}"
        child_ids = []
        seen_layers = set()

        for node in section.nodes:
            el_id = f"el_{node.id}"
            if node.kind == "crate":
                layer = node.metadata.get("layer", "")
                if layer and layer not in seen_layers:
                    seen_layers.add(layer)
                    lid = f"el_layer_label_{layer}_{section.id}"
                    els = _badge_box(lid, node.x, node.y - 30, 150, 22, node.color_key, layer, "", frame_id=frame_id)
                    elements.extend(els)
                    child_ids.extend([e["id"] for e in els])

                # Build crate card manually
                elements.append(_rect(el_id, node.x, node.y, node.w, node.h, node.color_key, frame_id))
                child_ids.append(el_id)
                elements.append(_text(f"{el_id}_label", node.x + 10, node.y + 10, node.label, font_size=14, frame_id=frame_id))
                child_ids.append(f"{el_id}_label")
                if node.subtitle:
                    elements.append(_text(f"{el_id}_sub", node.x + 10, node.y + 28, node.subtitle, font_size=10, opacity=70, frame_id=frame_id))
                    child_ids.append(f"{el_id}_sub")

                for fi, feat in enumerate(node.metadata.get("features", [])[:5]):
                    fid = f"{el_id}_f_{fi}"
                    els = _badge_box(fid, node.x + 10 + fi*60, node.y + node.h - 22, 55, 16, node.color_key, feat, "", frame_id=frame_id)
                    elements.extend(els)
                    child_ids.extend([e["id"] for e in els])

            elif node.kind in ("pipeline_stage", "error_type", "data_type", "skill", "agent", "command", "strategy"):
                elements.append(_rect(el_id, node.x, node.y, node.w, node.h, node.color_key, frame_id))
                child_ids.append(el_id)
                elements.append(_text(f"{el_id}_txt", node.x + 10, node.y + 10, node.label, font_size=12, frame_id=frame_id, width=node.w-20))
                child_ids.append(f"{el_id}_txt")

            elif node.kind == "section_label":
                elements.append(_text(el_id, node.x, node.y, node.label, font_size=14, color="#495057", frame_id=frame_id))
                child_ids.append(el_id)

        f = _frame(frame_id, section.title.upper(), child_ids, elements)
        if f: elements.append(f)

    # ── Arrows ──
    for edge in scene.all_edges:
        src, tgt = node_map.get(edge.source_id), node_map.get(edge.target_id)
        if src and tgt:
            elements.append(_orthogonal_arrow(f"el_{edge.id}", src, tgt))

    return {"type": "excalidraw", "version": 2, "source": "https://excalidraw.com",
            "elements": elements, "appState": {"gridSize": None, "viewBackgroundColor": "#ffffff"}, "files": {}}
