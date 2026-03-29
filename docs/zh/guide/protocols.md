# MCP 协议

DCC-MCP-Core 提供 MCP（模型上下文协议）类型定义，作为 Rust 支持的 Python 类。遵循 [MCP 规范](https://modelcontextprotocol.io/specification/2025-11-25)。

## 工具定义

```python
from dcc_mcp_core import ToolDefinition, ToolAnnotations

tool = ToolDefinition(
    name="create_sphere",
    description="在场景中创建球体",
    input_schema='{"type": "object", "properties": {"radius": {"type": "number"}}}',
    output_schema='{"type": "object", "properties": {"name": {"type": "string"}}}',
)

annotations = ToolAnnotations(
    title="创建球体",
    read_only_hint=False,
    destructive_hint=False,
    idempotent_hint=True,
)
```

## 资源定义

```python
from dcc_mcp_core import ResourceDefinition, ResourceTemplateDefinition

resource = ResourceDefinition(
    uri="scene://objects",
    name="场景对象",
    description="当前场景中的所有对象",
    mime_type="application/json",
)

template = ResourceTemplateDefinition(
    uri_template="scene://objects/{category}/{name}",
    name="按类别获取对象",
    description="按类别筛选对象",
    mime_type="application/json",
)
```

## 提示词定义

```python
from dcc_mcp_core import PromptDefinition, PromptArgument

prompt = PromptDefinition(
    name="model_review",
    description="审查 3D 模型质量",
)

arg = PromptArgument(
    name="object_name",
    description="要审查的对象名称",
    required=True,
)
```

## 类型总览

| 类型 | 说明 | 主要字段 |
|------|------|---------|
| `ToolDefinition` | MCP 工具定义 | `name`, `description`, `input_schema`, `output_schema` |
| `ToolAnnotations` | 工具行为提示 | `title`, `read_only_hint`, `destructive_hint`, `idempotent_hint`, `open_world_hint` |
| `ResourceDefinition` | MCP 资源 | `uri`, `name`, `description`, `mime_type` |
| `ResourceTemplateDefinition` | 参数化资源 | `uri_template`, `name`, `description`, `mime_type` |
| `PromptArgument` | 提示词参数 | `name`, `description`, `required` |
| `PromptDefinition` | MCP 提示词 | `name`, `description` |

所有类型支持属性读写，由 Rust 结构体和 serde 序列化支撑。
