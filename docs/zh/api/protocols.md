# 协议 API

MCP 协议类型定义，由 Rust 结构体实现。

## 类型总览

| 类型 | 说明 |
|------|------|
| `ToolDefinition` | MCP 工具定义（`name`, `description`, `input_schema`, `output_schema`） |
| `ToolAnnotations` | 工具行为提示（`title`, `read_only_hint`, `destructive_hint`, `idempotent_hint`, `open_world_hint`） |
| `ResourceDefinition` | MCP 资源（`uri`, `name`, `description`, `mime_type`） |
| `ResourceTemplateDefinition` | 参数化资源（`uri_template`, `name`, `description`, `mime_type`） |
| `PromptArgument` | 提示词参数（`name`, `description`, `required`） |
| `PromptDefinition` | MCP 提示词（`name`, `description`） |

## 使用示例

```python
from dcc_mcp_core import ToolDefinition, ResourceDefinition, PromptDefinition

tool = ToolDefinition(
    name="create_sphere",
    description="创建球体",
    input_schema='{"type": "object", "properties": {"radius": {"type": "number"}}}',
)

resource = ResourceDefinition(
    uri="scene://objects",
    name="场景对象",
    description="当前场景中的对象",
)

prompt = PromptDefinition(name="model_review", description="审查 3D 模型")
```
