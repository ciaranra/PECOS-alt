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
    >>> llvm_builder = classical.LlvmEngineBuilder()
    >>> selene_builder = classical.SeleneEngineBuilder()
"""

# Import from the unified sim module
from pecos_rslib.sim import (
    qasm_engine,
    llvm_engine,
    selene_engine,
    QasmEngineBuilder,
    LlvmEngineBuilder,
    SeleneEngineBuilder,
)

# Create namespace-friendly aliases
qasm = qasm_engine
llvm = llvm_engine
selene = selene_engine

__all__ = [
    # Free functions
    "qasm",
    "llvm",
    "selene",
    # Builder classes
    "QasmEngineBuilder",
    "LlvmEngineBuilder",
    "SeleneEngineBuilder",
]
