"""Unified simulation API following the sim(engine_builder) pattern.

This module provides a thin Python wrapper around the Rust sim(engine_builder)
pattern, offering a consistent API across both languages.

Examples:
    Basic usage:
    >>> from pecos_rslib.sim import sim, qasm_engine
    >>> results = sim(qasm_engine().qasm("OPENQASM 2.0; qreg q[1]; h q[0];")).seed(42).run(1000)
    
    With noise:
    >>> results = sim(qasm_engine().qasm(source)).seed(42).noise_depolarizing(0.01).run(1000)
    
    With parallelization:
    >>> results = sim(llvm_engine().llvm_ir(ir)).auto_workers().run(10000)
    
    Different engines:
    >>> # QASM engine
    >>> results = sim(qasm_engine().qasm(qasm_source)).run(1000)
    >>>
    >>> # LLVM engine  
    >>> results = sim(llvm_engine().llvm_ir(llvm_ir).max_qubits(10)).run(1000)
    >>>
    >>> # Selene engine
    >>> results = sim(selene_engine().llvm_ir(llvm_ir)).quantum_engine("sparsestabilizer").run(1000)
"""

# Import everything from the Rust module
from pecos_rslib._pecos_rslib import (
    # Engine builders
    qasm_engine,
    llvm_engine,
    selene_engine,
    QasmEngineBuilder,
    LlvmEngineBuilder,
    SeleneEngineBuilder,
    # Main sim function
    sim,
    SimBuilder,
    # Result type
    ShotVec,
)

__all__ = [
    # Engine builders
    "qasm_engine",
    "llvm_engine", 
    "selene_engine",
    "QasmEngineBuilder",
    "LlvmEngineBuilder",
    "SeleneEngineBuilder",
    # Main sim function
    "sim",
    "SimBuilder",
    # Result type
    "ShotVec",
]