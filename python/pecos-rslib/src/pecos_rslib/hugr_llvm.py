"""
HUGR/LLVM functionality using Rust backend

This module provides Python access to HUGR compilation and LLVM engine functionality
implemented in Rust for high performance.
"""

from typing import Optional, List, Tuple, Union
import warnings

try:
    from ._pecos_rslib import (
        HugrCompiler,
        HugrLlvmEngine,
        is_hugr_support_available,
        compile_hugr_bytes_to_llvm,
        compile_hugr_file_to_llvm,
    )

    RUST_HUGR_AVAILABLE = True
except ImportError as e:
    warnings.warn(f"Rust HUGR backend not available: {e}", stacklevel=2)
    RUST_HUGR_AVAILABLE = False

    # Provide stub classes for graceful degradation
    class HugrCompiler:
        def __init__(self, *args: object, **kwargs: object) -> None:
            raise ImportError("Rust HUGR backend not available")

    class HugrLlvmEngine:
        def __init__(self, *args: object, **kwargs: object) -> None:
            raise ImportError("Rust HUGR backend not available")

    def is_hugr_support_available() -> bool:
        return False

    def compile_hugr_bytes_to_llvm(*args: object, **kwargs: object) -> None:
        raise ImportError("Rust HUGR backend not available")

    def compile_hugr_file_to_llvm(*args: object, **kwargs: object) -> None:
        raise ImportError("Rust HUGR backend not available")


class RustHugrCompiler:
    """
    High-performance HUGR to LLVM compiler using Rust backend.

    This class provides a Python interface to the Rust-implemented HUGR compiler,
    offering better performance than pure Python implementations.
    """

    def __init__(self):
        """
        Initialize the HUGR compiler.
        """
        if not RUST_HUGR_AVAILABLE:
            raise ImportError("Rust HUGR backend not available")

        self._compiler = HugrCompiler()

    def compile_bytes_to_llvm(self, hugr_bytes: bytes) -> str:
        """
        Compile HUGR bytes to LLVM IR string.

        Args:
            hugr_bytes: HUGR data as bytes

        Returns:
            LLVM IR as string
        """
        return self._compiler.compile_bytes_to_llvm(hugr_bytes)

    def compile_file_to_llvm(self, hugr_path: str, llvm_path: str) -> None:
        """
        Compile HUGR file to LLVM IR file.

        Args:
            hugr_path: Path to input HUGR file
            llvm_path: Path for output LLVM IR file
        """
        self._compiler.compile_file_to_llvm(hugr_path, llvm_path)


class RustHugrLlvmEngine:
    """
    High-performance LLVM engine created from HUGR using Rust backend.

    This class provides a Python interface to LLVM engines compiled from HUGR,
    with execution handled by the Rust-implemented PECOS LLVM runtime.
    """

    def __init__(
        self,
        hugr_bytes: bytes,
        shots: int = 1000,
    ):
        """
        Create LLVM engine from HUGR bytes.

        Args:
            hugr_bytes: HUGR data as bytes
            shots: Number of shots to execute
        """
        if not RUST_HUGR_AVAILABLE:
            raise ImportError("Rust HUGR backend not available")

        self._engine = HugrLlvmEngine(hugr_bytes, shots)

    @classmethod
    def from_file(
        cls,
        hugr_path: str,
        shots: int = 1000,
    ) -> "RustHugrLlvmEngine":
        """
        Create LLVM engine from HUGR file.

        Args:
            hugr_path: Path to HUGR file
            shots: Number of shots to execute

        Returns:
            New RustHugrLlvmEngine instance
        """
        if not RUST_HUGR_AVAILABLE:
            raise ImportError("Rust HUGR backend not available")

        instance = cls.__new__(cls)
        instance._engine = HugrLlvmEngine.from_file(hugr_path, shots)
        return instance

    def get_shots(self) -> int:
        """Get number of shots."""
        return self._engine.get_shots()

    def set_shots(self, shots: int) -> None:
        """Set number of shots."""
        self._engine.set_shots(shots)

    def run(self) -> List[int]:
        """
        Run the quantum program.

        Returns:
            List of measurement results
        """
        return list(self._engine.run())

    def __repr__(self) -> str:
        """String representation."""
        return f"RustHugrLlvmEngine(shots={self.get_shots()})"


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


def create_llvm_engine_from_hugr_rust(
    hugr_data: Union[bytes, str],
    shots: int = 1000,
) -> RustHugrLlvmEngine:
    """
    Create LLVM engine from HUGR using Rust backend.

    Args:
        hugr_data: HUGR data as bytes or path to HUGR file
        shots: Number of shots to execute

    Returns:
        RustHugrLlvmEngine instance
    """
    if isinstance(hugr_data, bytes):
        return RustHugrLlvmEngine(hugr_data, shots)
    else:
        return RustHugrLlvmEngine.from_file(hugr_data, shots)


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
    "RustHugrCompiler",
    "RustHugrLlvmEngine",
    "compile_hugr_to_llvm_rust",
    "create_llvm_engine_from_hugr_rust",
    "check_rust_hugr_availability",
    "RUST_HUGR_AVAILABLE",
]
