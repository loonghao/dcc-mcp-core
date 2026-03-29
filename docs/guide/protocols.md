# MCP Protocols

DCC-MCP-Core provides MCP (Model Context Protocol) type definitions as Rust-backed Python classes. These types follow the [MCP Specification](https://modelcontextprotocol.io/specification/2025-11-25).

## Tool Definitions

```python
from dcc_mcp_core import ToolDefinition, ToolAnnotations

# Create a tool definition
tool = ToolDefinition(
    name="create_sphere",
    description="Creates a sphere in the scene",
    input_schema='{"type": "object", "properties": {"radius": {"type": "number"}}}',
    output_schema='{"type": "object", "properties": {"name": {"type": "string"}}}',
)

# Access fields
print(tool.name)           # "create_sphere"
print(tool.description)    # "Creates a sphere in the scene"
print(tool.input_schema)   # JSON string
print(tool.output_schema)  # JSON string or None
```

### ToolAnnotations

Behavior hints for MCP tools:

```python
annotations = ToolAnnotations(
    title="Create Sphere",
    read_only_hint=False,
    destructive_hint=False,
    idempotent_hint=True,
    open_world_hint=False,
)
```

| Field | Type | Description |
|-------|------|-------------|
| `title` | `Optional[str]` | Human-readable title |
| `read_only_hint` | `Optional[bool]` | Whether the tool only reads data |
| `destructive_hint` | `Optional[bool]` | Whether the tool may cause destructive changes |
| `idempotent_hint` | `Optional[bool]` | Whether repeated calls produce the same result |
| `open_world_hint` | `Optional[bool]` | Whether the tool interacts with external systems |

## Resource Definitions

```python
from dcc_mcp_core import ResourceDefinition, ResourceTemplateDefinition

# Static resource
resource = ResourceDefinition(
    uri="scene://objects",
    name="Scene Objects",
    description="All objects in the current scene",
    mime_type="application/json",
)

# Resource template (parameterized)
template = ResourceTemplateDefinition(
    uri_template="scene://objects/{category}/{name}",
    name="Scene Object by Category",
    description="Get objects filtered by category",
    mime_type="application/json",
)
```

## Prompt Definitions

```python
from dcc_mcp_core import PromptDefinition, PromptArgument

prompt = PromptDefinition(
    name="model_review",
    description="Review a 3D model for quality",
)

arg = PromptArgument(
    name="object_name",
    description="Name of the object to review",
    required=True,
)
```

## Type Summary

| Type | Description | Key Fields |
|------|-------------|------------|
| `ToolDefinition` | MCP Tool schema | `name`, `description`, `input_schema`, `output_schema` |
| `ToolAnnotations` | Tool behavior hints | `title`, `read_only_hint`, `destructive_hint`, `idempotent_hint`, `open_world_hint` |
| `ResourceDefinition` | MCP Resource | `uri`, `name`, `description`, `mime_type` |
| `ResourceTemplateDefinition` | Parameterized resource | `uri_template`, `name`, `description`, `mime_type` |
| `PromptArgument` | Prompt parameter | `name`, `description`, `required` |
| `PromptDefinition` | MCP Prompt | `name`, `description` |

All types support property access (getters/setters) and are backed by Rust structs with serde serialization.
