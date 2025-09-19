"""Guppy to Selene Compiler.

This module provides Python-side compilation of Guppy programs for Selene execution.
This allows the entire compilation pipeline to happen in Python, where Guppy lives,
before passing the compiled artifacts to Rust for execution.
"""

import contextlib
import shutil
import tempfile
from collections.abc import Callable
from pathlib import Path

try:
    from guppylang import GuppyModule
    from guppylang.decorator import guppy

    GUPPY_AVAILABLE = True
except ImportError:
    GUPPY_AVAILABLE = False
    GuppyModule = None
    guppy = None

# We don't compile to plugins - Selene uses native executables
# from pecos_rslib import compile_llvm_to_plugin


class GuppySeleneCompiler:
    """Compiles Guppy quantum programs for Selene execution.

    This compiler handles the compilation pipeline in Python:
    1. Guppy function → HUGR (using guppylang)
    2. HUGR → LLVM IR (using pecos-selene with HUGR 0.13)

    The LLVM IR can then be executed by Selene's runtime.
    """

    def __init__(self, output_dir: Path | None = None) -> None:
        """Initialize the compiler.

        Args:
            output_dir: Directory for output files. If None, uses temp directory.
        """
        if not GUPPY_AVAILABLE:
            msg = "guppylang is required for GuppySeleneCompiler"
            raise ImportError(msg)

        self.output_dir = output_dir
        self._temp_dir = None

    def compile_function(self, func: Callable) -> Path:
        """Compile a Guppy function for Selene execution.

        Args:
            func: A Guppy-decorated quantum function

        Returns:
            Path to the output directory containing HUGR and LLVM IR files
        """
        # Get output directory
        if self.output_dir is None:
            if self._temp_dir is None:
                self._temp_dir = tempfile.mkdtemp(prefix="guppy_selene_")
            output_path = Path(self._temp_dir)
        else:
            output_path = self.output_dir
            output_path.mkdir(parents=True, exist_ok=True)

        # Step 1: Compile Guppy to HUGR
        hugr_bytes = self._compile_to_hugr(func)

        # Step 2: Generate LLVM IR from HUGR
        # For now, we'll use a placeholder since full HUGR parsing isn't implemented
        # Get function name from GuppyFunctionDefinition
        func_name = getattr(func, "name", getattr(func, "__name__", "quantum_func"))
        llvm_ir = self._generate_llvm_ir_from_hugr_bytes(hugr_bytes, func_name)

        # Save HUGR file for debugging
        hugr_file = output_path / f"{func_name}.hugr"
        hugr_file.write_bytes(hugr_bytes)

        # Save LLVM IR file
        llvm_file = output_path / f"{func_name}.ll"
        llvm_file.write_text(llvm_ir)

        # Return the output directory containing the compiled artifacts
        return output_path

    def _compile_to_hugr(self, func: Callable) -> bytes:
        """Compile a Guppy function to HUGR bytes."""
        # Compile the function directly
        hugr = func.compile()
        # Convert to HUGR envelope format (binary) that Selene expects
        return hugr.to_bytes()

    def _generate_llvm_ir_from_hugr_bytes(
        self,
        hugr_bytes: bytes,
        _func_name: str,
    ) -> str:
        """Generate LLVM IR from HUGR.

        This uses Selene's HUGR to LLVM compiler which properly handles
        control flow and conditionals.
        """
        try:
            # Use Selene's HUGR compiler which properly handles conditionals
            from selene_hugr_qis_compiler import compile_to_llvm_ir

            # selene_hugr_qis_compiler expects HUGR envelope format (binary)
            # which we now provide directly from _compile_to_hugr
            llvm_ir = compile_to_llvm_ir(hugr_bytes)
            print(f"Selene HUGR compiler produced {len(llvm_ir)} chars of LLVM IR")
        except (ImportError, RuntimeError, ValueError) as e:
            print(f"Warning: Selene HUGR compiler failed: {e}")
            # Fall back to trying our internal compiler
            try:
                from pecos_rslib import compile_hugr_to_llvm

                return compile_hugr_to_llvm(hugr_bytes)
            except (ImportError, RuntimeError, ValueError) as e2:
                print(f"Warning: Internal HUGR compiler also failed: {e2}")
        else:
            return llvm_ir

        # No fallback - if we can't compile HUGR, fail properly
        msg = (
            "Failed to compile HUGR to LLVM: Neither Selene's hugr_qis compiler nor "
            "the internal HUGR compiler is available. Please ensure Selene is properly "
            "installed with: pip install selene-hugr-qis-compiler"
        )
        raise RuntimeError(
            msg,
        )

    # Removed _compile_llvm_to_plugin - we don't compile to plugins

    def __del__(self) -> None:
        """Clean up temporary directory if created."""
        if self._temp_dir:

            with contextlib.suppress(Exception):
                shutil.rmtree(self._temp_dir)


def compile_guppy_for_selene(func: Callable, output_dir: Path | None = None) -> Path:
    """Convenience function to compile a Guppy function for Selene execution.

    Args:
        func: A Guppy-decorated quantum function
        output_dir: Directory for output files. If None, uses temp directory.

    Returns:
        Path to the output directory containing compiled artifacts
    """
    compiler = GuppySeleneCompiler(output_dir)
    return compiler.compile_function(func)
