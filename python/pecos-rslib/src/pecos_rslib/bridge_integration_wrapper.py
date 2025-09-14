"""Integration wrapper to ensure PECOS sim() uses Bridge plugin automatically.

This module provides a monkey-patch approach to integrate the Bridge plugin
seamlessly with the existing PECOS sim() infrastructure.
"""

import logging
from typing import TYPE_CHECKING
from collections.abc import Callable

if TYPE_CHECKING:
    from pecos_rslib.sim_wrapper import ProgramType

logger = logging.getLogger(__name__)


def patch_selene_for_bridge() -> bool:
    """Patch Selene to automatically use PECOS Bridge plugin.

    This function modifies the selene_sim module to make the Bridge plugin
    available and used by default when building Selene instances for PECOS.
    """
    try:
        # Import required modules
        import selene_sim
        from pecos.selene_plugins.simulators import PecosBridgePlugin
    except ImportError as e:
        logger.warning("Could not patch Selene for Bridge plugin: %s", e)
        return False
    else:
        # Add Bridge plugin to selene_sim namespace
        selene_sim.PecosBridge = PecosBridgePlugin

        # Store the original build function
        if not hasattr(selene_sim, "_original_build"):
            selene_sim._original_build = selene_sim.build

            def pecos_aware_build(*args: object, **kwargs: object) -> object:
                """Modified build function that's aware of PECOS integration."""
                # Call the original build
                instance = selene_sim._original_build(*args, **kwargs)

                # Tag the instance as PECOS-compatible
                instance._pecos_compatible = True
                instance._pecos_bridge_available = True

                return instance

            # Replace build function
            selene_sim.build = pecos_aware_build

        logger.info("Successfully patched Selene for PECOS Bridge plugin integration")
        return True


def ensure_bridge_integration() -> None:
    """Ensure Bridge plugin integration is set up.

    This function should be called during PECOS initialization to ensure
    the Bridge plugin is properly integrated.
    """
    return patch_selene_for_bridge()


def get_integrated_sim_function() -> Callable[[object], object]:
    """Get the sim() function with Bridge plugin integration.

    Returns:
        Enhanced sim function that automatically uses Bridge plugin
    """
    # Import the standard sim function
    from pecos_rslib._pecos_rslib import sim as rust_sim

    def integrated_sim(program: "ProgramType"):
        """Integrated sim() function with automatic Bridge plugin usage."""
        # Ensure Bridge plugin is available
        ensure_bridge_integration()

        # Call the standard sim function
        return rust_sim(program)

    return integrated_sim


# Automatically patch when this module is imported
_patched = ensure_bridge_integration()
