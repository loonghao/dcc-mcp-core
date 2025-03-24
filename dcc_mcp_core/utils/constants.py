"""Constants for the DCC-MCP ecosystem.

This module provides centralized constants that are used throughout the DCC-MCP ecosystem.
"""

# Application information
APP_NAME = "dcc-mcp"
APP_AUTHOR = "dcc-mcp"

# Logging
LOG_APP_NAME = "dcc-mcp-core"
DEFAULT_LOG_LEVEL = "DEBUG"

# Environment variables
ENV_LOG_LEVEL = "MCP_LOG_LEVEL"
ENV_ACTION_PATH_PREFIX = "DCC_MCP_ACTION_PATH_"
ENV_ACTIONS_DIR = "DCC_MCP_ACTIONS_DIR"

# File names
ACTION_PATHS_CONFIG = "action_paths.json"

# Boolean flag keys for parameter processing
BOOLEAN_FLAG_KEYS = ["query", "q", "edit", "e", "select", "sl", "selection", "visible", "v", "hidden", "h"]

# Action metadata configuration
ACTION_METADATA = {
    "name": {
        "attr": "__action_name__",
        "default": None,  # Will use action_name as default
    },
    "version": {"attr": "__action_version__", "default": "0.1.0"},
    "description": {"attr": "__action_description__", "default": "No description provided."},
    "author": {"attr": "__action_author__", "default": "mcp"},
    "requires": {"attr": "__action_requires__", "default": []},
}
