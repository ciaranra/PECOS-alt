"""Runtime patch to ensure PECOS sim() uses Bridge plugin automatically.

This module patches the Selene build process used by PECOS to automatically
configure Selene instances to use the Bridge plugin at runtime.
"""

import logging
import os
from typing import Any

logger = logging.getLogger(__name__)


class BridgeEnabledSeleneInstance:
    """Wrapper for Selene instances that automatically uses Bridge plugin."""
    
    def __init__(self, original_instance):
        self.original_instance = original_instance
        self.bridge_plugin = None
        
        # Import Bridge plugin if available
        try:
            from pecos.selene_plugins.simulators import PecosBridgePlugin
            self.bridge_plugin = PecosBridgePlugin()
            logger.info(f"BridgeEnabledSeleneInstance: Bridge plugin loaded")
        except ImportError:
            logger.warning("BridgeEnabledSeleneInstance: Bridge plugin not available")
    
    def __getattr__(self, name):
        """Delegate all other attributes to original instance."""
        return getattr(self.original_instance, name)
    
    def run(self, *args, **kwargs):
        """Enhanced run method that uses Bridge plugin automatically."""
        # Set SELENE_IPC for Bridge plugin
        os.environ['SELENE_IPC'] = '1'
        
        if self.bridge_plugin is not None:
            # If no simulator specified, use Bridge plugin
            if len(args) == 0 or not hasattr(args[0], 'library_file'):
                logger.info("PECOS: Automatically using Bridge plugin for Selene execution")
                return self.original_instance.run(self.bridge_plugin, *args, **kwargs)
        
        # Fall back to original behavior
        return self.original_instance.run(*args, **kwargs)


def patch_selene_build_for_pecos():
    """Patch selene_sim.build to return Bridge-enabled instances."""
    try:
        import selene_sim
        
        # Store original build if not already patched
        if not hasattr(selene_sim, '_pecos_original_build'):
            selene_sim._pecos_original_build = selene_sim.build
            
            def bridge_enabled_build(*args, **kwargs):
                """Build function that returns Bridge-enabled instances."""
                # Call original build
                instance = selene_sim._pecos_original_build(*args, **kwargs)
                
                # Wrap with Bridge-enabled functionality
                bridge_instance = BridgeEnabledSeleneInstance(instance)
                
                logger.info("PECOS: Created Bridge-enabled Selene instance")
                return bridge_instance
            
            # Replace build function
            selene_sim.build = bridge_enabled_build
            logger.info("PECOS: Patched selene_sim.build for automatic Bridge plugin usage")
            
        return True
        
    except ImportError as e:
        logger.warning(f"Could not patch selene_sim.build for Bridge plugin: {e}")
        return False


def ensure_bridge_runtime_integration():
    """Ensure Bridge plugin runtime integration is active."""
    return patch_selene_build_for_pecos()


# Automatically apply the patch when this module is imported
_runtime_patched = ensure_bridge_runtime_integration()
if _runtime_patched:
    logger.info("PECOS: Bridge runtime integration activated")