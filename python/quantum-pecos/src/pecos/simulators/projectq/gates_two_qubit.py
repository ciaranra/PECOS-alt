# Copyright 2019 The PECOS Developers
# Copyright 2018 National Technology & Engineering Solutions of Sandia, LLC (NTESS). Under the terms of Contract
# DE-NA0003525 with NTESS, the U.S. Government retains certain rights in this software.
#
# Licensed under the Apache License, Version 2.0 (the "License"); you may not use this file except in compliance with
# the License.You may obtain a copy of the License at
#
#     https://www.apache.org/licenses/LICENSE-2.0
#
# Unless required by applicable law or agreed to in writing, software distributed under the License is distributed on an
# "AS IS" BASIS, WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied. See the License for the
# specific language governing permissions and limitations under the License.

"""Two-qubit gate operations for ProjectQ simulator.

This module provides two-qubit quantum gate operations for the ProjectQ simulator, including CNOT gates,
controlled gates, and other fundamental two-qubit operations using the ProjectQ framework.
"""

from __future__ import annotations

from typing import TYPE_CHECKING

if TYPE_CHECKING:
    from pecos.simulators.projectq.state import ProjectQSim
    from pecos.typing import SimulatorGateParams

from numpy import pi
from projectq import ops

from pecos.simulators.projectq.gates_one_qubit import H


def II(
    state: ProjectQSim,
    qubits: tuple[int, int],
    **_params: SimulatorGateParams,
) -> None:
    """Apply two-qubit identity gate (no operation).

    Args:
        state: ProjectQ simulator state.
        qubits: Tuple of two target qubit indices.
    """


def G2(
    state: ProjectQSim,
    qubits: tuple[int, int],
    **_params: SimulatorGateParams,
) -> None:
    """Applies a CZ.H(1).H(2).CZ."""
    CZ(state, qubits)
    H(state, qubits[0])
    H(state, qubits[1])
    CZ(state, qubits)


def CNOT(
    state: ProjectQSim,
    qubits: tuple[int, int],
    **_params: SimulatorGateParams,
) -> None:
    """Apply controlled-NOT (CNOT) gate.

    Args:
        state: ProjectQ simulator state.
        qubits: Tuple of (control, target) qubit indices.
    """
    q1 = state.qids[qubits[0]]
    q2 = state.qids[qubits[1]]

    ops.CNOT | (q1, q2)


def CZ(
    state: ProjectQSim,
    qubits: tuple[int, int],
    **_params: SimulatorGateParams,
) -> None:
    """Apply controlled-Z (CZ) gate.

    Args:
        state: ProjectQ simulator state.
        qubits: Tuple of (control, target) qubit indices.
    """
    q1 = state.qids[qubits[0]]
    q2 = state.qids[qubits[1]]

    ops.C(ops.Z) | (q1, q2)


def CY(
    state: ProjectQSim,
    qubits: tuple[int, int],
    **_params: SimulatorGateParams,
) -> None:
    """Apply controlled-Y (CY) gate.

    Args:
        state: ProjectQ simulator state.
        qubits: Tuple of (control, target) qubit indices.
    """
    q1 = state.qids[qubits[0]]
    q2 = state.qids[qubits[1]]

    ops.C(ops.Y) | (q1, q2)


def SWAP(
    state: ProjectQSim,
    qubits: tuple[int, int],
    **_params: SimulatorGateParams,
) -> None:
    """Apply SWAP gate to exchange qubit states.

    Args:
        state: ProjectQ simulator state.
        qubits: Tuple of two qubit indices to swap.
    """
    q1 = state.qids[qubits[0]]
    q2 = state.qids[qubits[1]]

    ops.Swap | (q1, q2)


def SXX(
    state: ProjectQSim,
    qubits: tuple[int, int],
    **_params: SimulatorGateParams,
) -> None:
    """Square root of XX rotation to generators.

    state (SparseSim): Instance representing the stabilizer state.
    qubit (int): Integer that indexes the qubit being acted on.

    Returns: None
    """
    q1 = state.qids[qubits[0]]
    q2 = state.qids[qubits[1]]
    ops.Rxx(pi / 2) | (q1, q2)


def SXXdg(
    state: ProjectQSim,
    qubits: tuple[int, int],
    **_params: SimulatorGateParams,
) -> None:
    """Adjoint of square root of XX rotation.

    state: Instance representing the stabilizer state.
    qubit: Integer that indexes the qubit being acted on.

    Returns: None
    """
    q1 = state.qids[qubits[0]]
    q2 = state.qids[qubits[1]]
    ops.Rxx(-pi / 2) | (q1, q2)


