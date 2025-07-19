"""Enhanced LLVM simulation with full feature parity with qasm_sim.

This module provides a Python interface to the Rust llvm_sim implementation,
offering noise models, parallelization, and multiple quantum engines.
"""

from typing import Dict, List, Optional, Union, Callable
from pathlib import Path
from dataclasses import dataclass

# Import the Rust bindings
from pecos_rslib._pecos_rslib import (
    llvm_sim_builder as _rust_llvm_sim_builder,
    LlvmNoiseModel,
    LlvmQuantumEngine,
    ShotVec,
)


@dataclass
class PassThroughNoise:
    """No noise configuration."""
    pass


@dataclass
class DepolarizingNoise:
    """Standard depolarizing noise configuration."""
    p: float


@dataclass
class DepolarizingCustomNoise:
    """Custom depolarizing noise configuration."""
    p_prep: float
    p_meas: float
    p1: float
    p2: float


@dataclass
class BiasedDepolarizingNoise:
    """Biased depolarizing noise configuration."""
    p: float


class LlvmSimBuilder:
    """Builder for LLVM simulations with full feature parity with qasm_sim."""
    
    def __init__(self, rust_builder):
        """Initialize with a Rust builder instance."""
        self._rust_builder = rust_builder
    
    @classmethod
    def guppy(cls, guppy_func: Callable) -> "LlvmSimBuilder":
        """Create an LLVM simulation builder from a Guppy function.
        
        This method compiles a Guppy function to HUGR, then to LLVM IR,
        and creates an LLVM simulation builder with the same interface as llvm_sim().
        
        Args:
            guppy_func: A function decorated with @guppy
            
        Returns:
            LlvmSimBuilder: Builder for configuring the simulation
            
        Examples:
            >>> from guppylang import guppy
            >>> from guppylang.std.quantum import qubit, h, measure
            >>> 
            >>> @guppy
            ... def bell_test() -> tuple[bool, bool]:
            ...     q1, q2 = qubit(), qubit()
            ...     h(q1)
            ...     cx(q1, q2)
            ...     return measure(q1), measure(q2)
            ...
            >>> # Same interface as llvm_sim() but starting from Guppy
            >>> results = LlvmSimBuilder.guppy(bell_test).seed(42).run(1000)
            >>> 
            >>> # Or via the convenience function (see below)
            >>> results = llvm_sim.guppy(bell_test).max_qubits(10).run(1000)
        """
        try:
            # Import Guppy compilation tools
            from pecos.compilation_pipeline import compile_guppy_to_hugr, compile_hugr_to_llvm
        except ImportError:
            raise ImportError(
                "Guppy compilation tools not available. Install with: pip install quantum-pecos[guppy]"
            )
        
        # Compile Guppy to LLVM IR
        hugr_bytes = compile_guppy_to_hugr(guppy_func)
        llvm_ir = compile_hugr_to_llvm(hugr_bytes)
        
        # Create standard LLVM sim builder with the compiled IR
        rust_builder = _rust_llvm_sim_builder(llvm_ir)
        return cls(rust_builder)
    
    def seed(self, seed: int) -> "LlvmSimBuilder":
        """Set random seed for reproducibility."""
        self._rust_builder = self._rust_builder.seed(seed)
        return self
    
    def workers(self, workers: int) -> "LlvmSimBuilder":
        """Set number of worker threads for parallelization."""
        self._rust_builder = self._rust_builder.workers(workers)
        return self
    
    def with_depolarizing_noise(self, p: float) -> "LlvmSimBuilder":
        """Enable uniform depolarizing noise."""
        self._rust_builder = self._rust_builder.with_depolarizing_noise(p)
        return self
    
    def with_custom_depolarizing_noise(
        self,
        p_prep: float,
        p_meas: float,
        p1: float,
        p2: float
    ) -> "LlvmSimBuilder":
        """Enable custom depolarizing noise with different probabilities."""
        self._rust_builder = self._rust_builder.with_custom_depolarizing_noise(
            p_prep, p_meas, p1, p2
        )
        return self
    
    def with_biased_depolarizing_noise(self, p: float) -> "LlvmSimBuilder":
        """Enable biased depolarizing noise."""
        self._rust_builder = self._rust_builder.with_biased_depolarizing_noise(p)
        return self
    
    def with_state_vector_engine(self) -> "LlvmSimBuilder":
        """Use state vector quantum engine (default)."""
        self._rust_builder = self._rust_builder.with_state_vector_engine()
        return self
    
    def with_sparse_stabilizer_engine(self) -> "LlvmSimBuilder":
        """Use sparse stabilizer quantum engine."""
        self._rust_builder = self._rust_builder.with_sparse_stabilizer_engine()
        return self
    
    def noise(self, noise_model: Union[PassThroughNoise, DepolarizingNoise, 
                                       DepolarizingCustomNoise, BiasedDepolarizingNoise]) -> "LlvmSimBuilder":
        """Set noise model from configuration object."""
        if isinstance(noise_model, PassThroughNoise):
            rust_noise = LlvmNoiseModel.PassThrough()
        elif isinstance(noise_model, DepolarizingNoise):
            rust_noise = LlvmNoiseModel.Depolarizing(p=noise_model.p)
        elif isinstance(noise_model, DepolarizingCustomNoise):
            rust_noise = LlvmNoiseModel.DepolarizingCustom(
                p_prep=noise_model.p_prep,
                p_meas=noise_model.p_meas,
                p1=noise_model.p1,
                p2=noise_model.p2
            )
        elif isinstance(noise_model, BiasedDepolarizingNoise):
            rust_noise = LlvmNoiseModel.BiasedDepolarizing(p=noise_model.p)
        else:
            raise ValueError(f"Unknown noise model type: {type(noise_model)}")
        
        self._rust_builder = self._rust_builder.noise(rust_noise)
        return self
    
    def quantum_engine(self, engine: str) -> "LlvmSimBuilder":
        """Set quantum engine type by name."""
        if engine.lower() == "statevector":
            rust_engine = LlvmQuantumEngine.StateVector
        elif engine.lower() == "sparsestabilizer":
            rust_engine = LlvmQuantumEngine.SparseStabilizer
        else:
            raise ValueError(f"Unknown quantum engine: {engine}")
        
        self._rust_builder = self._rust_builder.quantum_engine(rust_engine)
        return self
    
    def verbose(self, verbose: bool = True) -> "LlvmSimBuilder":
        """Enable verbose output."""
        self._rust_builder = self._rust_builder.verbose(verbose)
        return self
    
    def debug(self, debug: bool = True) -> "LlvmSimBuilder":
        """Enable debug information."""
        self._rust_builder = self._rust_builder.debug(debug)
        return self
    
    def max_qubits(self, max_qubits: int) -> "LlvmSimBuilder":
        """Set maximum number of qubits allowed for allocation."""
        self._rust_builder = self._rust_builder.max_qubits(max_qubits)
        return self
    
    def keep_temp_files(self, keep: bool = True) -> "LlvmSimBuilder":
        """Keep temporary files after simulation."""
        self._rust_builder = self._rust_builder.keep_temp_files(keep)
        return self
    
    def build(self) -> "LlvmSimulation":
        """Build the simulation for multiple runs."""
        rust_sim = self._rust_builder.build()
        return LlvmSimulation(rust_sim)
    
    def run(self, shots: int) -> ShotVec:
        """Build and run the simulation in one call."""
        return self._rust_builder.run(shots)


