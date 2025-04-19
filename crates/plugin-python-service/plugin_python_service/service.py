from __future__ import annotations

import json
import logging
import os
import sys
from importlib.util import module_from_spec, spec_from_file_location
from pathlib import Path
from typing import Any

from plugin_python_service import (
    CoProcessorBase,
    DrivingProcessorBase,
)

# Configure logging
log_level = logging.DEBUG if os.getenv("PYTHON_DEBUG") else logging.WARNING
logging.basicConfig(
    level=log_level,
    format="%(asctime)s - %(levelname)s - %(message)s",
    stream=sys.stderr,
)
log = logging.getLogger(__name__)


class PluginLoadError(Exception):
    """Custom exception for plugin loading errors"""


class PluginRegistry:
    """Manages plugin registration and lookup"""

    def __init__(self):
        self.coprocessors: dict[str, type[CoProcessorBase]] = {}
        self.driving_processors: dict[str, type[DrivingProcessorBase]] = {}
        self.plugin_descriptions: dict[str, str] = {}
        self.plugin_styles: dict[str, str] = {}

    def register_plugin(self, cls: type[Any]) -> None:
        """Register a plugin class with the registry"""
        try:
            # Create temporary instance to get metadata
            instance = cls()
            name = getattr(instance, "name", "")
            description = getattr(instance, "description", "")

            if not name or not description:
                log.warning(
                    "Skipping plugin %s: missing name or description",
                    cls.__name__,
                )
                return

            if issubclass(cls, CoProcessorBase):
                self.coprocessors[name] = cls
                self.plugin_styles[name] = "coprocessor"
                self.plugin_descriptions[name] = description
                log.debug("Registered coprocessor: %s", name)

            elif issubclass(cls, DrivingProcessorBase):
                self.driving_processors[name] = cls
                self.plugin_styles[name] = "driving_processor"
                self.plugin_descriptions[name] = description
                log.debug("Registered driving processor: %s", name)

        except Exception as e:
            log.exception("Error registering plugin %s", cls.__name__)
            msg = f"Failed to register plugin {cls.__name__}: {e}"
            raise PluginLoadError(msg) from e

    def get_plugin_info(self) -> list[tuple[str, str, str]]:
        """Get information about all registered plugins"""
        plugin_info = []
        for name in sorted(
            set(self.coprocessors.keys()) | set(self.driving_processors.keys()),
        ):
            style = self.plugin_styles.get(name, "unknown")
            description = self.plugin_descriptions.get(name, "")
            plugin_info.append((name, style, description))
        return plugin_info

    def get_plugin(self, name: str) -> type[Any] | None:
        """Get a plugin by name"""
        return self.coprocessors.get(name) or self.driving_processors.get(name)


def _validate_spec(spec: Any, path: str) -> None:
    """Validate the module spec"""
    if spec is None or spec.loader is None:
        msg = f"Could not load spec for {path}"
        raise PluginLoadError(msg)


def _validate_plugin_style(style: str) -> None:
    """Validate the plugin style"""
    if style not in ["coprocessor", "driving_processor"]:
        msg = f"Invalid plugin style: {style}"
        raise ValueError(msg)


def _validate_command_type(command_type: str) -> None:
    """Validate the command type"""
    if command_type not in ["Execute", "ListPlugins", "Shutdown"]:
        msg = f"Unknown command type: {command_type}"
        raise ValueError(msg)


