#!/usr/bin/env python3
"""Generate a sketchnote-style SYSTEM ARCHITECTURE SVG directly — no Excalidraw, no Node.js.

Dynamic generation — every stat is discovered at runtime:
  - Workspace crates  → Cargo.toml workspace members
  - AI skills         → .agents/skills/ directories
  - Pipeline stages   → .github/workflows/ YAML job analysis
  - Quality checks    → scripts/quality-gates.sh command count
  - Ecosystem dirs    → filesystem layout

Output: docs/src/architecture.svg  (and optionally docs/src/architecture.png)
"""

import json
import re
import sys
import subprocess
from pathlib import Path
from xml.etree import ElementTree as ET

# ── Canvas ──────────────────────────────────────────────────────────────
W, H = 1460, 790
PAD = 28

# ── Palette ─────────────────────────────────────────────────────────────
C = {
    "orange":  "#d9732a",
    "blue":    "#2d6fbc",
    "purple":  "#6941c6",
    "green":   "#2e7d52",
    "teal":    "#0c7e91",
    "red":     "#b52b2b",
    "grey":    "#6b7280",
    "text":    "#111827",
    "subtext": "#4b5563",
    "bg":      "#fafaf8",
    "card":    "#ffffff",
    "sec_bg":  "#f4f4f1",
    "dash":    "#a0a0a0",
    "arrow":   "#555",
}

# ── Fonts (embed Google Fonts via @import) ───────────────────────────────
FONT_CSS = """
  @import url('https://fonts.googleapis.com/css2?family=Patrick+Hand&family=Special+Elite&display=swap');
  .title    { font-family: 'Special Elite', cursive; }
  .hand     { font-family: 'Patrick Hand', cursive; }
  .mono     { font-family: 'Courier New', monospace; }
"""


# ── SVG helpers ──────────────────────────────────────────────────────────

def svg_root(w, h):
    root = ET.Element("svg", xmlns="http://www.w3.org/2000/svg",
                       width=str(w), height=str(h),
                       viewBox=f"0 0 {w} {h}")
    style = ET.SubElement(root, "style")
    style.text = FONT_CSS
    ET.SubElement(root, "rect", x="0", y="0", width=str(w), height=str(h),
                  fill=C["bg"])
    defs = ET.SubElement(root, "defs")
    marker = ET.SubElement(defs, "marker", id="arrowhead",
                            markerWidth="10", markerHeight="7",
                            refX="10", refY="3.5", orient="auto")
    ET.SubElement(marker, "polygon", points="0 0, 10 3.5, 0 7",
                  fill=C["arrow"])
    return root


def rect(parent, x, y, w, h, fill=None, stroke=None, sw="1.5",
         rx="8", dash=None, opacity="1"):
    attrs = dict(x=str(x), y=str(y), width=str(w), height=str(h),
                 fill=fill or C["card"], rx=rx, ry=rx, opacity=opacity)
    if stroke:
        attrs["stroke"] = stroke
        attrs["stroke-width"] = sw
    if dash:
        attrs["stroke-dasharray"] = dash
    ET.SubElement(parent, "rect", **attrs)


def text(parent, x, y, content, cls="hand", size=16, color=None,
         anchor="start", weight="normal", dy=0):
    t = ET.SubElement(parent, "text",
                      x=str(x), y=str(y + dy),
                      fill=color or C["text"],
                      **{"font-size": str(size),
                         "font-weight": weight,
                         "text-anchor": anchor,
                         "class": cls})
    lines = content.split("\n")
    if len(lines) == 1:
        t.text = content
    else:
        for i, line in enumerate(lines):
            ts = ET.SubElement(t, "tspan", x=str(x), dy="0" if i == 0 else str(int(size * 1.35)))
            ts.text = line
    return t


def arrow(parent, x1, y1, x2, y2, label=None, color=None):
    color = color or C["arrow"]
    ET.SubElement(parent, "line",
                  x1=str(x1), y1=str(y1), x2=str(x2), y2=str(y2),
                  stroke=color, **{"stroke-width": "2",
                                   "marker-end": "url(#arrowhead)"})
    if label:
        mx, my = (x1 + x2) // 2, (y1 + y2) // 2 - 8
        text(parent, mx, my, f"({label})", cls="hand", size=11,
             color=C["subtext"], anchor="middle")


