# Copyright 2023 The PECOS Developers
#
# Licensed under the Apache License, Version 2.0 (the "License"); you may not use this file except in compliance with
# the License.You may obtain a copy of the License at
#
#     https://www.apache.org/licenses/LICENSE-2.0
#
# Unless required by applicable law or agreed to in writing, software distributed under the License is distributed on an
# "AS IS" BASIS, WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied. See the License for the
# specific language governing permissions and limitations under the License.

"""Common protocols used throughout PECOS."""

from __future__ import annotations

from typing import TYPE_CHECKING, Protocol, runtime_checkable

if TYPE_CHECKING:
    from collections.abc import Callable, Generator, Iterator, Sequence
    from typing import Any

    from pecos.circuits import LogicalCircuit, QuantumCircuit
    from pecos.error_models.class_errors_circuit import ErrorCircuits
    from pecos.error_models.parent_class_error_gen import ParentErrorModel
    from pecos.misc.symbol_library import JSONDict
    from pecos.type_defs import (
        ErrorParams,
        LocationSet,
        OutputDict,
        QECCGateParams,
        QECCInstrParams,
    )


class Decoder(Protocol):
    """Protocol for decoder objects."""

    def decode(self, syndrome: set[int]) -> QuantumCircuit | LogicalCircuit: ...


class CCOPProtocol(Protocol):
    """Protocol for CCOP (Classical Co-processor) objects."""

    def exec(self, func_name: str, args: list[int | float | bool]) -> int | None: ...


class SimulatorState(Protocol):
    """Protocol for simulator state objects."""

    # States typically have methods like apply_gate, measure, etc.


class CircuitRunner(Protocol):
    """Protocol for circuit runner objects."""

    def run(
        self,
        state: SimulatorState,
        circuit: QuantumCircuit | LogicalCircuit,
    ) -> tuple[SimulatorState, dict[str, int | list[int]]]: ...


class EngineRunner(Protocol):
    """Protocol for engine runner objects."""

    debug: bool
    ccop: CCOPProtocol | None
    circuit: QuantumCircuit


class CircuitInspector(Protocol):
    """Protocol for circuit inspection functionality."""

    def analyze(
        self,
        tick_circuit: QuantumCircuit,
        time: int,
        output: OutputDict,
    ) -> None: ...


class MachineProtocol(Protocol):
    """Protocol for machine implementations.

    This protocol replaces the Machine ABC and defines the interface that all
    machine implementations must follow.
    """

    machine_params: dict | None
    num_qubits: int | None
    metadata: dict | None
    pos: dict | None
    leaked_qubits: set[int]  # Used in leakage noise models
    lost_qubits: set[int]  # Used for tracking lost qubits
    qubit_set: set[int]  # Set of all qubit indices

    def reset(self) -> None:
        """Reset state to initialization state."""
        ...

    def init(self, num_qubits: int | None = None) -> None:
        """Initialize the machine with the given number of qubits."""
        ...

    def shot_reinit(self) -> None:
        """Run all code needed at the beginning of each shot, e.g., resetting state."""
        ...

    def process(self, op_buffer: list) -> list:
        """Process a buffer of operations."""
        ...


class ErrorModelProtocol(Protocol):
    """Protocol for error model implementations.

    This protocol replaces the ErrorModel ABC and defines the interface that all
    error model implementations must follow.
    """

    error_params: dict
    machine: MachineProtocol | None
    num_qubits: int | None

    def reset(self) -> None:
        """Reset state to initialization state."""
        ...

    def init(self, num_qubits: int, machine: MachineProtocol | None = None) -> None:
        """Initialize the error model with the given number of qubits and optional machine."""
        ...

    def shot_reinit(self) -> None:
        """Run all code needed at the beginning of each shot, e.g., resetting state."""
        ...

    def process(self, qops: list, call_back: Callable | None = None) -> list | None:
        """Process quantum operations and potentially apply errors."""
        ...


class OpProcessorProtocol(Protocol):
    """Protocol for operation processor implementations.

    This protocol replaces the OpProcessor ABC and defines the interface that all
    operation processor implementations must follow.
    """

    def reset(self) -> None:
        """Reset state to initialization state."""
        ...

    def init(self) -> None:
        """Initialize the operation processor."""
        ...

    def shot_reinit(self) -> None:
        """Run all code needed at the beginning of each shot."""
        ...

    def process(self, buffered_ops: list) -> list:
        """Process a buffer of operations."""
        ...

    def process_meas(self, measurements: dict) -> dict:
        """Process measurement operations."""
        ...


class ClassicalInterpreterProtocol(Protocol):
    """Protocol for classical interpreter implementations.

    This protocol replaces the ClassicalInterpreter ABC and defines the interface that all
    classical interpreter implementations must follow.
    """

    program: Any
    foreign_obj: Any

    def reset(self) -> None:
        """Reset state to initialization state."""
        ...

    def init(
        self,
        program: str | dict | QuantumCircuit,
        foreign_classical_obj: object | None = None,
    ) -> int:
        """Initialize the interpreter with a program and optional foreign object."""
        ...

    def shot_reinit(self) -> None:
        """Run all code needed at the beginning of each shot."""
        ...

    def execute(self, sequence: Sequence | None) -> Generator:
        """Execute the program with an optional sequence of inputs."""
        ...

    def receive_results(self, qsim_results: list[dict]) -> None:
        """Receive results from quantum simulation."""
        ...

    def results(self) -> dict:
        """Dumps program final results."""
        ...


