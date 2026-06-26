#!/usr/bin/env python3
"""Generate a human-friendly overview diagram in Excalidraw format.

Dynamic generation — every stat and section is discovered at runtime:
  - Workspace crates  → Cargo.toml workspace members
  - AI skills         → .agents/skills/ directories
  - AI agents         → .agents/ directories
  - Pipeline stages   → .github/workflows/ YAML job counts
  - Quality checks    → scripts/quality-gates.sh command count
  - Ecosystem dirs    → discovered from filesystem layout

Outputs: .template/overview.excalidraw  +  .template/overview.svg
"""

import json
import re
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))

from lib.discovery import discover_crates, discover_skills, discover_agents, discover_commands

# ── Palette (friendly, non-technical) ──────────────────────────────────
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


def _rect(x, y, w, h, bg=None, stroke="#dee2e6", sw=1, label=None,
          lfs=16, lac="center", lvc="middle", lc=None, rid=None,
          rough=0, opacity=100):
    bg = bg or C["card"]
    lc = lc or C["text"]
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
        el["label"] = {"text": label, "fontSize": lfs, "fontFamily": 3,
                        "textAlign": lac, "verticalAlign": lvc,
                        "strokeColor": lc}
    return el


def _text(x, y, text, fs=16, color=None, align="left", rid=None):
    color = color or C["text"]
    return {
        "id": _id("t"), "type": "text",
        "x": x, "y": y,
        "width": len(text) * fs * 0.55,
        "height": fs * 1.25,
        "angle": 0, "strokeColor": color, "backgroundColor": "transparent",
        "fillStyle": "solid", "strokeWidth": 2, "strokeStyle": "solid",
        "roughness": 0, "opacity": 100,
        "groupIds": [], "frameId": rid, "index": None, "roundness": None,
        "seed": _seed(f"t{x}{y}{text[:8]}"), "version": 1,
        "versionNonce": _seed(f"tn{x}{y}{text[:8]}"),
        "isDeleted": False, "boundElements": None,
        "updated": 1, "link": None, "locked": False,
        "text": text, "fontSize": fs, "fontFamily": 3,
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
        "startArrowhead": None, "endArrowhead": "arrow", "elbowed": True,
    }
    if label:
        el["label"] = {"text": label, "fontSize": 13, "fontFamily": 3}
    return el


def _frame(name, children):
    """Invisible logical grouping frame."""
    return {
        "id": _id("f"), "type": "frame",
        "x": 0, "y": 0, "width": 100, "height": 100,
        "angle": 0, "strokeColor": "transparent", "backgroundColor": "transparent",
        "fillStyle": "solid", "strokeWidth": 0, "strokeStyle": "solid",
        "roughness": 0, "opacity": 0,
        "groupIds": [], "frameId": None, "index": None, "roundness": None,
        "seed": _seed(name), "version": 1,
        "versionNonce": _seed(name + "_n"), "isDeleted": False,
        "boundElements": None, "updated": 1, "link": None, "locked": False,
        "name": name, "children": children,
    }


# ── Dynamic discovery helpers ───────────────────────────────────────────

def _count_pipeline_stages(root: Path) -> int:
    """Count distinct CI/CD jobs across all workflow YAML files."""
    workflows_dir = root / ".github" / "workflows"
    if not workflows_dir.exists():
        return 4  # sensible fallback
    job_names: set[str] = set()
    for wf in workflows_dir.glob("*.yml"):
        try:
            text = wf.read_text(encoding="utf-8")
            # Simple heuristic: lines with `  <name>:` under a `jobs:` block
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
    """Count distinct quality checks in scripts/quality-gates.sh."""
    qg = root / "scripts" / "quality-gates.sh"
    if not qg.exists():
        # Try alternate locations
        for candidate in root.glob("scripts/quality*.sh"):
            qg = candidate
            break
    if not qg.exists():
        return 14  # sensible fallback
    try:
        text = qg.read_text(encoding="utf-8")
        # Count non-blank, non-comment lines that look like commands
        count = sum(
            1 for line in text.splitlines()
            if line.strip() and not line.strip().startswith("#")
            and any(cmd in line for cmd in [
                "cargo", "rustfmt", "clippy", "audit", "deny",
                "nextest", "llvm-cov", "mutants", "cargo-semver",
                "udeps", "machete", "pants", "typos",
            ])
        )
        return count if count > 0 else 14
    except OSError:
        return 14


