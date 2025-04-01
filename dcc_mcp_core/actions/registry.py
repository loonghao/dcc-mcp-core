"""Action registry for DCC-MCP-Core.

This module provides the ActionRegistry class for registering and discovering Action classes.
"""

# Import built-in modules
import importlib
import inspect
import logging
from pathlib import Path
import pkgutil
from typing import Any
from typing import Dict
from typing import List
from typing import Optional
from typing import Type

# Import local modules
from dcc_mcp_core.actions.base import Action
from dcc_mcp_core.utils.module_loader import load_module_from_path


class ActionRegistry:
    """Registry for Action classes.

    This class provides functionality for registering, discovering, and retrieving
    Action classes. It follows the singleton pattern to ensure a single registry
    instance is used throughout the application.
    """

    _instance = None
    _logger = logging.getLogger(__name__)

    def __new__(cls):
        """Ensure only one instance of ActionRegistry exists (Singleton pattern)."""
        if cls._instance is None:
            cls._instance = super().__new__(cls)
            cls._instance._actions = {}
            cls._instance._dcc_actions = {}
            cls._logger.debug("Created new ActionRegistry instance")
        return cls._instance

    @classmethod
    def _reset_instance(cls):
        """Reset the singleton instance.

        This method is primarily used for testing purposes.
        """
        cls._instance = None
        cls._logger.debug("Reset ActionRegistry singleton instance")

    def register(self, action_class: Type[Action]) -> None:
        """Register an Action class.

        Args:
            action_class: The Action subclass to register

        Raises:
            TypeError: If action_class is not a subclass of Action

        """
        if not issubclass(action_class, Action):
            raise TypeError(f"{action_class.__name__} must be a subclass of Action")

        name = action_class.name or action_class.__name__
        dcc = action_class.dcc

        # Register in the main registry
        self._actions[name] = action_class

        # Register in the DCC-specific registry
        if dcc not in self._dcc_actions:
            self._dcc_actions[dcc] = {}
        self._dcc_actions[dcc][name] = action_class

        self._logger.debug(f"Registered action '{name}' for DCC '{dcc}'")

    def get_action(self, name: str, dcc_name: Optional[str] = None) -> Optional[Type[Action]]:
        """Get an Action class by name.

        Args:
            name: Name of the Action
            dcc_name: Optional DCC name to get a DCC-specific action

        Returns:
            Optional[Type[Action]]: The Action class or None if not found

        """
        if dcc_name:
            # If DCC name is specified
            if dcc_name in self._dcc_actions:
                # Look in that DCC's registry
                action = self._dcc_actions[dcc_name].get(name)
                # If found in DCC registry, return it; otherwise return None
                # This means if we specify a DCC, we ONLY look in that DCC's registry
                return action
            else:
                # If the specified DCC doesn't exist, fall back to main registry
                return self._actions.get(name)
        else:
            # If no DCC specified, look in main registry
            return self._actions.get(name)

    def list_actions(self, dcc_name: Optional[str] = None) -> List[Dict[str, Any]]:
        """List all registered Actions and their metadata.

        Args:
            dcc_name: Optional DCC name to filter actions

        Returns:
            List[Dict[str, Any]]: List of action metadata dictionaries

        """
        result = []

        if dcc_name and dcc_name in self._dcc_actions:
            # List only actions for the specified DCC
            actions_to_list = self._dcc_actions[dcc_name].items()
        else:
            # List all actions
            actions_to_list = self._actions.items()

        for name, action_class in actions_to_list:
            # Skip if we're filtering by DCC name and this action is for a different DCC
            if dcc_name and action_class.dcc != dcc_name:
                continue

            # Extract input schema
            input_schema = action_class.InputModel.model_json_schema()

            # Extract output schema if available
            output_schema = None
            if hasattr(action_class, "OutputModel") and action_class.OutputModel:
                output_schema = action_class.OutputModel.model_json_schema()

            result.append(
                {
                    "name": name,
                    "description": action_class.description,
                    "tags": action_class.tags,
                    "dcc": action_class.dcc,
                    "input_schema": input_schema,
                    "output_schema": output_schema,
                    "version": getattr(action_class, "version", "1.0.0"),
                    "author": getattr(action_class, "author", None),
                    "examples": getattr(action_class, "examples", None),
                }
            )
        return result

    def discover_actions(self, package_name: str, dcc_name: Optional[str] = None) -> List[Type[Action]]:
        """Discover and register Action classes from a package.

        This method recursively searches through a package and its subpackages
        for Action subclasses and registers them.

        Args:
            package_name: Name of the package to search
            dcc_name: Optional DCC name to set for discovered actions

        Returns:
            List of discovered and registered Action classes

        """
        discovered_actions = []
        try:
            package = importlib.import_module(package_name)
            package_path = Path(package.__file__).parent

            for _, module_name, is_pkg in pkgutil.iter_modules([str(package_path)]):
                if is_pkg:
                    # Recursively process subpackages
                    discovered_actions.extend(self.discover_actions(f"{package_name}.{module_name}", dcc_name))
                else:
                    # Import module and find Action subclasses
                    try:
                        module = importlib.import_module(f"{package_name}.{module_name}")

                        for name, obj in inspect.getmembers(module):
                            if inspect.isclass(obj) and issubclass(obj, Action) and obj is not Action:
                                # Set DCC name if provided and not already set
                                if dcc_name and not obj.dcc:
                                    obj.dcc = dcc_name

                                self.register(obj)
                                discovered_actions.append(obj)
                                self._logger.debug(f"Discovered action '{obj.__name__}' in module '{module_name}'")
                    except (ImportError, AttributeError) as e:
                        # Log error but continue processing other modules
                        self._logger.warning(f"Error importing module {module_name}: {e}")
        except ImportError as e:
            self._logger.warning(f"Error importing package {package_name}: {e}")

        return discovered_actions

    def discover_actions_from_path(
        self, path: str, dependencies: Optional[Dict[str, Any]] = None, dcc_name: Optional[str] = None
    ) -> List[Type[Action]]:
        """Discover and register Action classes from a file path.

        This method loads a Python module from a file path and registers any Action
        subclasses found in the module.

        Args:
            path: Path to the Python file to load
            dependencies: Optional dictionary of dependencies to inject into the module
            dcc_name: Optional DCC name to inject DCC-specific dependencies

        Returns:
            List of discovered and registered Action classes

        """
        discovered_actions = []
        try:
            module = load_module_from_path(path, dependencies=dependencies, dcc_name=dcc_name)

            # Find and register Action subclasses
            for name, obj in inspect.getmembers(module):
                if inspect.isclass(obj) and issubclass(obj, Action) and obj is not Action:
                    # Set DCC name if not already set and dcc_name is provided
                    if dcc_name and not obj.dcc:
                        obj.dcc = dcc_name

                    # Register the action class
                    self.register(obj)
                    discovered_actions.append(obj)
                    self._logger.debug(f"Discovered action '{obj.__name__}' from path '{path}'")
        except (ImportError, AttributeError) as e:
            # Log error but continue processing
            self._logger.warning(f"Error discovering actions from {path}: {e}")
        return discovered_actions

    def get_actions_by_dcc(self, dcc_name: str) -> Dict[str, Type[Action]]:
        """Get all actions for a specific DCC.

        Args:
            dcc_name: Name of the DCC

        Returns:
            Dict[str, Type[Action]]: Dictionary of action name to action class

        """
        if dcc_name in self._dcc_actions:
            return self._dcc_actions[dcc_name]
        return {}

    def get_all_dccs(self) -> List[str]:
        """Get a list of all DCCs that have registered actions.

        Returns:
            List[str]: List of DCC names

        """
        return list(self._dcc_actions.keys())

    def reset(self):
        """Reset the registry to its initial state.

        This method is primarily used for testing purposes.
        """
        self._actions.clear()
        self._dcc_actions.clear()
        self._logger.debug("Reset ActionRegistry instance")
