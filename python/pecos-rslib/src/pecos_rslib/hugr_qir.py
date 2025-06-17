"""
HUGR/QIR functionality using Rust backend

This module provides Python access to HUGR compilation and QIR engine functionality
implemented in Rust for high performance.
"""

from typing import Optional, List, Tuple, Union
import warnings

try:
    from ._pecos_rslib import (
        HugrCompiler,
        HugrQirEngine,
        is_hugr_support_available,
        get_supported_naming_conventions,
        compile_hugr_bytes_to_qir,
        compile_hugr_file_to_qir,
    )
    RUST_HUGR_AVAILABLE = True
except ImportError as e:
    warnings.warn(f"Rust HUGR backend not available: {e}")
    RUST_HUGR_AVAILABLE = False
    
    # Provide stub classes for graceful degradation
    class HugrCompiler:
        def __init__(self, *args, **kwargs):
            raise ImportError("Rust HUGR backend not available")
    
    class HugrQirEngine:
        def __init__(self, *args, **kwargs):
            raise ImportError("Rust HUGR backend not available")
    
    def is_hugr_support_available() -> bool:
        return False
    
    def get_supported_naming_conventions() -> List[str]:
        return []
    
    def compile_hugr_bytes_to_qir(*args, **kwargs):
        raise ImportError("Rust HUGR backend not available")
    
    def compile_hugr_file_to_qir(*args, **kwargs):
        raise ImportError("Rust HUGR backend not available")


class RustHugrCompiler:
    """
    High-performance HUGR to QIR compiler using Rust backend.
    
    This class provides a Python interface to the Rust-implemented HUGR compiler,
    offering better performance than pure Python implementations.
    """
    
    def __init__(self, debug_info: bool = False, naming_convention: str = "standard"):
        """
        Initialize the HUGR compiler.
        
        Args:
            debug_info: Whether to include debug information in compiled QIR
            naming_convention: Quantum operation naming convention 
                              ("standard", "hugr", "pecos")
        """
        if not RUST_HUGR_AVAILABLE:
            raise ImportError("Rust HUGR backend not available")
        
        self._compiler = HugrCompiler(debug_info, naming_convention)
    
    def compile_bytes_to_qir(self, hugr_bytes: bytes) -> str:
        """
        Compile HUGR bytes to QIR string.
        
        Args:
            hugr_bytes: HUGR data as bytes
            
        Returns:
            QIR as string
        """
        return self._compiler.compile_bytes_to_qir(hugr_bytes)
    
    def compile_file_to_qir(self, hugr_path: str, qir_path: str) -> None:
        """
        Compile HUGR file to QIR file.
        
        Args:
            hugr_path: Path to input HUGR file
            qir_path: Path for output QIR file
        """
        self._compiler.compile_file_to_qir(hugr_path, qir_path)
    
    def set_debug_info(self, debug_info: bool) -> None:
        """Set debug information flag."""
        self._compiler.set_debug_info(debug_info)
    
    def set_naming_convention(self, naming_convention: str) -> None:
        """Set quantum operation naming convention."""
        self._compiler.set_naming_convention(naming_convention)
    
    def get_naming_convention(self) -> str:
        """Get current naming convention."""
        return self._compiler.get_naming_convention()
    
    @staticmethod
    def get_supported_naming_conventions() -> List[str]:
        """Get list of supported naming conventions."""
        if not RUST_HUGR_AVAILABLE:
            return []
        return HugrCompiler.get_supported_naming_conventions()


