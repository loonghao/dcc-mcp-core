# Skills 技能包系统

Skills 系统允许将任何脚本（Python、MEL、MaxScript、BAT、Shell 等）零代码注册为 MCP 可发现工具。直接复用 [OpenClaw Skills](https://docs.openclaw.ai/tools) 生态格式。

## 快速上手

### 1. 创建 Skill 目录

```
maya-geometry/
├── SKILL.md
├── scripts/
│   ├── create_sphere.py
│   ├── batch_rename.mel
│   └── export_fbx.bat
└── metadata/          # 可选
    ├── depends.md     # 依赖声明
    └── help.md        # 补充文档
```

### 2. 编写 SKILL.md

```yaml
---
name: maya-geometry
description: "Maya 几何体创建和修改工具"
tools: ["Bash", "Read"]
tags: ["maya", "geometry"]
dcc: maya
version: "1.0.0"
---
# Maya Geometry Skill

使用这些工具在 Maya 中创建和修改几何体。
```

### 3. 设置环境变量

```bash
# Linux/macOS
export DCC_MCP_SKILL_PATHS="/path/to/my-skills"

# Windows
set DCC_MCP_SKILL_PATHS=C:\path\to\my-skills

# 多路径
export DCC_MCP_SKILL_PATHS="/path/skills1:/path/skills2"
```

### 4. 扫描与加载

```python
from dcc_mcp_core import SkillScanner, scan_skill_paths, parse_skill_md

# 方式 1：使用 SkillScanner 完全控制
scanner = SkillScanner()
skill_dirs = scanner.scan(
    extra_paths=["/my/skills"],
    dcc_name="maya",
    force_refresh=False,
)

# 方式 2：便捷函数
skill_dirs = scan_skill_paths(extra_paths=["/my/skills"], dcc_name="maya")

# 方式 3：解析特定技能目录
metadata = parse_skill_md("/path/to/maya-geometry")
if metadata:
    print(f"技能: {metadata.name}")
    print(f"脚本: {metadata.scripts}")
```

## 支持的脚本类型

| 扩展名 | 类型 | 执行方式 |
|--------|------|---------|
| `.py` | Python | `subprocess` |
| `.mel` | MEL (Maya) | 通过 DCC 适配器 |
| `.ms` | MaxScript | 通过 DCC 适配器 |
| `.bat`, `.cmd` | Batch | `cmd /c` |
| `.sh`, `.bash` | Shell | `bash` |
| `.ps1` | PowerShell | `powershell -File` |
| `.js`, `.jsx` | JavaScript | `node` |
| `.vbs` | VBScript | `cscript` |

## 工作原理

1. **SkillScanner** 扫描目录寻找 `SKILL.md` 文件（基于修改时间缓存）
2. **parse_skill_md** 解析 YAML frontmatter 并枚举 `scripts/` 目录
3. 发现 **metadata/** 目录中的附加文件
4. 合并 `metadata/depends.md` 中声明的依赖

### 搜索路径优先级

1. 传入 `scan()` 的 **extra_paths**（最高优先）
2. 环境变量 **DCC_MCP_SKILL_PATHS**
3. **平台技能目录**（`get_skills_dir(dcc_name)`）
4. **全局技能目录**（`get_skills_dir(None)`）

## SkillMetadata 字段

| 字段 | 类型 | 默认值 | 说明 |
|------|------|--------|------|
| `name` | `str` | — | 唯一标识符 |
| `description` | `str` | `""` | 描述 |
| `tools` | `List[str]` | `[]` | 所需工具权限 |
| `dcc` | `str` | `"python"` | 目标 DCC |
| `tags` | `List[str]` | `[]` | 分类标签 |
| `scripts` | `List[str]` | `[]` | 发现的脚本文件路径 |
| `skill_path` | `str` | `""` | 技能目录绝对路径 |
| `version` | `str` | `"1.0.0"` | 版本 |
| `depends` | `List[str]` | `[]` | 依赖的技能名称 |
| `metadata_files` | `List[str]` | `[]` | metadata/ 目录中的文件 |

## 缓存

```python
scanner = SkillScanner()
dirs = scanner.scan()                    # 使用缓存
dirs = scanner.scan(force_refresh=True)  # 强制刷新
scanner.clear_cache()                    # 清除缓存
```
