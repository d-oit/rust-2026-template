"""Excalidraw renderer — convert SceneDocument to Excalidraw v2 JSON format."""

import hashlib
from typing import Any

from .scene_model import SceneDocument, SceneNode, SceneEdge
from .config import THEME


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


def _deterministic_seed(id_str: str) -> int:
    h = 0
    for c in id_str:
        h = (h * 31 + ord(c)) & 0xFFFFFFFF
    return h


def _make_rect(el_id: str, node: SceneNode, group_ids: list[str] | None = None) -> dict:
    colors = EXCALIDRAW_COLORS.get(node.color_key, EXCALIDRAW_COLORS["interface"])
    return {
        "id": el_id,
        "type": "rectangle",
        "x": node.x,
        "y": node.y,
        "width": node.w,
        "height": node.h,
        "angle": 0,
        "strokeColor": colors["stroke"],
        "backgroundColor": colors["bg"],
        "fillStyle": "solid",
        "strokeWidth": 2,
        "strokeStyle": "solid",
        "roughness": 1,
        "opacity": 100,
        "groupIds": group_ids or [],
        "frameId": None,
        "index": None,
        "roundness": {"type": 3},
        "seed": _deterministic_seed(el_id),
        "version": 1,
        "versionNonce": _deterministic_seed(el_id + "_v"),
        "isDeleted": False,
        "boundElements": None,
        "updated": 1,
        "link": None,
        "locked": False,
    }


def _make_text(el_id: str, x: float, y: float, text: str, font_size: int = 20,
               font_family: int = 2, text_align: str = "left",
               group_ids: list[str] | None = None, color: str = "#1e1e1e") -> dict:
    return {
        "id": el_id,
        "type": "text",
        "x": x,
        "y": y,
        "width": len(text) * font_size * 0.6,
        "height": font_size * 1.25,
        "angle": 0,
        "strokeColor": color,
        "backgroundColor": "transparent",
        "fillStyle": "solid",
        "strokeWidth": 2,
        "strokeStyle": "solid",
        "roughness": 1,
        "opacity": 100,
        "groupIds": group_ids or [],
        "frameId": None,
        "index": None,
        "roundness": None,
        "seed": _deterministic_seed(el_id),
        "version": 1,
        "versionNonce": _deterministic_seed(el_id + "_v"),
        "isDeleted": False,
        "boundElements": None,
        "updated": 1,
        "link": None,
        "locked": False,
        "text": text,
        "fontSize": font_size,
        "fontFamily": font_family,
        "textAlign": text_align,
        "verticalAlign": "top",
        "containerId": None,
        "originalText": text,
        "lineHeight": 1.25,
    }


def _make_arrow(el_id: str, source: SceneNode, target: SceneNode,
                group_ids: list[str] | None = None) -> dict:
    sx = source.x + source.w / 2
    sy = source.y + source.h
    tx = target.x + target.w / 2
    ty = target.y
    return {
        "id": el_id,
        "type": "arrow",
        "x": sx,
        "y": sy,
        "width": abs(tx - sx) if abs(tx - sx) > 1 else 1,
        "height": abs(ty - sy) if abs(ty - sy) > 1 else 1,
        "angle": 0,
        "strokeColor": "#6366f1",
        "backgroundColor": "transparent",
        "fillStyle": "solid",
        "strokeWidth": 2,
        "strokeStyle": "dashed",
        "roughness": 1,
        "opacity": 60,
        "groupIds": group_ids or [],
        "frameId": None,
        "index": None,
        "roundness": {"type": 2},
        "seed": _deterministic_seed(el_id),
        "version": 1,
        "versionNonce": _deterministic_seed(el_id + "_v"),
        "isDeleted": False,
        "boundElements": None,
        "updated": 1,
        "link": None,
        "locked": False,
        "points": [[0, 0], [tx - sx, ty - sy]],
        "startBinding": None,
        "endBinding": None,
        "startArrowhead": None,
        "endArrowhead": "arrow",
        "elbowed": False,
    }


