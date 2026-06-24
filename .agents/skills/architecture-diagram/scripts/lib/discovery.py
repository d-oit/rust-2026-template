"""Repository discovery functions — scan live project structure for architecture data."""

import json
import re
import subprocess  # nosec B404
from pathlib import Path


def _read_frontmatter_name(path: Path) -> str:
    try:
        content = path.read_text(encoding="utf-8")
        if content.startswith("---"):
            m = re.search(r"^name:\s*(.+)$", content, re.MULTILINE)
            if m:
                return m.group(1).strip().strip('"').strip("'")
    except Exception:
        pass
    return path.stem


def discover_skills(root: Path) -> list[str]:
    skills_dir = root / ".agents" / "skills"
    if not skills_dir.is_dir():
        return []
    return [
        _read_frontmatter_name(d / "SKILL.md")
        for d in sorted(skills_dir.iterdir())
        if d.is_dir() and (d / "SKILL.md").exists()
    ]


def discover_agents(root: Path) -> list[str]:
    agents_dir = root / ".opencode" / "agents"
    if not agents_dir.is_dir():
        return []
    return sorted(p.stem for p in agents_dir.glob("*.md"))


def discover_commands(root: Path) -> list[str]:
    commands_dir = root / ".opencode" / "commands"
    if not commands_dir.is_dir():
        return []
    return sorted(
        (p.stem if p.stem.startswith("/") else "/" + p.stem)
        for p in commands_dir.glob("*.md")
    )


def discover_crates(root: Path) -> list[dict]:
    try:
        result = subprocess.run(  # nosec B603
            ["cargo", "metadata", "--format-version", "1", "--no-deps"],
            capture_output=True, text=True, check=True,
            cwd=str(root.resolve()), shell=False,
        )
        data = json.loads(result.stdout)
        crates = []
        workspace_members = data.get("workspace_members", [])
        for pkg in data.get("packages", []):
            if pkg["id"] in workspace_members:
                deps = [d["name"] for d in pkg.get("dependencies", []) if d.get("path")]
                features = [f for f in pkg.get("features", {}).keys() if f != "default"]
                crates.append({
                    "name": pkg["name"],
                    "version": pkg["version"],
                    "dependencies": deps,
                    "features": sorted(features),
                    "description": pkg.get("description", ""),
                })
        return sorted(crates, key=lambda x: x["name"])
    except (subprocess.CalledProcessError, json.JSONDecodeError, KeyError):
        return []


def discover_error_types(root: Path) -> list[dict]:
    error_types = []
    crates_dir = root / "crates"
    if not crates_dir.is_dir():
        return error_types
    for crate_dir in crates_dir.iterdir():
        if not crate_dir.is_dir():
            continue
        for rs_file in crate_dir.rglob("*.rs"):
            try:
                content = rs_file.read_text(encoding="utf-8")
                for match in re.finditer(r'pub enum (\w*Error)\s*\{([^}]+)\}', content, re.DOTALL):
                    enum_name = match.group(1)
                    variants_text = match.group(2)
                    variants = []
                    for line in variants_text.split('\n'):
                        line = line.strip()
                        vm = re.match(r'(\w+)(?:\(|\s|$)', line)
                        if vm:
                            v = vm.group(1)
                            if v[0].isupper() and v not in ('Debug', 'Error', 'Display', 'From', 'Source'):
                                variants.append(v)
                    variants = list(dict.fromkeys(variants))
                    if variants and enum_name not in [e["name"] for e in error_types]:
                        error_types.append({
                            "name": enum_name,
                            "variants": variants[:6],
                            "crate": crate_dir.name,
                        })
            except Exception:
                continue
    return error_types[:5]


def discover_agent_roles(root: Path) -> list[dict]:
    orch_file = root / ".agents" / "ORCHESTRATION.md"
    if not orch_file.exists():
        return []
    try:
        content = orch_file.read_text(encoding="utf-8")
        roles = []
        for match in re.finditer(r'\|\s*\*\*(\w+-\w+)\*\*\s*\|[^|]*\|\s*`([^`]+)`', content):
            role_name = match.group(1)
            skills_text = match.group(2)
            skills = [s.strip() for s in skills_text.split(',')]
            colors = {"code": "teal", "release": "green", "quality": "blue", "meta": "templates"}
            color_key = colors.get(role_name.split("-")[0], "interface")
            roles.append({
                "name": role_name,
                "skills": skills,
                "color": color_key,
            })
        return roles
    except Exception:
        return []


def discover_handoff_items(root: Path) -> list[dict]:
    orch_file = root / ".agents" / "ORCHESTRATION.md"
    if not orch_file.exists():
        return []
    try:
        content = orch_file.read_text(encoding="utf-8")
        items = []
        seen = set()
        for match in re.finditer(r'`([^`]+\.(?:json|jsonl|md))`', content):
            file_path = match.group(1)
            if file_path not in seen:
                seen.add(file_path)
                if file_path.endswith(".jsonl"):
                    desc = "Metrics & analytics"
                elif "metrics" in file_path:
                    desc = "Aggregated metrics"
                elif "state" in file_path:
                    desc = "Workflow state"
                elif file_path.endswith(".md"):
                    desc = "Documentation"
                else:
                    desc = "Configuration"
                items.append({"path": file_path, "desc": desc})
        return items[:3]
    except Exception:
        return []


def discover_data_types(root: Path) -> list[dict]:
    data_types = []
    crates_dir = root / "crates"
    if not crates_dir.is_dir():
        return data_types
    for crate_dir in crates_dir.iterdir():
        if not crate_dir.is_dir():
            continue
        for rs_file in crate_dir.rglob("*.rs"):
            try:
                content = rs_file.read_text(encoding="utf-8")
                for match in re.finditer(r'/// ([^\n]+)\npub struct (\w+)', content):
                    desc = match.group(1)
                    struct_name = match.group(2)
                    data_types.append({
                        "name": struct_name,
                        "desc": desc[:50],
                        "crate": crate_dir.name,
                    })
            except Exception:
                continue
    return data_types[:4]