def _discover_feature_pills(root: Path) -> list[str]:
    """Dynamically determine key feature pills from workspace config."""
    pills = []
    # Check for MCP server template
    if (root / "crates" / "mcp-server-template").exists():
        pills.append("MCP Server")
    # Check for actor runtime
    if (root / "crates" / "actor-runtime-template").exists():
        pills.append("Actor Runtime")
    # Check for hybrid storage
    if (root / "crates" / "hybrid-storage-template").exists():
        pills.append("Hybrid Storage")
    # Check for AI skills
    if (root / ".agents" / "skills").exists():
        pills.append("AI-Native Skills")
    # Check for CI workflows
    if (root / ".github" / "workflows").exists():
        pills.append("CI/CD Pipeline")
    # Check for quality gates script
    if list(root.glob("scripts/quality*.sh")):
        pills.append("Quality Gates")
    # Check for security audit config
    if (root / "deny.toml").exists() or (root / ".cargo" / "audit.toml").exists():
        pills.append("Security Audits")
    # Always include mutation testing if cargo-mutants config exists
    if (root / ".cargo" / "mutants.toml").exists() or list(root.glob(".mutants*")):
        pills.append("Mutation Testing")
    # Fallback defaults if nothing was discovered
    if not pills:
        pills = ["AI-Native Skills", "CI/CD Pipeline", "Quality Gates",
                 "Mutation Testing", "Security Audits", "MCP Server"]
    return pills


def _discover_ecosystem_dirs(root: Path, crates, skills) -> list[tuple]:
    """Discover ecosystem directories dynamically."""
    dirs = []
    # crates/
    if (root / "crates").exists():
        dirs.append((
            "crates/",
            f"Your code lives here.\n{len(crates)} workspace members\nwith feature flags.",
            C["primary"],
        ))
    # .agents/skills/
    if (root / ".agents" / "skills").exists():
        dirs.append((
            ".agents/skills/",
            f"AI procedures that\nguide coding agents.\n{len(skills)} reusable skills.",
            C["teal"],
        ))
    # .github/workflows/
    if (root / ".github" / "workflows").exists():
        wf_count = len(list((root / ".github" / "workflows").glob("*.yml")))
        dirs.append((
            ".github/workflows/",
            f"Automated CI/CD.\n{wf_count} workflow file(s).\nSecurity + quality gates.",
            C["green"],
        ))
    # scripts/
    if (root / "scripts").exists():
        script_count = len(list((root / "scripts").glob("*.sh")))
        dirs.append((
            "scripts/",
            f"Local dev tools.\n{script_count} shell scripts.\nQuality, bootstrap, release.",
            C["accent"],
        ))
    # Fallback
    if not dirs:
        dirs = [
            ("crates/", f"Your code lives here.\n{len(crates)} workspace members.", C["primary"]),
            (".agents/skills/", f"{len(skills)} AI skills.", C["teal"]),
            (".github/workflows/", "Automated CI/CD.", C["green"]),
            ("scripts/", "Local dev tools.", C["accent"]),
        ]
    return dirs


# ── Main diagram generator ──────────────────────────────────────────────

