"""Python interface for QASM simulation with enhanced API.

For detailed usage examples and documentation, see docs/qasm_sim_usage.md
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
    get_noise_models,
    get_quantum_engines,
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
    "BiasedMeasurementNoise",
    "GeneralNoise",
    # Main interface
    "run_qasm",
    "qasm_sim",
]


# Noise model dataclasses


@dataclass
class PassThroughNoise:
    """No noise - ideal quantum simulation."""

    pass


@dataclass
class DepolarizingNoise:
    """Standard depolarizing noise with uniform probability.

    Args:
        p: Uniform error probability for all operations
    """

    p: float = 0.001


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


@dataclass
class BiasedDepolarizingNoise:
    """Biased depolarizing noise model.

    Args:
        p: Uniform probability for all operations
    """

    p: float = 0.001


@dataclass
class BiasedMeasurementNoise:
    """Biased measurement noise with different probabilities for 0→1 and 1→0 errors.

    Args:
        p0: Probability of measuring 1 when the true state is 0
        p1: Probability of measuring 0 when the true state is 1
    """

    p0: float = 0.01
    p1: float = 0.01


@dataclass
class GeneralNoise:
    """General noise model using default configuration."""

    pass


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
        >>> sim = qasm_sim(qasm) \\
        ...     .seed(42) \\
        ...     .noise(DepolarizingNoise(p=0.01)) \\
        ...     .build()
        >>> 
        >>> results_100 = sim.run(100)
        >>> results_1000 = sim.run(1000)
        >>> 
        >>> # Or run directly without building
        >>> results = qasm_sim(qasm) \\
        ...     .noise(DepolarizingNoise(p=0.01)) \\
        ...     .workers(4) \\
        ...     .run(1000)
    """
    return _qasm_sim(qasm)
