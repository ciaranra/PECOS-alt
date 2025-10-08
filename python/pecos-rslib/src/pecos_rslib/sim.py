"""Simulation API for all engine types.

This module provides the new API pattern:
    engine().program(...).to_sim().run(shots)

Examples:
    # QASM simulation
    from pecos_rslib import qasm_engine
    from pecos_rslib.programs import QasmProgram

    results = qasm_engine().program(QasmProgram.from_string("H q[0];")).to_sim().run(1000)

    # LLVM simulation
    from pecos_rslib import qis_engine
    from pecos_rslib.programs import QisProgram

    results = qis_engine().program(QisProgram.from_string(llvm_ir)).to_sim().run(1000)

    # QIS engine simulation with HUGR
    from pecos_rslib import qis_engine
    from pecos_rslib.programs import HugrProgram

    results = qis_engine().program(HugrProgram.from_bytes(hugr_bytes)).to_sim().run(1000)
"""

# Import the Rust bindings
from pecos_rslib._pecos_rslib import (
    BiasedDepolarizingNoiseModelBuilder,
    DepolarizingNoiseModelBuilder,
    GeneralNoiseModelBuilder,
    HugrProgram,
    QisEngineBuilder,
    QisProgram,
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
    qis_engine,
    phir_json_engine,
    qasm_engine,
    sparse_stab,
    sparse_stabilizer,
    state_vector,
)

# Note: selene_engine has been replaced with qis_engine for QIS/HUGR programs

# QIS engine provides unified runtime support for QIS/HUGR programs

# Re-export for convenience
__all__ = [
    "BiasedDepolarizingNoiseModelBuilder",
    "DepolarizingNoiseModelBuilder",
    "GeneralNoiseModelBuilder",
    "HugrProgram",
    "QisEngineBuilder",
    "QisProgram",
    "PhirJsonEngineBuilder",
    "PhirJsonProgram",
    "QasmEngineBuilder",
    "QasmProgram",
    "SimBuilder",
    "SparseStabilizerEngineBuilder",
    "StateVectorEngineBuilder",
    "biased_depolarizing_noise",
    "depolarizing_noise",
    "general_noise",
    "qis_engine",
    "phir_json_engine",
    "qasm_engine",
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
