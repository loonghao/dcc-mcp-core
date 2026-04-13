"""Set up a standardized Maya project directory structure."""

from __future__ import annotations

import argparse
from pathlib import Path

from dcc_mcp_core.skill import run_main
from dcc_mcp_core.skill import skill_entry
from dcc_mcp_core.skill import skill_exception
from dcc_mcp_core.skill import skill_success


@skill_entry
def setup_project(name: str = "UntitledProject", root: str = ".") -> dict:
    """Create Maya project directory structure."""
    project_dir = Path(root) / name
    subdirs = ["scenes", "textures", "cache", "renders", "exports"]
    created = []

    try:
        for sub in subdirs:
            d = project_dir / sub
            d.mkdir(parents=True, exist_ok=True)
            created.append(str(d))
    except OSError as exc:
        return skill_exception(
            exc,
            message=f"Failed to create project directory: {project_dir}",
            prompt=("Check that the root path is writable. Use dcc_diagnostics__audit_log to see recent actions."),
        )

    return skill_success(
        f"Created project '{name}' with {len(subdirs)} directories",
        prompt=(
            f"Project '{name}' set up at {project_dir}. "
            "Next: call maya_geometry__create_sphere or open a scene file, "
            "then call maya_pipeline__export_usd when ready to export."
        ),
        project_dir=str(project_dir),
        directories=created,
    )


def main(**kwargs: object) -> dict:
    return setup_project(**kwargs)


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description="Set up a Maya project structure.")
    parser.add_argument("--name", default="UntitledProject")
    parser.add_argument("--root", default=".")
    args = parser.parse_args()
    run_main(lambda: main(name=args.name, root=args.root))
