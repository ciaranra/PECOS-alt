"""PecosSeleneBridgeSimulator wrapper for Python.

This module provides a Python wrapper for the PecosSeleneBridgeSimulator
that can be used with Selene's build() and run() APIs.
"""

import os
from pathlib import Path
from typing import Optional, List
from selene_core.simulator import Simulator


class PecosSeleneBridgeSimulator(Simulator):
    """Python wrapper for the PecosSeleneBridgeSimulator plugin.
    
    This simulator acts as a bridge between Selene's execution model
    and PECOS's ByteMessage system.
    """
    
    def __init__(self, plugin_path: Optional[Path] = None):
        """Initialize the bridge simulator.
        
        Args:
            plugin_path: Path to the bridge plugin library. If None,
                        will attempt to auto-detect from standard locations.
        """
        super().__init__()
        self._plugin_path = plugin_path or self._find_bridge_plugin()
        
    def _find_bridge_plugin(self) -> Path:
        """Find the bridge plugin library in standard locations."""
        # Look for the plugin in standard build locations
        possible_paths = [
            # Development build
            Path("target/debug/libpecos_selene_bridge.so"),
            Path("target/debug/libpecos_selene_bridge.dylib"),
            Path("target/debug/pecos_selene_bridge.dll"),
            # Release build
            Path("target/release/libpecos_selene_bridge.so"),
            Path("target/release/libpecos_selene_bridge.dylib"),
            Path("target/release/pecos_selene_bridge.dll"),
        ]
        
        # Check from PECOS root directory
        pecos_root = Path(__file__).parent.parent.parent.parent.parent
        for path in possible_paths:
            full_path = pecos_root / path
            if full_path.exists():
                return full_path
                
        # Also check absolute paths from current directory
        for path in possible_paths:
            if path.exists():
                return path
                
        raise FileNotFoundError(
            "Could not find PecosSeleneBridgeSimulator plugin library. "
            "Make sure to build it with: cargo build --package pecos-selene-bridge"
        )
    
    @property
    def library_file(self) -> Path:
        """Return the path to the bridge plugin library."""
        return self._plugin_path
    
    @property
    def library_search_dirs(self) -> List[Path]:
        """Return additional library search directories."""
        # Include the directory containing the plugin
        return [self._plugin_path.parent]
    
    def get_init_args(self) -> List:
        """Return initialization arguments for the simulator.
        
        The bridge simulator doesn't need special init args.
        """
        return []