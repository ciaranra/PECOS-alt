"""Classical control engine builders for the unified simulation API.

This module provides a namespace for all classical control engine builders, making them easily
discoverable through IDE autocomplete and documentation.

Examples:
    >>> from pecos_rslib import classical
    >>>
    >>> # Available classical engines via namespace
    >>> qasm_builder = classical.qasm()
    >>> llvm_builder = classical.llvm()
    >>> selene_builder = classical.selene()
    >>>
    >>> # Direct class instantiation also available
    >>> qasm_builder = classical.QasmEngineBuilder()
    >>> llvm_builder = classical.QisEngineBuilder()
    >>> selene_builder = classical.SeleneEngineBuilder()
"""

# Import from the unified sim module
from pecos_rslib.sim import (
    QisEngineBuilder,
    QasmEngineBuilder,
    SeleneEngineBuilder,
    qis_engine,
    qasm_engine,
    selene_engine,
)

# Create namespace-friendly aliases
qasm = qasm_engine
llvm = qis_engine
selene = selene_engine

__all__ = [
    # Free functions
    "qasm",
    "llvm",
    "selene",
    # Builder classes
    "QasmEngineBuilder",
    "QisEngineBuilder",
    "SeleneEngineBuilder",
]
