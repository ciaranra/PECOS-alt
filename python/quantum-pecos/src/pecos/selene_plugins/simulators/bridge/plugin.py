"""PECOS Bridge Plugin for Selene.

This plugin acts as a bridge between Selene's quantum circuit execution
and PECOS's quantum simulation infrastructure, using ByteMessage communication.
"""

import platform
from dataclasses import dataclass
from pathlib import Path

from selene_core import Simulator


@dataclass
class PecosBridgePlugin(Simulator):
    """A plugin for using PECOS quantum simulation infrastructure as the backend for Selene quantum circuit execution.

    This plugin bridges Selene's execution model with PECOS's
    ByteMessage-based quantum simulation system.
    """

    def __post_init__(self) -> None:
        """Post-initialization hook."""

    @property
    def library_file(self) -> Path:
        """Return the path to the PECOS Bridge plugin library."""
        # Look for the plugin library in the standard PECOS locations
        # From python/quantum-pecos/src/pecos/selene_plugins/simulators/bridge/plugin.py, go up to PECOS root
        plugin_dir = Path(__file__).parent  # bridge/
        simulators_dir = plugin_dir.parent  # simulators/
        selene_plugins_dir = simulators_dir.parent  # selene_plugins/
        pecos_dir = selene_plugins_dir.parent  # pecos/
        src_dir = pecos_dir.parent  # src/
        quantum_pecos_dir = src_dir.parent  # quantum-pecos/
        python_dir = quantum_pecos_dir.parent  # python/
        pecos_root = python_dir.parent  # PECOS/

        possible_paths = [
            # Development builds
            pecos_root
            / "target"
            / "debug"
            / f"libpecos_selene_bridge{self._get_lib_extension()}",
            pecos_root
            / "target"
            / "release"
            / f"libpecos_selene_bridge{self._get_lib_extension()}",
            # Installed location (if we package it later)
            plugin_dir
            / "_dist"
            / "lib"
            / f"libpecos_selene_bridge{self._get_lib_extension()}",
        ]

        for path in possible_paths:
            if path.exists():
                return path

        msg = (
            f"Could not find PECOS Bridge plugin library. Searched paths: {possible_paths}\n"
            "Make sure to build it with: cargo build --package pecos-selene-bridge"
        )
        raise FileNotFoundError(
            msg,
        )

    def _get_lib_extension(self) -> str:
        """Get the appropriate library extension for the platform."""
        match platform.system():
            case "Linux":
                return ".so"
            case "Darwin":
                return ".dylib"
            case "Windows":
                return ".dll"
            case _:
                msg = f"Unsupported platform: {platform.system()}"
                raise RuntimeError(msg)

    @property
    def library_search_dirs(self) -> list[Path]:
        """Return additional library search directories."""
        return [self.library_file.parent]

    def get_init_args(self) -> list:
        """Return initialization arguments for the simulator.

        The PECOS Bridge simulator doesn't need special init args.
        """
        return []
