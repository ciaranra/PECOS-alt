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

# Import the Rust bindings
from pecos_rslib._pecos_rslib import (
    BiasedDepolarizingNoiseModelBuilder,
    DepolarizingNoiseModelBuilder,
    GeneralNoiseModelBuilder,
    HugrProgram,
    LlvmEngineBuilder,
    LlvmProgram,
    PhirJsonEngineBuilder,
    PhirJsonProgram,
    QasmEngineBuilder,
    QasmProgram,
    SimBuilder,
    SparseStabilizerEngineBuilder,
    StateVectorEngineBuilder,
    biased_depolarizing_noise,
    depolarizing_noise,
    general_noise,
    llvm_engine,
    phir_json_engine,
    qasm_engine,
    sparse_stab,
    sparse_stabilizer,
    state_vector,
)

# Import our Python wrapper for selene_engine with Guppy support
from pecos_rslib.selene_engine import SeleneEngineBuilder, selene_engine

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
    "BiasedDepolarizingNoiseModelBuilder",
    "DepolarizingNoiseModelBuilder",
    "GeneralNoiseModelBuilder",
    "HugrProgram",
    "LlvmEngineBuilder",
    "LlvmProgram",
    "PhirJsonEngineBuilder",
    "PhirJsonProgram",
    "QasmEngineBuilder",
    "QasmProgram",
    "SeleneEngineBuilder",
    "SimBuilder",
    "SparseStabilizerEngineBuilder",
    "StateVectorEngineBuilder",
    "biased_depolarizing_noise",
    "depolarizing_noise",
    "general_noise",
    "llvm_engine",
    "phir_json_engine",
    "qasm_engine",
    "selene_engine",
    "sim",
    "sparse_stab",
    "sparse_stabilizer",
    "state_vector",
]

# Import the enhanced sim function that handles Guppy
try:
    from pecos_rslib.sim_wrapper import sim
except ImportError:
    # Fall back to Rust sim if wrapper not available
    from pecos_rslib._pecos_rslib import sim as _rust_sim

    sim = _rust_sim
