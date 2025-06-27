"""Execute LLVM module - PECOS's implementation of HUGR to LLVM compilation.

This module provides the same interface as the external execute_llvm module
but uses PECOS's own HUGR compilation infrastructure.
"""

from pathlib import Path


def compile_module_to_string(hugr_bytes: bytes) -> str:
    """Compile HUGR bytes to LLVM IR string using PECOS infrastructure.

    This function provides compatibility with code expecting the execute_llvm
    interface while using PECOS's own HUGR compilation backends.

    Args:
        hugr_bytes: HUGR module serialized as bytes

    Returns:
        LLVM IR as a string

    Raises:
        RuntimeError: If compilation fails
    """
    # Try to use PECOS's Rust backend first (fastest)
    try:
        from pecos_rslib import compile_hugr_to_qir_rust

        return compile_hugr_to_qir_rust(
            hugr_bytes,
            None,  # output_path
            False,  # debug_info  # noqa: FBT003
        )

    except ImportError:
        # If Rust backend isn't available, try external compiler
        pass
    except RuntimeError:
        # Re-raise runtime errors
        raise

    # Fall back to external HUGR compiler
    try:
        # Check if hugr_llvm_compiler module is available
        import importlib.util

        spec = importlib.util.find_spec("pecos.frontends.hugr_llvm_compiler")
        if spec is None:
            msg = "PECOS HUGR compiler module not available"
            raise RuntimeError(msg) from None

        # Module is available but we won't actually use it here
        # For now, just raise a clear error
        msg = (
            "Rust backend failed. External HUGR compiler would be tried next, "
            "but it requires the hugr_quantum_llvm binary to be built."
        )
        raise RuntimeError(
            msg,
        )

    except ImportError as err:
        msg = "PECOS HUGR compiler module not available"
        raise RuntimeError(msg) from err


def compile_module_to_file(hugr_bytes: bytes, output_path: str | Path) -> None:
    """Compile HUGR bytes to LLVM IR file.

    Args:
        hugr_bytes: HUGR module serialized as bytes
        output_path: Path where the LLVM IR should be written
    """
    llvm_ir = compile_module_to_string(hugr_bytes)
    with Path(output_path).open("w") as f:
        f.write(llvm_ir)


def compile_hugr_file_to_string(hugr_path: str | Path) -> str:
    """Compile HUGR file to LLVM IR string.

    Args:
        hugr_path: Path to HUGR file

    Returns:
        LLVM IR as a string
    """
    with Path(hugr_path).open("rb") as f:
        hugr_bytes = f.read()
    return compile_module_to_string(hugr_bytes)


def compile_hugr_file_to_file(
    hugr_path: str | Path,
    output_path: str | Path,
) -> None:
    """Compile HUGR file to LLVM IR file.

    Args:
        hugr_path: Path to HUGR file
        output_path: Path where the LLVM IR should be written
    """
    llvm_ir = compile_hugr_file_to_string(hugr_path)
    with Path(output_path).open("w") as f:
        f.write(llvm_ir)


def is_available() -> bool:
    """Check if execute_llvm functionality is available.

    Returns:
        True if at least one HUGR->LLVM backend is available, False otherwise
    """
    try:
        # Check Rust backend
        from pecos_rslib import compile_hugr_to_qir_rust  # noqa: F401
    except ImportError:
        pass
    else:
        return True

    try:
        # Check external compiler
        from pecos.frontends.hugr_llvm_compiler import HugrLlvmCompiler

        compiler = HugrLlvmCompiler()
        return compiler.is_available()
    except ImportError:
        return False


# Additional metadata
__all__ = [
    "compile_hugr_file_to_file",
    "compile_hugr_file_to_string",
    "compile_module_to_file",
    "compile_module_to_string",
    "is_available",
]