class ForeignObjectProtocol(Protocol):
    """Protocol for foreign object implementations.

    This protocol replaces the ForeignObject ABC and defines the interface that all
    foreign object implementations must follow.
    """

    def init(self) -> None:
        """Initialize object before a set of simulations."""
        ...

    def new_instance(self) -> None:
        """Create new instance/internal state."""
        ...

    def get_funcs(self) -> list[str]:
        """Get a list of function names available from the object."""
        ...

    def exec(self, func_name: str, args: Sequence) -> tuple:
        """Execute a function given a list of arguments."""
        ...


class SimulatorProtocol(Protocol):
    """Protocol for quantum simulators.

    This protocol defines the interface that all simulator implementations must follow.
    For convenience, simulators can inherit from DefaultSimulator which provides
    default implementations of these methods, or they can implement this protocol
    directly for custom behavior.
    """

    bindings: dict

    def run_gate(
        self,
        symbol: str,
        locations: set[int] | set[tuple[int, ...]],
        **params: dict,
    ) -> dict[int | tuple[int, ...], Any]: ...

    def run_circuit(
        self,
        circuit: QuantumCircuit,
        removed_locations: set | None = None,
    ) -> dict[int | tuple[int, ...], Any]: ...

    def run_circuit_with_errors(
        self,
        circuit: QuantumCircuit,
        error_gen: ParentErrorModel,
        error_params: dict,
    ) -> tuple[dict, OutputDict]: ...


class ErrorGenerator(Protocol):
    """Protocol for error generators/models."""

    error_params: ErrorParams

    def start(
        self,
        circuit: QuantumCircuit,
        error_params: ErrorParams,
    ) -> ErrorCircuits: ...

    def generate_tick_errors(
        self,
        time: int,
        gate_time: dict[str, set[int]],
    ) -> QuantumCircuit: ...


@runtime_checkable
class LogicalGateProtocol(Protocol):
    """Protocol for logical gate implementations.

    This protocol replaces the LogicalGate parent class and defines the interface
    that all logical gate implementations must follow.
    """

    symbol: str
    qecc: Any  # Reference to the QECC instance
    gate_params: QECCGateParams
    params: QECCGateParams
    instr_symbols: list[str] | None
    instr_instances: list[Any]
    circuits: list[Any]
    error_free: bool
    forced_outcome: bool
    qecc_params_tuple: tuple

    def __eq__(self, other: object) -> bool: ...
    def __hash__(self) -> int: ...


@runtime_checkable
class LogicalInstructionProtocol(Protocol):
    """Protocol for logical instruction implementations.

    This protocol replaces the LogicalInstruction parent class and defines the interface
    that all logical instruction implementations must follow.
    """

    symbol: str
    qecc: Any  # Reference to the QECC instance
    params: QECCInstrParams
    gate_params: QECCInstrParams
    abstract_circuit: Any | None
    circuit: Any | None
    data_qudit_set: set[int]
    ancilla_qudit_set: set[int]
    qudit_set: set[int]
    output_set: set[int]
    gate_params_tuple: tuple

    def items(self) -> Iterator[tuple[str, LocationSet, JSONDict]]: ...
    def __eq__(self, other: object) -> bool: ...
    def __hash__(self) -> int: ...
    def plot(self, figsize: tuple[int, int] | None = None) -> None: ...


@runtime_checkable
class QECCProtocol(Protocol):
    """Protocol for Quantum Error Correcting Codes.

    This protocol defines the interface that all QECC implementations must follow.
    For convenience, QECCs can inherit from DefaultQECC which provides default
    implementations of common methods, or they can implement this protocol
    directly for custom behavior.
    """

    # Required attributes
    name: str | None
    qecc_params: dict
    distance: int | None
    num_data_qudits: int | None
    num_ancilla_qudits: int | None
    num_logical_qudits: int | None
    num_qudits: int

    # Qudit management
    qudit_set: set[int]
    data_qudit_set: set[int]
    ancilla_qudit_set: set[int]

    # Layout and geometry
    layout: dict
    position2qudit: dict
    lattice_dimensions: dict
    sides: dict

    # Gate and instruction management
    sym2gate_class: dict
    sym2instruction_class: dict
    instr_set: set
    gate_set: set

    # Circuit compilation
    circuit_compiler: Any
    mapping: Any

    def gate(
        self,
        symbol: str,
        **gate_params: QECCGateParams,
    ) -> LogicalGateProtocol: ...
    def instruction(
        self,
        symbol: str,
        **instr_params: QECCInstrParams,
    ) -> LogicalInstructionProtocol: ...
    def distance(self, *args: int, **kwargs: int) -> int: ...
    def plot(self, figsize: tuple[int, int] | None = None) -> None: ...
