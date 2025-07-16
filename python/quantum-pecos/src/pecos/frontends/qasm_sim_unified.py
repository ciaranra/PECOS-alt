"""
Example implementation of qasm_sim() using the new unified API approach.

This shows how the existing qasm_sim() function could be reimplemented
as a wrapper around the new qasm_engine().to_sim() approach.
"""

from typing import Optional, Dict, List, Any
from pecos_rslib import qasm_engine  # The new unified API
from pecos_rslib import DepolarizingNoise, QuantumEngine


class QasmSimulationBuilderUnified:
    """A backward-compatible builder that wraps the new unified API."""
    
    def __init__(self, qasm: str):
        self._builder = qasm_engine().qasm(qasm).to_sim()
        self._wasm_path = None
    
    def seed(self, seed: int) -> "QasmSimulationBuilderUnified":
        """Set the random seed."""
        self._builder = self._builder.seed(seed)
        return self
    
    def noise(self, noise_model: Any) -> "QasmSimulationBuilderUnified":
        """Set the noise model."""
        self._builder = self._builder.noise(noise_model)
        return self
    
    def workers(self, workers: int) -> "QasmSimulationBuilderUnified":
        """Set the number of worker threads."""
        self._builder = self._builder.workers(workers)
        return self
    
    def auto_workers(self) -> "QasmSimulationBuilderUnified":
        """Automatically set workers based on CPU cores."""
        self._builder = self._builder.auto_workers()
        return self
    
    def quantum_engine(self, engine: QuantumEngine) -> "QasmSimulationBuilderUnified":
        """Set the quantum simulation engine."""
        self._builder = self._builder.quantum_engine(engine)
        return self
    
    def wasm(self, wasm_path: str) -> "QasmSimulationBuilderUnified":
        """Set WASM module for foreign functions."""
        # This would need to be handled differently in the unified API
        # For now, store it for later use
        self._wasm_path = wasm_path
        return self
    
    def with_binary_string_format(self) -> "QasmSimulationBuilderUnified":
        """Enable binary string format for results."""
        # This might need to be adapted based on the unified API's approach
        return self
    
    def build(self) -> "QasmSimulation":
        """Build a reusable simulation object."""
        return QasmSimulation(self._builder.build(), self._wasm_path)
    
    def run(self, shots: int) -> Dict[str, List[int]]:
        """Run the simulation directly."""
        return self._builder.run(shots)


class QasmSimulation:
    """A reusable simulation object wrapping the unified API."""
    
    def __init__(self, sim, wasm_path: Optional[str] = None):
        self._sim = sim
        self._wasm_path = wasm_path
    
    def run(self, shots: int) -> Dict[str, List[int]]:
        """Run the simulation with the specified number of shots."""
        # If WASM was specified, it might need special handling
        return self._sim.run(shots)
    
    def reset(self):
        """Reset the simulation state."""
        return self._sim.reset()


def qasm_sim(qasm: str) -> QasmSimulationBuilderUnified:
    """
    Create a QASM simulation builder using the new unified API.
    
    This function provides backward compatibility with the existing API
    while leveraging the new unified simulation architecture.
    
    Args:
        qasm: QASM code as a string
    
    Returns:
        QasmSimulationBuilderUnified that can be configured and run
    
    Example:
        >>> qasm = '''
        ... OPENQASM 2.0;
        ... include "qelib1.inc";
        ... qreg q[2];
        ... creg c[2];
        ... h q[0];
        ... cx q[0], q[1];
        ... measure q -> c;
        ... '''
        >>> 
        >>> # Direct run
        >>> results = qasm_sim(qasm).seed(42).noise(DepolarizingNoise(p=0.01)).run(1000)
        >>> 
        >>> # Build once, run multiple times
        >>> sim = qasm_sim(qasm).seed(42).build()
        >>> results_100 = sim.run(100)
        >>> results_1000 = sim.run(1000)
    """
    return QasmSimulationBuilderUnified(qasm)


# Alternative implementation that could be even simpler if the APIs align well:
def qasm_sim_simple(qasm: str):
    """
    Even simpler implementation if we just want to expose the unified API directly.
    
    This would be a breaking change but provides a cleaner API.
    """
    return qasm_engine().qasm(qasm).to_sim()


# The beauty of this approach is that all the existing code like:
#   qasm_sim(qasm).seed(42).noise(DepolarizingNoise(p=0.01)).run(1000)
# 
# Would work exactly the same, but underneath it's using the new unified architecture!