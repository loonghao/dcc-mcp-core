"""Action registry for DCC-MCP-Core.

This module provides the ActionRegistry class for registering and discovering Action classes.
"""

# Import built-in modules
import importlib
import inspect
import logging
from pathlib import Path
from typing import Any, ClassVar, Dict, List, Optional, Set, Type, Callable

# Import local modules
from dcc_mcp_core.actions.base import Action
from dcc_mcp_core.utils.module_loader import load_module_from_path


class ActionRegistry:
    """Registry for Action classes.

    This class provides functionality for registering, discovering, and retrieving
    Action classes. It follows the singleton pattern to ensure a single registry
    instance is used throughout the application.
    """

    _instance: ClassVar[Optional["ActionRegistry"]] = None
    _logger: ClassVar = logging.getLogger(__name__)
    # Action discovery hooks - functions that can be registered to discover actions from specific packages
    _action_discovery_hooks: ClassVar[Dict[str, Callable]] = {}

    def __new__(cls):
        """Ensure only one instance of ActionRegistry exists (Singleton pattern)."""
        if cls._instance is None:
            cls._instance = super().__new__(cls)
            # Main registry: maps action name to action class
            cls._instance._actions = {}
            # DCC-specific registry: maps DCC name to a dict of {action_name: action_class}
            cls._instance._dcc_actions = {}
            cls._logger.debug("Created new ActionRegistry instance")
        return cls._instance

    @classmethod
    def reset(cls, full_reset=False):
        """Reset the registry to its initial state.

        This method is primarily used for testing purposes.
        
        Args:
            full_reset: If True, completely resets the singleton instance.
                       If False, only clears the current instance data.
        """
        if cls._instance is not None:
            cls._instance._actions = {}
            cls._instance._dcc_actions = {}
            cls._logger.debug("Cleared ActionRegistry instance state")

        if full_reset:
            cls._instance = None
            cls._logger.debug("Reset ActionRegistry singleton instance")
            
    @classmethod
    def _reset_instance(cls):
        """Reset the registry singleton instance.
        
        This method is kept for backward compatibility with existing tests.
        New code should use reset(full_reset=True) instead.
        """
        cls.reset(full_reset=True)

    def register(self, action_class: Type[Action]) -> None:
        """Register an Action class.

        This method registers an Action subclass in both the main registry and the
        DCC-specific registry. The action is indexed by its name in both registries.

        Args:
            action_class: The Action subclass to register

        Raises:
            TypeError: If action_class is not a subclass of Action
            ValueError: If action_class is abstract or does not implement _execute method

        """
        # Verify that action_class is a subclass of Action
        if not issubclass(action_class, Action):
            raise TypeError(f"{action_class.__name__} must be a subclass of Action")

        # Skip abstract Action classes
        if getattr(action_class, "abstract", False):
            self._logger.debug(f"Skipping registration of abstract action class: {action_class.__name__}")
            return

        # Verify that Action class implements _execute method
        if not hasattr(action_class, "_execute") or action_class._execute is Action._execute:
            self._logger.debug(
                f"Skipping registration of action class without _execute implementation: {action_class.__name__}"
            )
            return

        # Get action name and DCC type
        name = action_class.name or action_class.__name__
        dcc = action_class.dcc

        # Register in main registry
        self._actions[name] = action_class
        self._logger.debug(f"Registered action '{name}' in main registry")

        # Register in DCC-specific registry
        if dcc not in self._dcc_actions:
            self._dcc_actions[dcc] = {}
            self._logger.debug(f"Created registry for DCC '{dcc}'")

        self._dcc_actions[dcc][name] = action_class
        self._logger.debug(f"Registered action '{name}' in DCC-specific registry for '{dcc}'")

    def get_action(self, name: str, dcc_name: Optional[str] = None) -> Optional[Type[Action]]:
        """Get an Action class by name.

        Args:
            name: Name of the Action
            dcc_name: Optional DCC name to get a DCC-specific action

        Returns:
            Optional[Type[Action]]: The Action class or None if not found
        """
        # If specified DCC name, search only in DCC-specific registry
        if dcc_name:
            if dcc_name in self._dcc_actions:
                return self._dcc_actions[dcc_name].get(name)
            return None  # If specified DCC does not exist, return None
            
        # If no DCC name, search in main registry
        return self._actions.get(name)

    def list_actions(self, dcc_name: Optional[str] = None) -> List[Dict[str, Any]]:
        """List all registered Actions and their metadata.

        Args:
            dcc_name: Optional DCC name to filter actions

        Returns:
            List[Dict[str, Any]]: List of action metadata dictionaries
        """
        result = []
        
        # Determine actions to list
        if dcc_name and dcc_name in self._dcc_actions:
            actions_to_list = list(self._dcc_actions[dcc_name].items())
        else:
            actions_to_list = list(self._actions.items())

        # Process each action class
        for name, action_class in actions_to_list:
            # If filtering by DCC name, skip actions that don't match
            if dcc_name and action_class.dcc != dcc_name:
                continue

            # Create action metadata
            action_metadata = self._create_action_metadata(name, action_class)
            result.append(action_metadata)

        return result
        
    def _create_action_metadata(self, name: str, action_class: Type[Action]) -> Dict[str, Any]:
        """Create metadata dictionary for an Action class.
        
        Args:
            name: Internal name of the action
            action_class: The Action class
            
        Returns:
            Dict[str, Any]: Action metadata dictionary
        """
        # Get display name and source file (if available)
        display_name = getattr(action_class, "_original_name", name)
        source_file = getattr(action_class, "_source_file", None)

        # Create basic metadata
        metadata = {
            "name": display_name,  # Display name for user interface
            "internal_name": name,  # Internal reference name
            "description": action_class.description,
            "category": getattr(action_class, "category", ""),
            "tags": getattr(action_class, "tags", []),
            "dcc": action_class.dcc,
            "version": getattr(action_class, "version", "1.0.0"),
            "author": getattr(action_class, "author", None),
            "examples": getattr(action_class, "examples", None),
            "source_file": source_file,
        }

        # Add input model JSON Schema
        metadata["input_schema"] = self._get_model_schema(action_class, "InputModel")
        
        # Add output model JSON Schema
        metadata["output_schema"] = self._get_model_schema(action_class, "OutputModel")

        return metadata
        
    def _get_model_schema(self, action_class: Type[Action], model_attr: str) -> Dict[str, Any]:
        """Get JSON schema for a Pydantic model attribute of an Action class.
        
        Args:
            action_class: The Action class
            model_attr: Name of the model attribute ("InputModel" or "OutputModel")
            
        Returns:
            Dict[str, Any]: Simplified JSON schema
        """
        default_schema = {"title": model_attr, "type": "object", "properties": {}}
        
        try:
            if hasattr(action_class, model_attr) and getattr(action_class, model_attr):
                model = getattr(action_class, model_attr)
                schema = model.model_json_schema()
                return self._simplify_schema(schema)
        except Exception as e:
            self._logger.warning(f"Error extracting {model_attr} schema for {action_class.__name__}: {e}")
            
        return default_schema

    def _simplify_schema(self, schema: Dict[str, Any]) -> Dict[str, Any]:
        """Simplify JSON Schema, removing unnecessary complexity.

        Args:
            schema: Original JSON Schema

        Returns:
            Dict[str, Any]: Simplified Schema

        """
        # Create basic structure
        simplified = {"title": schema.get("title", ""), "type": "object", "properties": {}}

        # Extract property information
        properties = schema.get("properties", {})
        for prop_name, prop_info in properties.items():
            # Skip internal fields
            if prop_name.startswith("_"):
                continue

            simplified["properties"][prop_name] = {
                "type": prop_info.get("type", "string"),
                "description": prop_info.get("description", ""),
            }

            # Handle enum type
            if "enum" in prop_info:
                simplified["properties"][prop_name]["enum"] = prop_info["enum"]

            # Handle default value
            if "default" in prop_info:
                simplified["properties"][prop_name]["default"] = prop_info["default"]

        return simplified

    def list_actions_for_dcc(self, dcc_name: str) -> List[str]:
        """List all action names for a specific DCC.

        Args:
            dcc_name: Name of the DCC to list actions for

        Returns:
            A list of action names for the specified DCC

        """
        if dcc_name not in self._dcc_actions:
            return []

        return list(self._dcc_actions[dcc_name].keys())

    @classmethod
    def register_discovery_hook(cls, package_name: str, hook_func: Callable):
        """Register an Action discovery hook function.

        This method allows registering custom Action discovery logic for specific packages.
        This is useful for testing, plugin systems, or special package structures.

        Args:
            package_name: Package name, used as the hook key
            hook_func: Hook function that takes (registry, dcc_name) parameters and returns a list of Action classes

        Example:
            >>> def my_hook(registry, dcc_name=None):
            ...     # Custom discovery logic
            ...     return [MyAction1, MyAction2]
            >>> ActionRegistry.register_discovery_hook("my_package", my_hook)
        """
        cls._action_discovery_hooks[package_name] = hook_func
        cls._logger.debug(f"Registered discovery hook for package '{package_name}'")
        
    @classmethod
    def clear_discovery_hooks(cls):
        """Clear all Action discovery hooks.
        
        This is primarily used for testing purposes to reset the discovery hooks state.
        """
        cls._action_discovery_hooks.clear()
        cls._logger.debug("Cleared all action discovery hooks")

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
        # Check for custom action discovery hook
        if package_name in self.__class__._action_discovery_hooks:
            self._logger.debug(f"Using custom discovery hook for package {package_name}")
            return self.__class__._action_discovery_hooks[package_name](self, dcc_name)

        # Standard package handling
        discovered_actions = []
        try:
            # Import package
            package = importlib.import_module(package_name)
            package_path = Path(package.__file__).parent
            self._logger.debug(f"Discovering actions from package {package_name} at {package_path}")

            # Process Action classes in the package
            discovered_actions.extend(self._discover_actions_from_module_object(package, dcc_name))

            # Find all Python modules in the package
            modules_to_process = self._find_modules_in_package(package_name, package_path)

            # Process each module
            for module_name in modules_to_process:
                try:
                    module = importlib.import_module(module_name)
                    discovered_actions.extend(self._discover_actions_from_module_object(module, dcc_name))
                except ImportError as e:
                    self._logger.warning(f"Error importing module {module_name}: {e}")

        except ImportError as e:
            self._logger.warning(f"Error importing package {package_name}: {e}")

        return discovered_actions
        
    def _discover_actions_from_module_object(self, module: Any, dcc_name: Optional[str] = None) -> List[Type[Action]]:
        """Discover and register Action classes from a module object.
        
        Args:
            module: The module object to search for Action classes
            dcc_name: Optional DCC name to set for discovered actions
            
        Returns:
            List of discovered and registered Action classes
        """
        discovered = []
        
        for name, obj in inspect.getmembers(module):
            if inspect.isclass(obj) and issubclass(obj, Action) and obj is not Action:
                # If DCC name is provided and action class has no DCC set, set it
                if dcc_name and not obj.dcc:
                    obj.dcc = dcc_name
                    
                # Register and add to discovered list
                self.register(obj)
                discovered.append(obj)
                
        return discovered
        
    def _find_modules_in_package(self, package_name: str, package_path: Path) -> List[str]:
        """Find all Python modules in a package.
        
        Args:
            package_name: Name of the package
            package_path: Path to the package directory
            
        Returns:
            List of module names
        """
        modules = []
        
        for path in package_path.glob("**/*.py"):
            # Skip __pycache__ directory and __init__.py file
            if "__pycache__" in str(path) or path.name == "__init__.py":
                continue

            # Build module name
            rel_path = path.relative_to(package_path)
            parts = list(rel_path.parent.parts)
            
            if parts:
                module_name = ".".join([package_name, *parts, rel_path.stem])
            else:
                module_name = f"{package_name}.{rel_path.stem}"

            modules.append(module_name)
            
        return modules

    def discover_actions_from_path(
        self, path: str, dependencies: Optional[Dict[str, Any]] = None, dcc_name: Optional[str] = None
    ) -> List[Type[Action]]:
        """Load a Python module from a file path and register Action subclasses.

        This function is useful for loading actions from standalone Python files
        that are not part of a package.

        Args:
            path: Path to the Python file to load
            dependencies: Optional dictionary of dependencies to inject into the module
            dcc_name: Optional DCC name to inject DCC-specific dependencies

        Returns:
            List[Type[Action]]: List of discovered and registered Action classes

        Example:
            >>> registry = ActionRegistry()
            >>> actions = registry.discover_actions_from_path('/path/to/my_actions.py')
            >>> len(actions)
            2  # Discovered two actions in the file
        """
        self._logger.debug(f"Discovering actions from path: {path}")
        discovered_actions = []
        
        try:
            # Load module using load_module_from_path
            module = load_module_from_path(path, dependencies=dependencies, dcc_name=dcc_name)
            
            # Find and register Action subclasses
            for name, obj in inspect.getmembers(module):
                if inspect.isclass(obj) and issubclass(obj, Action) and obj is not Action:
                    # Set DCC name (if provided and not already set)
                    if dcc_name and not obj.dcc:
                        obj.dcc = dcc_name

                    # Set source file path
                    setattr(obj, "_source_file", path)

                    # Register and add to discovered list
                    self.register(obj)
                    discovered_actions.append(obj)
                    self._logger.debug(f"Discovered action '{obj.__name__}' from path '{path}'")
                    
            return discovered_actions
            
        except (ImportError, AttributeError) as e:
            # Log error but continue processing
            self._logger.warning(f"Error discovering actions from {path}: {e}")
            return discovered_actions

    def get_actions_by_dcc(self, dcc_name: str) -> Dict[str, Type[Action]]:
        """Get all actions for a specific DCC.

        This method returns a dictionary of all actions registered for a specific DCC.
        The dictionary maps action names to action classes.

        Args:
            dcc_name: Name of the DCC to get actions for

        Returns:
            Dict[str, Type[Action]]: Dictionary of action name to action class
                                     Returns an empty dict if no actions are found

        """
        # Return a copy of the DCC-specific registry to prevent modification
        if dcc_name in self._dcc_actions:
            return dict(self._dcc_actions[dcc_name])

        # Return an empty dict if the DCC is not found
        self._logger.debug(f"No actions found for DCC '{dcc_name}'")
        return {}

    def get_all_dccs(self) -> List[str]:
        """Get a list of all DCCs that have registered actions.

        Returns:
            List[str]: List of DCC names

        """
        return list(self._dcc_actions.keys())
