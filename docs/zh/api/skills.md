# Skills API

## SkillScanner

技能包目录扫描器，支持基于修改时间的缓存。

```python
from dcc_mcp_core import SkillScanner

scanner = SkillScanner()
skill_dirs = scanner.scan(extra_paths=["/my/skills"], dcc_name="maya")
scanner.clear_cache()
```

## 函数

### parse_skill_md

```python
from dcc_mcp_core import parse_skill_md

metadata = parse_skill_md("/path/to/skill-dir")  # 返回 Optional[SkillMetadata]
```

### scan_skill_paths

```python
from dcc_mcp_core import scan_skill_paths

dirs = scan_skill_paths(extra_paths=["/my/skills"], dcc_name="maya")
```

## 环境变量

| 变量 | 说明 |
|------|------|
| `DCC_MCP_SKILL_PATHS` | 技能包搜索路径（Windows 用 `;`，Unix 用 `:`） |
