"""PHIR (PECOS High-level IR) compilation pipeline.

This module provides access to the PHIR JSON intermediate representation
and compilation pipeline.
"""

# Import PHIR functions from the Rust bindings
from pecos_rslib._pecos_rslib import (
    PhirJsonEngine,
    PhirJsonEngineBuilder,
    PhirJsonProgram,
    PhirJsonSimulation,
    compile_hugr_to_llvm,
    phir_json_engine,
)

__all__ = [
    "PhirJsonEngine",
    "PhirJsonEngineBuilder",
    "PhirJsonProgram",
    "PhirJsonSimulation",
    "compile_hugr_to_llvm",
    "phir_json_engine",
]
