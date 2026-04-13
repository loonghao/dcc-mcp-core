"""Hello World skill script — prints a greeting message.

Reads JSON parameters from stdin (preferred) or falls back to sys.argv
for backwards-compatible invocation.
"""

from __future__ import annotations

import json
import sys


def main() -> None:
    """Entry point for the greet action."""
    # Primary: read JSON params from stdin (dcc-mcp-core execute_script protocol)
    try:
        raw = sys.stdin.read()
        params = json.loads(raw) if raw.strip() else {}
    except Exception:
        params = {}

    name = params.get("name", "World")

    result = {
        "success": True,
        "message": f"Hello, {name}!",
    }
    print(json.dumps(result))


if __name__ == "__main__":
    main()
