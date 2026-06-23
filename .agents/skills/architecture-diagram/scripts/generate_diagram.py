#!/usr/bin/env python3
"""
generate_diagram.py — Architecture diagram generator

Generates Excalidraw scene JSON (default) or legacy SVG from the live project structure.
Discovers Rust workspace crates, skills, agents, and commands automatically.

Usage:
  python generate_diagram.py --root . [--format excalidraw|svg] [--out PATH]
"""
import argparse
import json
import subprocess  # nosec B404
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))

from lib.config import DEFAULT_CONFIG
from lib.discovery import (
    discover_crates, discover_skills, discover_agents, discover_commands,
)
from lib.layout import (
    has_graphviz, layout_with_graphviz, layout_grid_fallback, fix_overlaps,
)
from lib.scene_builder import build_scene
from lib.excalidraw_render import scene_to_excalidraw
from lib.svg_render import scene_to_svg


def _load_config(root: Path) -> dict:
    cfg = dict(DEFAULT_CONFIG)
    cfg_file = root / "docs" / "diagram-config.json"
    if cfg_file.exists():
        try:
            with open(cfg_file, encoding="utf-8") as f:
                cfg.update(json.load(f))
        except Exception:
            pass
    return cfg


def _classify_crates(crates: list[dict], cfg: dict) -> dict:
    layers = {"apps": [], "core": [], "templates": [], "other": []}
    for crate in crates:
        name = crate["name"]
        if name == cfg.get("project_name", "").lower().replace(" ", "-") or name == "sample-app":
            layers["apps"].append(crate)
        elif "-template" in name:
            layers["templates"].append(crate)
        elif "example-" in name or name == "hello-world-example":
            layers["other"].append(crate)
        else:
            layers["core"].append(crate)
    return layers


def _export_via_node(excalidraw_path: Path, svg_path: Path, png_path: Path | None) -> bool:
    exporter = Path(__file__).resolve().parent / "export_excalidraw.mjs"
    if not exporter.exists():
        return False

    ok = True
    try:
        subprocess.run(  # nosec B603
            ["node", str(exporter), "-i", str(excalidraw_path), "-o", str(svg_path), "-f", "svg"],
            capture_output=True, text=True, timeout=30, check=True, shell=False,
        )
        print(f"Exported: {svg_path}")
    except (FileNotFoundError, subprocess.TimeoutExpired, subprocess.CalledProcessError) as e:
        print(f"Warning: SVG export failed ({e})", file=sys.stderr)
        ok = False

    if png_path:
        try:
            subprocess.run(  # nosec B603
                ["node", str(exporter), "-i", str(excalidraw_path), "-o", str(png_path), "-f", "png"],
                capture_output=True, text=True, timeout=30, check=True, shell=False,
            )
            print(f"Exported: {png_path}")
        except (FileNotFoundError, subprocess.TimeoutExpired, subprocess.CalledProcessError) as e:
            print(f"Warning: PNG export failed ({e})", file=sys.stderr)

    return ok


def main():
    parser = argparse.ArgumentParser(description="Generate Architecture Diagram")
    parser.add_argument("--root", default=".", help="Workspace root")
    parser.add_argument("--out", default=None, help="Output path (default depends on format)")
    parser.add_argument("--format", choices=["excalidraw", "svg"], default="excalidraw",
                        help="Output format (default: excalidraw)")
    parser.add_argument("--svg-out", default=".template/architecture.svg",
                        help="SVG export path (default: .template/architecture.svg)")
    parser.add_argument("--png-out", default=None,
                        help="PNG export path (optional)")
    parser.add_argument("--no-export", action="store_true",
                        help="Skip SVG/PNG export (excalidraw only)")
    parser.add_argument("--legacy-svg", action="store_true",
                        help="Generate SVG directly via Python (no Node.js required)")
    parser.add_argument("--no-graphviz", action="store_true",
                        help="Force grid layout (skip Graphviz auto-layout)")
    args = parser.parse_args()

    root = Path(args.root).resolve()
    cfg = _load_config(root)

    crates = discover_crates(root)
    skills, agents, commands = discover_skills(root), discover_agents(root), discover_commands(root)
    labels = {"crates": len(crates), "skills": len(skills), "agents": len(agents), "commands": len(commands)}

    d_crates = crates or [{"name": "(no workspace)", "version": "0.0.0", "dependencies": [], "features": [], "description": ""}]
    layers = _classify_crates(d_crates, cfg)

    use_graphviz = not args.no_graphviz and has_graphviz()
    crate_coords = {}
    if use_graphviz:
        crate_coords = layout_with_graphviz(d_crates, layers, 200, 1200)
    if not crate_coords:
        crate_coords = layout_grid_fallback(d_crates, layers, 200, 1200)
        use_graphviz = False

    overlap_fixes = fix_overlaps(crate_coords)
    if overlap_fixes:
        print(f"Fixed {overlap_fixes} overlapping elements", file=sys.stderr)

    scene = build_scene(cfg, d_crates, skills, agents, commands, labels, root, crate_coords, layers, no_graphviz=args.no_graphviz)

    if args.legacy_svg:
        out = Path(args.out) if args.out else Path(args.svg_out)
        if not out.is_absolute():
            out = root / out
        out.parent.mkdir(parents=True, exist_ok=True)
        svg = scene_to_svg(scene, crate_coords, layers, no_graphviz=args.no_graphviz)
        out.write_text(svg, encoding="utf-8")
        print(f"Written: {out}")
        return

    # Default: Excalidraw + export
    excalidraw_out = Path(args.out) if args.out else root / ".template" / "architecture.excalidraw"
    if not excalidraw_out.is_absolute():
        excalidraw_out = root / excalidraw_out
    excalidraw_out.parent.mkdir(parents=True, exist_ok=True)

    excalidraw_json = scene_to_excalidraw(scene)
    excalidraw_out.write_text(json.dumps(excalidraw_json, indent=2), encoding="utf-8")
    print(f"Written: {excalidraw_out}")

    if not args.no_export:
        svg_out = Path(args.svg_out)
        if not svg_out.is_absolute():
            svg_out = root / svg_out
        svg_out.parent.mkdir(parents=True, exist_ok=True)

        png_out = None
        if args.png_out:
            png_out = Path(args.png_out)
            if not png_out.is_absolute():
                png_out = root / png_out
            png_out.parent.mkdir(parents=True, exist_ok=True)

        if not _export_via_node(excalidraw_out, svg_out, png_out):
            print("Falling back to Python SVG renderer", file=sys.stderr)
            svg = scene_to_svg(scene, crate_coords, layers, no_graphviz=args.no_graphviz)
            svg_out.write_text(svg, encoding="utf-8")
            print(f"Written: {svg_out}")


if __name__ == "__main__":
    main()
