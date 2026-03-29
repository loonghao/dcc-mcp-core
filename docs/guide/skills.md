# Skills System

The Skills system allows you to register any script (Python, MEL, MaxScript, BAT, Shell, etc.) as an MCP-discoverable tool with **zero Python code**. It directly reuses the [OpenClaw Skills](https://docs.openclaw.ai/tools) ecosystem format.

## Quick Start

### 1. Create a Skill Directory

```
maya-geometry/
├── SKILL.md
├── scripts/
│   ├── create_sphere.py
│   ├── batch_rename.mel
│   └── export_fbx.bat
└── metadata/          # Optional
    ├── depends.md     # Dependency declarations
    └── help.md        # Additional documentation
```

### 2. Write the SKILL.md

```yaml
---
name: maya-geometry
description: "Maya geometry creation and modification tools"
tools: ["Bash", "Read"]
tags: ["maya", "geometry"]
dcc: maya
version: "1.0.0"
---
# Maya Geometry Skill

Use these tools to create and modify geometry in Maya.
```

### 3. Set Environment Variable

```bash
# Linux/macOS
export DCC_MCP_SKILL_PATHS="/path/to/my-skills"

# Windows
set DCC_MCP_SKILL_PATHS=C:\path\to\my-skills

# Multiple paths (use platform path separator)
export DCC_MCP_SKILL_PATHS="/path/skills1:/path/skills2"
```

### 4. Scan and Load

```python
from dcc_mcp_core import SkillScanner, scan_skill_paths, parse_skill_md

# Option 1: Use SkillScanner for full control
scanner = SkillScanner()
skill_dirs = scanner.scan(
    extra_paths=["/my/skills"],
    dcc_name="maya",
    force_refresh=False,
)

# Option 2: Convenience function
skill_dirs = scan_skill_paths(extra_paths=["/my/skills"], dcc_name="maya")

# Option 3: Parse a specific skill directory
metadata = parse_skill_md("/path/to/maya-geometry")
if metadata:
    print(f"Skill: {metadata.name}")
    print(f"Scripts: {metadata.scripts}")
    print(f"Tags: {metadata.tags}")
```

## Supported Script Types

| Extension | Type | Execution |
|-----------|------|-----------|
| `.py` | Python | `subprocess` with system Python |
| `.mel` | MEL (Maya) | Via DCC adapter in context |
| `.ms` | MaxScript | Via DCC adapter in context |
| `.bat`, `.cmd` | Batch | `cmd /c` |
| `.sh`, `.bash` | Shell | `bash` |
| `.ps1` | PowerShell | `powershell -File` |
| `.js`, `.jsx` | JavaScript | `node` |
| `.vbs` | VBScript | `cscript` |

## How It Works

1. **SkillScanner** scans directories for `SKILL.md` files (with mtime-based caching)
2. **parse_skill_md** parses the YAML frontmatter and enumerates `scripts/` directory
3. **metadata/** directory is discovered for additional files (depends.md, help.md, etc.)
4. Dependencies declared in `metadata/depends.md` are merged into the metadata

### Search Path Priority

1. **Extra paths** passed to `scan()` (highest priority)
2. **Environment variable** `DCC_MCP_SKILL_PATHS`
3. **Platform skills directory** (`get_skills_dir(dcc_name)`)
4. **Global skills directory** (`get_skills_dir(None)`)

## SkillMetadata Fields

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `name` | `str` | — | Unique identifier |
| `description` | `str` | `""` | Human-readable description |
| `tools` | `List[str]` | `[]` | Required tool permissions |
| `dcc` | `str` | `"python"` | Target DCC application |
| `tags` | `List[str]` | `[]` | Classification tags |
| `scripts` | `List[str]` | `[]` | Discovered script file paths |
| `skill_path` | `str` | `""` | Absolute path to skill directory |
| `version` | `str` | `"1.0.0"` | Skill version |
| `depends` | `List[str]` | `[]` | Dependency skill names |
| `metadata_files` | `List[str]` | `[]` | Files in metadata/ directory |

## Caching

`SkillScanner` caches results based on SKILL.md file modification times. Use `force_refresh=True` to bypass the cache:

```python
scanner = SkillScanner()
# Normal scan (uses cache)
dirs = scanner.scan()
# Force re-scan
dirs = scanner.scan(force_refresh=True)
# Clear cache entirely
scanner.clear_cache()
```
