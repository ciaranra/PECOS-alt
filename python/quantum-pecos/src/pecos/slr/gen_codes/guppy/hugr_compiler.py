"""HUGR compiler for Guppy code generation."""

from __future__ import annotations

import tempfile
import os
from typing import Any, TYPE_CHECKING

from .hugr_error_handler import HugrErrorHandler

if TYPE_CHECKING:
    from .generator import GuppyGenerator

try:
    from guppylang import guppy
    from guppylang.std import quantum
    from guppylang.std.builtins import array, owned, result
    GUPPY_AVAILABLE = True
except ImportError:
    GUPPY_AVAILABLE = False


class HugrCompiler:
    """Compiles generated Guppy code to HUGR."""
    
    def __init__(self, generator: GuppyGenerator):
        """Initialize the HUGR compiler.
        
        Args:
            generator: The GuppyGenerator instance with generated code
        """
        self.generator = generator
        
    def compile_to_hugr(self) -> Any:
        """Compile the generated Guppy code to HUGR.
        
        Returns:
            The compiled HUGR module
            
        Raises:
            ImportError: If guppylang is not available
            RuntimeError: If compilation fails
        """
        if not GUPPY_AVAILABLE:
            raise ImportError(
                "guppylang is not installed. Install it with: pip install guppylang"
            )
            
        # Get the generated Guppy code
        guppy_code = self.generator.get_output()
        
        # Create a temporary file to hold the generated code
        # This is necessary because guppy.compile() needs to be able to inspect the source
        with tempfile.NamedTemporaryFile(mode='w', suffix='.py', delete=False) as f:
            temp_file = f.name
            f.write(guppy_code)
        
        try:
            # Import the module from the temporary file
            import importlib.util
            import sys
            import linecache
            
            # Add the source to linecache for better error tracking
            lines = guppy_code.splitlines(keepends=True)
            linecache.cache[temp_file] = (
                len(guppy_code),
                None,
                lines,
                temp_file
            )
            
            spec = importlib.util.spec_from_file_location("_guppy_generated", temp_file)
            if spec is None or spec.loader is None:
                raise RuntimeError("Failed to create module spec")
                
            module = importlib.util.module_from_spec(spec)
            
            # Ensure the module has proper file tracking
            module.__file__ = temp_file
            
            # Add to sys.modules temporarily to help with source tracking
            sys.modules["_guppy_generated"] = module
            
            spec.loader.exec_module(module)
            
            # Get the main function
            if not hasattr(module, 'main'):
                raise RuntimeError("No main function found in generated code")
                
            main_func = module.main
            
            # Compile to HUGR
            try:
                hugr_module = guppy.compile(main_func)
                return hugr_module
            except Exception as e:
                # Use the enhanced error handler
                error_handler = HugrErrorHandler(guppy_code)
                detailed_error = error_handler.analyze_error(e)
                raise RuntimeError(detailed_error)
                
        finally:
            # Clean up
            try:
                # Remove from sys.modules
                import sys
                if "_guppy_generated" in sys.modules:
                    del sys.modules["_guppy_generated"]
                
                # Clean up the temporary file
                os.unlink(temp_file)
            except:
                pass