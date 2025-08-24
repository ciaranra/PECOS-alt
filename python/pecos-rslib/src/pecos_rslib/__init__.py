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

# Shot result types
from pecos_rslib._pecos_rslib import ShotVec
from pecos_rslib._pecos_rslib import ShotMap

# QASM simulation exports - these are from the old API
# from pecos_rslib._pecos_rslib import NoiseModel
# from pecos_rslib._pecos_rslib import QuantumEngine
# from pecos_rslib._pecos_rslib import run_qasm
# from pecos_rslib._pecos_rslib import get_noise_models
# from pecos_rslib._pecos_rslib import get_quantum_engines
# from pecos_rslib._pecos_rslib import GeneralNoiseModelBuilder  # Not currently registered
# These noise free functions need to be exposed from Rust first
# from pecos_rslib._pecos_rslib import general_noise
# from pecos_rslib._pecos_rslib import depolarizing_noise
# from pecos_rslib._pecos_rslib import biased_depolarizing_noise

# LLVM execution exports
# from pecos_rslib._pecos_rslib import execute_llvm  # Not currently registered
# from pecos_rslib._pecos_rslib import reset_llvm_runtime  # Not currently registered

# HUGR to LLVM compilation
# Note: compile_llvm_to_plugin has been removed - Selene uses native executables, not plugins

# HUGR to LLVM compilation - currently not registered
# try:
#     from pecos_rslib._pecos_rslib import compile_hugr_to_llvm
# except ImportError:
#     # Not available if compiled without hugr-013 feature
#     def compile_hugr_to_llvm(*args, **kwargs):
#         raise ImportError("compile_hugr_to_llvm requires pecos-rslib to be compiled with hugr-013 feature")

# LLVM and Selene are now part of the unified API

# Guppy conversion utilities - try importing but don't fail
try:
    from pecos_rslib.guppy_conversion import guppy_to_hugr
except ImportError:
    def guppy_to_hugr(*args, **kwargs):
        raise ImportError("guppy_to_hugr not available")

# Program types - try importing but don't fail
try:
    from pecos_rslib.programs import (
        QasmProgram,
        LlvmProgram,
        HugrProgram,
        PhirJsonProgram,
        WasmProgram,
        WatProgram,
    )
except ImportError:
    # Provide stubs if not available
    class QasmProgram:
        @staticmethod
        def from_string(qasm):
            raise ImportError("QasmProgram not available")
    
    class LlvmProgram:
        @staticmethod
        def from_string(llvm):
            raise ImportError("LlvmProgram not available")
    
    class HugrProgram:
        @staticmethod
        def from_bytes(bytes):
            raise ImportError("HugrProgram not available")
    
    class PhirJsonProgram:
        @staticmethod
        def from_json(json):
            raise ImportError("PhirJsonProgram not available")
    
    class WasmProgram:
        @staticmethod
        def from_bytes(bytes):
            raise ImportError("WasmProgram not available")
    
    class WatProgram:
        @staticmethod
        def from_string(wat):
            raise ImportError("WatProgram not available")

# Import the new sim API - directly from Rust module
try:
    from pecos_rslib._pecos_rslib import sim
except ImportError:
    def sim(*args, **kwargs):
        raise ImportError("sim() function not available - ensure pecos-rslib is built with sim support")

# Try to import other sim-related functions but don't fail if unavailable
try:
    from pecos_rslib.sim import (
        qasm_engine,
        llvm_engine,
        selene_engine,
        phir_json_engine,
        QasmEngineBuilder,
        LlvmEngineBuilder,
        SeleneEngineBuilder,
        PhirJsonEngineBuilder,
        SimBuilder,
        GeneralNoiseModelBuilder,
        DepolarizingNoiseModelBuilder,
        BiasedDepolarizingNoiseModelBuilder,
    )
except ImportError:
    # Provide stubs if not available
    def qasm_engine(*args, **kwargs):
        raise ImportError("qasm_engine not available")
    
    def llvm_engine(*args, **kwargs):
        raise ImportError("llvm_engine not available")
    
    def selene_engine(*args, **kwargs):
        raise ImportError("selene_engine not available")
    
    def phir_json_engine(*args, **kwargs):
        raise ImportError("phir_json_engine not available")
    
    # Builder classes
    class QasmEngineBuilder:
        def __init__(self):
            raise ImportError("QasmEngineBuilder not available")
    
    class LlvmEngineBuilder:
        def __init__(self):
            raise ImportError("LlvmEngineBuilder not available")
    
    class SeleneEngineBuilder:
        def __init__(self):
            raise ImportError("SeleneEngineBuilder not available")
    
    class PhirJsonEngineBuilder:
        def __init__(self):
            raise ImportError("PhirJsonEngineBuilder not available")
    
    class SimBuilder:
        def __init__(self):
            raise ImportError("SimBuilder not available")
    
    class GeneralNoiseModelBuilder:
        def __init__(self):
            raise ImportError("GeneralNoiseModelBuilder not available")
    
    class DepolarizingNoiseModelBuilder:
        def __init__(self):
            raise ImportError("DepolarizingNoiseModelBuilder not available")
    
    class BiasedDepolarizingNoiseModelBuilder:
        def __init__(self):
            raise ImportError("BiasedDepolarizingNoiseModelBuilder not available")

