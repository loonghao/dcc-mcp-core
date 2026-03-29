# Actions 与注册表

**ActionRegistry** 是 DCC-MCP-Core 中管理动作元数据的核心组件。它提供线程安全的注册、查询和列举功能。

## ActionRegistry

`ActionRegistry` 使用 Rust 的 `DashMap` 实现无锁并发读取。每个注册表实例独立运作，避免跨 DCC 污染。

```python
from dcc_mcp_core import ActionRegistry

# 创建新注册表
registry = ActionRegistry()

# 注册动作
registry.register(
    name="create_sphere",
    description="在场景中创建球体",
    category="geometry",
    tags=["geometry", "creation"],
    dcc="maya",
    version="1.0.0",
    input_schema='{"type": "object", "properties": {"radius": {"type": "number"}}}',
    output_schema='{"type": "object", "properties": {"name": {"type": "string"}}}',
    source_file="/path/to/action.py",
)
```

## 注册参数

| 参数 | 类型 | 默认值 | 说明 |
|------|------|--------|------|
| `name` | `str` | — | 动作名称（用于查询） |
| `description` | `str` | `""` | 动作描述 |
| `category` | `str` | `""` | 分类 |
| `tags` | `List[str]` | `[]` | 标签 |
| `dcc` | `str` | `"python"` | 目标 DCC |
| `version` | `str` | `"1.0.0"` | 版本 |
| `input_schema` | `Optional[str]` | `None` | 输入 JSON Schema 字符串 |
| `output_schema` | `Optional[str]` | `None` | 输出 JSON Schema 字符串 |
| `source_file` | `Optional[str]` | `None` | 源文件路径 |

## 查询动作

```python
# 获取动作元数据（返回 dict 或 None）
meta = registry.get_action("create_sphere")
meta = registry.get_action("create_sphere", dcc_name="maya")

# 列出指定 DCC 的所有动作名称
names = registry.list_actions_for_dcc("maya")

# 列出所有动作及完整元数据
all_actions = registry.list_actions()
maya_actions = registry.list_actions(dcc_name="maya")

# 获取所有已注册 DCC 名称
dccs = registry.get_all_dccs()

# 注册表信息
print(len(registry))  # 已注册动作数量
```

## ActionResultModel

所有动作执行应返回 `ActionResultModel`：

```python
from dcc_mcp_core import ActionResultModel, success_result, error_result

# 直接构造
result = ActionResultModel(
    success=True,
    message="已创建球体",
    prompt="现在可以修改球体属性",
    context={"object_name": "sphere1"},
)

# 工厂函数（推荐）
result = success_result("已创建球体", prompt="修改属性", object_name="sphere1")
error = error_result("失败", "文件未找到", prompt="检查路径")

# 访问字段
print(result.success)    # True
print(result.message)    # "已创建球体"
print(result.context)    # {"object_name": "sphere1"}

# 创建修改后的副本
with_err = result.with_error("出错了")
with_ctx = result.with_context(extra_data="value")

# 序列化
d = result.to_dict()
```
