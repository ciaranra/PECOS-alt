"""Execute LLVM module - HUGR to LLVM compilation interface.

This module provides HUGR to LLVM compilation with explicit compiler selection:
- 'selene': Selene's hugr-qis compiler (default)
- 'rust': PECOS's Rust HUGR compiler

No automatic fallback - the specified compiler must be available.
"""

import importlib.util
from pathlib import Path


def compile_module_to_string(hugr_bytes: bytes, compiler: str = "selene") -> str:
    """Compile HUGR bytes to LLVM IR string.

    Args:
        hugr_bytes: HUGR module serialized as bytes
        compiler: Which compiler to use ("selene" or "rust")
                  Default is "selene" for Selene's hugr-qis compiler

    Returns:
        LLVM IR as a string

    Raises:
        RuntimeError: If compilation fails
        ValueError: If invalid compiler specified
    """
    if compiler == "selene":
        try:
            from pecos_rslib import compile_hugr_to_llvm_selene

            return compile_hugr_to_llvm_selene(hugr_bytes)
        except ImportError as e:
            msg = (
                "Selene's HUGR compiler is not available. "
                "Install it with: pip install selene-hugr-qis-compiler"
            )
            raise RuntimeError(
                msg,
            ) from e
    elif compiler == "rust":
        try:
            from pecos_rslib import compile_hugr_to_llvm_rust

            return compile_hugr_to_llvm_rust(hugr_bytes)
        except ImportError as e:
            msg = (
                "PECOS's Rust HUGR compiler is not available. "
                "Build pecos-rslib with hugr-llvm-pipeline feature to enable it."
            )
            raise RuntimeError(
                msg,
            ) from e
    else:
        msg = f"Invalid compiler '{compiler}'. Choose 'selene' or 'rust'."
        raise ValueError(
            msg,
        )


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
    # Check Selene's hugr-qis compiler
    spec = importlib.util.find_spec("selene_hugr_qis_compiler")
    if spec is not None:
        return True

    # Check Rust backend
    spec = importlib.util.find_spec("pecos_rslib.compile_hugr_to_llvm_rust")
    if spec is not None:
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
