"""Enhanced Selene simulation with full feature parity with qasm_sim.

This module provides a Python interface to the Rust selene_sim implementation,
offering noise models, parallelization, and multiple quantum engines.
"""

from typing import Union, Callable
from pathlib import Path
from dataclasses import dataclass

# Import the Rust bindings
try:
    from pecos_rslib._pecos_rslib import (
        selene_sim_builder as _rust_selene_sim_builder,
        SeleneNoiseModel,
        SeleneQuantumEngine,
        ShotVec,
    )
except ImportError:
    # Old bindings not available, use stubs
    _rust_selene_sim_builder = None
    SeleneNoiseModel = None
    SeleneQuantumEngine = None
    from pecos_rslib._pecos_rslib import ShotVec

# Try to import HUGR support if available
try:
    from pecos_rslib._pecos_rslib import (
        selene_sim_builder_hugr as _rust_selene_sim_builder_hugr,
    )

    HUGR_SUPPORT = True
except ImportError:
    HUGR_SUPPORT = False


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


class SeleneSimBuilder:
    """Builder for Selene simulations with full feature parity with qasm_sim."""

    def __init__(self, rust_builder):
        """Initialize with a Rust builder instance."""
        self._rust_builder = rust_builder

    @classmethod
    def guppy(cls, guppy_func: Callable) -> "SeleneSimBuilder":
        """Create a Selene simulation builder from a Guppy function.

        This method compiles a Guppy function to HUGR and sends it directly
        to Selene for execution, bypassing the LLVM IR stage that has issues
        with quantum intrinsics.

        Args:
            guppy_func: A function decorated with @guppy

        Returns:
            SeleneSimBuilder: Builder for configuring the simulation

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
            >>> # Same interface as selene_sim() but starting from Guppy
            >>> results = SeleneSimBuilder.guppy(bell_test).seed(42).run(1000)
            >>>
            >>> # Or via the convenience function (see below)
            >>> results = selene_sim.guppy(bell_test).qubits(10).run(1000)
        """
        try:
            # Import Guppy compilation tools
            from pecos.compilation_pipeline import compile_guppy_to_hugr
        except ImportError:
            raise ImportError(
                "Guppy compilation tools not available. Install with: pip install quantum-pecos[guppy]"
            )

        if not HUGR_SUPPORT:
            raise RuntimeError(
                "HUGR support not available in Rust backend. "
                "Rebuild PECOS with HUGR feature enabled."
            )

        # Compile Guppy to HUGR
        hugr_bytes = compile_guppy_to_hugr(guppy_func)

        # Create Selene sim builder directly from HUGR bytes
        rust_builder = _rust_selene_sim_builder_hugr(hugr_bytes)
        return cls(rust_builder)

    @classmethod
    def hugr(cls, hugr_bytes: bytes) -> "SeleneSimBuilder":
        """Create a Selene simulation builder from HUGR bytes.

        This provides direct access to Selene's HUGR execution capability.

        Args:
            hugr_bytes: HUGR data as bytes

        Returns:
            SeleneSimBuilder: Builder for configuring the simulation
        """
        if not HUGR_SUPPORT:
            raise RuntimeError(
                "HUGR support not available in Rust backend. "
                "Rebuild PECOS with HUGR feature enabled."
            )

        rust_builder = _rust_selene_sim_builder_hugr(hugr_bytes)
        return cls(rust_builder)

    def seed(self, seed: int) -> "SeleneSimBuilder":
        """Set random seed for reproducibility."""
        self._rust_builder.seed(seed)
        return self

    def qubits(self, qubits: int) -> "SeleneSimBuilder":
        """Set number of qubits."""
        self._rust_builder.qubits(qubits)
        return self

    def noise(
        self,
        noise_model: Union[
            PassThroughNoise,
            DepolarizingNoise,
            DepolarizingCustomNoise,
            BiasedDepolarizingNoise,
        ],
    ) -> "SeleneSimBuilder":
        """Set noise model from configuration object."""
        if isinstance(noise_model, PassThroughNoise):
            rust_noise = SeleneNoiseModel.PassThrough()
        elif isinstance(noise_model, DepolarizingNoise):
            rust_noise = SeleneNoiseModel.Depolarizing(p=noise_model.p)
        elif isinstance(noise_model, DepolarizingCustomNoise):
            rust_noise = SeleneNoiseModel.DepolarizingCustom(
                p_prep=noise_model.p_prep,
                p_meas=noise_model.p_meas,
                p1=noise_model.p1,
                p2=noise_model.p2,
            )
        elif isinstance(noise_model, BiasedDepolarizingNoise):
            rust_noise = SeleneNoiseModel.BiasedDepolarizing(p=noise_model.p)
        else:
            raise ValueError(f"Unknown noise model type: {type(noise_model)}")

        self._rust_builder.noise(rust_noise)
        return self

    def quantum_engine(
        self, engine: Union[str, SeleneQuantumEngine]
    ) -> "SeleneSimBuilder":
        """Set quantum engine type by name or object."""
        if isinstance(engine, str):
            if engine.lower() == "statevector":
                rust_engine = SeleneQuantumEngine.StateVector()
            elif engine.lower() == "sparsestabilizer":
                rust_engine = SeleneQuantumEngine.SparseStabilizer()
            else:
                raise ValueError(f"Unknown quantum engine: {engine}")
        else:
            rust_engine = engine

        self._rust_builder.quantum_engine(rust_engine)
        return self

    def optimize(self) -> "SeleneSimBuilder":
        """Enable optimization."""
        self._rust_builder.optimize()
        return self

    def build(self) -> "SeleneSimulation":
        """Build the simulation for multiple runs."""
        rust_sim = self._rust_builder.build()
        return SeleneSimulation(rust_sim)

    def run(self, shots: int) -> ShotVec:
        """Build and run the simulation in one call."""
        # Build the simulation and run it
        sim = self.build()
        return sim.run(shots)