class PluginService:
    """Manages plugin execution and lifecycle"""

    def __init__(self):
        self.registry = PluginRegistry()
        self.instances: dict[str, Any] = {}

    def load_plugin_file(self, path: str) -> None:
        """Load plugins from a Python file"""
        try:
            # Create module name from filename
            module_name = Path(path).stem

            # Load module from file
            spec = spec_from_file_location(module_name, path)
            _validate_spec(spec, path)

            module = module_from_spec(spec)
            spec.loader.exec_module(module)  # type: ignore

            # Find and register plugin classes
            for item_name in dir(module):
                item = getattr(module, item_name)
                if (
                    isinstance(item, type)
                    and (
                        issubclass(item, CoProcessorBase)
                        or issubclass(item, DrivingProcessorBase)
                    )
                    and item not in (CoProcessorBase, DrivingProcessorBase)
                ):
                    self.registry.register_plugin(item)

        except Exception as e:
            log.exception("Error loading plugin file %s", path)
            msg = f"Failed to load plugin file {path}: {e}"
            raise PluginLoadError(msg) from e

    def load_plugins_from_directory(self, directory: str) -> None:
        """Load all plugins from a directory"""
        if not Path(directory):
            log.warning("Plugin directory does not exist: %s", directory)
            return

        for filename in sorted(os.listdir(directory)):
            if filename.endswith(".py"):
                path = Path(directory) / filename
                try:
                    log.debug("Loading plugins from: %s", filename)
                    self.load_plugin_file(path)
                except PluginLoadError:
                    log.exception("Failed to load %s", filename)

    def get_or_create_instance(self, name: str) -> Any:
        """Get or create a plugin instance"""
        if name not in self.instances:
            plugin_class = self.registry.get_plugin(name)
            if plugin_class is None:
                msg = f"Plugin not found: {name}"
                raise ValueError(msg)
            self.instances[name] = plugin_class()
        return self.instances[name]

    def process_command(self, command: dict[str, Any]) -> dict[str, Any]:
        """Process a command from the Rust side"""
        command_type = command.get("type")

        try:
            _validate_command_type(command_type)

            if command_type == "Execute":
                payload = command["payload"]
                operation = payload["operation"]
                style = payload["style"].lower()
                args = payload.get("args", [])

                instance = self.get_or_create_instance(operation)

                _validate_plugin_style(style)

                if style == "coprocessor":
                    result = instance.process({"numbers": args})
                    if isinstance(result, dict) and "numbers" in result:
                        first_num = result["numbers"][0] if result["numbers"] else 0
                        return {"type": "Result", "value": first_num}

                elif style == "driving_processor":
                    stage, value = instance.start({"numbers": args})
                    while stage == "needs_coprocessing":
                        stage, value = instance.continue_processing(value)
                    if isinstance(value, dict) and "numbers" in value:
                        first_num = value["numbers"][0] if value["numbers"] else 0
                        return {"type": "Result", "value": first_num}

            elif command_type == "ListPlugins":
                plugins = self.registry.get_plugin_info()
                return {
                    "type": "PluginList",
                    "plugins": [
                        {"name": name, "style": style, "description": desc}
                        for name, style, desc in plugins
                    ],
                }

            elif command_type == "Shutdown":
                return {"type": "Result", "value": 0}

        except Exception as e:
            log.exception("Error processing command")
            return {"type": "Error", "message": str(e)}

        return {"type": "Error", "message": "Invalid command processing path"}


def main() -> None:
    """Main entry point for the Python service"""
    try:
        service = PluginService()
        log.debug("Python service starting...")

        # Load plugins from all directories specified as command-line arguments
        plugin_dirs = sys.argv[1:] if len(sys.argv) > 1 else ["plugins"]
        for plugin_dir in plugin_dirs:
            service.load_plugins_from_directory(plugin_dir)

        # Process commands from stdin
        while True:
            try:
                line = sys.stdin.readline()
                if not line:
                    break

                command = json.loads(line)
                response = service.process_command(command)

                if command.get("type") == "Shutdown":
                    break

                print(json.dumps(response), flush=True)

            except json.JSONDecodeError:
                log.exception("Invalid JSON input")
                print(
                    json.dumps({"type": "Error", "message": "Invalid JSON"}),
                    flush=True,
                )
            except Exception as e:
                log.exception("Error processing input")
                print(json.dumps({"type": "Error", "message": str(e)}), flush=True)

    except Exception:
        log.exception("Critical error in Python service")
        sys.exit(1)


if __name__ == "__main__":
    main()
