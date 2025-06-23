"""Python interface for QASM simulation with enhanced API.

This module provides a clean Python interface for running quantum circuit simulations
using OpenQASM 2.0. It supports various noise models, quantum engines, and parallel execution.

For detailed usage examples, see the PECOS documentation:
https://github.com/CQCL/PECOS/blob/master/docs/user-guide/qasm-simulation.md
"""

from dataclasses import dataclass
from typing import List, Dict, Optional, Any
from pecos_rslib._pecos_rslib import (
    NoiseModel,
    QuantumEngine,
    QasmSimulation,
    QasmSimulationBuilder,
    run_qasm as _run_qasm,
    qasm_sim as _qasm_sim,
    get_noise_models as _get_noise_models,
    get_quantum_engines as _get_quantum_engines,
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
    # Main interface
    "run_qasm",
    "qasm_sim",
    "register_noise_model",
]


# Noise model dataclasses


@dataclass
class PassThroughNoise:
    """No noise - ideal quantum simulation."""

    @classmethod
    def from_config(cls, config: Dict[str, Any]) -> "PassThroughNoise":
        """Create PassThroughNoise from configuration dictionary.

        Args:
            config: Configuration dictionary (no parameters needed)

        Returns:
            PassThroughNoise instance
        """
        return cls()


@dataclass
class DepolarizingNoise:
    """Standard depolarizing noise with uniform probability.

    Args:
        p: Uniform error probability for all operations
    """

    p: float = 0.001

    @classmethod
    def from_config(cls, config: Dict[str, Any]) -> "DepolarizingNoise":
        """Create DepolarizingNoise from configuration dictionary.

        Args:
            config: Configuration dictionary with optional 'p' field

        Returns:
            DepolarizingNoise instance
        """
        return cls(p=config.get("p", 0.001))


@dataclass
class DepolarizingCustomNoise:
    """Depolarizing noise with custom probabilities for different operations.

    Args:
        p_prep: State preparation error probability
        p_meas: Measurement error probability
        p1: Single-qubit gate error probability
        p2: Two-qubit gate error probability
    """

    p_prep: float = 0.001
    p_meas: float = 0.001
    p1: float = 0.001
    p2: float = 0.002

    @classmethod
    def from_config(cls, config: Dict[str, Any]) -> "DepolarizingCustomNoise":
        """Create DepolarizingCustomNoise from configuration dictionary.

        Args:
            config: Configuration dictionary with optional probability fields

        Returns:
            DepolarizingCustomNoise instance
        """
        return cls(
            p_prep=config.get("p_prep", 0.001),
            p_meas=config.get("p_meas", 0.001),
            p1=config.get("p1", 0.001),
            p2=config.get("p2", 0.002),
        )


@dataclass
class BiasedDepolarizingNoise:
    """Biased depolarizing noise model.

    Args:
        p: Uniform probability for all operations
    """

    p: float = 0.001

    @classmethod
    def from_config(cls, config: Dict[str, Any]) -> "BiasedDepolarizingNoise":
        """Create BiasedDepolarizingNoise from configuration dictionary.

        Args:
            config: Configuration dictionary with optional 'p' field

        Returns:
            BiasedDepolarizingNoise instance
        """
        return cls(p=config.get("p", 0.001))


@dataclass
class GeneralNoise:
    """General noise model using default configuration."""

    @classmethod
    def from_config(cls, config: Dict[str, Any]) -> "GeneralNoise":
        """Create GeneralNoise from configuration dictionary.

        Args:
            config: Configuration dictionary (no parameters needed)

        Returns:
            GeneralNoise instance
        """
        return cls()


def run_qasm(
    qasm: str,
    shots: int,
    noise_model: Optional[Any] = None,
    engine: Optional[QuantumEngine] = None,
    workers: Optional[int] = None,
    seed: Optional[int] = None,
) -> Dict[str, List[int]]:
    """Run a QASM simulation with specified parameters.

    Args:
        qasm: QASM code as a string
        shots: Number of measurement shots to perform
        noise_model: Noise model instance (e.g., DepolarizingNoise(p=0.01)) or None for no noise
        engine: Quantum simulation engine (QuantumEngine.StateVector or QuantumEngine.SparseStabilizer)
        workers: Number of worker threads (None for default of 1)
        seed: Random seed for reproducibility (None for non-deterministic)

    Returns:
        Dict mapping register names to lists of measurement values (as integers).
        For example: {"c": [0, 3, 0, 3, ...]} for a Bell state measurement.

    Example:
        >>> from pecos_rslib.qasm_sim import run_qasm, DepolarizingNoise, QuantumEngine
        >>> qasm = '''
        ... OPENQASM 2.0;
        ... include "qelib1.inc";
        ... qreg q[2];
        ... creg c[2];
        ... h q[0];
        ... cx q[0], q[1];
        ... measure q -> c;
        ... '''
        >>> results = run_qasm(qasm, shots=1000, noise_model=DepolarizingNoise(p=0.01))
        >>> # Results are in columnar format
        >>> print(f"Got {len(results['c'])} measurements")
        >>> # Count occurrences of each measurement outcome
        >>> from collections import Counter
        >>> counts = Counter(results["c"])
        >>> print(counts)  # Should show roughly equal counts of 0 (00) and 3 (11)
    """
    return _run_qasm(qasm, shots, noise_model, engine, workers, seed)