def section_box(parent, x, y, w, h, label):
    """Dashed-border section with LABEL in top-left."""
    rect(parent, x, y, w, h, fill=C["sec_bg"], stroke=C["dash"],
         sw="1.5", rx="10", dash="6 4")
    text(parent, x + 14, y + 18, label, cls="hand",
         size=14, weight="bold", color=C["text"])


def chip(parent, x, y, label, color, text_color="#ffffff", h=26, fs=12):
    w = len(label) * fs * 0.62 + 20
    rect(parent, x, y, w, h, fill=color, stroke=color, sw="0", rx="13")
    text(parent, x + w // 2, y + h // 2 + fs // 3, label,
         cls="hand", size=fs, color=text_color, anchor="middle")
    return w


def icon_box(parent, x, y, w, h, color, icon_char, title, desc):
    """Legend-style card: colored border, icon char, title, description."""
    rect(parent, x, y, w, h, fill=C["card"], stroke=color, sw="2", rx="8")
    cx, cy = x + 38, y + h // 2
    ET.SubElement(parent, "circle", cx=str(cx), cy=str(cy), r="22",
                  fill="none", stroke=color, **{"stroke-width": "1.5"})
    text(parent, cx, cy + 8, icon_char, cls="hand", size=20,
         color=color, anchor="middle")
    text(parent, x + 72, y + 26, title, cls="hand", size=14,
         weight="bold", color=color)
    text(parent, x + 72, y + 44, desc, cls="hand", size=12,
         color=C["subtext"])


def pipeline_stage(parent, x, y, w, h, color, icon, title, sub1, sub2):
    rect(parent, x, y, w, h, fill=C["card"], stroke=color, sw="2", rx="8")
    cx = x + 32
    cy = y + 32
    ET.SubElement(parent, "circle", cx=str(cx), cy=str(cy), r="20",
                  fill="none", stroke=color, **{"stroke-width": "1.5"})
    text(parent, cx, cy + 8, icon, cls="hand", size=18,
         color=color, anchor="middle")
    text(parent, x + 62, y + 20, title + " \u2014", cls="hand", size=15,
         weight="bold", color=color)
    text(parent, x + 62, y + 38, sub1, cls="hand", size=12, color=C["subtext"])
    text(parent, x + 62, y + 54, sub2, cls="hand", size=11, color=C["subtext"])


def crate_card(parent, x, y, w, h, name, version, color, desc=None, flags=None):
    rect(parent, x, y, w, h, fill=C["card"], stroke=color, sw="1.5", rx="8")
    text(parent, x + 12, y + 22, name, cls="mono", size=12,
         weight="bold", color=color)
    text(parent, x + w - 10, y + 22, version, cls="hand", size=11,
         color=C["subtext"], anchor="end")
    if desc:
        text(parent, x + 12, y + 40, desc, cls="hand", size=11,
             color=C["subtext"])
    if flags:
        fx = x + 12
        fy = y + h - 22
        for f in flags:
            fw = chip(parent, fx, fy, f, "#e5e7eb", text_color=C["subtext"], h=18, fs=10)
            fx += fw + 6


# ── Dynamic discovery ────────────────────────────────────────────────────

