#!/usr/bin/env python3
"""Generate a human-friendly overview diagram in Excalidraw format.

This produces a non-technical infographic showing:
1. What the project is (hero)
2. How to get started (workflow)
3. What's inside (component grid)
4. How it all connects (ecosystem)

Output: .template/overview.excalidraw + .template/overview.svg
"""

import json
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))

from lib.discovery import discover_crates, discover_skills, discover_agents, discover_commands

# ── Palette (friendly, not technical) ──────────────────────────────────
C = {
    "bg":      "#ffffff",
    "primary": "#4263eb",
    "accent":  "#f76707",
    "green":   "#2b8a3e",
    "teal":    "#0c8599",
    "purple":  "#7048e8",
    "rose":    "#e03131",
    "muted":   "#f1f3f5",
    "text":    "#212529",
    "subtext": "#868e96",
    "card":    "#f8f9fa",
}

_seed_counter = 0

def _seed(s: str) -> int:
    h = 0
    for c in s:
        h = (h * 31 + ord(c)) & 0xFFFFFFFF
    return h

def _id(prefix: str) -> str:
    global _seed_counter
    _seed_counter += 1
    return f"{prefix}_{_seed_counter}"


def _rect(x, y, w, h, bg=C["card"], stroke="#dee2e6", sw=1, label=None,
          lfs=16, lac="center", lvc="middle", lc=C["text"], rid=None,
          rough=0, opacity=100):
    el = {
        "id": _id("r"), "type": "rectangle",
        "x": x, "y": y, "width": w, "height": h,
        "angle": 0, "strokeColor": stroke, "backgroundColor": bg,
        "fillStyle": "solid", "strokeWidth": sw, "strokeStyle": "solid",
        "roughness": 0, "opacity": opacity,
        "groupIds": [], "frameId": rid, "index": None,
        "roundness": {"type": 3}, "seed": _seed(f"r{x}{y}"),
        "version": 1, "versionNonce": _seed(f"rn{x}{y}"),
        "isDeleted": False, "boundElements": None,
        "updated": 1, "link": None, "locked": False,
    }
    if label:
        el["label"] = {"text": label, "fontSize": lfs, "fontFamily": 2,
                        "textAlign": lac, "verticalAlign": lvc,
                        "strokeColor": lc}
    return el


def _text(x, y, text, fs=16, color=C["text"], align="left", rid=None):
    return {
        "id": _id("t"), "type": "text",
        "x": x, "y": y,
        "width": len(text) * fs * 0.55,
        "height": fs * 1.25,
        "angle": 0, "strokeColor": color, "backgroundColor": "transparent",
        "fillStyle": "solid", "strokeWidth": 2, "strokeStyle": "solid",
        "roughness": 0, "opacity": 100,
        "groupIds": [], "frameId": rid, "index": None, "roundness": None,
        "seed": _seed(f"t{x}{y}"), "version": 1,
        "versionNonce": _seed(f"tn{x}{y}"),
        "isDeleted": False, "boundElements": None,
        "updated": 1, "link": None, "locked": False,
        "text": text, "fontSize": fs, "fontFamily": 2,
        "textAlign": align, "verticalAlign": "top",
        "containerId": None, "originalText": text, "lineHeight": 1.25,
    }


def _arrow(x, y, start_id, end_id, points=None, sc="#adb5bd", sw=2, label=None):
    if points is None:
        points = [[0, 0], [100, 0]]
    el = {
        "id": _id("a"), "type": "arrow",
        "x": x, "y": y,
        "width": abs(points[-1][0]) or 1,
        "height": abs(points[-1][1]) or 1,
        "angle": 0, "strokeColor": sc, "backgroundColor": "transparent",
        "fillStyle": "solid", "strokeWidth": sw, "strokeStyle": "solid",
        "roughness": 0, "opacity": 100,
        "groupIds": [], "frameId": None, "index": None,
        "roundness": {"type": 2}, "seed": _seed(f"a{x}{y}"),
        "version": 1, "versionNonce": _seed(f"an{x}{y}"),
        "isDeleted": False, "boundElements": None,
        "updated": 1, "link": None, "locked": False,
        "points": points,
        "startBinding": {"elementId": start_id, "focus": 0, "gap": 5},
        "endBinding": {"elementId": end_id, "focus": 0, "gap": 5},
        "startArrowhead": None, "endArrowhead": "arrow", "elbowed": False,
    }
    if label:
        el["label"] = {"text": label, "fontSize": 14, "fontFamily": 2}
    return el


def _frame(name, children):
    return {
        "id": _id("f"), "type": "frame",
        "x": 0, "y": 0, "width": 100, "height": 100,
        "angle": 0, "strokeColor": "#868e96", "backgroundColor": "transparent",
        "fillStyle": "solid", "strokeWidth": 1, "strokeStyle": "solid",
        "roughness": 0, "opacity": 100,
        "groupIds": [], "frameId": None, "index": None, "roundness": None,
        "seed": _seed("frame"), "version": 1,
        "versionNonce": _seed("framen"), "isDeleted": False,
        "boundElements": None, "updated": 1, "link": None, "locked": False,
        "name": name, "children": children,
    }


