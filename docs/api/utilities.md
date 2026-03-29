# Utilities API

Utility functions and types provided by the `dcc-mcp-utils` Rust crate.

## Type Wrappers (RPyC)

Type wrappers ensure RPyC remote calls preserve Python type information:

```python
from dcc_mcp_core import (
    wrap_value, unwrap_value, unwrap_parameters,
    BooleanWrapper, IntWrapper, FloatWrapper, StringWrapper,
)

# Wrap a value
wrapped = wrap_value(True)          # BooleanWrapper(True)
wrapped = wrap_value(42)            # IntWrapper(42)
wrapped = wrap_value(3.14)          # FloatWrapper(3.14)
wrapped = wrap_value("hello")       # StringWrapper("hello")

# Unwrap a value
original = unwrap_value(wrapped)    # True / 42 / 3.14 / "hello"

# Unwrap all values in a dict
params = {"visible": BooleanWrapper(True), "count": IntWrapper(5)}
unwrapped = unwrap_parameters(params)  # {"visible": True, "count": 5}
```

### Wrapper Classes

| Class | Wrapped Type | Special Methods |
|-------|-------------|-----------------|
| `BooleanWrapper(value)` | `bool` | `__bool__`, `__eq__` |
| `IntWrapper(value)` | `int` | `__int__`, `__index__` |
| `FloatWrapper(value)` | `float` | `__float__` |
| `StringWrapper(value)` | `str` | `__str__` |

All wrappers have a `.value` property and `__repr__`.

## Filesystem

Platform-specific directory utilities, replacing `platformdirs` with the Rust `dirs` crate:

```python
from dcc_mcp_core import (
    get_platform_dir,
    get_config_dir, get_data_dir, get_log_dir,
    get_actions_dir, get_skills_dir,
    get_skill_paths_from_env,
)

# Generic platform directory
config = get_platform_dir("config")    # ~/.config/dcc-mcp (Linux)
data = get_platform_dir("data")        # ~/.local/share/dcc-mcp (Linux)
cache = get_platform_dir("cache")      # ~/.cache/dcc-mcp (Linux)

# Convenience functions
config_dir = get_config_dir()          # Platform config directory
data_dir = get_data_dir()              # Platform data directory
log_dir = get_log_dir()                # .../dcc-mcp/log/

# DCC-specific directories
actions = get_actions_dir("maya")      # .../dcc-mcp/actions/maya/
skills = get_skills_dir("maya")        # .../dcc-mcp/skills/maya/
skills_global = get_skills_dir()       # .../dcc-mcp/skills/

# Environment paths
paths = get_skill_paths_from_env()     # From DCC_MCP_SKILL_PATHS
```

### Supported `get_platform_dir` Types

| Type | Description |
|------|-------------|
| `"config"` | User configuration directory |
| `"data"` | User data directory |
| `"cache"` | User cache directory |
| `"log"` / `"state"` | Local data directory |
| `"documents"` | User documents directory |

## Constants

```python
from dcc_mcp_core import (
    APP_NAME,            # "dcc-mcp"
    APP_AUTHOR,          # "dcc-mcp"
    DEFAULT_DCC,         # "python"
    DEFAULT_LOG_LEVEL,   # "DEBUG"
    ENV_LOG_LEVEL,       # "MCP_LOG_LEVEL"
    ENV_SKILL_PATHS,     # "DCC_MCP_SKILL_PATHS"
    SKILL_METADATA_FILE, # "SKILL.md"
    SKILL_SCRIPTS_DIR,   # "scripts"
)
```

## Environment Variables

| Variable | Description |
|----------|-------------|
| `DCC_MCP_SKILL_PATHS` | Skill search paths (platform path separator) |
| `MCP_LOG_LEVEL` | Log level for tracing (e.g., `"debug"`, `"info"`, `"warn"`) |