def generate(root: Path) -> dict:
    # ── Discover everything dynamically ────────────────────────────────
    crates   = discover_crates(root)
    skills   = discover_skills(root)
    agents   = discover_agents(root)
    commands = discover_commands(root)

    pipeline_stage_count = _count_pipeline_stages(root)
    quality_check_count  = _count_quality_checks(root)
    feature_pills        = _discover_feature_pills(root)
    ecosystem_dirs       = _discover_ecosystem_dirs(root, crates, skills)

    # Project name from root Cargo.toml [package] or directory name
    project_name = root.name
    try:
        cargo_toml = (root / "Cargo.toml").read_text(encoding="utf-8")
        m = re.search(r'^name\s*=\s*"([^"]+)"', cargo_toml, re.MULTILINE)
        if m:
            project_name = m.group(1)
    except OSError:
        pass

    elements = []

    # ════════════════════════════════════════════════════════════════════
    # SECTION 1: HERO — What is this?
    # ════════════════════════════════════════════════════════════════════
    hero_children = []

    hero_bg = _rect(30, 20, 1140, 190, bg="#edf2ff", stroke=C["primary"], sw=2)
    elements.append(hero_bg)
    hero_children.append(hero_bg["id"])

    title_text = project_name.replace("-", " ").title()
    title = _text(80, 50, title_text, fs=36, color=C["primary"])
    elements.append(title)
    hero_children.append(title["id"])

    # Tagline from README if available, else generic
    tagline_str = "Production-ready Rust workspace with AI agents, quality gates, and modern tooling"
    readme = root / "README.md"
    if readme.exists():
        try:
            first_para = [
                ln.strip() for ln in readme.read_text(encoding="utf-8").splitlines()
                if ln.strip() and not ln.startswith("#")
            ]
            if first_para:
                tagline_str = first_para[0][:100]
        except OSError:
            pass

    tagline = _text(80, 105, tagline_str, fs=18, color=C["subtext"])
    elements.append(tagline)
    hero_children.append(tagline["id"])

    # Dynamic feature pills
    pill_x = 80
    for feat in feature_pills[:8]:  # cap at 8 pills
        pw = len(feat) * 9 + 24
        pill = _rect(pill_x, 150, pw, 30, bg=C["primary"], stroke=C["primary"], sw=0)
        elements.append(pill)
        hero_children.append(pill["id"])
        pt = _text(pill_x + pw // 2 - len(feat) * 3, 155, feat, fs=12, color="#ffffff")
        pt["textAlign"] = "center"
        elements.append(pt)
        hero_children.append(pt["id"])
        pill_x += pw + 12

    hero_frame = _frame("WHAT IS THIS?", hero_children)
    elements.append(hero_frame)

    # ════════════════════════════════════════════════════════════════════
    # SECTION 2: QUICK START — How to get started
    # ════════════════════════════════════════════════════════════════════
    qs_y = 255
    qs_children = []

    qs_title = _text(50, qs_y, "GETTING STARTED", fs=14, color=C["text"])
    elements.append(qs_title)
    qs_children.append(qs_title["id"])

    # Check which bootstrap scripts actually exist
    bootstrap_cmd = "./scripts/bootstrap.sh" if (root / "scripts" / "bootstrap.sh").exists() else "cargo build"
    quality_cmd   = "./scripts/quality-gates.sh" if list(root.glob("scripts/quality*.sh")) else "cargo clippy"

    steps = [
        ("1. Clone",    "Use this template\nto start a new repo",           C["primary"]),
        ("2. Setup",    f"{bootstrap_cmd}\ninstalls all tools",              C["teal"]),
        ("3. Develop",  "Write code in crates/\nwith feature flags",         C["green"]),
        ("4. Quality",  f"{quality_cmd}\nruns all checks",                   C["accent"]),
        ("5. Release",  "Tag-triggered CI\npublishes to crates.io",          C["purple"]),
    ]

    step_w, step_h, gap = 200, 110, 25
    sx = 50
    step_ids = []
    for i, (step_title, desc, color) in enumerate(steps):
        sid = f"step_{i}"
        card = _rect(sx, qs_y + 30, step_w, step_h, bg=C["card"], stroke=color, sw=2)
        card["id"] = sid
        elements.append(card)
        qs_children.append(sid)
        step_ids.append(sid)

        st = _text(sx + 15, qs_y + 45, step_title, fs=16, color=color)
        elements.append(st)
        qs_children.append(st["id"])

        sd = _text(sx + 15, qs_y + 75, desc, fs=12, color=C["text"])
        elements.append(sd)
        qs_children.append(sd["id"])

        sx += step_w + gap

    for i in range(len(step_ids) - 1):
        mid_x = 50 + (i + 1) * (step_w + gap) - gap // 2
        arr = _arrow(mid_x - 10, qs_y + 80, step_ids[i], step_ids[i + 1],
                     points=[[0, 0], [gap, 0]], sc="#adb5bd", sw=2)
        elements.append(arr)
        qs_children.append(arr["id"])

    qs_frame = _frame("HOW TO START", qs_children)
    elements.append(qs_frame)

    # ════════════════════════════════════════════════════════════════════
    # SECTION 3: BY THE NUMBERS — Dynamic stats cards
    # ════════════════════════════════════════════════════════════════════
    ws_y = 435
    ws_children = []

    ws_title = _text(50, ws_y, "BY THE NUMBERS", fs=14, color=C["text"])
    elements.append(ws_title)
    ws_children.append(ws_title["id"])

    stats: list[tuple[str, str, str, str]] = [
        (str(len(crates)),               "Workspace Crates",   "Libraries, apps,\nand templates",               C["primary"]),
        (str(len(skills)),               "AI Skills",          "Reusable procedures\nfor code agents",          C["teal"]),
        (str(pipeline_stage_count),      "Pipeline Jobs",      "CI/CD jobs discovered\nin .github/workflows/",  C["green"]),
        (str(quality_check_count),       "Quality Checks",     "Commands in\nquality-gates.sh",                 C["accent"]),
    ]
    # Add agents row if any agents found
    if agents:
        stats.append((str(len(agents)), "AI Agents",  "Orchestrated agents\nin .agents/",  C["purple"]))

    stat_w, stat_h, stat_gap = 240, 130, 20
    stat_x = 50
    for num, label, desc, color in stats:
        card = _rect(stat_x, ws_y + 30, stat_w, stat_h, bg=C["card"], stroke=color, sw=2)
        elements.append(card)
        ws_children.append(card["id"])

        n = _text(stat_x + 20, ws_y + 45, num, fs=36, color=color)
        elements.append(n)
        ws_children.append(n["id"])

        l = _text(stat_x + 20, ws_y + 93, label, fs=14, color=C["text"])
        elements.append(l)
        ws_children.append(l["id"])

        d = _text(stat_x + 20, ws_y + 113, desc, fs=11, color=C["subtext"])
        elements.append(d)
        ws_children.append(d["id"])

        stat_x += stat_w + stat_gap

    ws_frame = _frame("BY THE NUMBERS", ws_children)
    elements.append(ws_frame)

    # ════════════════════════════════════════════════════════════════════
    # SECTION 4: ECOSYSTEM MAP — How it connects
    # ════════════════════════════════════════════════════════════════════
    eco_y = 630
    eco_children = []

    eco_title = _text(50, eco_y, "HOW IT CONNECTS", fs=14, color=C["text"])
    elements.append(eco_title)
    eco_children.append(eco_title["id"])

    box_w, box_h = 260, 125
    spacing = 20
    total_w  = len(ecosystem_dirs) * box_w + (len(ecosystem_dirs) - 1) * spacing
    start_x  = max(50, (1200 - total_w) // 2)

    box_ids: list[str] = []
    box_positions: list[tuple[int, int, int, int]] = []

    for i, (label, desc, color) in enumerate(ecosystem_dirs):
        bx = start_x + i * (box_w + spacing)
        bid = f"eco_{i}"
        card = _rect(bx, eco_y + 30, box_w, box_h, bg=C["card"], stroke=color, sw=2)
        card["id"] = bid
        elements.append(card)
        eco_children.append(bid)
        box_ids.append(bid)
        box_positions.append((bx, eco_y + 30, box_w, box_h))

        l = _text(bx + 15, eco_y + 45, label, fs=14, color=color)
        elements.append(l)
        eco_children.append(l["id"])

        d = _text(bx + 15, eco_y + 72, desc, fs=11, color=C["text"])
        elements.append(d)
        eco_children.append(d["id"])

    # Sequential left-to-right arrows between adjacent boxes
    connection_labels = ["skills use", "CI triggers", "scripts run", "config feeds", "→"]
    for idx in range(len(box_ids) - 1):
        src_x, src_y, src_w, src_h = box_positions[idx]
        tgt_x, tgt_y, _tw, tgt_h  = box_positions[idx + 1]
        start_x_arrow = src_x + src_w
        start_y_arrow = src_y + src_h / 2
        end_x_arrow   = tgt_x
        end_y_arrow   = tgt_y + tgt_h / 2
        lbl = connection_labels[idx] if idx < len(connection_labels) else "→"
        arr = _arrow(
            start_x_arrow, start_y_arrow,
            box_ids[idx], box_ids[idx + 1],
            points=[[0, 0], [end_x_arrow - start_x_arrow, end_y_arrow - start_y_arrow]],
            sc="#adb5bd", sw=2, label=lbl,
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
    parser = argparse.ArgumentParser(
        description="Generate a dynamic human-friendly overview diagram"
    )
    parser.add_argument("--root",    default=".",
                        help="Workspace root (default: current directory)")
    parser.add_argument("--out",     default=".template/overview.excalidraw",
                        help="Output .excalidraw file")
    parser.add_argument("--svg-out", default=".template/overview.svg",
                        help="Output SVG file")
    parser.add_argument("--png-out", default=None,
                        help="Output PNG file (optional)")
    parser.add_argument("--no-export", action="store_true",
                        help="Skip SVG/PNG export step")
    parser.add_argument("--print-stats", action="store_true",
                        help="Print discovered stats to stdout and exit")
    args = parser.parse_args()

    root = Path(args.root).resolve()

    if args.print_stats:
        crates   = discover_crates(root)
        skills   = discover_skills(root)
        agents   = discover_agents(root)
        pipeline = _count_pipeline_stages(root)
        quality  = _count_quality_checks(root)
        pills    = _discover_feature_pills(root)
        eco      = _discover_ecosystem_dirs(root, crates, skills)
        print(json.dumps({
            "crates":           len(crates),
            "skills":           len(skills),
            "agents":           len(agents),
            "pipeline_stages":  pipeline,
            "quality_checks":   quality,
            "feature_pills":    pills,
            "ecosystem_dirs":   [d[0] for d in eco],
        }, indent=2))
        return

    out = Path(args.out)
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