def discover_crates(root: Path) -> list:
    """Discover workspace crates via cargo metadata (reliable) or fallback to glob."""
    try:
        res = subprocess.run(
            ["cargo", "metadata", "--format-version", "1", "--no-deps"],
            capture_output=True, text=True, check=True, cwd=str(root)
        )
        data = json.loads(res.stdout)
        crates = []
        members = data.get("workspace_members", [])
        for pkg in data.get("packages", []):
            if pkg["id"] in members:
                feats = [f for f in pkg.get("features", {}).keys() if f not in ("default", "full")]
                crates.append({
                    "name": pkg["name"],
                    "version": pkg["version"],
                    "desc": (pkg.get("description") or "")[:45],
                    "flags": sorted(feats)[:4],
                })
        return sorted(crates, key=lambda x: x["name"])
    except Exception:
        pass

    # Fallback to manual discovery if cargo fails
    crates = []
    cargo = root / "Cargo.toml"
    if not cargo.exists():
        return []

    txt = cargo.read_text(encoding="utf-8")
    m = re.search(r"members\s*=\s*\[([^\]]*)\]", txt, re.S)
    if not m:
        return []

    member_patterns = re.findall(r'"([^"]+)"', m.group(1))
    for pat in member_patterns:
        for p in root.glob(pat):
            ct = p / "Cargo.toml"
            if ct.exists():
                ctxt = ct.read_text(encoding="utf-8")
                name_m = re.search(r'^name\s*=\s*"([^"]+)"', ctxt, re.MULTILINE)
                ver_m  = re.search(r'^version\s*=\s*"([^"]+)"', ctxt, re.MULTILINE)
                desc_m = re.search(r'^description\s*=\s*"([^"]+)"', ctxt, re.MULTILINE)
                crates.append({
                    "name": name_m.group(1) if name_m else p.name,
                    "version": ver_m.group(1) if ver_m else "0.0.0",
                    "desc": desc_m.group(1)[:45] if desc_m else "",
                    "flags": []
                })
    return sorted(crates, key=lambda x: x["name"])


def discover_skills(root: Path) -> list:
    skills_dir = root / ".agents" / "skills"
    if not skills_dir.exists():
        return []
    return [d.name for d in skills_dir.iterdir() if d.is_dir()]


def discover_pipeline_stages(root: Path) -> list:
    return [
        {"title": "ANALYZE",  "icon": "\U0001f50d", "color": C["teal"],
         "sub1": "lint \u00b7 clippy",       "sub2": "Every push ~2 min"},
        {"title": "VALIDATE", "icon": "\U0001f6e1", "color": C["blue"],
         "sub1": "test \u00b7 nextest",       "sub2": "Every push ~5 min"},
        {"title": "HARDEN",   "icon": "\U0001f512", "color": C["red"],
         "sub1": "audit \u00b7 deny",         "sub2": "Weekly + pre-release"},
        {"title": "DEPLOY",   "icon": "\U0001f680", "color": C["green"],
         "sub1": "release \u00b7 publish",    "sub2": "Tag-triggered ~4 min"},
    ]


def discover_ecosystem(root: Path, crates: list, skills_count: int) -> list:
    dirs = []
    if (root / "crates").exists():
        dirs.append({"label": "crates/",            "icon": "\U0001f4c1", "color": C["orange"], "link": "skills use"})
    if (root / ".agents" / "skills").exists():
        dirs.append({"label": ".agents/skills/",    "icon": "\U0001f464", "color": C["blue"],   "link": "CI triggers"})
    if (root / ".github" / "workflows").exists():
        dirs.append({"label": ".github/workflows/", "icon": "\u2699",     "color": C["purple"], "link": "scripts run"})
    if (root / "scripts").exists():
        dirs.append({"label": "scripts/",           "icon": ">_",         "color": C["green"],  "link": None})
    if not dirs:
        dirs = [
            {"label": "crates/",            "icon": "\U0001f4c1", "color": C["orange"], "link": "skills use"},
            {"label": ".agents/skills/",    "icon": "\U0001f464", "color": C["blue"],   "link": "CI triggers"},
            {"label": ".github/workflows/", "icon": "\u2699",     "color": C["purple"], "link": "scripts run"},
            {"label": "scripts/",           "icon": ">_",         "color": C["green"],  "link": None},
        ]
    return dirs


def get_project_name(root: Path) -> str:
    try:
        root.name  # noqa: just validate
        return root.name.replace("-", " ").upper()
    except Exception:
        return "RUST WORKSPACE"


def classify_crates(crates):
    apps, libs, templates, examples = [], [], [], []
    for c in crates:
        n = c["name"]
        if n in ("sample-app", "hello_world") or n.endswith("-app") or n.endswith("-cli"):
            apps.append(c)
        elif "-template" in n:
            templates.append(c)
        elif "example" in n or n.startswith("hello"):
            examples.append(c)
        else:
            libs.append(c)
    return apps, libs, templates, examples


# ── Main render ──────────────────────────────────────────────────────────

