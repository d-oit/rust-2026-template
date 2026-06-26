#!/usr/bin/env python3
"""Generate a compact hand-drawn overview diagram in Excalidraw format.

Text is rendered as separate text elements positioned inside rectangles.
All elements use roughness=1 for a sketchy hand-drawn look.
"""

import json
import re
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))

from lib.discovery import discover_crates, discover_skills, discover_agents, discover_commands

C = {
    "bg":      "#ffffff",
    "primary": "#4263eb",
    "accent":  "#f76707",
    "green":   "#2b8a3e",
    "teal":    "#0c8599",
    "purple":  "#7048e8",
    "text":    "#212529",
    "subtext": "#868e96",
    "card":    "#f8f9fa",
}

_seed_counter = 0

def _id(prefix: str) -> str:
    global _seed_counter
    _seed_counter += 1
    return f"{prefix}_{_seed_counter}"

def _seed(s: str) -> int:
    h = 0
    for c in s:
        h = (h * 31 + ord(c)) & 0xFFFFFFFF
    return h

def _rect(x, y, w, h, bg=None, stroke="#dee2e6", sw=1, rid=None):
    bg = bg or C["card"]
    return {
        "id": _id("r"), "type": "rectangle",
        "x": x, "y": y, "width": w, "height": h,
        "angle": 0, "strokeColor": stroke, "backgroundColor": bg,
        "fillStyle": "solid", "strokeWidth": sw, "strokeStyle": "solid",
        "roughness": 1, "opacity": 100,
        "groupIds": [], "frameId": rid, "index": None,
        "roundness": {"type": 3}, "seed": _seed(f"r{x}{y}"),
        "version": 1, "versionNonce": _seed(f"rn{x}{y}"),
        "isDeleted": False, "boundElements": None,
        "updated": 1, "link": None, "locked": False,
    }

def _text(x, y, text, fs=14, color=None, align="left", rid=None):
    color = color or C["text"]
    return {
        "id": _id("t"), "type": "text",
        "x": x, "y": y,
        "width": len(text) * fs * 0.55, "height": fs * 1.25,
        "angle": 0, "strokeColor": color, "backgroundColor": "transparent",
        "fillStyle": "solid", "strokeWidth": 2, "strokeStyle": "solid",
        "roughness": 0, "opacity": 100,
        "groupIds": [], "frameId": rid, "index": None, "roundness": None,
        "seed": _seed(f"t{x}{y}{text[:8]}"), "version": 1,
        "versionNonce": _seed(f"tn{x}{y}{text[:8]}"),
        "isDeleted": False, "boundElements": None,
        "updated": 1, "link": None, "locked": False,
        "text": text, "fontSize": fs, "fontFamily": 1,
        "textAlign": align, "verticalAlign": "top",
        "containerId": None, "originalText": text, "lineHeight": 1.25,
    }