def _make_container_frame(el_id: str, x: float, y: float, w: float, h: float,
                          label: str, group_ids: list[str] | None = None) -> list[dict]:
    frame = {
        "id": el_id,
        "type": "rectangle",
        "x": x,
        "y": y,
        "width": w,
        "height": h,
        "angle": 0,
        "strokeColor": "#94a3b8",
        "backgroundColor": "transparent",
        "fillStyle": "solid",
        "strokeWidth": 1,
        "strokeStyle": "dashed",
        "roughness": 0,
        "opacity": 30,
        "groupIds": group_ids or [],
        "frameId": None,
        "index": None,
        "roundness": {"type": 3},
        "seed": _deterministic_seed(el_id),
        "version": 1,
        "versionNonce": _deterministic_seed(el_id + "_v"),
        "isDeleted": False,
        "boundElements": None,
        "updated": 1,
        "link": None,
        "locked": False,
    }
    label_el = _make_text(el_id + "_label", x + 10, y + 5, label,
                          font_size=11, font_family=2, group_ids=group_ids,
                          color="#64748b")
    return [frame, label_el]


def scene_to_excalidraw(scene: SceneDocument) -> dict:
    """Convert SceneDocument to Excalidraw v2 JSON format."""
    elements: list[dict] = []
    node_map: dict[str, SceneNode] = {n.id: n for n in scene.all_nodes}

    for section in scene.sections:
        section_nodes = [n for n in section.nodes if n.id in node_map]
        if not section_nodes:
            continue

        if section.id == "legend":
            continue

        if section.id == "header":
            for node in section_nodes:
                el_id = f"el_{node.id}"
                font_size = 28 if node.metadata.get("cls") == "tl" else 14
                elements.append(_make_text(
                    el_id, node.x - node.w / 2, node.y,
                    node.label, font_size=font_size, font_family=2,
                    text_align="center", color="#1e1e1e",
                ))
            continue

        min_x = min(n.x for n in section_nodes)
        min_y = min(n.y for n in section_nodes)
        max_x = max(n.x + n.w for n in section_nodes)
        max_y = max(n.y + n.h for n in section_nodes)
        padding = 15

        container_id = f"container_{section.id}"
        group_id = f"group_{section.id}"
        container_els = _make_container_frame(
            container_id,
            min_x - padding, min_y - padding,
            max_x - min_x + 2 * padding, max_y - min_y + 2 * padding,
            section.title.upper(),
            group_ids=[group_id],
        )
        elements.extend(container_els)

        for node in section_nodes:
            el_id = f"el_{node.id}"
            elements.append(_make_rect(el_id, node, group_ids=[group_id]))

            if node.kind == "crate":
                title_id = f"el_{node.id}_title"
                elements.append(_make_text(
                    title_id, node.x + 10, node.y + 8,
                    node.label, font_size=14, font_family=2,
                    group_ids=[group_id], color="#1e1e1e",
                ))
                if node.subtitle:
                    ver_id = f"el_{node.id}_version"
                    elements.append(_make_text(
                        ver_id, node.x + node.w - 10, node.y + 8,
                        node.subtitle, font_size=11, font_family=2,
                        text_align="right", group_ids=[group_id], color="#64748b",
                    ))
                if node.metadata.get("description"):
                    desc_lines = _wrap_excalidraw_text(node.metadata["description"], max_chars=45)
                    for di, line in enumerate(desc_lines):
                        did = f"el_{node.id}_desc_{di}"
                        elements.append(_make_text(
                            did, node.x + 10, node.y + 30 + di * 16,
                            line, font_size=12, font_family=2,
                            group_ids=[group_id], color="#475569",
                        ))
                features = node.metadata.get("features", [])
                if features:
                    feat_y = node.y + node.h - 25
                    feat_x = node.x + 10
                    for feat in features:
                        fid = f"el_{node.id}_feat_{feat}"
                        fw = len(feat) * 7 + 16
                        if feat_x + fw > node.x + node.w - 10:
                            feat_x = node.x + 10
                            feat_y += 20
                        feat_rect = _make_rect(fid, SceneNode(
                            id=fid, kind="command",
                            x=feat_x, y=feat_y, w=fw, h=18,
                            label=feat, color_key=node.color_key,
                        ), group_ids=[group_id])
                        feat_rect["roundness"] = {"type": 3}
                        elements.append(feat_rect)
                        elements.append(_make_text(
                            fid + "_text", feat_x + 4, feat_y + 2,
                            feat, font_size=10, font_family=2,
                            group_ids=[group_id], color=EXCALIDRAW_COLORS.get(node.color_key, {}).get("stroke", "#1e1e1e"),
                        ))
                        feat_x += fw + 6
            elif node.kind == "pipeline_stage":
                elements.append(_make_text(
                    f"el_{node.id}_title", node.x + node.w / 2 - len(node.label) * 4,
                    node.y + 10, node.label, font_size=14, font_family=2,
                    text_align="center", group_ids=[group_id], color="#1e1e1e",
                ))
                if node.subtitle:
                    elements.append(_make_text(
                        f"el_{node.id}_desc", node.x + 10, node.y + 30,
                        node.subtitle, font_size=11, font_family=2,
                        group_ids=[group_id], color="#475569",
                    ))
                if node.metadata.get("timing"):
                    elements.append(_make_text(
                        f"el_{node.id}_timing", node.x + 10, node.y + 45,
                        node.metadata["timing"], font_size=10, font_family=2,
                        group_ids=[group_id], color="#64748b",
                    ))
            elif node.kind == "skill":
                elements.append(_make_text(
                    f"el_{node.id}_text", node.x, node.y,
                    node.label, font_size=13, font_family=2,
                    group_ids=[group_id], color="#334155",
                ))
            elif node.kind == "agent":
                elements.append(_make_text(
                    f"el_{node.id}_text", node.x, node.y,
                    node.label, font_size=13, font_family=2,
                    group_ids=[group_id], color="#334155",
                ))
            elif node.kind == "strategy":
                elements.append(_make_text(
                    f"el_{node.id}_label", node.x + 15, node.y + 10,
                    node.label, font_size=14, font_family=2,
                    group_ids=[group_id], color="#1e1e1e",
                ))
                if node.subtitle:
                    elements.append(_make_text(
                        f"el_{node.id}_desc", node.x + 15, node.y + 30,
                        node.subtitle, font_size=11, font_family=2,
                        group_ids=[group_id], color="#475569",
                    ))
            elif node.kind == "error_type":
                elements.append(_make_text(
                    f"el_{node.id}_label", node.x + 15, node.y + 5,
                    node.label, font_size=13, font_family=2,
                    group_ids=[group_id], color="#9f1239",
                ))
                if node.subtitle:
                    elements.append(_make_text(
                        f"el_{node.id}_variants", node.x + 15, node.y + 22,
                        node.subtitle, font_size=11, font_family=2,
                        group_ids=[group_id], color="#475569",
                    ))
                if node.metadata.get("crate"):
                    elements.append(_make_text(
                        f"el_{node.id}_crate", node.x + node.w - 15, node.y + 5,
                        node.metadata["crate"], font_size=11, font_family=2,
                        text_align="right", group_ids=[group_id], color="#64748b",
                    ))
            elif node.kind == "data_type":
                elements.append(_make_text(
                    f"el_{node.id}_label", node.x + 15, node.y + 5,
                    node.label, font_size=13, font_family=2,
                    group_ids=[group_id], color="#1e1e1e",
                ))
                if node.subtitle:
                    elements.append(_make_text(
                        f"el_{node.id}_desc", node.x + 15, node.y + 22,
                        node.subtitle, font_size=11, font_family=2,
                        group_ids=[group_id], color="#475569",
                    ))
            elif node.kind == "command":
                elements.append(_make_text(
                    f"el_{node.id}_text", node.x + 10, node.y + 5,
                    node.label, font_size=13, font_family=2,
                    group_ids=[group_id], color="#334155",
                ))
            elif node.kind == "section_label":
                if node.subtitle:
                    elements.append(_make_text(
                        f"el_{node.id}_label", node.x, node.y,
                        node.label, font_size=13, font_family=2,
                        group_ids=[group_id], color="#1e1e1e",
                    ))
                    elements.append(_make_text(
                        f"el_{node.id}_desc", node.x, node.y + 18,
                        node.subtitle, font_size=11, font_family=2,
                        group_ids=[group_id], color="#64748b",
                    ))
                else:
                    elements.append(_make_text(
                        f"el_{node.id}_label", node.x, node.y,
                        node.label, font_size=13, font_family=2,
                        group_ids=[group_id], color="#1e1e1e",
                    ))

    # ── Edges ──
    for edge in scene.all_edges:
        source = node_map.get(edge.source_id)
        target = node_map.get(edge.target_id)
        if source and target:
            elements.append(_make_arrow(f"el_{edge.id}", source, target))

    return {
        "type": "excalidraw",
        "version": 2,
        "source": "https://github.com/excalidraw/excalidraw",
        "elements": elements,
        "appState": {
            "gridSize": None,
            "viewBackgroundColor": "#ffffff",
        },
        "files": {},
    }


def _wrap_excalidraw_text(text: str, max_chars: int = 45) -> list[str]:
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
