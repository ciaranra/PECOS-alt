# Copyright 2024 The PECOS Developers
#
# Licensed under the Apache License, Version 2.0 (the "License"); you may not use this file except in compliance with
# the License.You may obtain a copy of the License at
#
#     https://www.apache.org/licenses/LICENSE-2.0
#
# Unless required by applicable law or agreed to in writing, software distributed under the License is distributed on an
# "AS IS" BASIS, WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied. See the License for the
# specific language governing permissions and limitations under the License.

"""PECOS Rust library Python bindings.

This package provides Python bindings for high-performance Rust implementations of quantum simulators and computational
components within the PECOS framework, enabling efficient quantum circuit simulation and error correction computations.
"""

# ruff: noqa: TID252
from importlib.metadata import PackageNotFoundError, version

from pecos_rslib.rssparse_sim import SparseSimRs
from pecos_rslib.rsstate_vec import StateVecRs
from pecos_rslib._pecos_rslib import ByteMessage
from pecos_rslib._pecos_rslib import ByteMessageBuilder
from pecos_rslib._pecos_rslib import StateVecEngineRs
from pecos_rslib._pecos_rslib import SparseStabEngineRs

# QASM simulation exports
from pecos_rslib._pecos_rslib import NoiseModel
from pecos_rslib._pecos_rslib import QuantumEngine
from pecos_rslib._pecos_rslib import run_qasm
from pecos_rslib._pecos_rslib import get_noise_models
from pecos_rslib._pecos_rslib import get_quantum_engines

# QIR execution exports
from pecos_rslib._pecos_rslib import execute_qir
from pecos_rslib._pecos_rslib import reset_qir_runtime

# Import the qasm_sim function and noise models for easy access
from pecos_rslib.qasm_sim import qasm_sim, register_noise_model

# Also import the noise model dataclasses for convenience
from pecos_rslib.qasm_sim import (
    PassThroughNoise,
    DepolarizingNoise,
    DepolarizingCustomNoise,
    BiasedDepolarizingNoise,
    GeneralNoise,
)

# Import HUGR-LLVM pipeline functionality (with graceful fallback)
try:
    from pecos_rslib.hugr_qir import (
        RustHugrCompiler,
        RustHugrQirEngine,
        compile_hugr_to_qir_rust,
        create_qir_engine_from_hugr_rust,
        check_rust_hugr_availability,
        RUST_HUGR_AVAILABLE,
    )
    HUGR_LLVM_PIPELINE_AVAILABLE = True
except ImportError:
    # Provide stub implementations for graceful degradation
    RUST_HUGR_AVAILABLE = False
    HUGR_LLVM_PIPELINE_AVAILABLE = False

    def check_rust_hugr_availability():
        return False, "HUGR-LLVM pipeline not available"

    def RustHugrCompiler(*args, **kwargs):
        raise ImportError("HUGR-LLVM pipeline not available")

    def RustHugrQirEngine(*args, **kwargs):
        raise ImportError("HUGR-LLVM pipeline not available")

    def compile_hugr_to_qir_rust(*args, **kwargs):
        raise ImportError("HUGR-LLVM pipeline not available")

    def create_qir_engine_from_hugr_rust(*args, **kwargs):
        raise ImportError("HUGR-LLVM pipeline not available")

# Import PMIR pipeline functionality (with graceful fallback)
try:
    from pecos_rslib.pmir import (
        hugr_to_past_ron,
        hugr_to_pmir_mlir,
        past_ron_to_pmir_mlir,
        past_ron_to_llvm_ir,
        compile_hugr_via_pmir,
        compile_and_execute_via_pmir,
        PMIRCompiler,
    )
    PMIR_PIPELINE_AVAILABLE = True
except ImportError:
    # Provide stub implementations for graceful degradation
    PMIR_PIPELINE_AVAILABLE = False
    
    def hugr_to_past_ron(*args, **kwargs):
        raise ImportError("PMIR pipeline not available")
    
    def hugr_to_pmir_mlir(*args, **kwargs):
        raise ImportError("PMIR pipeline not available")
    
    def past_ron_to_pmir_mlir(*args, **kwargs):
        raise ImportError("PMIR pipeline not available")
    
    def past_ron_to_llvm_ir(*args, **kwargs):
        raise ImportError("PMIR pipeline not available")
    
    def compile_hugr_via_pmir(*args, **kwargs):
        raise ImportError("PMIR pipeline not available")
    
    def compile_and_execute_via_pmir(*args, **kwargs):
        raise ImportError("PMIR pipeline not available")
    
    def PMIRCompiler(*args, **kwargs):
        raise ImportError("PMIR pipeline not available")

# Legacy compatibility
PMIR_AVAILABLE = PMIR_PIPELINE_AVAILABLE

def get_compilation_backends():
    """Get information about available compilation backends.
    
    Returns:
        dict: Dictionary with backend availability information
    """
    return {
        "pmir_pipeline_available": PMIR_PIPELINE_AVAILABLE,
        "hugr_llvm_pipeline_available": HUGR_LLVM_PIPELINE_AVAILABLE,
        "default_backend": "pmir" if PMIR_PIPELINE_AVAILABLE else ("hugr-llvm" if HUGR_LLVM_PIPELINE_AVAILABLE else "none"),
        "backends": {
            "pmir": {
                "available": PMIR_PIPELINE_AVAILABLE,
                "description": "PMIR pipeline: HUGR → PAST → PMIR (MLIR) → LLVM IR",
                "dependencies": ["MLIR tools"]
            },
            "hugr-llvm": {
                "available": HUGR_LLVM_PIPELINE_AVAILABLE,
                "description": "HUGR-LLVM pipeline: HUGR → QIR (via hugr-llvm)",
                "dependencies": ["hugr-llvm"]
            }
        }
    }


try:
    __version__ = version("pecos-rslib")
except PackageNotFoundError:
    __version__ = "0.0.0"

__all__ = [
    "SparseSimRs",
    "StateVecRs",
    "ByteMessage",
    "ByteMessageBuilder",
    "StateVecEngineRs",
    "SparseStabEngineRs",
    # QASM simulation
    "NoiseModel",
    "QuantumEngine",
    "run_qasm",
    "get_noise_models",
    "get_quantum_engines",
    "qasm_sim",
    "register_noise_model",
    # QIR execution
    "execute_qir",
    "reset_qir_runtime",
    # Noise model dataclasses
    "PassThroughNoise",
    "DepolarizingNoise",
    "DepolarizingCustomNoise",
    "BiasedDepolarizingNoise",
    "GeneralNoise",
    # HUGR-LLVM pipeline functionality
    "RustHugrCompiler",
    "RustHugrQirEngine", 
    "compile_hugr_to_qir_rust",
    "create_qir_engine_from_hugr_rust",
    "check_rust_hugr_availability",
    "RUST_HUGR_AVAILABLE",
    "HUGR_LLVM_PIPELINE_AVAILABLE",
    # PMIR pipeline functionality
    "hugr_to_past_ron",
    "hugr_to_pmir_mlir",
    "past_ron_to_pmir_mlir",
    "past_ron_to_llvm_ir",
    "compile_hugr_via_pmir",
    "compile_and_execute_via_pmir",
    "PMIRCompiler",
    "PMIR_PIPELINE_AVAILABLE",
    # Legacy compatibility
    "PMIR_AVAILABLE",
    # Backend information
    "get_compilation_backends",
]
