"""Scene model — renderer-agnostic intermediate representation of the architecture diagram."""

from dataclasses import dataclass, field
from typing import Any


@dataclass
class SceneNode:
    id: str
    kind: str  # "crate" | "pipeline_stage" | "skill" | "agent" | "error_type" | "data_type" | "command" | "section_label" | "strategy"
    x: float
    y: float
    w: float
    h: float
    label: str
    subtitle: str | None = None
    color_key: str = "core"
    metadata: dict[str, Any] = field(default_factory=dict)


@dataclass
class SceneEdge:
    id: str
    source_id: str
    target_id: str
    label: str | None = None
    style: str = "dependency"  # "dependency" | "pipeline"


@dataclass
class SceneSection:
    id: str
    title: str
    nodes: list[SceneNode] = field(default_factory=list)
    edges: list[SceneEdge] = field(default_factory=list)


@dataclass
class SceneDocument:
    title: str
    project_name: str
    author: str
    sections: list[SceneSection] = field(default_factory=list)
    all_nodes: list[SceneNode] = field(default_factory=list)
    all_edges: list[SceneEdge] = field(default_factory=list)
    width: float = 1200
    height: float = 3000
    labels: dict[str, int] = field(default_factory=dict)