class RustHugrQirEngine:
    """
    High-performance QIR engine created from HUGR using Rust backend.
    
    This class provides a Python interface to QIR engines compiled from HUGR,
    with execution handled by the Rust-implemented PECOS QIR runtime.
    """
    
    def __init__(
        self, 
        hugr_bytes: bytes, 
        shots: int = 1000,
        debug_info: bool = False,
        naming_convention: str = "standard"
    ):
        """
        Create QIR engine from HUGR bytes.
        
        Args:
            hugr_bytes: HUGR data as bytes
            shots: Number of shots to execute
            debug_info: Whether to include debug information
            naming_convention: Quantum operation naming convention
        """
        if not RUST_HUGR_AVAILABLE:
            raise ImportError("Rust HUGR backend not available")
        
        self._engine = HugrQirEngine(hugr_bytes, shots, debug_info, naming_convention)
    
    @classmethod
    def from_file(
        cls,
        hugr_path: str,
        shots: int = 1000,
        debug_info: bool = False,
        naming_convention: str = "standard"
    ) -> "RustHugrQirEngine":
        """
        Create QIR engine from HUGR file.
        
        Args:
            hugr_path: Path to HUGR file
            shots: Number of shots to execute
            debug_info: Whether to include debug information
            naming_convention: Quantum operation naming convention
            
        Returns:
            New RustHugrQirEngine instance
        """
        if not RUST_HUGR_AVAILABLE:
            raise ImportError("Rust HUGR backend not available")
        
        instance = cls.__new__(cls)
        instance._engine = HugrQirEngine.from_file(hugr_path, shots, debug_info, naming_convention)
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
        return f"RustHugrQirEngine(shots={self.get_shots()})"


def compile_hugr_to_qir_rust(
    hugr_data: Union[bytes, str],
    output_path: Optional[str] = None,
    debug_info: bool = False,
    naming_convention: str = "standard"
) -> Optional[str]:
    """
    Compile HUGR to QIR using Rust backend.
    
    Args:
        hugr_data: HUGR data as bytes or path to HUGR file
        output_path: Path for output QIR file (if None, returns QIR as string)
        debug_info: Whether to include debug information
        naming_convention: Quantum operation naming convention
        
    Returns:
        QIR as string if output_path is None, otherwise None
    """
    if not RUST_HUGR_AVAILABLE:
        raise ImportError("Rust HUGR backend not available")
    
    if isinstance(hugr_data, bytes):
        if output_path is None:
            return compile_hugr_bytes_to_qir(hugr_data, debug_info, naming_convention)
        else:
            # For bytes to file, we'd need to write to temp file first
            import tempfile
            with tempfile.NamedTemporaryFile(suffix='.hugr', delete=False) as f:
                f.write(hugr_data)
                temp_path = f.name
            try:
                compile_hugr_file_to_qir(temp_path, output_path, debug_info, naming_convention)
            finally:
                import os
                os.unlink(temp_path)
            return None
    else:
        # hugr_data is a file path
        if output_path is None:
            # Read file and compile to string
            with open(hugr_data, 'rb') as f:
                hugr_bytes = f.read()
            return compile_hugr_bytes_to_qir(hugr_bytes, debug_info, naming_convention)
        else:
            compile_hugr_file_to_qir(hugr_data, output_path, debug_info, naming_convention)
            return None


def create_qir_engine_from_hugr_rust(
    hugr_data: Union[bytes, str],
    shots: int = 1000,
    debug_info: bool = False,
    naming_convention: str = "standard"
) -> RustHugrQirEngine:
    """
    Create QIR engine from HUGR using Rust backend.
    
    Args:
        hugr_data: HUGR data as bytes or path to HUGR file
        shots: Number of shots to execute
        debug_info: Whether to include debug information
        naming_convention: Quantum operation naming convention
        
    Returns:
        RustHugrQirEngine instance
    """
    if isinstance(hugr_data, bytes):
        return RustHugrQirEngine(hugr_data, shots, debug_info, naming_convention)
    else:
        return RustHugrQirEngine.from_file(hugr_data, shots, debug_info, naming_convention)


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
    "RustHugrQirEngine", 
    "compile_hugr_to_qir_rust",
    "create_qir_engine_from_hugr_rust",
    "check_rust_hugr_availability",
    "RUST_HUGR_AVAILABLE",
]