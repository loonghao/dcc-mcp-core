# 数据模型

## ActionResultModel

所有动作执行的标准化结果类型，由 Rust 实现。

### 字段

| 字段 | 类型 | 默认值 | 说明 |
|------|------|--------|------|
| `success` | `bool` | `True` | 是否成功 |
| `message` | `str` | `""` | 结果描述（可读写） |
| `prompt` | `Optional[str]` | `None` | 给 AI 的下一步建议 |
| `error` | `Optional[str]` | `None` | 错误消息 |
| `context` | `dict` | `{}` | 附加上下文数据 |

### 方法

| 方法 | 返回值 | 说明 |
|------|--------|------|
| `with_error(error)` | `ActionResultModel` | 创建带错误信息的副本 |
| `with_context(**kwargs)` | `ActionResultModel` | 创建更新上下文的副本 |
| `to_dict()` | `dict` | 转换为字典 |

### 工厂函数

```python
from dcc_mcp_core import success_result, error_result, from_exception, validate_action_result

result = success_result("创建了 5 个球体", prompt="使用 modify_spheres", count=5)
error = error_result("失败", "文件未找到", prompt="检查路径", possible_solutions=["检查文件是否存在"])
exc_result = from_exception("ImportError", message="导入失败", include_traceback=True)
validated = validate_action_result(some_value)  # 任意值 → ActionResultModel
```

## SkillMetadata

从 SKILL.md frontmatter 解析的元数据，由 Rust 实现。

| 字段 | 类型 | 默认值 | 说明 |
|------|------|--------|------|
| `name` | `str` | — | 唯一标识符 |
| `description` | `str` | `""` | 描述 |
| `tools` | `List[str]` | `[]` | 所需工具权限 |
| `dcc` | `str` | `"python"` | 目标 DCC |
| `tags` | `List[str]` | `[]` | 分类标签 |
| `scripts` | `List[str]` | `[]` | 脚本文件路径 |
| `skill_path` | `str` | `""` | 技能目录路径 |
| `version` | `str` | `"1.0.0"` | 版本 |
| `depends` | `List[str]` | `[]` | 依赖技能名称 |
| `metadata_files` | `List[str]` | `[]` | metadata/ 目录文件 |