def qasm_sim(qasm: str) -> QasmSimulationBuilder:
    """Create a QASM simulation builder for flexible configuration.

    This provides a builder pattern for QASM simulations, allowing you to
    build once and run multiple times with different shot counts.

    Args:
        qasm: QASM code as a string

    Returns:
        QasmSimulationBuilder that can be configured and run

    Example:
        >>> from pecos_rslib.qasm_sim import qasm_sim, DepolarizingNoise, QuantumEngine
        >>> qasm = '''
        ... OPENQASM 2.0;
        ... include "qelib1.inc";
        ... qreg q[2];
        ... creg c[2];
        ... h q[0];
        ... cx q[0], q[1];
        ... measure q -> c;
        ... '''
        >>> # Build once, run multiple times
        >>> sim = qasm_sim(qasm).seed(42).noise(DepolarizingNoise(p=0.01)).build()
        >>>
        >>> results_100 = sim.run(100)
        >>> results_1000 = sim.run(1000)
        >>>
        >>> # Or run directly without building
        >>> results = (
        ...     qasm_sim(qasm).noise(DepolarizingNoise(p=0.01)).workers(4).run(1000)
        ... )
        >>>
        >>> # Or use a configuration dictionary
        >>> config = {
        ...     "seed": 42,
        ...     "workers": 4,
        ...     "noise": {"type": "DepolarizingNoise", "p": 0.01},
        ...     "binary_string_format": True,
        ... }
        >>> sim = qasm_sim(qasm).config(config).build()
        >>> results = sim.run(1000)
    """
    return _qasm_sim(qasm)


def get_noise_models() -> List[str]:
    """Get a list of available noise model names.

    Returns:
        List of string names of available noise models, such as
        'PassThrough', 'Depolarizing', 'DepolarizingCustom', etc.

    Example:
        >>> from pecos_rslib.qasm_sim import get_noise_models
        >>> noise_models = get_noise_models()
        >>> print(noise_models)
        ['PassThrough', 'Depolarizing', 'DepolarizingCustom', ...]
    """
    return _get_noise_models()


def get_quantum_engines() -> List[str]:
    """Get a list of available quantum engine names.

    Returns:
        List of string names of available quantum engines, such as
        'StateVector', 'SparseStabilizer', etc.

    Example:
        >>> from pecos_rslib.qasm_sim import get_quantum_engines
        >>> engines = get_quantum_engines()
        >>> print(engines)
        ['StateVector', 'SparseStabilizer']
    """
    return _get_quantum_engines()


# Noise model registry for configuration-based creation
_NOISE_MODEL_REGISTRY = {
    "PassThroughNoise": PassThroughNoise,
    "DepolarizingNoise": DepolarizingNoise,
    "DepolarizingCustomNoise": DepolarizingCustomNoise,
    "BiasedDepolarizingNoise": BiasedDepolarizingNoise,
    "GeneralNoise": GeneralNoise,
}


def register_noise_model(name: str, noise_class: type) -> None:
    """Register a custom noise model parser for use with config dictionaries.

    The noise class must have a classmethod 'from_config' that takes
    a configuration dictionary and returns an instance of one of the
    built-in noise models (PassThroughNoise, DepolarizingNoise, etc.).

    Note: Due to the Rust backend, custom Python noise models cannot be
    used directly. The from_config method must return one of the built-in
    noise types. This is useful for custom configuration parsing, validation,
    or changing default values.

    Args:
        name: Name to use in configuration 'type' field
        noise_class: Class with from_config classmethod that returns a built-in noise type

    Example:
        >>> class CustomDepolarizingParser:
        ...     @classmethod
        ...     def from_config(cls, config):
        ...         # Custom validation or defaults
        ...         p = config.get("p", 0.1)  # Different default
        ...         if p > 0.5:
        ...             raise ValueError("p too high")
        ...         return DepolarizingNoise(p=p)
        ...
        >>> register_noise_model("MyDepolarizing", CustomDepolarizingParser)
        >>>
        >>> config = {"noise": {"type": "MyDepolarizing", "p": 0.05}}
        >>> sim = qasm_sim("...").config(config).build()
    """
    if not hasattr(noise_class, "from_config"):
        raise ValueError(
            f"Noise class {noise_class.__name__} must have a 'from_config' classmethod"
        )
    _NOISE_MODEL_REGISTRY[name] = noise_class
