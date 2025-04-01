"""Tests for the ActionRegistry class.

This module contains tests for the ActionRegistry class functionality,
including registration, discovery, and retrieval of Action classes.
"""

# Import built-in modules
import os
from pathlib import Path
import sys
from typing import Any
from typing import Dict
from typing import List

# Import third-party modules
from pydantic import Field
import pytest

# Import local modules
from dcc_mcp_core.actions.base import Action
from dcc_mcp_core.actions.registry import ActionRegistry
from dcc_mcp_core.models import ActionResultModel


# Define test Action classes
class TestAction1(Action):
    """Test Action 1."""

    name = "test_action1"
    description = "Test Action 1"
    dcc = "test"

    class InputModel(Action.InputModel):
        """Input model for TestAction1."""

        value: int

    def _execute(self) -> None:
        self.output = self.OutputModel(prompt="Test Action 1 executed successfully")


class TestAction2(Action):
    """Test Action 2."""

    name = "test_action2"
    description = "Test Action 2"
    dcc = "maya"

    class InputModel(Action.InputModel):
        """Input model for TestAction2."""

        value: str

    def _execute(self) -> None:
        value = self.input.value
        self.output = self.OutputModel(prompt=f"Test Action 2 executed with value: {value}")


class TestAction(Action):
    """Test action for registry."""

    name = "test_action"
    description = "A test action"
    dcc = "test"

    class InputModel(Action.InputModel):
        """Input model for TestAction."""

        value: int


class MayaAction(Action):
    """Maya-specific test action."""

    name = "maya_action"
    description = "A Maya-specific test action"
    dcc = "maya"

    class InputModel(Action.InputModel):
        """Input model for MayaAction."""

        value: str


@pytest.fixture
def clean_registry():
    """测试用的 ActionRegistry 实例。"""
    # 重置 ActionRegistry 单例实例
    ActionRegistry._reset_instance()
    
    # 获取新的实例
    registry = ActionRegistry()
    
    yield registry
    
    # 测试结束后再次重置单例实例
    ActionRegistry._reset_instance()


def test_action_registry_singleton():
    """Test that ActionRegistry follows the singleton pattern."""
    registry1 = ActionRegistry()
    registry2 = ActionRegistry()

    # Both instances should be the same object
    assert registry1 is registry2

    # Modifying one should affect the other
    registry1._actions["test"] = "value"
    assert registry2._actions["test"] == "value"


def test_action_registry_register(clean_registry):
    """Test registering Actions."""
    registry = clean_registry

    # Register test actions
    registry.register(TestAction1)
    registry.register(TestAction2)

    # Check registration
    assert len(registry._actions) == 2
    assert "test_action1" in registry._actions
    assert "test_action2" in registry._actions
    assert registry._actions["test_action1"] is TestAction1
    assert registry._actions["test_action2"] is TestAction2


def test_action_registry_register_invalid(clean_registry):
    """Test registering invalid Action classes."""
    registry = clean_registry

    # Try to register a non-Action class
    class NotAnAction:
        pass

    with pytest.raises(TypeError):
        registry.register(NotAnAction)


def test_action_registry_list_actions(clean_registry):
    """Test listing registered Actions."""
    registry = clean_registry

    # Register test actions
    registry.register(TestAction1)
    registry.register(TestAction2)

    # List all actions
    actions = registry.list_actions()
    assert len(actions) == 2

    # Check action metadata
    action1 = next(a for a in actions if a["name"] == "test_action1")
    assert action1["description"] == "Test Action 1"
    assert action1["dcc"] == "test"

    # Filter by DCC
    maya_actions = registry.list_actions(dcc_name="maya")
    assert len(maya_actions) == 1
    assert maya_actions[0]["name"] == "test_action2"

    test_actions = registry.list_actions(dcc_name="test")
    assert len(test_actions) == 1
    assert test_actions[0]["name"] == "test_action1"

    # Filter by non-existent DCC
    houdini_actions = registry.list_actions(dcc_name="houdini")
    assert len(houdini_actions) == 0


