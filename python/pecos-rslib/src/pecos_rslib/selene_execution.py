"""Execute quantum programs using Selene's infrastructure.

This module provides functions to execute quantum programs through Selene,
respecting Selene's architecture where programs are compiled to SeleneInstances
and executed through Selene's simulation backends.
"""

import logging
import tempfile
from pathlib import Path
from typing import Callable, Union

logger = logging.getLogger(__name__)


ProgramInput = Union[Callable, bytes, dict, str]


def execute_via_selene(
    program: ProgramInput,
    shots: int = 1000,
    backend: str | None = None,
    **kwargs: object,
) -> dict[str, object]:
    """Execute a quantum program using Selene's infrastructure.

    This function compiles the program to a SeleneInstance and executes it
    using Selene's simulation backends (Quest, Stim, Coinflip, etc.).

    Args:
        program: Either a Guppy function, HUGR Package, or HUGR bytes
        shots: Number of shots to run (default 1000)
        backend: Selene backend to use ('quest', 'stim', 'coinflip', etc.)
        **kwargs: Additional arguments passed to the backend

    Returns:
        Dictionary containing execution results

    Raises:
        ImportError: If Selene is not available
        RuntimeError: If compilation or execution fails
    """
    try:
        from selene_sim import Coinflip, Quest, Stim
        from selene_sim.build import build
    except ImportError as e:
        raise ImportError(f"Selene simulation infrastructure not available: {e}") from e

    # Prepare the HUGR package
    hugr_package = None

    # Check if it's a Guppy function
    if callable(program) and hasattr(program, "compile"):
        # It's a Guppy function
        try:
            hugr_package = program.compile()
            logger.info("Compiled Guppy function to HUGR package")
        except (AttributeError, RuntimeError, ValueError) as e:
            raise RuntimeError(f"Failed to compile Guppy function: {e}") from e

    # Check if it's already a HUGR Package
    elif hasattr(program, "to_json"):
        hugr_package = program
        logger.info("Using provided HUGR package")

    # Check if it's HUGR bytes
    elif isinstance(program, bytes):
        # Parse HUGR bytes
        import json

        try:
            hugr_data = json.loads(program)
            # Create a Package-like object or use the raw data
            hugr_package = hugr_data
            logger.info("Parsed HUGR bytes")
        except json.JSONDecodeError as e:
            raise ValueError("Invalid HUGR bytes - not valid JSON") from e

    else:
        raise ValueError(f"Unsupported program type: {type(program)}")

    # Build the SeleneInstance
    with tempfile.TemporaryDirectory() as tmpdir:
        build_dir = Path(tmpdir)

        try:
            instance = build(
                hugr_package,
                name="pecos_program",
                build_dir=build_dir,
                verbose=False,
            )
            logger.info("Built SeleneInstance at %s", instance.executable)
        except (ImportError, RuntimeError, ValueError) as e:
            raise RuntimeError(f"Failed to build SeleneInstance: {e}") from e

        # Select the backend
        if backend is None or backend == "quest":
            simulator = Quest()
        elif backend == "stim":
            simulator = Stim()
        elif backend == "coinflip":
            simulator = Coinflip()
        else:
            raise ValueError(f"Unknown backend: {backend}")

        # Run the simulation
        try:
            # Determine number of qubits (this should be extracted from HUGR)
            n_qubits = kwargs.pop("n_qubits", 10)  # Default to 10 qubits

            # Run multiple shots
            all_results = []
            for _ in range(shots):
                results = dict(instance.run(simulator, n_qubits=n_qubits, **kwargs))
                all_results.append(results)

            # Aggregate results
            return {
                "results": all_results,
                "shots": shots,
                "backend": f'selene_{backend or "quest"}',
                "executable": str(instance.executable),
            }

        except (ImportError, RuntimeError, ValueError, KeyError) as e:
            raise RuntimeError(f"Failed to execute SeleneInstance: {e}") from e


def create_selene_sim_builder(program: ProgramInput) -> object:
    """Create a simulation builder that uses Selene's infrastructure.

    This creates a builder pattern interface compatible with PECOS's sim()
    function, but using Selene's execution infrastructure underneath.

    Args:
        program: A Guppy function, HUGR Package, or HUGR bytes

    Returns:
        A SeleneSimBuilder instance
    """

    class SeleneSimBuilder:
        """Builder for Selene-based simulation."""

        def __init__(self, program: ProgramInput) -> None:
            self.program = program
            self.shots = 1000
            self.backend_name = "quest"
            self.n_qubits = None
            self.random_seed = None

        def qubits(self, n: int):
            """Set the number of qubits."""
            self.n_qubits = n
            return self

        def backend(self, name: str):
            """Set the Selene backend."""
            self.backend_name = name
            return self

        def seed(self, s: int):
            """Set the random seed."""
            self.random_seed = s
            return self

        def run(self, shots: int | None = None):
            """Execute the simulation."""
            if shots is not None:
                self.shots = shots

            kwargs = {}
            if self.n_qubits is not None:
                kwargs["n_qubits"] = self.n_qubits
            if self.random_seed is not None:
                kwargs["random_seed"] = self.random_seed

            return execute_via_selene(
                self.program,
                shots=self.shots,
                backend=self.backend_name,
                **kwargs,
            )

    return SeleneSimBuilder(program)
