"""Simulation API for all engine types.

This module provides the new API pattern:
    engine().program(...).to_sim().run(shots)

Examples:
    # QASM simulation
    from pecos_rslib import qasm_engine
    from pecos_rslib.programs import QasmProgram
    
    results = qasm_engine().program(QasmProgram.from_string("H q[0];")).to_sim().run(1000)
    
    # LLVM simulation
    from pecos_rslib import llvm_engine
    from pecos_rslib.programs import LlvmProgram
    
    results = llvm_engine().program(LlvmProgram.from_string(llvm_ir)).to_sim().run(1000)
    
    # Selene simulation with Guppy
    from pecos_rslib import selene_engine
    
    def my_quantum_func(q: Qubit) -> None:
        H(q)
        measure(q)
    
    results = selene_engine().program(my_quantum_func).to_sim().run(1000)
"""

from typing import TYPE_CHECKING

# Import the Rust bindings
from pecos_rslib._pecos_rslib import (
    qasm_engine,
    llvm_engine,
    selene_engine as _rust_selene_engine,
    phir_json_engine,
    QasmEngineBuilder,
    LlvmEngineBuilder,
    SeleneEngineBuilder as _RustSeleneEngineBuilder,
    PhirJsonEngineBuilder,
    SimBuilder,
    QasmProgram,
    LlvmProgram,
    HugrProgram,
    PhirJsonProgram,
    GeneralNoiseModelBuilder,
    DepolarizingNoiseModelBuilder,
    BiasedDepolarizingNoiseModelBuilder,
    StateVectorEngineBuilder,
    SparseStabilizerEngineBuilder,
    state_vector,
    sparse_stabilizer,
    sparse_stab,
    general_noise,
    depolarizing_noise,
    biased_depolarizing_noise,
)

# Import our Python wrapper for selene_engine with Guppy support
from pecos_rslib.selene_engine import selene_engine, SeleneEngineBuilder

# Automatically set up Bridge plugin integration for Selene
try:
    from pecos_rslib.selene_auto_bridge import _auto_patched
    if _auto_patched:
        import logging
        logging.getLogger(__name__).info("Bridge plugin auto-integration enabled")
except ImportError:
    pass  # Bridge plugin not available

# Re-export for convenience
__all__ = [
    "qasm_engine",
    "llvm_engine", 
    "selene_engine",
    "phir_json_engine",
    "sim",
    "QasmEngineBuilder",
    "LlvmEngineBuilder",
    "SeleneEngineBuilder",
    "PhirJsonEngineBuilder",
    "SimBuilder",
    "QasmProgram",
    "LlvmProgram",
    "HugrProgram",
    "PhirJsonProgram",
    "GeneralNoiseModelBuilder",
    "DepolarizingNoiseModelBuilder",
    "BiasedDepolarizingNoiseModelBuilder",
    "StateVectorEngineBuilder",
    "SparseStabilizerEngineBuilder",
    "state_vector",
    "sparse_stabilizer",
    "sparse_stab",
    "general_noise",
    "depolarizing_noise",
    "biased_depolarizing_noise",
]

# Import the enhanced sim function that handles Guppy
try:
    from pecos_rslib.sim_wrapper import sim
except ImportError:
    # Fall back to Rust sim if wrapper not available
    from pecos_rslib._pecos_rslib import sim as _rust_sim
    sim = _rust_sim