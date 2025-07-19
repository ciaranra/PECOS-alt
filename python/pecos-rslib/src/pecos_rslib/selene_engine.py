"""Selene engine builder for classical control engines.

This module provides a Python interface to the Rust selene_engine implementation,
offering a builder pattern for creating Selene classical control engines.
"""

from typing import Union
from pathlib import Path

# Import the Rust bindings
from pecos_rslib._pecos_rslib import (
    selene_engine_builder as _rust_selene_engine_builder,
    SeleneEngine,
)


class SeleneEngineBuilder:
    """Builder for Selene classical control engines."""
    
    def __init__(self, rust_builder):
        """Initialize with a Rust builder instance."""
        self._rust_builder = rust_builder
    
    def qubits(self, qubits: int) -> "SeleneEngineBuilder":
        """Set number of qubits."""
        self._rust_builder.qubits(qubits)
        return self
    
    def optimize(self) -> "SeleneEngineBuilder":
        """Enable optimization."""
        self._rust_builder.optimize()
        return self
    
    def verbose(self, verbose: bool = True) -> "SeleneEngineBuilder":
        """Enable verbose output."""
        self._rust_builder.verbose(verbose)
        return self
    
    def build(self) -> SeleneEngine:
        """Build the engine."""
        return self._rust_builder.build()


def selene_engine(source: Union[str, Path]) -> SeleneEngineBuilder:
    """Create a Selene engine builder for classical control engines.
    
    This creates a Selene classical control engine that can be used with
    the unified simulation API for quantum-classical hybrid programs.
    
    Args:
        source: LLVM IR string or file path
        
    Returns:
        SeleneEngineBuilder: Builder for configuring the engine
        
    Examples:
        >>> # From LLVM IR string
        >>> llvm_ir = '''
        ... define void @main() #0 {
        ...     %0 = call i64 @__quantum__rt__qubit_allocate()
        ...     call void @__quantum__qis__h__body(i64 %0)
        ...     ret void
        ... }
        ... attributes #0 = { "EntryPoint" }
        ... '''
        >>> engine = selene_engine(llvm_ir).qubits(1).optimize().build()
        
        >>> # With verbose output
        >>> engine = selene_engine(llvm_ir) \\
        ...     .qubits(1) \\
        ...     .verbose(True) \\
        ...     .build()
    """
    if isinstance(source, Path):
        source = str(source)
    
    rust_builder = _rust_selene_engine_builder(source)
    return SeleneEngineBuilder(rust_builder)


# Export the main function and classes
__all__ = [
    "selene_engine",
    "SeleneEngineBuilder",
]