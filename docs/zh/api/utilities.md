# 工具函数 API

`dcc-mcp-utils` Rust crate 提供的工具函数和类型。

## 类型包装器（RPyC）

```python
from dcc_mcp_core import wrap_value, unwrap_value, unwrap_parameters

wrapped = wrap_value(True)          # BooleanWrapper(True)
original = unwrap_value(wrapped)    # True

params = {"visible": wrap_value(True), "count": wrap_value(5)}
unwrapped = unwrap_parameters(params)  # {"visible": True, "count": 5}
```

### 包装器类

| 类 | 包装类型 | 特殊方法 |
|---|---------|---------|
| `BooleanWrapper` | `bool` | `__bool__`, `__eq__` |
| `IntWrapper` | `int` | `__int__`, `__index__` |
| `FloatWrapper` | `float` | `__float__` |
| `StringWrapper` | `str` | `__str__` |

## 文件系统

```python
from dcc_mcp_core import (
    get_platform_dir, get_config_dir, get_data_dir,
    get_log_dir, get_actions_dir, get_skills_dir,
    get_skill_paths_from_env,
)

config = get_config_dir()          # 平台配置目录
data = get_data_dir()              # 平台数据目录
log = get_log_dir()                # .../dcc-mcp/log/
actions = get_actions_dir("maya")  # .../dcc-mcp/actions/maya/
skills = get_skills_dir("maya")    # .../dcc-mcp/skills/maya/
paths = get_skill_paths_from_env() # 从 DCC_MCP_SKILL_PATHS 读取
```

## 常量

```python
from dcc_mcp_core import (
    APP_NAME,            # "dcc-mcp"
    APP_AUTHOR,          # "dcc-mcp"
    DEFAULT_DCC,         # "python"
    ENV_SKILL_PATHS,     # "DCC_MCP_SKILL_PATHS"
    SKILL_METADATA_FILE, # "SKILL.md"
    SKILL_SCRIPTS_DIR,   # "scripts"
)
```

## 环境变量

| 变量 | 说明 |
|------|------|
| `DCC_MCP_SKILL_PATHS` | 技能包搜索路径 |
| `MCP_LOG_LEVEL` | 日志级别（如 `"debug"`, `"info"`, `"warn"`） |