# Create a temporary module for testing discovery
@pytest.fixture
def setup_test_package(tmp_path):
    """Create a temporary package with Action classes for testing discovery."""
    # Create package structure
    pkg_dir = tmp_path / "test_pkg"
    pkg_dir.mkdir()
    (pkg_dir / "__init__.py").write_text("")

    # Create a module with an Action class
    module_content = """
# Import local modules
from dcc_mcp_core.actions.base import Action
from dcc_mcp_core.models import ActionResultModel

class DiscoveredAction(Action):
    name = "discovered_action"
    description = "Discovered during testing"
    dcc = "test"

    class InputModel(Action.InputModel):
        value: str

    def _execute(self) -> None:
        value = self.input.value
        self.output = self.OutputModel()
"""
    (pkg_dir / "module.py").write_text(module_content)

    # Create a subpackage with another Action class
    subpkg_dir = pkg_dir / "subpkg"
    subpkg_dir.mkdir()
    (subpkg_dir / "__init__.py").write_text("")

    submodule_content = """
# Import local modules
from dcc_mcp_core.actions.base import Action
from dcc_mcp_core.models import ActionResultModel

class SubpackageAction(Action):
    name = "subpackage_action"
    description = "Action in a subpackage"
    dcc = "maya"

    class InputModel(Action.InputModel):
        value: int

    def _execute(self) -> None:
        value = self.input.value
        self.output = self.OutputModel()
"""
    (subpkg_dir / "submodule.py").write_text(submodule_content)

    # Add to sys.path so it can be imported
    sys.path.insert(0, str(tmp_path))
    yield "test_pkg"

    # Clean up
    sys.path.remove(str(tmp_path))


@pytest.mark.skipif(os.environ.get("CI") == "true", reason="Skip in CI environment")
def test_action_registry_discover_actions(clean_registry, setup_test_package):
    """Test discovering Actions from a package."""
    registry = clean_registry
    pkg_name = setup_test_package

    # Discover actions
    registry.discover_actions(pkg_name)

    # Check discovered actions
    assert "discovered_action" in registry._actions
    assert "subpackage_action" in registry._actions

    # Check action metadata
    actions = registry.list_actions()
    discovered = next(a for a in actions if a["name"] == "discovered_action")
    assert discovered["description"] == "Discovered during testing"
    assert discovered["dcc"] == "test"

    subpackage = next(a for a in actions if a["name"] == "subpackage_action")
    assert subpackage["description"] == "Action in a subpackage"
    assert subpackage["dcc"] == "maya"


def test_action_registry_dcc_specific_registry(clean_registry):
    """测试 DCC 特定的动作注册表。"""
    registry = clean_registry

    # 注册测试动作
    registry.register(TestAction1)  # dcc = "test"
    registry.register(TestAction2)  # dcc = "maya"
    registry.register(TestAction)   # dcc = "test"
    registry.register(MayaAction)   # dcc = "maya"

    # 检查主注册表
    assert len(registry._actions) == 4
    assert "test_action1" in registry._actions
    assert "test_action2" in registry._actions
    assert "test_action" in registry._actions
    assert "maya_action" in registry._actions

    # 检查 DCC 特定注册表
    assert len(registry._dcc_actions) == 2
    assert "test" in registry._dcc_actions
    assert "maya" in registry._dcc_actions
    
    # 检查 test DCC 注册表
    assert len(registry._dcc_actions["test"]) == 2
    assert "test_action1" in registry._dcc_actions["test"]
    assert "test_action" in registry._dcc_actions["test"]
    
    # 检查 maya DCC 注册表
    assert len(registry._dcc_actions["maya"]) == 2
    assert "test_action2" in registry._dcc_actions["maya"]
    assert "maya_action" in registry._dcc_actions["maya"]


def test_action_registry_get_action_with_dcc(clean_registry):
    """测试按 DCC 名称获取动作。"""
    registry = clean_registry

    # 注册测试动作
    registry.register(TestAction1)  # dcc = "test"
    registry.register(TestAction2)  # dcc = "maya"

    # 不指定 DCC 名称
    action1 = registry.get_action("test_action1")
    assert action1 is TestAction1
    
    action2 = registry.get_action("test_action2")
    assert action2 is TestAction2

    # 指定正确的 DCC 名称
    action1_test = registry.get_action("test_action1", dcc_name="test")
    assert action1_test is TestAction1
    
    action2_maya = registry.get_action("test_action2", dcc_name="maya")
    assert action2_maya is TestAction2

    # 指定错误的 DCC 名称，应该返回 None
    action1_maya = registry.get_action("test_action1", dcc_name="maya")
    assert action1_maya is None
    
    action2_test = registry.get_action("test_action2", dcc_name="test")
    assert action2_test is None

    # 指定不存在的 DCC 名称，应该返回主注册表中的动作
    action1_houdini = registry.get_action("test_action1", dcc_name="houdini")
    assert action1_houdini is TestAction1


