# Models API

## ActionResultModel

Standardized result type for all action executions. Implemented in Rust, exposed via PyO3.

```python
from dcc_mcp_core import ActionResultModel
```

### Constructor

```python
ActionResultModel(
    success=True,
    message="",
    prompt=None,
    error=None,
    context=None,   # Optional[dict]
)
```

### Properties

| Property | Type | Description |
|----------|------|-------------|
| `success` | `bool` | Whether the execution was successful |
| `message` | `str` | Human-readable result description (read/write) |
| `prompt` | `Optional[str]` | Suggestion for AI about next steps |
| `error` | `Optional[str]` | Error message when `success` is `False` |
| `context` | `dict` | Additional context data |

### Methods

| Method | Returns | Description |
|--------|---------|-------------|
| `with_error(error)` | `ActionResultModel` | Create copy with error info (`success=False`) |
| `with_context(**kwargs)` | `ActionResultModel` | Create copy with updated context |
| `to_dict()` | `dict` | Convert to dictionary |

### Factory Functions

```python
from dcc_mcp_core import success_result, error_result, from_exception, validate_action_result

# Success result
result = success_result(
    "Created 5 spheres",
    prompt="Use modify_spheres next",
    count=5,                           # Extra kwargs go to context
)

# Error result
error = error_result(
    "Failed to create",               # message
    "File not found: /bad/path",      # error
    prompt="Check file path",
    possible_solutions=["Check if file exists", "Verify permissions"],
    path="/bad/path",                  # Extra kwargs go to context
)

# From exception
exc_result = from_exception(
    "ImportError: module not found",   # error_message
    message="Import failed",
    prompt="Install the missing module",
    include_traceback=True,
    possible_solutions=["pip install missing-module"],
)

# Validate/convert any value to ActionResultModel
validated = validate_action_result(some_value)  # dict, ActionResultModel, or any → ActionResultModel
```

## SkillMetadata

Metadata parsed from SKILL.md frontmatter. Implemented in Rust.

```python
from dcc_mcp_core import SkillMetadata
```

### Constructor

```python
SkillMetadata(
    name="my-skill",
    description="A skill",
    tools=[],
    dcc="python",
    tags=[],
    scripts=[],
    skill_path="",
    version="1.0.0",
    depends=[],
    metadata_files=[],
)
```

### Properties (all read/write)

| Property | Type | Default | Description |
|----------|------|---------|-------------|
| `name` | `str` | — | Unique identifier |
| `description` | `str` | `""` | Human-readable description |
| `tools` | `List[str]` | `[]` | Required tool permissions |
| `dcc` | `str` | `"python"` | Target DCC application |
| `tags` | `List[str]` | `[]` | Classification tags |
| `scripts` | `List[str]` | `[]` | Discovered script file paths |
| `skill_path` | `str` | `""` | Absolute path to skill directory |
| `version` | `str` | `"1.0.0"` | Skill version |
| `depends` | `List[str]` | `[]` | Dependency skill names |
| `metadata_files` | `List[str]` | `[]` | Files found in metadata/ directory |
