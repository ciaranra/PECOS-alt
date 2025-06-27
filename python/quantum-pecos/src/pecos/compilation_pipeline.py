"""Clean API for the quantum compilation pipeline.

This module provides a structured interface for the compilation pipeline:
1. Guppy -> HUGR (Python)
2. HUGR -> LLVM/QIR (Rust via PyO3)
3. LLVM/QIR -> Execution (PECOS)
"""

from collections.abc import Callable
from pathlib import Path

from pecos.hugr_types import HugrTypeError


# Step 1: Guppy -> HUGR
def compile_guppy_to_hugr(guppy_function: Callable) -> bytes:
    """Compile a Guppy function to HUGR bytes.

    Args:
        guppy_function: A function decorated with @guppy

    Returns:
        HUGR package as bytes

    Raises:
        ImportError: If guppylang is not available
        ValueError: If function is not a Guppy function
        RuntimeError: If compilation fails
    """
    try:
        from guppylang import guppy as guppy_module
    except ImportError as err:
        msg = (
            "guppylang is not available. Install with: pip install quantum-pecos[guppy]"
        )
        raise ImportError(
            msg,
        ) from err

    # Check if this is a Guppy function
    is_guppy = (
        hasattr(guppy_function, "_guppy_compiled")
        or hasattr(guppy_function, "name")
        or str(type(guppy_function)).find("GuppyDefinition") != -1
    )

    if not is_guppy:
        msg = "Function must be decorated with @guppy"
        raise ValueError(msg)

    try:
        # Compile the function
        compiled = guppy_module.compile_function(guppy_function)
        return compiled.package.to_bytes()
    except Exception as e:
        msg = f"Failed to compile Guppy to HUGR: {e}"
        raise RuntimeError(msg) from e


# Step 2: HUGR -> LLVM/QIR
def compile_hugr_to_llvm(
    hugr_bytes: bytes,
    *,
    debug_info: bool = False,
) -> str:
    """Compile HUGR bytes to LLVM IR string.

    Args:
        hugr_bytes: HUGR package as bytes
        debug_info: Whether to include debug information

    Returns:
        LLVM IR as string (HUGR convention)

    Raises:
        ImportError: If Rust HUGR backend is not available
        RuntimeError: If compilation fails
    """
    try:
        from pecos_rslib import compile_hugr_to_qir_rust

        rust_backend_available = True
    except ImportError:
        rust_backend_available = False

    if rust_backend_available:
        try:
            return compile_hugr_to_qir_rust(
                hugr_bytes,
                None,
                debug_info,
            )
        except RuntimeError as e:
            error_msg = str(e)
            if "Unknown type:" in error_msg:
                raise HugrTypeError(error_msg) from e
            msg = f"Failed to compile HUGR to LLVM: {e}"
            raise RuntimeError(msg) from e
    else:
        # Try our execute_llvm module as fallback
        try:
            from pecos import execute_llvm

            return execute_llvm.compile_module_to_string(hugr_bytes)
        except Exception as e:
            msg = "No HUGR backend available. Build PECOS with HUGR support."
            raise ImportError(
                msg,
            ) from e


# Step 3: Execute LLVM/QIR
def execute_llvm(
    llvm_ir: str | Path,
    shots: int = 1000,
    config: dict | None = None,  # noqa: ARG001
) -> dict:
    """Execute LLVM IR/QIR code.

    Args:
        llvm_ir: LLVM IR as string or path to file
        shots: Number of shots to run
        config: Optional execution configuration

    Returns:
        Execution results dictionary

    Raises:
        ImportError: If execution backend is not available
        RuntimeError: If execution fails
    """
    try:
        from pecos_rslib import QirEngine
    except ImportError as err:
        msg = "QIR execution backend not available"
        raise ImportError(msg) from err

    # If llvm_ir is a path, read the file
    if isinstance(llvm_ir, str | Path) and Path(llvm_ir).exists():
        with Path(llvm_ir).open() as f:
            llvm_ir_str = f.read()
    else:
        llvm_ir_str = str(llvm_ir)

    try:
        # Create engine and run
        engine = QirEngine.from_qir_string(llvm_ir_str, shots)
        results = engine.run()
    except Exception as e:
        msg = f"Failed to execute LLVM/QIR: {e}"
        raise RuntimeError(msg) from e
    else:
        return {
            "results": results,
            "shots": shots,
            "backend": "pecos_qir",
        }


# Convenience functions for common pipelines
def compile_guppy_to_llvm(
    guppy_function: Callable,
    *,
    debug_info: bool = False,
) -> str:
    """Compile a Guppy function directly to LLVM IR.

    Args:
        guppy_function: A function decorated with @guppy
        debug_info: Whether to include debug information

    Returns:
        LLVM IR as string (HUGR convention)
    """
    hugr_bytes = compile_guppy_to_hugr(guppy_function)
    return compile_hugr_to_llvm(hugr_bytes, debug_info=debug_info)


def run_guppy_function(
    guppy_function: Callable,
    shots: int = 1000,
    *,
    debug_info: bool = False,
) -> dict:
    """Compile and execute a Guppy function.

    Args:
        guppy_function: A function decorated with @guppy
        shots: Number of shots to run
        debug_info: Whether to include debug information

    Returns:
        Execution results dictionary
    """
    llvm_ir = compile_guppy_to_llvm(
        guppy_function,
        debug_info=debug_info,
    )
    return execute_llvm(llvm_ir, shots)


# Export all functions
__all__ = [
    # Core pipeline functions
    "compile_guppy_to_hugr",
    "compile_guppy_to_llvm",
    "compile_hugr_to_llvm",
    "execute_llvm",
    # Convenience functions
    "run_guppy_function",
]