def test_action_registry_get_actions_by_dcc(clean_registry):
    """测试获取特定 DCC 的所有动作。"""
    registry = clean_registry

    # 注册测试动作
    registry.register(TestAction1)  # dcc = "test"
    registry.register(TestAction2)  # dcc = "maya"
    registry.register(TestAction)   # dcc = "test"
    registry.register(MayaAction)   # dcc = "maya"

    # 获取特定 DCC 的所有动作
    test_actions = registry.get_actions_by_dcc("test")
    assert len(test_actions) == 2
    assert "test_action1" in test_actions
    assert "test_action" in test_actions
    assert test_actions["test_action1"] is TestAction1
    assert test_actions["test_action"] is TestAction

    maya_actions = registry.get_actions_by_dcc("maya")
    assert len(maya_actions) == 2
    assert "test_action2" in maya_actions
    assert "maya_action" in maya_actions
    assert maya_actions["test_action2"] is TestAction2
    assert maya_actions["maya_action"] is MayaAction

    # 获取不存在的 DCC 的动作
    houdini_actions = registry.get_actions_by_dcc("houdini")
    assert len(houdini_actions) == 0


def test_action_registry_get_all_dccs(clean_registry):
    """测试获取所有 DCC 列表。"""
    registry = clean_registry

    # 初始状态应该没有 DCC
    assert len(registry.get_all_dccs()) == 0

    # 注册测试动作
    registry.register(TestAction1)  # dcc = "test"
    registry.register(TestAction2)  # dcc = "maya"

    # 检查 DCC 列表
    dccs = registry.get_all_dccs()
    assert len(dccs) == 2
    assert "test" in dccs
    assert "maya" in dccs

    # 注册更多动作
    registry.register(TestAction)   # dcc = "test", 已存在
    registry.register(MayaAction)   # dcc = "maya", 已存在

    # DCC 列表应该不变
    dccs = registry.get_all_dccs()
    assert len(dccs) == 2
    assert "test" in dccs
    assert "maya" in dccs


@pytest.mark.skipif(os.environ.get("CI") == "true", reason="Skip in CI environment")
def test_action_registry_discover_actions_return_value(clean_registry, setup_test_package):
    """测试动作发现机制返回值。"""
    registry = clean_registry
    pkg_name = setup_test_package

    # 发现动作并获取返回值
    discovered_actions = registry.discover_actions(pkg_name)

    # 检查返回值
    assert len(discovered_actions) == 2
    assert any(a.__name__ == "DiscoveredAction" for a in discovered_actions)
    assert any(a.__name__ == "SubpackageAction" for a in discovered_actions)

    # 检查注册表
    assert "discovered_action" in registry._actions
    assert "subpackage_action" in registry._actions

    # 检查 DCC 特定注册表
    assert "test" in registry._dcc_actions
    assert "maya" in registry._dcc_actions
    assert "discovered_action" in registry._dcc_actions["test"]
    assert "subpackage_action" in registry._dcc_actions["maya"]


@pytest.mark.skipif(os.environ.get("CI") == "true", reason="Skip in CI environment")
def test_action_registry_discover_actions_with_dcc(clean_registry, setup_test_package):
    """测试指定 DCC 名称的动作发现机制。"""
    registry = clean_registry
    pkg_name = setup_test_package

    # 发现动作并指定 DCC 名称
    discovered_actions = registry.discover_actions(pkg_name, dcc_name="custom_dcc")

    # 检查返回值
    assert len(discovered_actions) == 2
    assert any(a.__name__ == "DiscoveredAction" for a in discovered_actions)
    assert any(a.__name__ == "SubpackageAction" for a in discovered_actions)
    
    # 检查注册表
    assert "discovered_action" in registry._actions
    assert "subpackage_action" in registry._actions

    # 检查 DCC 特定注册表
    assert "test" in registry._dcc_actions  # DiscoveredAction 已经有 dcc = "test"
    assert "maya" in registry._dcc_actions  # SubpackageAction 已经有 dcc = "maya"
    
    # 检查动作的 DCC 名称没有被覆盖
    discovered_action = registry._actions.get("discovered_action")
    assert discovered_action is not None
    assert discovered_action.dcc == "test"
    
    subpackage_action = registry._actions.get("subpackage_action")
    assert subpackage_action is not None
    assert subpackage_action.dcc == "maya"
