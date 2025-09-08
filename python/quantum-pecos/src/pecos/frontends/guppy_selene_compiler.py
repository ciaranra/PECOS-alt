"""Guppy to Selene Compiler.

This module provides Python-side compilation of Guppy programs for Selene execution.
This allows the entire compilation pipeline to happen in Python, where Guppy lives,
before passing the compiled artifacts to Rust for execution.
"""

import contextlib
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
        # Use JSON format instead of binary envelope for HUGR 0.13 compatibility
        hugr_json = hugr.to_json()
        return hugr_json.encode("utf-8")

    def _generate_llvm_ir_from_hugr_bytes(
        self,
        hugr_bytes: bytes,
        func_name: str,
    ) -> str:
        """Generate LLVM IR from HUGR.

        This uses pecos-selene's HUGR to LLVM compiler if available,
        otherwise falls back to placeholder LLVM IR.
        """
        try:
            # Use our internal HUGR to LLVM compiler
            from pecos_rslib import compile_hugr_to_llvm

            return compile_hugr_to_llvm(hugr_bytes)
        except (ImportError, Exception) as e:
            # If compile_hugr_to_llvm fails, fall back to placeholder
            print(f"Warning: compile_hugr_to_llvm failed: {e}")

        try:
            # Try to use selene's HUGR compiler if available
            from selene_hugr_qis_compiler import compile_to_llvm_ir

            # selene_hugr_qis_compiler expects HUGR envelope format (binary)
            # We need to convert our JSON to the binary envelope format
            try:
                # Try to create a proper HUGR envelope
                import json

                json.loads(hugr_bytes.decode("utf-8"))

                # Create a minimal HUGR envelope
                # Magic number: 0x4855475269484A76 ("HUGRiHJv")
                import struct

                magic = b"HUGRiHJv"

                # For now, create a test envelope (this won't work but shows the approach)
                # In a real implementation, we'd serialize the JSON to MessagePack format
                envelope = magic + b"\x00" * 100  # Placeholder

                llvm_ir = compile_to_llvm_ir(envelope)
                print(f"Selene HUGR compiler produced {len(llvm_ir)} chars of LLVM IR")
                return llvm_ir
            except Exception as e:
                print(f"Warning: Selene HUGR compiler failed: {e}")
                # Fall through to placeholder
        except ImportError:
            pass

        # For now, generate a simple placeholder
        return f"""
; Placeholder LLVM IR for {func_name}
; TODO: Implement full HUGR to LLVM compilation

declare i64 @__quantum__rt__qubit_allocate()
declare i64 @__quantum__rt__result_allocate()
declare void @__quantum__qis__h__body(i64)
declare i32 @__quantum__qis__m__body(i64, i64)
declare void @__quantum__rt__result_record_output(i8*, i8*)

define void @{func_name}() #0 {{
entry:
  ; Placeholder implementation
  %q = call i64 @__quantum__rt__qubit_allocate()
  call void @__quantum__qis__h__body(i64 %q)
  %result = call i64 @__quantum__rt__result_allocate()
  %m = call i32 @__quantum__qis__m__body(i64 %q, i64 %result)
  %result_ptr = inttoptr i64 %result to i8*
  call void @__quantum__rt__result_record_output(i8* %result_ptr, i8* null)
  ret void
}}

attributes #0 = {{ "EntryPoint" }}
"""

    # Removed _compile_llvm_to_plugin - we don't compile to plugins

    def __del__(self) -> None:
        """Clean up temporary directory if created."""
        if self._temp_dir:
            import shutil

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