def _arrow(x, y, start_id, end_id, points=None, sc="#adb5bd", sw=2):
    if points is None:
        points = [[0, 0], [100, 0]]
    return {
        "id": _id("a"), "type": "arrow",
        "x": x, "y": y,
        "width": abs(points[-1][0]) or 1, "height": abs(points[-1][1]) or 1,
        "angle": 0, "strokeColor": sc, "backgroundColor": "transparent",
        "fillStyle": "solid", "strokeWidth": sw, "strokeStyle": "solid",
        "roughness": 1, "opacity": 100,
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

def _frame(name, children):
    return {
        "id": _id("f"), "type": "frame",
        "x": 0, "y": 0, "width": 100, "height": 100,
        "angle": 0, "strokeColor": "transparent", "backgroundColor": "transparent",
        "fillStyle": "solid", "strokeWidth": 0, "strokeStyle": "solid",
        "roughness": 0, "opacity": 0,
        "groupIds": [], "frameId": None, "index": None, "roundness": None,
        "seed": _seed("frame"), "version": 1,
        "versionNonce": _seed("framen"), "isDeleted": False,
        "boundElements": None, "updated": 1, "link": None, "locked": False,
        "name": "", "children": children,
    }


def _count_pipeline_stages(root: Path) -> int:
    workflows_dir = root / ".github" / "workflows"
    if not workflows_dir.exists():
        return 4
    job_names: set[str] = set()
    for wf in workflows_dir.glob("*.yml"):
        try:
            text = wf.read_text(encoding="utf-8")
            in_jobs = False
            for line in text.splitlines():
                if re.match(r"^jobs:\s*$", line):
                    in_jobs = True
                    continue
                if in_jobs and re.match(r"^  [a-zA-Z_-]+:\s*$", line):
                    job_names.add(line.strip().rstrip(":"))
                elif in_jobs and line and not line.startswith(" "):
                    in_jobs = False
        except OSError:
            continue
    return len(job_names) if job_names else 4

def _count_quality_checks(root: Path) -> int:
    qg = root / "scripts" / "quality-gates.sh"
    if not qg.exists():
        return 14
    try:
        text = qg.read_text(encoding="utf-8")
        count = sum(
            1 for line in text.splitlines()
            if line.strip() and not line.strip().startswith("#")
            and any(cmd in line for cmd in [
                "cargo", "rustfmt", "clippy", "audit", "deny",
                "nextest", "llvm-cov", "mutants", "machete",
            ])
        )
        return count if count > 0 else 14
    except OSError:
        return 14

def _discover_ecosystem_dirs(root: Path, crates, skills) -> list[tuple]:
    dirs = []
    if (root / "crates").exists():
        dirs.append(("crates/", f"{len(crates)} workspace members", C["primary"]))
    if (root / ".agents" / "skills").exists():
        dirs.append((".agents/skills/", f"{len(skills)} reusable skills", C["teal"]))
    if (root / ".github" / "workflows").exists():
        wf_count = len(list((root / ".github" / "workflows").glob("*.yml")))
        dirs.append((".github/workflows/", f"{wf_count} workflow files", C["green"]))
    if (root / "scripts").exists():
        script_count = len(list((root / "scripts").glob("*.sh")))
        dirs.append(("scripts/", f"{script_count} shell scripts", C["accent"]))
    if not dirs:
        dirs = [
            ("crates/", f"{len(crates)} crates", C["primary"]),
            (".agents/skills/", f"{len(skills)} skills", C["teal"]),
            (".github/workflows/", "CI/CD", C["green"]),
            ("scripts/", "tools", C["accent"]),
        ]
    return dirs


def generate(root: Path) -> dict:
    crates = discover_crates(root)
    skills = discover_skills(root)
    pipeline_count = _count_pipeline_stages(root)
    quality_count = _count_quality_checks(root)
    ecosystem_dirs = _discover_ecosystem_dirs(root, crates, skills)

    project_name = root.name
    try:
        cargo_toml = (root / "Cargo.toml").read_text(encoding="utf-8")
        m = re.search(r'^name\s*=\s*"([^"]+)"', cargo_toml, re.MULTILINE)
        if m:
            project_name = m.group(1)
    except OSError:
        pass

    elements = []
    PAD = 30
    GAP = 8

    # ── HERO ────────────────────────────────────────────────────────────
    hero_children = []
    hero_h = 80
    hero_bg = _rect(PAD, PAD, 1140, hero_h, bg="#edf2ff", stroke=C["primary"], sw=2)
    elements.append(hero_bg)
    hero_children.append(hero_bg["id"])

    title = _text(PAD + 20, PAD + 12, project_name.replace("-", " ").title(),
                  fs=28, color=C["primary"])
    elements.append(title)
    hero_children.append(title["id"])

    tagline_str = "Production-ready Rust workspace with AI agents, quality gates, and modern tooling"
    readme = root / "README.md"
    if readme.exists():
        try:
            first_para = [
                ln.strip() for ln in readme.read_text(encoding="utf-8").splitlines()
                if ln.strip() and not ln.startswith("#") and not ln.startswith("<!")
            ]
            if first_para:
                tagline_str = first_para[0][:100]
        except OSError:
            pass

    tagline = _text(PAD + 20, PAD + 48, tagline_str, fs=13, color=C["subtext"])
    elements.append(tagline)
    hero_children.append(tagline["id"])

    hero_frame = _frame("HEADER", hero_children)
    elements.append(hero_frame)

    y = PAD + hero_h + GAP

    # ── GETTING STARTED ─────────────────────────────────────────────────
    qs_children = []
    qs_title = _text(PAD, y, "GETTING STARTED", fs=12, color=C["text"])
    elements.append(qs_title)
    qs_children.append(qs_title["id"])
    y += 18

    bootstrap_cmd = "./scripts/bootstrap.sh" if (root / "scripts" / "bootstrap.sh").exists() else "cargo build"
    quality_cmd = "./scripts/quality-gates.sh" if list(root.glob("scripts/quality*.sh")) else "cargo clippy"

    steps = [
        ("1. Clone",    f"Use this template\nto start a new repo",          C["primary"]),
        ("2. Setup",    f"{bootstrap_cmd}\ninstalls all tools",             C["teal"]),
        ("3. Develop",  f"Write code in crates/\nwith feature flags",       C["green"]),
        ("4. Quality",  f"{quality_cmd}\nruns all checks",                  C["accent"]),
        ("5. Release",  f"Tag-triggered CI\npublishes to crates.io",        C["purple"]),
    ]

    step_w, step_h, step_gap = 210, 70, 12
    sx = PAD
    step_ids = []
    for i, (step_title, desc, color) in enumerate(steps):
        sid = f"step_{i}"
        card = _rect(sx, y, step_w, step_h, bg=C["card"], stroke=color, sw=2)
        card["id"] = sid
        elements.append(card)
        qs_children.append(sid)
        step_ids.append(sid)

        elements.append(_text(sx + 10, y + 8, step_title, fs=13, color=color))
        qs_children.append(elements[-1]["id"])
        elements.append(_text(sx + 10, y + 28, desc, fs=10, color=C["text"]))
        qs_children.append(elements[-1]["id"])

        sx += step_w + step_gap

    for i in range(len(step_ids) - 1):
        mid_x = PAD + (i + 1) * (step_w + step_gap) - step_gap // 2
        arr = _arrow(mid_x - 5, y + step_h // 2, step_ids[i], step_ids[i + 1],
                     points=[[0, 0], [step_gap, 0]], sc="#adb5bd", sw=2)
        elements.append(arr)
        qs_children.append(arr["id"])

    qs_frame = _frame("STEPS", qs_children)
    elements.append(qs_frame)
    y += step_h + GAP

    # ── BY THE NUMBERS ──────────────────────────────────────────────────
    ws_children = []
    ws_title = _text(PAD, y, "BY THE NUMBERS", fs=12, color=C["text"])
    elements.append(ws_title)
    ws_children.append(ws_title["id"])
    y += 18

    stats = [
        (str(len(crates)),    "Crates",         C["primary"]),
        (str(len(skills)),    "AI Skills",      C["teal"]),
        (str(pipeline_count), "Pipeline Jobs",  C["green"]),
        (str(quality_count),  "Quality Checks", C["accent"]),
    ]

    stat_w, stat_h, stat_gap = 270, 65, 12
    stat_x = PAD
    for num, label, color in stats:
        card = _rect(stat_x, y, stat_w, stat_h, bg=C["card"], stroke=color, sw=2)
        elements.append(card)
        ws_children.append(card["id"])

        elements.append(_text(stat_x + stat_w // 2 - len(num) * 5, y + 8, num, fs=22, color=color, align="center"))
        ws_children.append(elements[-1]["id"])
        elements.append(_text(stat_x + stat_w // 2 - len(label) * 3, y + 38, label, fs=12, color=C["text"], align="center"))
        ws_children.append(elements[-1]["id"])

        stat_x += stat_w + stat_gap

    ws_frame = _frame("STATS", ws_children)
    elements.append(ws_frame)
    y += stat_h + GAP

    # ── HOW IT CONNECTS ─────────────────────────────────────────────────
    eco_children = []
    eco_title = _text(PAD, y, "HOW IT CONNECTS", fs=12, color=C["text"])
    elements.append(eco_title)
    eco_children.append(eco_title["id"])
    y += 18

    box_w, box_h, box_spacing = 260, 55, 20
    total_w = len(ecosystem_dirs) * box_w + (len(ecosystem_dirs) - 1) * box_spacing
    bx_start = max(PAD, (1200 - total_w) // 2)

    box_ids = []
    box_positions = []
    for i, (label, desc, color) in enumerate(ecosystem_dirs):
        bx = bx_start + i * (box_w + box_spacing)
        bid = f"eco_{i}"
        card = _rect(bx, y, box_w, box_h, bg=C["card"], stroke=color, sw=2)
        card["id"] = bid
        elements.append(card)
        eco_children.append(bid)
        box_ids.append(bid)
        box_positions.append((bx, y, box_w, box_h))

        elements.append(_text(bx + 10, y + 8, label, fs=12, color=color))
        eco_children.append(elements[-1]["id"])
        elements.append(_text(bx + 10, y + 28, desc, fs=10, color=C["text"]))
        eco_children.append(elements[-1]["id"])

    for idx in range(len(box_ids) - 1):
        src_x, src_y, src_w, _ = box_positions[idx]
        tgt_x, tgt_y, _, _ = box_positions[idx + 1]
        sx_arr = src_x + src_w
        sy_arr = src_y + box_h // 2
        ex_arr = tgt_x
        ey_arr = tgt_y + box_h // 2
        arr = _arrow(sx_arr, sy_arr, box_ids[idx], box_ids[idx + 1],
                     points=[[0, 0], [ex_arr - sx_arr, ey_arr - sy_arr]], sc="#adb5bd", sw=2)
        elements.append(arr)
        eco_children.append(arr["id"])

    eco_frame = _frame("ECOSYSTEM", eco_children)
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
    parser = argparse.ArgumentParser(description="Generate compact overview diagram")
    parser.add_argument("--root", default=".")
    parser.add_argument("--out", default=".template/overview.excalidraw")
    parser.add_argument("--svg-out", default=".template/overview.svg")
    parser.add_argument("--png-out", default=None)
    parser.add_argument("--no-export", action="store_true")
    parser.add_argument("--print-stats", action="store_true")
    args = parser.parse_args()

    root = Path(args.root).resolve()

    if args.print_stats:
        crates = discover_crates(root)
        skills = discover_skills(root)
        agents = discover_agents(root)
        pipeline = _count_pipeline_stages(root)
        quality = _count_quality_checks(root)
        eco = _discover_ecosystem_dirs(root, crates, skills)
        print(json.dumps({
            "crates": len(crates), "skills": len(skills), "agents": len(agents),
            "pipeline_stages": pipeline, "quality_checks": quality,
            "ecosystem_dirs": [d[0] for d in eco],
        }, indent=2))
        return

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

            if args.png_out:
                png_out = Path(args.png_out)
                if not png_out.is_absolute():
                    png_out = root / png_out
                png_out.parent.mkdir(parents=True, exist_ok=True)
                try:
                    subprocess.run(
                        ["node", str(exporter), "-i", str(out), "-o", str(png_out), "-f", "png"],
                        capture_output=True, text=True, timeout=30, check=True, shell=False,
                    )
                    print(f"Exported: {png_out}")
                except Exception as e:
                    print(f"Warning: PNG export failed ({e})", file=sys.stderr)


if __name__ == "__main__":
    main()
