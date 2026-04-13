"""Batch rename objects with a prefix/suffix pattern.

Demonstrates a multi-parameter action script.
"""

from __future__ import annotations

import argparse

from dcc_mcp_core.skill import run_main
from dcc_mcp_core.skill import skill_entry
from dcc_mcp_core.skill import skill_error
from dcc_mcp_core.skill import skill_success


@skill_entry
def batch_rename(prefix: str = "", suffix: str = "", objects: str = "") -> dict:
    """Batch rename objects with prefix/suffix pattern."""
    object_list = [o.strip() for o in objects.split(",") if o.strip()] if objects else []
    if not object_list:
        return skill_error(
            "No objects specified",
            "objects parameter is empty",
            prompt=(
                "Provide a comma-separated list of object names via the 'objects' parameter. "
                "Use dcc_diagnostics__audit_log to check recent actions if the scene state is unclear."
            ),
        )

    renamed = [f"{prefix}{obj}{suffix}" for obj in object_list]
    return skill_success(
        f"Renamed {len(renamed)} objects",
        prompt=(
            f"Renamed {len(renamed)} objects. "
            "Consider running maya_pipeline__export_usd to export the updated scene, "
            "or dcc_diagnostics__screenshot to visually verify the naming in the viewport."
        ),
        renamed=renamed,
        count=len(renamed),
    )


def main(**kwargs: object) -> dict:
    return batch_rename(**kwargs)


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description="Batch rename objects.")
    parser.add_argument("--prefix", default="")
    parser.add_argument("--suffix", default="")
    parser.add_argument("--objects", default="", help="Comma-separated list of objects")
    args = parser.parse_args()
    run_main(lambda: main(prefix=args.prefix, suffix=args.suffix, objects=args.objects))
