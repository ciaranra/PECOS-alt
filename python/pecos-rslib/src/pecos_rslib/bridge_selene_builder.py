"""Custom Selene builder that uses PECOS Bridge plugin by default.

This module provides a modified Selene build process that automatically
configures the built executable to use the PECOS Bridge plugin for simulation.
"""

import tempfile
import yaml
from pathlib import Path
from typing import Any, Dict, Optional
import logging

logger = logging.getLogger(__name__)


def build_selene_with_bridge_config(hugr_package, name: str = "pecos_program", build_dir: Optional[Path] = None) -> Any:
    """Build a Selene executable configured to use PECOS Bridge plugin.
    
    This creates a Selene configuration that specifies the Bridge plugin
    as the default simulator, then builds the executable with that configuration.
    
    Args:
        hugr_package: HUGR package to build
        name: Program name
        build_dir: Build directory (optional)
        
    Returns:
        SeleneInstance configured with Bridge plugin
    """
    from selene_sim import build
    
    # Create build directory if not provided
    if build_dir is None:
        build_dir = Path(tempfile.mkdtemp(prefix=f"pecos_selene_{name}_"))
    
    try:
        # Get Bridge plugin path
        from pecos.selene_plugins.simulators import PecosBridgePlugin
        bridge_plugin = PecosBridgePlugin()
        bridge_lib_path = bridge_plugin.library_file
        
        logger.info(f"Building Selene with Bridge plugin: {bridge_lib_path}")
        
        # Create a custom configuration that uses Bridge plugin
        config = create_bridge_config(bridge_lib_path, name)
        
        # Write config to temporary file
        config_file = build_dir / "selene_bridge_config.yaml"
        with open(config_file, 'w') as f:
            yaml.dump(config, f)
        
        logger.info(f"Created Bridge configuration: {config_file}")
        
        # Build with custom configuration
        # Note: This approach may need modification based on Selene's actual config API
        instance = build(
            hugr_package,
            name=name,
            build_dir=build_dir,
            verbose=False,
            # TODO: Figure out how to pass custom config to Selene build
        )
        
        # Store Bridge plugin reference for later use
        instance._pecos_bridge_plugin = bridge_plugin
        
        logger.info(f"Built Selene executable with Bridge plugin configuration")
        return instance
        
    except ImportError:
        # Fall back to standard build if Bridge plugin not available
        logger.warning("Bridge plugin not available, using standard Selene build")
        return build(hugr_package, name=name, build_dir=build_dir, verbose=False)


def create_bridge_config(bridge_lib_path: Path, program_name: str) -> Dict[str, Any]:
    """Create a Selene configuration that uses the Bridge plugin.
    
    Args:
        bridge_lib_path: Path to Bridge plugin library
        program_name: Name of the program
        
    Returns:
        Configuration dict for Selene
    """
    return {
        'n_qubits': 10,  # Default, will be overridden at runtime
        'output_stream': 'stdout',
        'artifact_dir': '/tmp',  # Will be overridden
        'simulator': {
            'name': 'pecos_selene_bridge',
            'file': str(bridge_lib_path),
            'args': []
        },
        'error_model': {
            'name': 'ideal',
            'file': 'libselene_ideal_error_model_plugin.so',  # Selene default
            'args': []
        },
        'runtime': {
            'name': 'simple',
            'file': 'libselene_simple_runtime_plugin.so',  # Selene default
            'args': []
        },
        'event_hooks': {
            'provide_instruction_log': False,
            'provide_metrics': False
        },
        'shots': {
            'count': 1,
            'offset': 0,
            'increment': 1
        }
    }


def get_bridge_simulator_for_instance(instance) -> Any:
    """Get the Bridge simulator associated with a Selene instance.
    
    Args:
        instance: SeleneInstance built with Bridge plugin
        
    Returns:
        PecosBridgePlugin instance to use for running
    """
    if hasattr(instance, '_pecos_bridge_plugin'):
        return instance._pecos_bridge_plugin
    else:
        # Create new Bridge plugin instance
        from pecos.selene_plugins.simulators import PecosBridgePlugin
        return PecosBridgePlugin()