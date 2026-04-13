"""Export scene to USD format (simulation — actual Maya API not required).

Demonstrates a script that depends on another skill (usd-tools) for validation.
"""

from __future__ import annotations

import argparse
from pathlib import Path

from dcc_mcp_core.skill import run_main
from dcc_mcp_core.skill import skill_entry
from dcc_mcp_core.skill import skill_exception
from dcc_mcp_core.skill import skill_success


@skill_entry
def export_usd(input_file: str = "", output_file: str = "", validate: bool = False) -> dict:
    """Simulate USD export and optionally validate."""
    if not input_file:
        from dcc_mcp_core.skill import skill_error

        return skill_error(
            "Missing input_file parameter",
            "input_file is required",
            prompt=("Provide the input Maya file path. Use dcc_diagnostics__audit_log to review recent scene actions."),
        )

    if not output_file:
        output_file = str(Path(input_file).with_suffix(".usda"))

    try:
        usda_content = f"""\
#usda 1.0
(
    defaultPrim = "World"
    doc = "Exported from {input_file}"
)

def Xform "World"
{{
    def Mesh "ExportedGeometry"
    {{
    }}
}}
"""
        Path(output_file).write_text(usda_content, encoding="utf-8")
    except OSError as exc:
        return skill_exception(
            exc,
            message=f"Failed to write USD file: {output_file}",
            prompt=(
                "Check that the output directory exists and is writable. "
                "Use dcc_diagnostics__screenshot to capture the error state, "
                "or dcc_diagnostics__audit_log to see recent action history."
            ),
        )

    validation_note = ""
    if validate:
        validation_note = "In production, this calls usd-tools/scripts/validate.py"

    return skill_success(
        f"Exported {input_file} -> {output_file}",
        prompt=(
            f"USD export complete: {output_file}. "
            "Next: call usd_tools__inspect to view the USD stage, "
            "or usd_tools__validate to check schema compliance. "
            "Use dcc_diagnostics__screenshot to verify the scene before export if needed."
        ),
        input=input_file,
        output=output_file,
        format="usda",
        validated=validate,
        validation_note=validation_note,
    )


def main(**kwargs: object) -> dict:
    return export_usd(**kwargs)


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description="Export scene to USD format.")
    parser.add_argument("--input", required=True, dest="input_file")
    parser.add_argument("--output", default=None, dest="output_file")
    parser.add_argument("--validate", action="store_true")
    args = parser.parse_args()
    run_main(
        lambda: main(
            input_file=args.input_file,
            output_file=args.output_file or "",
            validate=args.validate,
        )
    )
