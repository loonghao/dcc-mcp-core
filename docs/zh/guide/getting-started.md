# 快速开始

## 安装

### 从 PyPI 安装

```bash
pip install dcc-mcp-core
```

### 从源码安装（开发）

```bash
git clone https://github.com/loonghao/dcc-mcp-core.git
cd dcc-mcp-core

# 安装 Rust 工具链（如未安装）
# 参见 https://rustup.rs/

# 开发模式构建安装
pip install maturin
maturin develop --features python-bindings,abi3-py38

# 或使用 vx（推荐）
vx just install
```

## 系统要求

- **Python**: >= 3.7（abi3 wheel 支持 3.8+）
- **Rust**: >= 1.75（从源码构建时需要）
- **许可证**: MIT
- **依赖**: Python 3.8+ 零运行时依赖

## 快速上手

```python
from dcc_mcp_core import (
    ActionResultModel, ActionRegistry,
    success_result, error_result,
    SkillScanner, SkillMetadata,
)

# 创建结果模型
result = success_result("操作完成", prompt="下一步建议", key="value")
print(result.success)   # True
print(result.message)   # "操作完成"
print(result.prompt)    # "下一步建议"

# 使用 ActionRegistry
registry = ActionRegistry()
registry.register(
    name="create_sphere",
    description="在 Maya 中创建球体",
    dcc="maya",
    tags=["geometry", "creation"],
)
actions = registry.list_actions(dcc_name="maya")

# 扫描技能包
scanner = SkillScanner()
skill_dirs = scanner.scan(extra_paths=["/path/to/skills"], dcc_name="maya")
```

## 开发环境设置

```bash
# 克隆仓库
git clone https://github.com/loonghao/dcc-mcp-core.git
cd dcc-mcp-core

# 创建并激活虚拟环境
python -m venv venv
source venv/bin/activate  # Windows: venv\Scripts\activate

# 安装开发依赖
pip install -e ".[dev]"

# 或使用 vx（推荐）
vx just install
```

## 构建 Rust 扩展

```bash
# Debug 构建（编译快，运行慢）
maturin develop --features python-bindings,abi3-py38

# Release 构建（编译慢，运行快）
maturin develop --release --features python-bindings,abi3-py38
```

## 运行测试

```bash
# 运行测试并生成覆盖率
vx just test

# 运行特定测试
vx uvx nox -s pytest -- tests/test_models.py -v

# 代码检查
vx just lint

# 代码检查并自动修复
vx just lint-fix
```

## 下一步

- 了解 [Actions 与注册表](/zh/guide/actions) — 管理动作元数据
- 探索 [事件系统](/zh/guide/events) 的发布/订阅通信
- 查看 [Skills 技能包](/zh/guide/skills) 零代码脚本注册
- 了解 [MCP 协议](/zh/guide/protocols) 类型定义
