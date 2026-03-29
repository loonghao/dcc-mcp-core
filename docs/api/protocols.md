# Protocols API

MCP protocol type definitions, implemented as Rust structs exposed via PyO3.

## ToolDefinition

```python
from dcc_mcp_core import ToolDefinition
```

| Property | Type | Description |
|----------|------|-------------|
| `name` | `str` | Tool name (read/write) |
| `description` | `str` | Tool description (read/write) |
| `input_schema` | `str` | JSON Schema string for input (read-only) |
| `output_schema` | `Optional[str]` | JSON Schema string for output (read-only) |

## ToolAnnotations

```python
from dcc_mcp_core import ToolAnnotations
```

| Property | Type | Description |
|----------|------|-------------|
| `title` | `Optional[str]` | Human-readable title |
| `read_only_hint` | `Optional[bool]` | Whether the tool only reads data |
| `destructive_hint` | `Optional[bool]` | Whether the tool may be destructive |
| `idempotent_hint` | `Optional[bool]` | Whether the tool is idempotent |
| `open_world_hint` | `Optional[bool]` | Whether the tool interacts with external systems |

## ResourceDefinition

```python
from dcc_mcp_core import ResourceDefinition
```

| Property | Type | Description |
|----------|------|-------------|
| `uri` | `str` | Resource URI |
| `name` | `str` | Resource name |
| `description` | `str` | Description |
| `mime_type` | `str` | MIME type (default: `"text/plain"`) |

## ResourceTemplateDefinition

```python
from dcc_mcp_core import ResourceTemplateDefinition
```

| Property | Type | Description |
|----------|------|-------------|
| `uri_template` | `str` | URI template with parameters |
| `name` | `str` | Template name |
| `description` | `str` | Description |
| `mime_type` | `str` | MIME type (default: `"text/plain"`) |

## PromptArgument

```python
from dcc_mcp_core import PromptArgument
```

| Property | Type | Description |
|----------|------|-------------|
| `name` | `str` | Argument name |
| `description` | `str` | Description |
| `required` | `bool` | Whether argument is required (default: `False`) |

## PromptDefinition

```python
from dcc_mcp_core import PromptDefinition
```

| Property | Type | Description |
|----------|------|-------------|
| `name` | `str` | Prompt name |
| `description` | `str` | Description |
