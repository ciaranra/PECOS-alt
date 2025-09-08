"""Quantum simulators/engines for the unified simulation API.

This module provides a namespace for all quantum simulators (quantum engines), making them easily
discoverable through IDE autocomplete and documentation.

Examples:
    >>> from pecos_rslib import quantum
    >>>
    >>> # Available quantum simulators via namespace
    >>> state_vector_engine = quantum.state_vector()
    >>> sparse_stabilizer_engine = quantum.sparse_stabilizer()
    >>> sparse_stab_engine = quantum.sparse_stab()  # alias
    >>>
    >>> # Direct class instantiation also available
    >>> state_vector_engine = quantum.StateVectorEngineBuilder()
    >>> sparse_stabilizer_engine = quantum.SparseStabilizerEngineBuilder()
    >>>
    >>> # Use in simulation
    >>> from pecos_rslib import classical
    >>> results = classical.qasm()\\
    >>>     .program(program)\\
    >>>     .to_sim()\\
    >>>     .quantum(state_vector_engine)\\
    >>>     .run(1000)
"""

# Import from the unified sim module (Rust-backed)
from pecos_rslib.sim import (
    StateVectorEngineBuilder,
    SparseStabilizerEngineBuilder,
    state_vector,
    sparse_stabilizer,
    sparse_stab,
)

__all__ = [
    # Free functions
    "state_vector",
    "sparse_stabilizer",
    "sparse_stab",
    # Builder classes
    "StateVectorEngineBuilder",
    "SparseStabilizerEngineBuilder",
]
