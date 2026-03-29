# Actions API

## ActionRegistry

`dcc_mcp_core.ActionRegistry` — Thread-safe action registry, implemented in Rust using DashMap.

```python
from dcc_mcp_core import ActionRegistry
```

### Constructor

```python
registry = ActionRegistry()
```

### Methods

| Method | Returns | Description |
|--------|---------|-------------|
| `register(name, ...)` | `bool` | Register an action with metadata |
| `get_action(name, dcc_name=None)` | `Optional[dict]` | Get action metadata by name |
| `list_actions(dcc_name=None)` | `List[dict]` | List all actions with full metadata |
| `list_actions_for_dcc(dcc_name)` | `List[str]` | List action names for a specific DCC |
| `get_all_dccs()` | `List[str]` | List all registered DCC names |
| `reset()` | `None` | Clear all registered actions |
| `len(registry)` | `int` | Number of registered actions |

### register()

```python
registry.register(
    name="create_sphere",
    description="Creates a sphere",
    category="geometry",
    tags=["geometry", "creation"],
    dcc="maya",
    version="1.0.0",
    input_schema='{"type": "object", "properties": {}}',   # JSON string
    output_schema='{"type": "object", "properties": {}}',   # JSON string
    source_file="/path/to/action.py",
)
```

### get_action()

Returns a dict with action metadata, or `None` if not found:

```python
meta = registry.get_action("create_sphere")           # Global lookup
meta = registry.get_action("create_sphere", dcc_name="maya")  # DCC-specific
```

### Action Metadata Dict Keys

| Key | Type | Description |
|-----|------|-------------|
| `name` | `str` | Action name |
| `internal_name` | `str` | Internal name (same as name) |
| `description` | `str` | Description |
| `category` | `str` | Category |
| `tags` | `List[str]` | Tags |
| `dcc` | `str` | Target DCC |
| `version` | `str` | Version |
| `input_schema` | `str` | JSON Schema string |
| `output_schema` | `str` | JSON Schema string |
| `source_file` | `Optional[str]` | Source file path |
