"""Python interface for Guppy simulation with builder pattern.

This module provides a clean Python interface for running Guppy quantum program simulations.
It follows the same builder pattern as qasm_sim for consistency.

Example usage:
    >>> from guppylang import guppy
    >>> from guppylang.std.quantum import qubit, h, cx, measure
    >>> 
    >>> @guppy
    ... def bell_state() -> tuple[bool, bool]:
    ...     q0, q1 = qubit(), qubit()
    ...     h(q0)
    ...     cx(q0, q1)
    ...     return measure(q0), measure(q1)
    ...
    >>> # Using builder pattern
    >>> sim = guppy_sim(bell_state).seed(42).build()
    >>> results = sim.run(1000)
    >>>
    >>> # Or run directly
    >>> results = guppy_sim(bell_state).seed(42).run(1000)
"""

from dataclasses import dataclass
from typing import Callable, Dict, List, Optional, Any, Union
from pathlib import Path

# Import the Rust bindings (to be implemented)
try:
    from pecos_rslib._pecos_rslib import (
        GuppySimulation,
        GuppySimulationBuilder as _GuppySimulationBuilder,
        NoiseModel,
        QuantumEngine,
        run_guppy as _run_guppy,
        guppy_sim as _guppy_sim,
    )
    RUST_BINDINGS_AVAILABLE = True
except ImportError:
    RUST_BINDINGS_AVAILABLE = False
    # Fallback for development/testing
    class _GuppySimulationBuilder:
        pass
    class GuppySimulation:
        pass

__all__ = [
    "GuppySimulation",
    "GuppySimulationBuilder",
    "run_guppy",
    "guppy_sim",
    "GuppyCompilationMode",
]


@dataclass
class GuppyCompilationMode:
    """Compilation mode for Guppy programs."""
    
    # Direct HUGR to LLVM compilation
    HUGR_LLVM = "hugr_llvm"
    # Via PHIR intermediate representation
    PHIR = "phir"
    # Auto-select best available
    AUTO = "auto"


class GuppySimulationBuilder:
    """Builder for Guppy simulations with fluent interface.
    
    This provides a builder pattern similar to qasm_sim, allowing
    configuration before building the simulation.
    """
    
    def __init__(self, guppy_func: Callable):
        """Initialize builder with a Guppy function.
        
        Args:
            guppy_func: A function decorated with @guppy
        """
        self._guppy_func = guppy_func
        self._config = {
            "seed": None,
            "workers": None,
            "noise_model": None,
            "engine": None,
            "compilation_mode": GuppyCompilationMode.AUTO,
            "debug": False,
            "optimize": True,
        }
        self._built = False
        self._simulation = None
    
    def seed(self, seed: int) -> "GuppySimulationBuilder":
        """Set random seed for reproducible results.
        
        Args:
            seed: Random seed value
            
        Returns:
            Self for method chaining
        """
        self._config["seed"] = seed
        return self
    
    def workers(self, num_workers: int) -> "GuppySimulationBuilder":
        """Set number of worker threads for parallel execution.
        
        Args:
            num_workers: Number of worker threads
            
        Returns:
            Self for method chaining
        """
        self._config["workers"] = num_workers
        return self
    
    def noise(self, noise_model: Any) -> "GuppySimulationBuilder":
        """Set noise model for simulation.
        
        Args:
            noise_model: Noise model instance (e.g., DepolarizingNoise)
            
        Returns:
            Self for method chaining
        """
        self._config["noise_model"] = noise_model
        return self
    
    def engine(self, engine: Union[str, Any]) -> "GuppySimulationBuilder":
        """Set quantum simulation engine.
        
        Args:
            engine: Engine type (e.g., "StateVector", "SparseStabilizer")
            
        Returns:
            Self for method chaining
        """
        self._config["engine"] = engine
        return self
    
    def compilation_mode(self, mode: str) -> "GuppySimulationBuilder":
        """Set compilation mode for Guppy to LLVM.
        
        Args:
            mode: Compilation mode ("hugr_llvm", "phir", or "auto")
            
        Returns:
            Self for method chaining
        """
        self._config["compilation_mode"] = mode
        return self
    
    def debug(self, enable: bool = True) -> "GuppySimulationBuilder":
        """Enable or disable debug output.
        
        Args:
            enable: Whether to enable debug output
            
        Returns:
            Self for method chaining
        """
        self._config["debug"] = enable
        return self
    
    def optimize(self, enable: bool = True) -> "GuppySimulationBuilder":
        """Enable or disable LLVM optimizations.
        
        Args:
            enable: Whether to enable optimizations
            
        Returns:
            Self for method chaining
        """
        self._config["optimize"] = enable
        return self
    
    def config(self, config_dict: Dict[str, Any]) -> "GuppySimulationBuilder":
        """Apply configuration from dictionary.
        
        Args:
            config_dict: Configuration dictionary
            
        Returns:
            Self for method chaining
        """
        self._config.update(config_dict)
        return self
    
    def build(self) -> "GuppySimulation":
        """Build the simulation with current configuration.
        
        Returns:
            GuppySimulation instance ready to run
            
        Raises:
            RuntimeError: If Rust bindings are not available
        """
        if not RUST_BINDINGS_AVAILABLE:
            # For now, use Python fallback
            return self._build_python_fallback()
        
        if self._built and self._simulation:
            return self._simulation
        
        # Call Rust builder
        self._simulation = _guppy_sim(self._guppy_func, self._config)
        self._built = True
        return self._simulation
    
    def run(self, shots: int) -> Dict[str, List[Any]]:
        """Build and run simulation in one call.
        
        Args:
            shots: Number of measurement shots
            
        Returns:
            Dictionary of measurement results
        """
        sim = self.build()
        return sim.run(shots)
    
    def _build_python_fallback(self) -> "GuppySimulation":
        """Python fallback implementation for development."""
        # Import here to avoid circular dependencies
        from pecos.compilation_pipeline import compile_guppy_to_llvm, execute_llvm
        
        class PythonGuppySimulation:
            def __init__(self, guppy_func, config):
                self.guppy_func = guppy_func
                self.config = config
                self._llvm_ir = None
                self._compiled = False
            
            def compile(self):
                """Compile Guppy to LLVM IR."""
                if not self._compiled:
                    self._llvm_ir = compile_guppy_to_llvm(
                        self.guppy_func,
                        debug_info=self.config.get("debug", False)
                    )
                    self._compiled = True
            
            def run(self, shots: int) -> Dict[str, List[Any]]:
                """Run simulation with given shots."""
                self.compile()
                
                # Execute via pecos-llvm-runtime
                result = execute_llvm(
                    self._llvm_ir,
                    shots=shots,
                    config=self.config
                )
                
                # Convert to columnar format like qasm_sim
                return self._format_results(result)
            
            def _format_results(self, result: Dict) -> Dict[str, List[Any]]:
                """Format results in columnar format."""
                # Result from execute_llvm has format:
                # {"results": [...], "shots": N, "backend": "..."}
                raw_results = result.get("results", [])
                
                if not raw_results:
                    return {}
                
                # Convert to columnar format
                # For Bell state: [(False, False), (True, True), ...] 
                # becomes {"_result": [0, 3, ...]}
                formatted = {}
                
                # Handle different result types
                if isinstance(raw_results[0], (list, tuple)):
                    # Multiple return values - convert tuple to int
                    values = []
                    for res in raw_results:
                        # Convert bool tuple to integer
                        val = 0
                        for i, b in enumerate(res):
                            if b:
                                val |= (1 << i)
                        values.append(val)
                    formatted["_result"] = values
                else:
                    # Single return value
                    formatted["_result"] = raw_results
                
                return formatted
        
        return PythonGuppySimulation(self._guppy_func, self._config)


