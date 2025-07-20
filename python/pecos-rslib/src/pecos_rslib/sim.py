"""Unified simulation API for all engine types.

This module provides the new unified API pattern:
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
    QasmSimBuilder,
    LlvmSimBuilder,
    SeleneSimBuilder,
    PhirJsonSimBuilder,
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
    "QasmSimBuilder",
    "LlvmSimBuilder",
    "SeleneSimBuilder",
    "PhirJsonSimBuilder",
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

# Alias for functional style
def sim(engine_builder):
    """Create a simulation builder from an engine builder.
    
    This is an alias for the functional style API.
    
    Args:
        engine_builder: An engine builder (e.g., from qasm_engine())
        
    Returns:
        The engine builder (for chaining)
        
    Example:
        >>> from pecos_rslib import sim, qasm_engine
        >>> from pecos_rslib.programs import QasmProgram
        >>> 
        >>> results = sim(qasm_engine().program(QasmProgram.from_string("H q[0];"))).to_sim().run(1000)
    """
    return engine_builder

# Type alias for SimBuilder - we'll use the specific sim builders from Rust
if TYPE_CHECKING:
    from typing import Union
    SimBuilder = Union[QasmSimBuilder, LlvmSimBuilder, SeleneSimBuilder]
else:
    SimBuilder = "SimBuilder"