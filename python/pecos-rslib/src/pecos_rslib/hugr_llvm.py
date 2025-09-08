"""
HUGR/LLVM functionality using Rust backend

This module provides Python access to HUGR compilation and LLVM engine functionality
implemented in Rust for high performance.
"""

from typing import Optional, Tuple, Union
import warnings

try:
    from ._pecos_rslib import (
        compile_hugr_to_llvm_rust,
        check_rust_hugr_availability,
        RUST_HUGR_AVAILABLE,
    )

    # Create aliases for backward compatibility (can be removed later)
    is_hugr_support_available = check_rust_hugr_availability
    compile_hugr_bytes_to_llvm = compile_hugr_to_llvm_rust

    def compile_hugr_file_to_llvm(hugr_path: str, llvm_path: str) -> None:
        """Compile HUGR file to LLVM IR file"""
        with open(hugr_path, "rb") as f:
            hugr_bytes = f.read()
        compile_hugr_to_llvm_rust(hugr_bytes, llvm_path)

except ImportError as e:
    warnings.warn(f"Rust HUGR backend not available: {e}", stacklevel=2)
    RUST_HUGR_AVAILABLE = False

    def is_hugr_support_available() -> bool:
        return False

    check_rust_hugr_availability = is_hugr_support_available

    def compile_hugr_bytes_to_llvm(*args: object, **kwargs: object) -> None:
        raise ImportError("Rust HUGR backend not available")

    compile_hugr_to_llvm_rust = compile_hugr_bytes_to_llvm

    def compile_hugr_file_to_llvm(*args: object, **kwargs: object) -> None:
        raise ImportError("Rust HUGR backend not available")


# Deprecated: These classes are no longer available in the Rust backend
# Use compile_hugr_to_llvm_rust directly instead


def compile_hugr_to_llvm_rust(
    hugr_data: Union[bytes, str],
    output_path: Optional[str] = None,
) -> Optional[str]:
    """
    Compile HUGR to LLVM IR using Rust backend.

    Args:
        hugr_data: HUGR data as bytes or path to HUGR file
        output_path: Path for output LLVM IR file (if None, returns LLVM IR as string)

    Returns:
        LLVM IR as string if output_path is None, otherwise None
    """
    if not RUST_HUGR_AVAILABLE:
        raise ImportError("Rust HUGR backend not available")

    if isinstance(hugr_data, bytes):
        return compile_hugr_bytes_to_llvm(hugr_data, output_path)
    else:
        # hugr_data is a file path
        if output_path is None:
            # Read file and compile to string
            with open(hugr_data, "rb") as f:
                hugr_bytes = f.read()
            return compile_hugr_bytes_to_llvm(hugr_bytes, None)
        else:
            compile_hugr_file_to_llvm(hugr_data, output_path)
            return None


# Deprecated: RustHugrLlvmEngine is no longer available


def check_rust_hugr_availability() -> Tuple[bool, str]:
    """
    Check if Rust HUGR backend is available.

    Returns:
        Tuple of (is_available, status_message)
    """
    if not RUST_HUGR_AVAILABLE:
        return False, "Rust HUGR backend not compiled or not available"

    if is_hugr_support_available():
        return True, "Rust HUGR backend available with full support"
    else:
        return False, "Rust HUGR backend available but HUGR support not compiled in"


# Export main functionality
__all__ = [
    "compile_hugr_to_llvm_rust",
    "check_rust_hugr_availability",
    "RUST_HUGR_AVAILABLE",
]