def generate(root: Path) -> dict:
    crates = discover_crates(root)
    skills = discover_skills(root)
    commands = discover_commands(root)

    elements = []

    # ════════════════════════════════════════════════════════════════════
    # SECTION 1: HERO — What is this?
    # ════════════════════════════════════════════════════════════════════
    hero_children = []

    # Background
    hero_bg = _rect(30, 20, 1140, 180, bg="#edf2ff", stroke=C["primary"],
                    sw=2, rough=1, opacity=100)
    elements.append(hero_bg)
    hero_children.append(hero_bg["id"])

    # Title
    title = _text(80, 50, "Rust 2026 Template", fs=36, color=C["primary"])
    elements.append(title)
    hero_children.append(title["id"])

    # Tagline
    tagline = _text(80, 105, "Production-ready Rust workspace with AI agents, quality gates, and modern tooling",
                    fs=18, color=C["subtext"])
    elements.append(tagline)
    hero_children.append(tagline["id"])

    # Feature pills
    features = ["AI-Native Skills", "CI/CD Pipeline", "Quality Gates",
                "Mutation Testing", "Security Audits", "MCP Server"]
    pill_x = 80
    for feat in features:
        pw = len(feat) * 9 + 24
        pill = _rect(pill_x, 145, pw, 30, bg=C["primary"], stroke=C["primary"],
                     sw=0, label=feat, lfs=12, lc="#ffffff", rough=1)
        elements.append(pill)
        hero_children.append(pill["id"])
        pill_x += pw + 12

    hero_frame = _frame("WHAT IS THIS?", hero_children)
    elements.append(hero_frame)

    # ════════════════════════════════════════════════════════════════════
    # SECTION 2: QUICK START — How to get started
    # ════════════════════════════════════════════════════════════════════
    qs_y = 240
    qs_children = []

    # Section title
    qs_title = _text(50, qs_y, "GETTING STARTED", fs=14, color=C["subtext"])
    elements.append(qs_title)
    qs_children.append(qs_title["id"])

    # Workflow steps
    steps = [
        ("1. Clone", "Use this template\nto start a new repo", C["primary"]),
        ("2. Setup", "./scripts/bootstrap.sh\ninstalls all tools", C["teal"]),
        ("3. Develop", "Write code in crates/\nwith feature flags", C["green"]),
        ("4. Quality", "./scripts/quality-gates.sh\nruns all checks", C["accent"]),
        ("5. Release", "Tag-triggered CI\npublishes to crates.io", C["purple"]),
    ]

    step_w, step_h, gap = 200, 100, 25
    sx = 50
    step_ids = []
    for i, (title, desc, color) in enumerate(steps):
        sid = f"step_{i}"
        card = _rect(sx, qs_y + 30, step_w, step_h, bg="#f8f9fa", stroke=color,
                     sw=2, rough=1, rid=None)
        card["id"] = sid
        elements.append(card)
        qs_children.append(sid)
        step_ids.append(sid)

        st = _text(sx + 15, qs_y + 45, title, fs=16, color=color)
        elements.append(st)
        qs_children.append(st["id"])

        sd = _text(sx + 15, qs_y + 75, desc, fs=12, color=C["subtext"])
        elements.append(sd)
        qs_children.append(sd["id"])

        sx += step_w + gap

    # Arrows between steps
    for i in range(len(step_ids) - 1):
        src = step_ids[i]
        tgt = step_ids[i + 1]
        sx_pos = 50 + (i + 1) * (step_w + gap) - gap // 2
        arr = _arrow(sx_pos - 10, qs_y + 80, src, tgt,
                     points=[[0, 0], [gap, 0]], sc="#adb5bd", sw=2)
        elements.append(arr)
        qs_children.append(arr["id"])

    qs_frame = _frame("HOW TO START", qs_children)
    elements.append(qs_frame)

    # ════════════════════════════════════════════════════════════════════
    # SECTION 3: WHAT'S INSIDE — Component grid
    # ════════════════════════════════════════════════════════════════════
    ws_y = 420
    ws_children = []

    ws_title = _text(50, ws_y, "WHAT'S INSIDE", fs=14, color=C["subtext"])
    elements.append(ws_title)
    ws_children.append(ws_title["id"])

    # Stats cards
    stats = [
        (f"{len(crates)}", "Workspace Crates", "Libraries, apps,\nand templates", C["primary"]),
        (f"{len(skills)}", "AI Skills", "Reusable procedures\nfor code agents", C["teal"]),
        ("5", "CI Workflows", "Lint, test, security,\nrelease automation", C["green"]),
        ("14", "Quality Checks", "Format, clippy, audit,\ndeny, privacy scan", C["accent"]),
    ]

    stat_w, stat_h, stat_gap = 260, 130, 20
    stat_x = 50
    for num, label, desc, color in stats:
        card = _rect(stat_x, ws_y + 30, stat_w, stat_h, bg="#f8f9fa",
                     stroke=color, sw=2, rough=1)
        elements.append(card)
        ws_children.append(card["id"])

        n = _text(stat_x + 20, ws_y + 45, num, fs=36, color=color)
        elements.append(n)
        ws_children.append(n["id"])

        l = _text(stat_x + 20, ws_y + 95, label, fs=14, color=C["text"])
        elements.append(l)
        ws_children.append(l["id"])

        d = _text(stat_x + 20, ws_y + 118, desc, fs=11, color=C["subtext"])
        elements.append(d)
        ws_children.append(d["id"])

        stat_x += stat_w + stat_gap

    ws_frame = _frame("BY THE NUMBERS", ws_children)
    elements.append(ws_frame)

    # ════════════════════════════════════════════════════════════════════
    # SECTION 4: ECOSYSTEM — How it connects
    # ════════════════════════════════════════════════════════════════════
    eco_y = 610
    eco_children = []

    eco_title = _text(50, eco_y, "HOW IT CONNECTS", fs=14, color=C["subtext"])
    elements.append(eco_title)
    eco_children.append(eco_title["id"])

    # Ecosystem boxes
    eco_boxes = [
        ("crates/", "Your code lives here.\n11 workspace members\nwith feature flags.", C["primary"], 50),
        (".agents/skills/", "AI procedures that\nguide coding agents.\n22 reusable skills.", C["teal"], 340),
        (".github/workflows/", "Automated CI/CD.\nRuns on every push.\nSecurity + quality gates.", C["green"], 630),
        ("scripts/", "Local dev tools.\nQuality gates, bootstrap,\nrelease automation.", C["accent"], 920),
    ]

    box_w, box_h = 260, 120
    box_ids = []
    for label, desc, color, bx in eco_boxes:
        bid = f"eco_{label.replace('/', '_').replace('.', '')}"
        card = _rect(bx, eco_y + 30, box_w, box_h, bg="#f8f9fa",
                     stroke=color, sw=2, rough=1)
        card["id"] = bid
        elements.append(card)
        eco_children.append(bid)
        box_ids.append(bid)

        l = _text(bx + 15, eco_y + 45, label, fs=14, color=color)
        elements.append(l)
        eco_children.append(l["id"])

        d = _text(bx + 15, eco_y + 72, desc, fs=11, color=C["subtext"])
        elements.append(d)
        eco_children.append(d["id"])

    # Connection arrows
    connections = [
        (0, 1, "skills use"),
        (1, 2, "CI triggers"),
        (2, 3, "scripts run"),
        (0, 3, "quality gates"),
    ]
    for src_i, tgt_i, label in connections:
        arr = _arrow(
            50 + (src_i + 1) * (box_w + 20) - 10, eco_y + 90,
            box_ids[src_i], box_ids[tgt_i],
            points=[[0, 0], [box_w + 20, 0]],
            sc="#adb5bd", sw=2, label=label,
        )
        elements.append(arr)
        eco_children.append(arr["id"])

    eco_frame = _frame("ECOSYSTEM MAP", eco_children)
    elements.append(eco_frame)

    return {
        "type": "excalidraw",
        "version": 2,
        "source": "https://excalidraw.com",
        "elements": elements,
        "appState": {"gridSize": None, "viewBackgroundColor": "#ffffff"},
        "files": {},
    }


