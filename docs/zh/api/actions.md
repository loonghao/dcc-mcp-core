# Actions API

## ActionRegistry

线程安全的动作注册表，使用 Rust DashMap 实现。

```python
from dcc_mcp_core import ActionRegistry

registry = ActionRegistry()
```

### 方法

| 方法 | 返回值 | 说明 |
|------|--------|------|
| `register(name, ...)` | `bool` | 注册动作 |
| `get_action(name, dcc_name=None)` | `Optional[dict]` | 获取动作元数据 |
| `list_actions(dcc_name=None)` | `List[dict]` | 列出所有动作 |
| `list_actions_for_dcc(dcc_name)` | `List[str]` | 列出指定 DCC 的动作名称 |
| `get_all_dccs()` | `List[str]` | 列出所有已注册 DCC |
| `reset()` | `None` | 清除所有注册 |
| `len(registry)` | `int` | 已注册动作数量 |

### 注册示例

```python
registry.register(
    name="create_sphere",
    description="创建球体",
    dcc="maya",
    tags=["geometry"],
    input_schema='{"type": "object", "properties": {}}',
)
```