def SYY(
    state: ProjectQSim,
    qubits: tuple[int, int],
    **_params: SimulatorGateParams,
) -> None:
    """Square root of YY rotation to generators.

    state: Instance representing the stabilizer state.
    qubit: Integer that indexes the qubit being acted on.

    Returns: None
    """
    q1 = state.qids[qubits[0]]
    q2 = state.qids[qubits[1]]
    ops.Ryy(pi / 2) | (q1, q2)


def SYYdg(
    state: ProjectQSim,
    qubits: tuple[int, int],
    **_params: SimulatorGateParams,
) -> None:
    """Adjoint of square root of YY rotation to generators.

    state: Instance representing the stabilizer state.
    qubit: Integer that indexes the qubit being acted on.

    Returns: None
    """
    q1 = state.qids[qubits[0]]
    q2 = state.qids[qubits[1]]
    ops.Ryy(-pi / 2) | (q1, q2)


def SZZ(
    state: ProjectQSim,
    qubits: tuple[int, int],
    **_params: SimulatorGateParams,
) -> None:
    """Applies a square root of ZZ rotation to generators.

    state: Instance representing the stabilizer state.
    qubit: Integer that indexes the qubit being acted on.

    Returns: None
    """
    q1 = state.qids[qubits[0]]
    q2 = state.qids[qubits[1]]
    ops.Rzz(pi / 2) | (q1, q2)


def SZZdg(
    state: ProjectQSim,
    qubits: tuple[int, int],
    **_params: SimulatorGateParams,
) -> None:
    """Applies an adjoint of square root of ZZ rotation to generators.

    state: Instance representing the stabilizer state.
    qubit: Integer that indexes the qubit being acted on.

    Returns: None
    """
    q1 = state.qids[qubits[0]]
    q2 = state.qids[qubits[1]]
    ops.Rzz(-pi / 2) | (q1, q2)


def RXX(
    state: ProjectQSim,
    qubits: tuple[int, int],
    angle: float | None = None,
    **_params: SimulatorGateParams,
) -> None:
    """Apply RXX rotation gate around XX axis.

    Args:
        state: ProjectQ simulator state.
        qubits: Tuple of two target qubit indices.
        angle: Rotation angle in radians.
    """
    q1 = state.qids[qubits[0]]
    q2 = state.qids[qubits[1]]
    ops.Rxx(angle) | (q1, q2)


def RYY(
    state: ProjectQSim,
    qubits: tuple[int, int],
    angle: float | None = None,
    **_params: SimulatorGateParams,
) -> None:
    """Apply RYY rotation gate around YY axis.

    Args:
        state: ProjectQ simulator state.
        qubits: Tuple of two target qubit indices.
        angle: Rotation angle in radians.
    """
    q1 = state.qids[qubits[0]]
    q2 = state.qids[qubits[1]]
    ops.Ryy(angle) | (q1, q2)


def RZZ(
    state: ProjectQSim,
    qubits: tuple[int, int],
    angle: float | None = None,
    **_params: SimulatorGateParams,
) -> None:
    """Apply RZZ rotation gate around ZZ axis.

    Args:
        state: ProjectQ simulator state.
        qubits: Tuple of two target qubit indices.
        angle: Rotation angle in radians.
    """
    q1 = state.qids[qubits[0]]
    q2 = state.qids[qubits[1]]
    ops.Rzz(angle) | (q1, q2)


def R2XXYYZZ(
    state: ProjectQSim,
    qubits: tuple[int, int],
    angles: tuple[float, float, float] | None = None,
    **_params: SimulatorGateParams,
) -> None:
    """Apply combined RXX, RYY, RZZ rotation gates.

    Sequentially applies RXX, RYY, and RZZ rotations with given angles.

    Args:
        state: ProjectQ simulator state.
        qubits: Tuple of two target qubit indices.
        angles: Tuple of (RXX angle, RYY angle, RZZ angle) in radians.
    """
    q1 = state.qids[qubits[0]]
    q2 = state.qids[qubits[1]]
    ops.Rxx(angles[0]) | (q1, q2)
    ops.Ryy(angles[1]) | (q1, q2)
    ops.Rzz(angles[2]) | (q1, q2)
