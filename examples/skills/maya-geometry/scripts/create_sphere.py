"""Create a polygon sphere in Maya.

This script demonstrates a typical DCC action that creates geometry.
In a real Maya environment, it would use `maya.cmds.polySphere`.
"""

from __future__ import annotations

import argparse

from dcc_mcp_core.skill import run_main
from dcc_mcp_core.skill import skill_entry
from dcc_mcp_core.skill import skill_success


@skill_entry
def create_sphere(radius: float = 1.0, subdivisions: int = 20, name: str = "pSphere1") -> dict:
    """Create a polygon sphere with configurable parameters."""
    # In a real Maya environment: maya.cmds.polySphere(r=radius, sx=subdivisions, sy=subdivisions, n=name)
    return skill_success(
        f"Created sphere '{name}' with radius={radius}, subdivisions={subdivisions}",
        prompt=(
            f"Sphere '{name}' created. "
            "Next: call maya_geometry__bevel_edges to add edge detail, "
            "or maya_pipeline__export_usd to export the scene. "
            "Use dcc_diagnostics__screenshot to visually verify the result."
        ),
        object_name=name,
        radius=radius,
        subdivisions=subdivisions,
    )


def main(**kwargs: object) -> dict:
    return create_sphere(**kwargs)


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description="Create a polygon sphere.")
    parser.add_argument("--radius", type=float, default=1.0)
    parser.add_argument("--subdivisions", type=int, default=20)
    parser.add_argument("--name", default="pSphere1")
    args = parser.parse_args()
    run_main(lambda: main(radius=args.radius, subdivisions=args.subdivisions, name=args.name))