def guppy_sim(guppy_func: Callable) -> GuppySimulationBuilder:
    """Create a Guppy simulation builder for flexible configuration.
    
    This provides a builder pattern for Guppy simulations, allowing you to
    build once and run multiple times with different shot counts.
    
    Args:
        guppy_func: A function decorated with @guppy
        
    Returns:
        GuppySimulationBuilder that can be configured and run
        
    Example:
        >>> from guppylang import guppy
        >>> from guppylang.std.quantum import qubit, h, measure
        >>> 
        >>> @guppy
        ... def simple_circuit() -> bool:
        ...     q = qubit()
        ...     h(q)
        ...     return measure(q)
        ...
        >>> # Build once, run multiple times
        >>> sim = guppy_sim(simple_circuit).seed(42).build()
        >>> results_100 = sim.run(100)
        >>> results_1000 = sim.run(1000)
        >>>
        >>> # Or run directly without building
        >>> results = guppy_sim(simple_circuit).seed(42).run(1000)
        >>>
        >>> # With noise model (when implemented in Rust)
        >>> from pecos_rslib.noise import DepolarizingNoise
        >>> results = (
        ...     guppy_sim(simple_circuit)
        ...     .noise(DepolarizingNoise(p=0.01))
        ...     .workers(4)
        ...     .run(1000)
        ... )
    """
    return GuppySimulationBuilder(guppy_func)


def run_guppy(
    guppy_func: Callable,
    shots: int,
    noise_model: Optional[Any] = None,
    engine: Optional[Any] = None,
    workers: Optional[int] = None,
    seed: Optional[int] = None,
) -> Dict[str, List[Any]]:
    """Run a Guppy simulation with specified parameters.
    
    NOTE: This function is provided for backward compatibility.
    Consider using the new unified API instead:
    
        from pecos_rslib import selene_engine
        
        results = selene_engine().program(guppy_func).qubits(n).to_sim().seed(42).noise(noise_model).run(shots)
    
    Args:
        guppy_func: A function decorated with @guppy
        shots: Number of measurement shots to perform
        noise_model: Noise model instance or None for no noise
        engine: Quantum simulation engine
        workers: Number of worker threads (None for default)
        seed: Random seed for reproducibility
        
    Returns:
        Dict mapping result names to lists of measurement values
        
    Example:
        >>> from guppylang import guppy
        >>> from guppylang.std.quantum import qubit, h, cx, measure
        >>> 
        >>> @guppy
        ... def bell_state() -> tuple[bool, bool]:
        ...     q0, q1 = qubit(), qubit()
        ...     h(q0)
        ...     cx(q0, q1)
        ...     return measure(q0), measure(q1)
        ...
        >>> results = run_guppy(bell_state, shots=1000, seed=42)
        >>> print(results)  # {"_result": [0, 3, 0, 3, ...]}
    """
    # Use the new unified API with selene_engine
    from pecos_rslib import selene_engine
    
    # For Guppy, we need to determine the number of qubits
    # This is a limitation - we'll use a default or try to infer
    # In practice, the user should use the new API directly
    num_qubits = 10  # Default, should be sufficient for most cases
    
    sim_builder = selene_engine().program(guppy_func).qubits(num_qubits).to_sim()
    
    if seed is not None:
        sim_builder = sim_builder.seed(seed)
    if workers is not None:
        sim_builder = sim_builder.workers(workers)
    if noise_model is not None:
        sim_builder = sim_builder.noise(noise_model)
    if engine is not None:
        sim_builder = sim_builder.quantum_engine(engine)
    
    return sim_builder.run(shots)