def main():
    import argparse
    parser = argparse.ArgumentParser(description="Generate overview diagram")
    parser.add_argument("--root", default=".")
    parser.add_argument("--out", default=".template/overview.excalidraw")
    parser.add_argument("--svg-out", default=".template/overview.svg")
    parser.add_argument("--no-export", action="store_true")
    args = parser.parse_args()

    root = Path(args.root).resolve()
    out = Path(args.out) if args.out else root / ".template" / "overview.excalidraw"
    if not out.is_absolute():
        out = root / out
    out.parent.mkdir(parents=True, exist_ok=True)

    data = generate(root)
    out.write_text(json.dumps(data, indent=2), encoding="utf-8")
    print(f"Written: {out}")

    if not args.no_export:
        svg_out = Path(args.svg_out)
        if not svg_out.is_absolute():
            svg_out = root / svg_out
        svg_out.parent.mkdir(parents=True, exist_ok=True)

        exporter = Path(__file__).resolve().parent / "export_excalidraw.mjs"
        if exporter.exists():
            import subprocess
            try:
                subprocess.run(
                    ["node", str(exporter), "-i", str(out), "-o", str(svg_out), "-f", "svg"],
                    capture_output=True, text=True, timeout=30, check=True, shell=False,
                )
                print(f"Exported: {svg_out}")
            except Exception as e:
                print(f"Warning: SVG export failed ({e})", file=sys.stderr)


if __name__ == "__main__":
    main()