class LlvmSimulation:
    """A built LLVM simulation ready to run multiple times."""
    
    def __init__(self, rust_simulation):
        """Initialize with a Rust simulation instance."""
        self._rust_simulation = rust_simulation
    
    def run(self, shots: int) -> ShotVec:
        """Run the simulation with the given number of shots."""
        return self._rust_simulation.run(shots)
    
    def stats(self) -> tuple[int, int]:
        """Get statistics about the simulation (total_shots, total_runs)."""
        return self._rust_simulation.stats()


def llvm_sim(source: Union[str, Path]) -> LlvmSimBuilder:
    """Create an LLVM simulation builder with full feature parity with qasm_sim.
    
    This is the main entry point for LLVM-based quantum simulations, providing
    noise models, parallelization, and multiple quantum engines.
    
    Args:
        source: LLVM IR string or file path
        
    Returns:
        LlvmSimBuilder: Builder for configuring the simulation
        
    Examples:
        >>> # From LLVM IR string
        >>> llvm_ir = '''
        ... define void @main() #0 {
        ...     %0 = call i64 @__quantum__rt__qubit_allocate()
        ...     call void @__quantum__qis__h__body(i64 %0)
        ...     ret void
        ... }
        ... attributes #0 = { "EntryPoint" }
        ... '''
        >>> results = llvm_sim(llvm_ir).seed(42).run(1000)
        
        >>> # From Guppy function (convenience method)
        >>> from guppylang import guppy
        >>> from guppylang.std.quantum import qubit, h, measure
        >>> 
        >>> @guppy
        ... def simple_circuit() -> bool:
        ...     q = qubit()
        ...     h(q)
        ...     return measure(q)
        ...
        >>> results = llvm_sim.guppy(simple_circuit).seed(42).run(1000)
        
        >>> # With noise and parallelization
        >>> results = llvm_sim(llvm_ir) \\
        ...     .seed(42) \\
        ...     .workers(8) \\
        ...     .with_depolarizing_noise(0.01) \\
        ...     .run(10000)
        
        >>> # With custom quantum engine
        >>> results = llvm_sim(llvm_ir) \\
        ...     .with_sparse_stabilizer_engine() \\
        ...     .run(1000)
        
        >>> # Build once, run many
        >>> sim = llvm_sim(llvm_ir).seed(42).build()
        >>> results1 = sim.run(100)
        >>> results2 = sim.run(1000)
    """
    if isinstance(source, Path):
        source = str(source)
    
    rust_builder = _rust_llvm_sim_builder(source)
    return LlvmSimBuilder(rust_builder)


# Add convenience method to the function object
llvm_sim.guppy = LlvmSimBuilder.guppy


# Export the main function and noise model classes
__all__ = [
    "llvm_sim",
    "LlvmSimBuilder",
    "LlvmSimulation",
    "PassThroughNoise",
    "DepolarizingNoise", 
    "DepolarizingCustomNoise",
    "BiasedDepolarizingNoise",
]