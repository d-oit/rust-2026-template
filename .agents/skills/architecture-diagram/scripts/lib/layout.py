"""Layout engine — Graphviz auto-layout with grid fallback and overlap resolution."""

import json
import subprocess  # nosec B404
from pathlib import Path

from .config import VIEWBOX_W, VIEWBOX_MARGIN


def estimate_text_width(text: str, font_size: int = 12) -> float:
    avg_char_w = font_size * 0.62
    return len(text) * avg_char_w


def compute_card_dimensions(crate: dict, card_w: int = 340) -> tuple[int, int]:
    desc = crate.get("description", "")
    from .svg_render import wrap_text
    desc_lines = wrap_text(desc, max_chars=max(int(card_w / 7), 30))
    desc_h = len(desc_lines) * 16 if desc_lines else 0
    features = crate.get("features", [])
    feat_h = 26 if features else 0
    card_h = 70 + desc_h + feat_h + 10
    return card_w, card_h


def has_graphviz() -> bool:
    try:
        result = subprocess.run(  # nosec B603
            ["dot", "-V"], capture_output=True, text=True, timeout=5, shell=False,
        )
        return result.returncode == 0 or "graphviz" in result.stderr.lower()
    except (FileNotFoundError, subprocess.TimeoutExpired):
        return False


def layout_with_graphviz(crates: list[dict], layers: dict, y_start: int, viewbox_w: int) -> dict:
    dot_lines = ['digraph G {', '  rankdir=TB;', '  node [shape=box, style=filled, fontname="Inter", fontsize=11];', '  edge [style=invis, weight=10];', '  graph [ranksep=0.8, nodesep=0.5, splines=ortho];']
    for layer_name in ["apps", "core", "templates", "other"]:
        layer_crates = layers.get(layer_name, [])
        if layer_crates:
            names = [c["name"] for c in layer_crates]
            dot_lines.append(f'  {{ rank=same; {"; ".join(f"{n}" for n in names)}; }}')

    for crate in crates:
        safe = crate["name"].replace("-", "_").replace(".", "_")
        label = crate["name"].replace("&", "&&")
        dot_lines.append(f'  {safe} [label="{label}", fillcolor="#eff6ff", fontcolor="#1e40af"];')

    for crate in crates:
        src = crate["name"].replace("-", "_").replace(".", "_")
        for dep in crate["dependencies"]:
            dst = dep.replace("-", "_").replace(".", "_")
            dot_lines.append(f'  {src} -> {dst};')
    dot_lines.append('}')
    dot_src = "\n".join(dot_lines)
    try:
        result = subprocess.run(  # nosec B603
            ["dot", "-Tjson"], input=dot_src, capture_output=True, text=True,
            timeout=30, shell=False,
        )
        if result.returncode != 0:
            return {}
        data = json.loads(result.stdout)
        objects = data.get("objects", [])
        if not objects:
            return {}
        min_x = min(obj.get("pos", "0,0").split(",")[0].strip("!") for obj in objects if "pos" in obj)
        max_x = max(obj.get("pos", "0,0").split(",")[0].strip("!") for obj in objects if "pos" in obj)
        min_y = min(obj.get("pos", "0,0").split(",")[1].strip("!") for obj in objects if "pos" in obj)
        max_y = max(obj.get("pos", "0,0").split(",")[1].strip("!") for obj in objects if "pos" in obj)
        range_x = max(float(max_x) - float(min_x), 1)
        range_y = max(float(max_y) - float(min_y), 1)
        margin = 30
        usable_w = viewbox_w - 2 * margin
        scale = usable_w / range_x if range_x > 0 else 1
        coords = {}
        for obj in objects:
            name = obj.get("name", "")
            if "pos" not in obj:
                continue
            px, py = [float(v.strip("!")) for v in obj["pos"].split(",")]
            w = float(obj.get("width", 1)) * 72
            h = float(obj.get("height", 1)) * 72
            sx = margin + (px - float(min_x)) * scale - w / 2
            sy = y_start + (py - float(min_y)) * scale * 0.8
            original = name.replace("_", "-")
            coords[original] = (int(sx), int(sy), int(w), int(h))
        return coords
    except (subprocess.TimeoutExpired, json.JSONDecodeError, ValueError):
        return {}


def layout_grid_fallback(crates: list[dict], layers: dict, y_start: int, viewbox_w: int) -> dict:
    coords = {}
    col_w = 340
    gap_x, gap_y = 40, 30
    temp_y = y_start
    for layer_name in ["apps", "core", "templates", "other"]:
        layer_crates = layers.get(layer_name, [])
        if not layer_crates:
            continue
        temp_y += 18
        for row_idx in range(0, len(layer_crates), 2):
            row_crates = layer_crates[row_idx:row_idx + 2]
            num = len(row_crates)
            start_x = (viewbox_w - (num * col_w + (num - 1) * gap_x)) // 2
            max_row_h = 0
            for i, crate in enumerate(row_crates):
                cx = start_x + i * (col_w + gap_x)
                _, box_h = compute_card_dimensions(crate, card_w=col_w)
                coords[crate["name"]] = (cx, temp_y, col_w, box_h)
                max_row_h = max(max_row_h, box_h)
            temp_y += max_row_h + gap_y
    return coords


def boxes_intersect(a: tuple, b: tuple) -> bool:
    ax, ay, aw, ah = a[0], a[1], a[2], a[3]
    bx, by, bw, bh = b[0], b[1], b[2], b[3]
    return not (ax + aw <= bx or bx + bw <= ax or ay + ah <= by or by + bh <= ay)


def detect_overlaps(coords: dict) -> list[tuple[str, str]]:
    items = list(coords.items())
    overlaps = []
    for i, (na, a) in enumerate(items):
        for nb, b in items[i + 1:]:
            if boxes_intersect(a, b):
                overlaps.append((na, nb))
    return overlaps


def fix_overlaps(coords: dict, max_iterations: int = 10) -> int:
    fix_count = 0
    for _ in range(max_iterations):
        overlaps = detect_overlaps(coords)
        if not overlaps:
            break
        for na, nb in overlaps:
            a, b = coords[na], coords[nb]
            overlap_y = min(a[1] + a[3], b[1] + b[3]) - max(a[1], b[1])
            overlap_x = min(a[0] + a[2], b[0] + b[2]) - max(a[0], b[0])
            if overlap_y > 0 and overlap_x > 0:
                if a[1] <= b[1]:
                    coords[nb] = (b[0], a[1] + a[3] + 15, b[2], b[3])
                else:
                    coords[na] = (a[0], b[1] + b[3] + 15, a[2], a[3])
                fix_count += 1
    return fix_count


def route_edge_avoiding_obstacles(sx, sy, tx, ty, obstacles, src_name, dst_name) -> str:
    min_gap = 20
    for obs_name, (ox, oy, ow, oh) in obstacles.items():
        if obs_name in (src_name, dst_name):
            continue
        if min(ox + ow, tx) - max(ox, sx) > 0 and min(oy + oh, ty) - max(oy, sy) > 0:
            if sx < ox and tx > ox:
                sx_adj = ox - min_gap
                return (f'M {sx} {sy} L {sx} {sy + 20} L {sx_adj} {sy + 20} '
                        f'L {sx_adj} {ty - 20} L {tx} {ty - 20} L {tx} {ty}')
    if abs(sx - tx) < 10:
        return f'M {sx} {sy} L {tx} {ty}'
    mid_y = (sy + ty) // 2
    return f'M {sx} {sy} L {sx} {mid_y} L {tx} {mid_y} L {tx} {ty}'
