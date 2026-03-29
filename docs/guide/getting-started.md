# Getting Started

## Installation

### From PyPI

```bash
pip install dcc-mcp-core
```

### From Source (Development)

```bash
git clone https://github.com/loonghao/dcc-mcp-core.git
cd dcc-mcp-core

# Install Rust toolchain (if not already installed)
# See https://rustup.rs/

# Build and install in development mode
pip install maturin
maturin develop --features python-bindings,abi3-py38

# Or use vx (recommended)
vx just install
```

## Requirements

- **Python**: >= 3.7 (abi3 wheel for 3.8+)
- **Rust**: >= 1.75 (for building from source)
- **License**: MIT
- **Dependencies**: Zero Python runtime dependencies for 3.8+

## Quick Start

```python
from dcc_mcp_core import (
    ActionResultModel, ActionRegistry,
    success_result, error_result,
    SkillScanner, SkillMetadata,
)

# Create a result model
result = success_result("Operation completed", prompt="Next step suggestion", key="value")
print(result.success)   # True
print(result.message)   # "Operation completed"
print(result.prompt)    # "Next step suggestion"

# Use the ActionRegistry
registry = ActionRegistry()
registry.register(
    name="create_sphere",
    description="Creates a sphere in Maya",
    dcc="maya",
    tags=["geometry", "creation"],
)
actions = registry.list_actions(dcc_name="maya")

# Scan for skills
scanner = SkillScanner()
skill_dirs = scanner.scan(extra_paths=["/path/to/skills"], dcc_name="maya")
```

## Development Setup

```bash
# Clone the repository
git clone https://github.com/loonghao/dcc-mcp-core.git
cd dcc-mcp-core

# Create and activate virtual environment
python -m venv venv
source venv/bin/activate  # On Windows: venv\Scripts\activate

# Install development dependencies
pip install -e ".[dev]"

# Or use vx (recommended)
vx just install
```

## Building the Rust Extension

```bash
# Debug build (fast compile, slower runtime)
maturin develop --features python-bindings,abi3-py38

# Release build (slow compile, optimized runtime)
maturin develop --release --features python-bindings,abi3-py38
```

## Running Tests

```bash
# Run tests with coverage
vx just test

# Run specific tests
vx uvx nox -s pytest -- tests/test_models.py -v

# Run linting
vx just lint

# Run linting with auto-fix
vx just lint-fix
```

## Next Steps

- Learn about [Actions & Registry](/guide/actions) — managing action metadata
- Explore the [Event System](/guide/events) for pub/sub communication
- Check out the [Skills System](/guide/skills) for zero-code script registration
- See [MCP Protocols](/guide/protocols) for protocol type definitions
