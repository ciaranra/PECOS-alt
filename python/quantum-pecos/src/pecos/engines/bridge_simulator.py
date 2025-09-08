"""PECOS-Selene Bridge Simulator Python wrapper.

This module provides a Python wrapper for the PecosSeleneBridgeSimulator
that can be passed to Selene during the build process.
"""

from pathlib import Path
from typing import Any

from selene_core.simulator import Simulator


class PecosBridgeSimulator(Simulator):
    """Python wrapper for the PECOS-Selene Bridge Simulator plugin.

    This simulator acts as a bridge between Selene's execution and PECOS's
    quantum engines, forwarding operations and returning results.
    """

    def __init__(self, random_seed: int | None = None) -> None:
        """Initialize the Bridge simulator.

        Args:
            random_seed: Optional random seed for deterministic behavior
        """
        super().__init__(random_seed=random_seed)
        self._find_plugin_library()

    def _find_plugin_library(self) -> None:
        """Find the compiled Bridge simulator plugin library."""
        # Look for the plugin in common locations
        # Start from this file and go up to find workspace root
        current = Path(__file__).parent
        workspace_root = None

        # Find workspace root by looking for Cargo.toml
        for _ in range(10):  # Don't go up more than 10 levels
            if (current / "Cargo.toml").exists():
                workspace_root = current
                break
            if current.parent == current:  # Reached root
                break
            current = current.parent

        if workspace_root is None:
            workspace_root = Path.cwd()  # Fallback to current directory

        possible_paths = [
            # Development builds (most common location)
            workspace_root / "target" / "debug" / "libpecos_selene_bridge.so",
            workspace_root / "target" / "debug" / "deps" / "libpecos_selene_bridge.so",
            # Release builds
            workspace_root
            / "target"
            / "release"
            / "deps"
            / "libpecos_selene_bridge.so",
            workspace_root / "target" / "release" / "libpecos_selene_bridge.so",
            # macOS
            workspace_root / "target" / "debug" / "libpecos_selene_bridge.dylib",
            workspace_root / "target" / "release" / "libpecos_selene_bridge.dylib",
            # Windows
            workspace_root / "target" / "debug" / "pecos_selene_bridge.dll",
            workspace_root / "target" / "release" / "pecos_selene_bridge.dll",
        ]

        for path in possible_paths:
            if path.exists():
                self._library_file = str(path)
                return

        msg = (
            "Could not find PecosSeleneBridgeSimulator plugin library. "
            "Make sure to build it with: cargo build --package pecos-selene-bridge"
        )
        raise FileNotFoundError(
            msg,
        )

    @property
    def library_file(self) -> str:
        """Path to the Bridge simulator plugin library.

        Returns:
            Absolute path to the compiled plugin .so/.dylib/.dll file
        """
        return self._library_file

    def get_init_args(self) -> dict[str, Any]:
        """Get initialization arguments for the plugin.

        Returns:
            Dictionary of arguments to pass to the plugin's initialization
        """
        # The Bridge simulator might need configuration about which
        # PECOS engine to connect to, but for now we'll use defaults
        return {
            # Could include things like:
            # "quantum_engine": "stabilizer",
            # "noise_model": None,
        }


# Make it easily importable
__all__ = ["PecosBridgeSimulator"]