class SeleneSimulation:
    """A built Selene simulation ready to run multiple times."""

    def __init__(self, rust_simulation):
        """Initialize with a Rust simulation instance."""
        self._rust_simulation = rust_simulation

    def run(self, shots: int) -> ShotVec:
        """Run the simulation with the given number of shots."""
        return self._rust_simulation.run(shots)


def selene_sim(source: Union[str, Path]) -> SeleneSimBuilder:
    """Create a Selene simulation builder with full feature parity with qasm_sim.

    This is the main entry point for Selene-based quantum simulations, providing
    noise models, parallelization, and multiple quantum engines.

    Args:
        source: LLVM IR string or file path

    Returns:
        SeleneSimBuilder: Builder for configuring the simulation

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
        >>> results = selene_sim(llvm_ir).qubits(1).seed(42).run(1000)

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
        >>> results = selene_sim.guppy(simple_circuit).qubits(1).seed(42).run(1000)

        >>> # With noise and optimization
        >>> results = selene_sim(llvm_ir) \\
        ...     .qubits(1) \\
        ...     .seed(42) \\
        ...     .noise(DepolarizingNoise(0.01)) \\
        ...     .optimize() \\
        ...     .run(10000)

        >>> # With custom quantum engine
        >>> results = selene_sim(llvm_ir) \\
        ...     .qubits(1) \\
        ...     .quantum_engine("sparsestabilizer") \\
        ...     .run(1000)

        >>> # Build once, run many
        >>> sim = selene_sim(llvm_ir).qubits(1).seed(42).build()
        >>> results1 = sim.run(100)
        >>> results2 = sim.run(1000)
    """
    if isinstance(source, Path):
        source = str(source)

    rust_builder = _rust_selene_sim_builder(source)
    return SeleneSimBuilder(rust_builder)


# Add convenience methods to the function object
selene_sim.guppy = SeleneSimBuilder.guppy
selene_sim.hugr = SeleneSimBuilder.hugr


# Export the main function and noise model classes
__all__ = [
    "selene_sim",
    "SeleneSimBuilder",
    "SeleneSimulation",
    "PassThroughNoise",
    "DepolarizingNoise",
    "DepolarizingCustomNoise",
    "BiasedDepolarizingNoise",
]