# Import quantum engine builders from sim module - try but don't fail
try:
    from pecos_rslib.sim import (
        StateVectorEngineBuilder,
        SparseStabilizerEngineBuilder,
        state_vector,
        sparse_stabilizer,
        sparse_stab,
        general_noise,
        depolarizing_noise,
        biased_depolarizing_noise,
    )
except ImportError:
    # Provide stubs
    class StateVectorEngineBuilder:
        def __init__(self):
            raise ImportError("StateVectorEngineBuilder not available")
    
    class SparseStabilizerEngineBuilder:
        def __init__(self):
            raise ImportError("SparseStabilizerEngineBuilder not available")
    
    def state_vector(*args, **kwargs):
        raise ImportError("state_vector not available")
    
    def sparse_stabilizer(*args, **kwargs):
        raise ImportError("sparse_stabilizer not available")
    
    def sparse_stab(*args, **kwargs):
        raise ImportError("sparse_stab not available")
    
    def general_noise(*args, **kwargs):
        raise ImportError("general_noise not available")
    
    def depolarizing_noise(*args, **kwargs):
        raise ImportError("depolarizing_noise not available")
    
    def biased_depolarizing_noise(*args, **kwargs):
        raise ImportError("biased_depolarizing_noise not available")

# Import GeneralNoiseFactory and convenience functions - try but don't fail
try:
    from pecos_rslib.general_noise_factory import (
        GeneralNoiseFactory,
        create_noise_from_dict,
        create_noise_from_json,
        IonTrapNoiseFactory,
    )
except ImportError:
    # Provide stubs
    class GeneralNoiseFactory:
        def __init__(self):
            raise ImportError("GeneralNoiseFactory not available")
    
    def create_noise_from_dict(*args, **kwargs):
        raise ImportError("create_noise_from_dict not available")
    
    def create_noise_from_json(*args, **kwargs):
        raise ImportError("create_noise_from_json not available")
    
    class IonTrapNoiseFactory:
        def __init__(self):
            raise ImportError("IonTrapNoiseFactory not available")

# Import namespace modules for better discoverability - try but don't fail
try:
    from pecos_rslib import noise, quantum, programs
except ImportError:
    # Create empty namespace objects
    import types
    noise = types.ModuleType('noise')
    quantum = types.ModuleType('quantum')
    programs = types.ModuleType('programs')

# HUGR-LLVM pipeline is not currently available
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

def compile_hugr_to_llvm(*args, **kwargs):
    raise ImportError("compile_hugr_to_llvm requires pecos-rslib to be compiled with hugr-013 feature")

# Import PHIR pipeline functionality (core part of PECOS) - try but don't fail
try:
    from pecos_rslib.phir import (
        hugr_to_phir_mlir,
        compile_hugr_via_phir,
        compile_and_execute_via_phir,
        PhirCompiler,
    )
except ImportError:
    # Provide stubs
    def hugr_to_phir_mlir(*args, **kwargs):
        raise ImportError("hugr_to_phir_mlir not available")
    
    def compile_hugr_via_phir(*args, **kwargs):
        raise ImportError("compile_hugr_via_phir not available")
    
    def compile_and_execute_via_phir(*args, **kwargs):
        raise ImportError("compile_and_execute_via_phir not available")
    
    class PhirCompiler:
        def __init__(self):
            raise ImportError("PhirCompiler not available")


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
    # Shot result types
    "ShotVec",
    "ShotMap",
    # Noise builders
    "GeneralNoiseModelBuilder",
    "DepolarizingNoiseModelBuilder",
    "BiasedDepolarizingNoiseModelBuilder",
    # LLVM execution - currently not available
    # "execute_llvm",
    # "reset_llvm_runtime",
    # HUGR/LLVM compilation - currently not available
    # "compile_hugr_to_llvm",
    # Guppy conversion - may not be available
    # "guppy_to_hugr",
    # Program types
    "QasmProgram",
    "LlvmProgram",
    "HugrProgram",
    "PhirJsonProgram",
    "WasmProgram",
    "WatProgram",
    # Noise factory
    "GeneralNoiseFactory",
    "create_noise_from_dict",
    "create_noise_from_json",
    "IonTrapNoiseFactory",
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
    # New sim API
    "sim",
    "qasm_engine",
    "llvm_engine",
    "selene_engine",
    "phir_json_engine",
    "QasmEngineBuilder",
    "LlvmEngineBuilder",
    "SeleneEngineBuilder",
    "PhirJsonEngineBuilder",
    "SimBuilder",
    # Quantum engine builders
    "StateVectorEngineBuilder",
    "SparseStabilizerEngineBuilder",
    "state_vector",
    "sparse_stabilizer",
    "sparse_stab",
    # Noise builder free functions
    "general_noise",
    "depolarizing_noise",
    "biased_depolarizing_noise",
    # Namespace modules for discoverability
    "noise",
    "quantum",
    "programs",
]
