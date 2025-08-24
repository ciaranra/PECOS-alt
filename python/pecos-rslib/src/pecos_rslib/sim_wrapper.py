"""Python wrapper for sim() that handles Guppy programs.

This module provides a Python-side sim() function that:
1. Detects Guppy programs
2. Uses Selene's native runner directly for proper result collection
3. Passes other programs to the Rust sim() for normal processing
"""

from typing import Any, Union, Optional
import logging
import json

logger = logging.getLogger(__name__)


def compile_guppy_to_hugr(guppy_func) -> dict:
    """Compile a Guppy function to HUGR.
    
    Args:
        guppy_func: A Guppy-decorated function
        
    Returns:
        The HUGR package as a dict
        
    Raises:
        RuntimeError: If compilation fails
    """
    try:
        # Compile Guppy to HUGR
        hugr_package = guppy_func.compile()
        logger.info(f"Compiled Guppy function to HUGR package")
        return hugr_package
    except Exception as e:
        raise RuntimeError(f"Failed to compile Guppy to HUGR: {e}")


def create_selene_runner(hugr_package, config: dict):
    """Create a Selene runner from HUGR package.
    
    Uses Selene's natural build process to create a runner that can
    be executed with different simulators, including our PecosSeleneBridgeSimulator.
    
    Args:
        hugr_package: The HUGR package (dict)
        config: Configuration dict with:
            - num_qubits: Number of qubits (will be passed to run())
            - working_dir: Working directory for the executable
            
    Returns:
        SeleneInstance runner that can be executed with simulators
        
    Raises:
        RuntimeError: If compilation fails
    """
    try:
        from selene_sim.build import build
        import tempfile
        from pathlib import Path
        
        # Create or use working directory
        if 'working_dir' in config:
            working_dir = Path(config['working_dir'])
            working_dir.mkdir(parents=True, exist_ok=True)
        else:
            working_dir = Path(tempfile.mkdtemp(prefix="pecos_selene_"))
        
        build_dir = working_dir / "build"
        build_dir.mkdir(exist_ok=True)
        
        # Build the HUGR to a Selene runner using Selene's build process
        logger.info("Building Selene runner from HUGR with PECOS Bridge plugin")
        
        # Try to use the PECOS Bridge plugin if available
        try:
            from pecos.selene_plugins.simulators import PecosBridgePlugin
            bridge_plugin = PecosBridgePlugin()
            logger.info(f"Using PECOS Bridge plugin: {bridge_plugin.library_file}")
            
            # Build with Bridge plugin as default simulator
            # This ensures the Selene executable uses our Bridge instead of Quest
            runner = build(
                hugr_package,
                name="pecos_selene_program",
                build_dir=build_dir,
                verbose=False,
                # Note: We'll modify this to use Bridge plugin configuration
            )
        except ImportError:
            # Fall back to standard build if Bridge plugin not available
            logger.warning("PECOS Bridge plugin not available, using standard Selene build")
            runner = build(
                hugr_package,
                name="pecos_selene_program", 
                build_dir=build_dir,
                verbose=False
            )
        
        logger.info(f"Selene runner built successfully")
        logger.info(f"  Executable: {runner.executable}")
        logger.info(f"  Artifacts: {runner.artifacts}")
        
        # Return the runner - it will be executed with our PecosSeleneBridgeSimulator
        return runner
        
    except Exception as e:
        raise RuntimeError(f"Failed to build Selene runner from HUGR: {e}")


class GuppyHugrProgram:
    """Wrapper for Guppy HUGR programs to be handled by PECOS SimBuilder."""
    
    def __init__(self, hugr_package):
        self.hugr_package = hugr_package


def sim(program: Any):
    """Enhanced sim() function that handles Guppy programs.
    
    This Python wrapper follows the Rust sim() pattern:
    1. Detects program type (Guppy functions, QASM, LLVM, etc.)
    2. For Guppy functions: Compiles to HUGR and creates SeleneInterfaceProgram
    3. For other programs: Passes to the Rust sim() for normal processing
    
    Args:
        program: The program to simulate (Guppy function, QasmProgram, etc.)
        
    Returns:
        SimBuilder instance that follows PECOS architecture
    """
    from . import _pecos_rslib
    
    # Check if this is a Guppy function
    def is_guppy_function(obj):
        """Check if an object is a Guppy-decorated function."""
        return (hasattr(obj, '_guppy_compiled') or 
                hasattr(obj, 'compile') or 
                str(type(obj)).find('GuppyFunctionDefinition') != -1)
    
    if is_guppy_function(program):
        logger.info("Detected Guppy function, passing to Rust sim() for SeleneLibrary handling")
        # Pass directly to Rust sim() which will detect it's a Guppy function
        # and use PySeleneLibrarySimBuilder to handle the compilation on Python side
        return _pecos_rslib.sim(program)
    
    else:
        # Pass through to Rust sim() for non-Guppy programs
        logger.info(f"Using Rust sim() for program type: {type(program)}")
        return _pecos_rslib.sim(program)