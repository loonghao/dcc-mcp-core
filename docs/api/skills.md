# Skills API

## SkillScanner

`dcc_mcp_core.SkillScanner` — Scan directories for SKILL.md files, with mtime-based caching.

```python
from dcc_mcp_core import SkillScanner
```

### Constructor

```python
scanner = SkillScanner()
```

### Methods

| Method | Returns | Description |
|--------|---------|-------------|
| `scan(extra_paths=None, dcc_name=None, force_refresh=False)` | `List[str]` | Scan for skill directories |
| `clear_cache()` | `None` | Clear the mtime cache |

### Properties

| Property | Type | Description |
|----------|------|-------------|
| `discovered_skills` | `List[str]` | Previously discovered skill directories |

### Search Path Priority

1. `extra_paths` parameter (highest priority)
2. `DCC_MCP_SKILL_PATHS` environment variable
3. Platform skills directory for the given DCC
4. Global skills directory (if `dcc_name` was specified)

## Functions

### parse_skill_md

```python
from dcc_mcp_core import parse_skill_md

metadata = parse_skill_md("/path/to/skill-dir")  # Returns Optional[SkillMetadata]
```

Parses a SKILL.md file from a skill directory. Returns `None` if the file is missing or invalid.

**Processing steps:**
1. Read and parse YAML frontmatter from `SKILL.md`
2. Enumerate scripts in `scripts/` subdirectory
3. Discover files in `metadata/` subdirectory
4. Merge dependencies from `metadata/depends.md`

### scan_skill_paths

```python
from dcc_mcp_core import scan_skill_paths

dirs = scan_skill_paths(extra_paths=["/my/skills"], dcc_name="maya")
```

Convenience function that creates a temporary `SkillScanner` and runs a scan.

## Environment Variables

| Variable | Description |
|----------|-------------|
| `DCC_MCP_SKILL_PATHS` | Skill search paths (platform path separator: `;` on Windows, `:` on Unix) |
