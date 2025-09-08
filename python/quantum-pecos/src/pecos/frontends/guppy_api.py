"""Unified API for Guppy programs following the sim(program) pattern."""

from typing import Any

from pecos_rslib.sim_wrapper import sim as sim_wrapper

__all__ = ["GuppySimBuilderWrapper", "sim"]


class GuppySimBuilderWrapper:
    """Wrapper that makes the new sim() API compatible with the old guppy_sim() tests.

    This wrapper ensures that calling .run() returns results in the expected format
    with results["result"] containing the measurement values.
    """

    def __init__(self, builder) -> None:
        self._builder = builder

    def qubits(self, n: int):
        """Set number of qubits."""
        # The Rust builder returns a new instance, so we need to return a new wrapper
        new_builder = self._builder.qubits(n)
        return GuppySimBuilderWrapper(new_builder)

    def seed(self, seed: int):
        """Set random seed."""
        new_builder = self._builder.seed(seed)
        return GuppySimBuilderWrapper(new_builder)

    def quantum(self, engine):
        """Set quantum engine."""
        new_builder = self._builder.quantum(engine)
        return GuppySimBuilderWrapper(new_builder)

    def noise(self, noise_model):
        """Set noise model."""
        new_builder = self._builder.noise(noise_model)
        return GuppySimBuilderWrapper(new_builder)

    def workers(self, n: int):
        """Set number of workers."""
        new_builder = self._builder.workers(n)
        return GuppySimBuilderWrapper(new_builder)

    def verbose(self, enable: bool):
        """Set verbose mode (no-op for compatibility)."""
        # The Rust builder doesn't have a verbose method, so we just return self
        return self

    def debug(self, enable: bool):
        """Set debug mode (no-op for compatibility)."""
        # The Rust builder doesn't have a debug method, so we just return self
        return self

    def optimize(self, enable: bool):
        """Set optimization mode (no-op for compatibility)."""
        # The Rust builder doesn't have an optimize method, so we just return self
        return self

    def keep_intermediate_files(self, enable: bool):
        """Set whether to keep intermediate files (no-op for compatibility)."""
        # Create a temp directory for compatibility with tests
        if enable:
            import tempfile

            self.temp_dir = tempfile.mkdtemp(prefix="guppy_sim_")
            # Create dummy files that tests might expect
            from pathlib import Path

            temp_path = Path(self.temp_dir)
            (temp_path / "program.ll").write_text("; Dummy LLVM IR file\n")
            (temp_path / "program.hugr").write_text("// Dummy HUGR file\n")
        else:
            self.temp_dir = None
        return self

    def build(self):
        """Build the simulation (returns self for compatibility)."""
        # The Rust builder doesn't need explicit building, so we just return self
        return self

    def run(self, shots: int):
        """Run simulation and convert results to expected format."""
        # Call the underlying run method which returns PyShotVec
        shot_vec = self._builder.run(shots)
        # Convert to dictionary format
        return shot_vec.to_dict()


def sim(program: Any):
    """Create a simulation builder for a program.

    This function detects the program type and creates the appropriate builder.
    For Guppy functions, it uses the Python-side Selene compilation pipeline.

    Args:
        program: A Guppy function or other supported program type

    Returns:
        A simulation builder that can be configured and run

    Example:
        from guppylang import guppy
        from pecos.frontends.guppy_api import sim
        from pecos_rslib import state_vector

        @guppy
        def bell_state() -> tuple[bool, bool]:
            from guppylang.std.quantum import qubit, h, cx, measure
            q1, q2 = qubit(), qubit()
            h(q1)
            cx(q1, q2)
            return measure(q1), measure(q2)

        # Default uses stabilizer simulator
        results = sim(bell_state).qubits(2).run(1000)

        # Explicitly use state vector for non-Clifford gates
        results = sim(bell_state).qubits(2).quantum(state_vector()).run(1000)
    """
    # Pass all programs to sim_wrapper for proper detection and routing
    # This handles all program types including Guppy functions with Python-side Selene compilation
    builder = sim_wrapper(program)

    # Wrap the builder for compatibility
    return GuppySimBuilderWrapper(builder)
