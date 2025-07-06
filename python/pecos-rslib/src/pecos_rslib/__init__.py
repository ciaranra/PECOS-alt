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

# LLVM execution exports
from pecos_rslib._pecos_rslib import execute_llvm
from pecos_rslib._pecos_rslib import reset_llvm_runtime

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
    from pecos_rslib.hugr_llvm import (
        RustHugrCompiler,
        RustHugrLlvmEngine,
        compile_hugr_to_llvm_rust,
        create_llvm_engine_from_hugr_rust,
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

    def RustHugrLlvmEngine(*args, **kwargs):
        raise ImportError("HUGR-LLVM pipeline not available")

    def compile_hugr_to_llvm_rust(*args, **kwargs):
        raise ImportError("HUGR-LLVM pipeline not available")

    def create_llvm_engine_from_hugr_rust(*args, **kwargs):
        raise ImportError("HUGR-LLVM pipeline not available")

# Import PHIR pipeline functionality (core part of PECOS)
from pecos_rslib.phir import (
    hugr_to_phir_mlir,
    compile_hugr_via_phir,
    compile_and_execute_via_phir,
    PhirCompiler,
)


def get_compilation_backends():
    """Get information about available compilation backends.
    
    Returns:
        dict: Dictionary with backend availability information
    """
    return {
        "default_backend": "phir",  # PHIR is the default backend
        "backends": {
            "phir": {
                "available": True,
                "description": "PHIR pipeline: HUGR → PHIR → LLVM IR",
                "dependencies": ["MLIR tools"]
            },
            "hugr-llvm": {
                "available": HUGR_LLVM_PIPELINE_AVAILABLE,
                "description": "HUGR-LLVM pipeline: HUGR → LLVM IR (via hugr-llvm)",
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
    # LLVM execution
    "execute_llvm",
    "reset_llvm_runtime",
    # Noise model dataclasses
    "PassThroughNoise",
    "DepolarizingNoise",
    "DepolarizingCustomNoise",
    "BiasedDepolarizingNoise",
    "GeneralNoise",
    # HUGR-LLVM pipeline functionality
    "RustHugrCompiler",
    "RustHugrLlvmEngine", 
    "compile_hugr_to_llvm_rust",
    "create_llvm_engine_from_hugr_rust",
    "check_rust_hugr_availability",
    "RUST_HUGR_AVAILABLE",
    "HUGR_LLVM_PIPELINE_AVAILABLE",
    # PHIR pipeline functionality
    "hugr_to_phir_mlir",
    "compile_hugr_via_phir",
    "compile_and_execute_via_phir",
    "PhirCompiler",
    # Backend information
    "get_compilation_backends",
]
