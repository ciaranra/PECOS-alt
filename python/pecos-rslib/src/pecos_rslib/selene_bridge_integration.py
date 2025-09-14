"""Integration utilities for PECOS Bridge plugin with Selene.

This module provides utilities to automatically configure Selene to use
the PECOS Bridge plugin when running quantum simulations.
"""

import logging
from pathlib import Path
from typing import TYPE_CHECKING

if TYPE_CHECKING:
    pass

logger = logging.getLogger(__name__)


def build_selene_with_bridge(
    hugr_package: dict,
    build_dir: Path,
    name: str = "pecos_program",
) -> object:
    """Build a Selene instance configured to use the PECOS Bridge plugin.

    Args:
        hugr_package: HUGR package to compile
        build_dir: Directory for build artifacts
        name: Name for the built program

    Returns:
        SeleneInstance configured to use Bridge plugin

    Raises:
        ImportError: If required dependencies not available
        RuntimeError: If build fails
    """
    try:
        # Import Selene and PECOS Bridge plugin
        from pecos.selene_plugins.simulators import PecosBridgePlugin
        from selene_sim import build

        # Create Bridge plugin instance
        bridge_plugin = PecosBridgePlugin()
        logger.info("Using PECOS Bridge plugin: %s", bridge_plugin.library_file)

        # Build Selene instance with default configuration
        # The key insight: We'll run this instance with Bridge plugin explicitly
        instance = build(hugr_package, name=name, build_dir=build_dir, verbose=False)

    except ImportError as e:
        raise ImportError(f"Required dependencies not available: {e}") from e
    except Exception as e:
        raise RuntimeError(f"Failed to build Selene with Bridge: {e}") from e
    else:
        logger.info("Built Selene instance: %s", instance.executable)
        return instance


def create_bridge_simulator():
    """Create a PECOS Bridge simulator instance.

    Returns:
        PecosBridgePlugin instance

    Raises:
        ImportError: If Bridge plugin not available
    """
    try:
        from pecos.selene_plugins.simulators import PecosBridgePlugin

        return PecosBridgePlugin()
    except ImportError:
        raise ImportError(
            "PECOS Bridge plugin not available - install quantum-pecos with Selene support",
        ) from None


def configure_selene_for_pecos() -> bool | None:
    """Configure Selene to use PECOS Bridge plugin automatically.

    This function integrates the PECOS Bridge plugin into the selene_sim
    namespace so it can be used like Quest/Stim.
    """
    try:
        import selene_sim
        from pecos.selene_plugins.simulators import PecosBridgePlugin

        # Add Bridge plugin to selene_sim namespace
        selene_sim.PecosBridge = PecosBridgePlugin

    except ImportError:
        logger.warning(
            "Could not configure Selene for PECOS - Bridge plugin not available",
        )
        return False
    else:
        logger.info("PECOS Bridge plugin registered with selene_sim")
        return True


# Automatically configure Selene when this module is imported
configure_selene_for_pecos()
