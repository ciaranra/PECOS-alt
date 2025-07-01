"""Type stubs for pecos_rslib.qasm_sim module."""

from dataclasses import dataclass
from typing import Dict, Any
from ._pecos_rslib import (
    NoiseModel,
    QuantumEngine,
    QasmSimulation,
    QasmSimulationBuilder,
    GeneralNoiseModelBuilder,
    run_qasm as run_qasm,
    qasm_sim as qasm_sim,
    get_noise_models as get_noise_models,
    get_quantum_engines as get_quantum_engines,
)

__all__ = [
    "NoiseModel",
    "QuantumEngine",
    "QasmSimulation",
    "QasmSimulationBuilder",
    "get_noise_models",
    "get_quantum_engines",
    # Noise model dataclasses
    "PassThroughNoise",
    "DepolarizingNoise",
    "DepolarizingCustomNoise",
    "BiasedDepolarizingNoise",
    "GeneralNoise",
    # Builder classes
    "GeneralNoiseModelBuilder",
    # Main interface
    "run_qasm",
    "qasm_sim",
]

# Re-export from _pecos_rslib with proper types
NoiseModel = NoiseModel
QuantumEngine = QuantumEngine
QasmSimulation = QasmSimulation
QasmSimulationBuilder = QasmSimulationBuilder
GeneralNoiseModelBuilder = GeneralNoiseModelBuilder

# Noise model dataclasses

@dataclass
class PassThroughNoise:
    """No noise - ideal quantum simulation."""

    @classmethod
    def from_config(cls, config: Dict[str, Any]) -> "PassThroughNoise": ...

@dataclass
class DepolarizingNoise:
    """Standard depolarizing noise with uniform probability."""

    p: float = 0.001
    @classmethod
    def from_config(cls, config: Dict[str, Any]) -> "DepolarizingNoise": ...

@dataclass
class DepolarizingCustomNoise:
    """Depolarizing noise with custom probabilities for different operations."""

    p_prep: float = 0.001
    p_meas: float = 0.001
    p1: float = 0.001
    p2: float = 0.002
    @classmethod
    def from_config(cls, config: Dict[str, Any]) -> "DepolarizingCustomNoise": ...

@dataclass
class BiasedDepolarizingNoise:
    """Biased depolarizing noise with separate X/Y and Z error probabilities."""

    px: float = 0.001
    py: float = 0.001
    pz: float = 0.001
    @classmethod
    def from_config(cls, config: Dict[str, Any]) -> "BiasedDepolarizingNoise": ...

@dataclass
class GeneralNoise:
    """GeneralNoiseModel created from configuration dictionary."""

    config: Dict[str, float]
    @classmethod
    def from_config(cls, config: Dict[str, Any]) -> "GeneralNoise": ...
