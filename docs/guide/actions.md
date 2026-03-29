# Actions & Registry

The **ActionRegistry** is the central component for managing action metadata in DCC-MCP-Core. It provides thread-safe registration, lookup, and listing of actions.

## ActionRegistry

The `ActionRegistry` is implemented in Rust using `DashMap` for lock-free concurrent reads. Unlike a singleton, each registry instance is independent — eliminating cross-DCC pollution.

```python
from dcc_mcp_core import ActionRegistry

# Create a new registry
registry = ActionRegistry()

# Register an action
registry.register(
    name="create_sphere",
    description="Creates a sphere in the scene",
    category="geometry",
    tags=["geometry", "creation"],
    dcc="maya",
    version="1.0.0",
    input_schema='{"type": "object", "properties": {"radius": {"type": "number"}}}',
    output_schema='{"type": "object", "properties": {"name": {"type": "string"}}}',
    source_file="/path/to/action.py",
)
```

## Registration Parameters

| Parameter | Type | Default | Description |
|-----------|------|---------|-------------|
| `name` | `str` | — | Action name (used for lookup) |
| `description` | `str` | `""` | What this action does |
| `category` | `str` | `""` | Organization category |
| `tags` | `List[str]` | `[]` | Classification tags |
| `dcc` | `str` | `"python"` | Target DCC application |
| `version` | `str` | `"1.0.0"` | Action version |
| `input_schema` | `Optional[str]` | `None` | JSON Schema string for input |
| `output_schema` | `Optional[str]` | `None` | JSON Schema string for output |
| `source_file` | `Optional[str]` | `None` | Source file path |

## Querying Actions

```python
# Get action metadata (returns dict or None)
meta = registry.get_action("create_sphere")
meta = registry.get_action("create_sphere", dcc_name="maya")

# List all action names for a DCC
names = registry.list_actions_for_dcc("maya")  # ["create_sphere", ...]

# List all actions with full metadata
all_actions = registry.list_actions()               # All actions
maya_actions = registry.list_actions(dcc_name="maya")  # Maya-specific

# Get all registered DCC names
dccs = registry.get_all_dccs()  # ["maya", "blender", ...]

# Registry info
print(len(registry))  # Number of registered actions
```

## Action Metadata Dict

When retrieved via `get_action()` or `list_actions()`, each action returns a dict with:

```python
{
    "name": "create_sphere",
    "internal_name": "create_sphere",
    "description": "Creates a sphere in the scene",
    "category": "geometry",
    "tags": ["geometry", "creation"],
    "dcc": "maya",
    "version": "1.0.0",
    "input_schema": '{"type": "object", ...}',   # JSON string
    "output_schema": '{"type": "object", ...}',   # JSON string
    "source_file": "/path/to/action.py",
}
```

## ActionResultModel

All action executions should return an `ActionResultModel`:

```python
from dcc_mcp_core import ActionResultModel, success_result, error_result

# Direct construction
result = ActionResultModel(
    success=True,
    message="Created sphere",
    prompt="You can now modify the sphere",
    context={"object_name": "sphere1"},
)

# Factory functions (recommended)
result = success_result("Created sphere", prompt="Modify next", object_name="sphere1")
error = error_result("Failed", "File not found", prompt="Check path")

# Access fields
print(result.success)    # True
print(result.message)    # "Created sphere"
print(result.prompt)     # "You can now modify the sphere"
print(result.context)    # {"object_name": "sphere1"}

# Create modified copies
with_err = result.with_error("Something went wrong")
with_ctx = result.with_context(extra_data="value")

# Serialize
d = result.to_dict()  # {"success": True, "message": ..., ...}
```

## Resetting the Registry

```python
registry.reset()  # Clear all registered actions
```