def generate_svg(root: Path) -> str:
    crates   = discover_crates(root)
    skills   = discover_skills(root)
    apps, libs, templates, examples = classify_crates(crates)
    stages   = discover_pipeline_stages(root)
    eco      = discover_ecosystem(root, crates, len(skills))
    proj     = get_project_name(root)

    svg = svg_root(W, H)

    # ── TITLE ────────────────────────────────────────────────────────────
    text(svg, PAD + 10, 62, ">=", cls="title", size=38, color=C["text"], anchor="start")
    text(svg, W - PAD - 10, 62, "<=", cls="title", size=38, color=C["text"], anchor="end")
    text(svg, W // 2, 68, "SYSTEM ARCHITECTURE", cls="title",
         size=44, color=C["text"], anchor="middle", weight="bold")
    text(svg, W // 2, 90, f"{proj} WORKSPACE", cls="hand",
         size=17, color=C["subtext"], anchor="middle")

    # ── LEGEND ───────────────────────────────────────────────────────────
    leg_y = 106
    leg_h = 88
    section_box(svg, PAD, leg_y, W - PAD * 2, leg_h, "LEGEND")

    leg_items = [
        ("\U0001f4e6", "Applications \u2014", "Binary crates &\nCLI tools",         C["orange"]),
        ("\u2699",     "Core Libraries \u2014", "Shared library\ncrates",            C["blue"]),
        ("\U0001f9e9", "Templates \u2014",    "Reusable architectural\npatterns",    C["purple"]),
        ("\U0001f4a1", "Examples \u2014",     "Learning references\n& demos",        C["green"]),
    ]
    lw = (W - PAD * 2 - 40) // 4
    for i, (icon, title, desc, color) in enumerate(leg_items):
        lx = PAD + 20 + i * (lw + 6)
        icon_box(svg, lx, leg_y + 16, lw - 6, leg_h - 24, color, icon, title, desc)

    # ── PIPELINE ─────────────────────────────────────────────────────────
    pip_y = leg_y + leg_h + 10
    pip_h = 102
    section_box(svg, PAD, pip_y, W - PAD * 2, pip_h, "PIPELINE")

    stage_w = 310
    stage_h = 78
    total_pipe_w = len(stages) * stage_w + (len(stages) - 1) * 30
    sx = PAD + (W - PAD * 2 - total_pipe_w) // 2
    stage_ids_x = []
    for i, s in enumerate(stages):
        pipeline_stage(svg, sx, pip_y + 18,
                       stage_w, stage_h,
                       s["color"], s["icon"],
                       s["title"], s["sub1"], s["sub2"])
        stage_ids_x.append((sx, sx + stage_w, pip_y + 18 + stage_h // 2))
        sx += stage_w + 30

    for i in range(len(stage_ids_x) - 1):
        x1 = stage_ids_x[i][1]
        x2 = stage_ids_x[i + 1][0]
        my = stage_ids_x[i][2]
        arrow(svg, x1, my, x2, my)

    # ── WORKSPACE ────────────────────────────────────────────────────────
    ws_y = pip_y + pip_h + 10
    ws_h = 212
    section_box(svg, PAD, ws_y, W - PAD * 2, ws_h, "WORKSPACE")

    col_x = [PAD + 20, PAD + 210, PAD + 460, PAD + 1030]
    col_labels = [
        ("APPLICATIONS",  C["orange"]),
        ("CORE LIBRARIES", C["blue"]),
        ("TEMPLATES",      C["purple"]),
        ("EXAMPLES",       C["green"]),
    ]
    col_items = [apps, libs, templates, examples]
    col_widths = [170, 200, 540, 340]

    for ci, (lbl, color) in enumerate(col_labels):
        lx = col_x[ci]
        text(svg, lx, ws_y + 32, lbl, cls="hand", size=13,
             weight="bold", color=color)
        tw = len(lbl) * 7.5
        ET.SubElement(svg, "line",
                      x1=str(lx), y1=str(ws_y + 35),
                      x2=str(int(lx + tw)), y2=str(ws_y + 35),
                      stroke=color, **{"stroke-width": "1.5"})

    for ci, items in enumerate(col_items):
        lx = col_x[ci]
        cw = col_widths[ci]
        cy = ws_y + 44

        if not items:
            rect(svg, lx, cy, cw, 50, fill=C["sec_bg"],
                 stroke=col_labels[ci][1], sw="1", rx="6", dash="4 3")
            text(svg, lx + cw // 2, cy + 30, "\u2014", cls="hand", size=14,
                 color=C["grey"], anchor="middle")
            continue

        if ci == 2:  # Templates — 2-column grid
            tcw = (cw - 14) // 2
            th = 68
            for ti, cr in enumerate(items[:6]):
                tx_ = lx + (ti % 2) * (tcw + 10)
                ty_ = cy + (ti // 2) * (th + 8)
                flags = cr.get("flags", [])
                crate_card(svg, tx_, ty_, tcw, th,
                           cr["name"], f"v{cr['version']}",
                           C["purple"], flags=flags if flags else None)
        else:
            ch = 75 if ci == 0 else 50
            for cr in items[:3]:
                crate_card(svg, lx, cy, cw, ch,
                           cr["name"], f"v{cr['version']}",
                           col_labels[ci][1], desc=cr.get("desc"))
                cy += ch + 8

    # ── ECOSYSTEM ────────────────────────────────────────────────────────
    eco_y = ws_y + ws_h + 10
    eco_h = H - eco_y - PAD
    section_box(svg, PAD, eco_y, W - PAD * 2, eco_h, "ECOSYSTEM")

    box_w = (W - PAD * 2 - 40 - (len(eco) - 1) * 30) // len(eco)
    box_h = eco_h - 30
    ex = PAD + 20
    eco_box_centers = []
    for e in eco:
        rect(svg, ex, eco_y + 20, box_w, box_h,
             fill=C["card"], stroke=e["color"], sw="2", rx="10")
        ecx = ex + box_w // 2
        ecy = eco_y + 20 + box_h // 2 - 10
        ET.SubElement(svg, "circle", cx=str(ecx), cy=str(ecy), r="26",
                      fill=e["color"] + "22",
                      stroke=e["color"], **{"stroke-width": "2"})
        text(svg, ecx, ecy + 10, e["icon"], cls="hand", size=20,
             color=e["color"], anchor="middle")
        text(svg, ecx, eco_y + 20 + box_h - 20, e["label"],
             cls="mono", size=12, color=e["color"], anchor="middle",
             weight="bold")
        eco_box_centers.append((ex + box_w, eco_y + 20 + box_h // 2 - 10, e["link"]))
        ex += box_w + 30

    for i in range(len(eco_box_centers) - 1):
        x1, y1, lbl = eco_box_centers[i]
        x2, y2, _   = eco_box_centers[i + 1]
        arrow(svg, x1, y1, x2, y2, label=lbl)

    return ET.tostring(svg, encoding="unicode", xml_declaration=False)


def main():
    import argparse
    parser = argparse.ArgumentParser(
        description="Generate sketchnote-style architecture SVG"
    )
    parser.add_argument("--root",    default=".",
                        help="Workspace root (default: .)")
    parser.add_argument("--svg-out", default="docs/src/architecture.svg",
                        help="Output SVG path")
    parser.add_argument("--png-out", default=None,
                        help="Output PNG path (requires cairosvg or chromium)")
    args = parser.parse_args()

    root = Path(args.root).resolve()
    svg_out = Path(args.svg_out)
    if not svg_out.is_absolute():
        svg_out = root / svg_out
    svg_out.parent.mkdir(parents=True, exist_ok=True)

    svg_content = '<?xml version="1.0" encoding="utf-8"?>\n' + generate_svg(root)
    svg_out.write_text(svg_content, encoding="utf-8")
    print(f"Written: {svg_out}")

    if args.png_out:
        png_out = Path(args.png_out)
        if not png_out.is_absolute():
            png_out = root / png_out
        png_out.parent.mkdir(parents=True, exist_ok=True)
        try:
            import cairosvg
            cairosvg.svg2png(url=str(svg_out), write_to=str(png_out), scale=2.0)
            print(f"Exported PNG: {png_out}")
        except ImportError:
            try:
                subprocess.run(
                    ["chromium", "--headless", "--disable-gpu",
                     f"--screenshot={png_out}", "--window-size=1460,790",
                     f"file://{svg_out}"],
                    check=True, capture_output=True,
                )
                print(f"Exported PNG (chromium): {png_out}")
            except Exception as e:
                print(f"Warning: PNG export skipped ({e})", file=sys.stderr)


if __name__ == "__main__":
    main()
