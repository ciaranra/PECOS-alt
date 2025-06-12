# Copyright 2024 The PECOS Developers
#
# Licensed under the Apache License, Version 2.0 (the "License"); you may not use this file except in compliance with
# the License.You may obtain a copy of the License at
#
#     https://www.apache.org/licenses/LICENSE-2.0
#
# Unless required by applicable law or agreed to in writing, software distributed under the License is distributed on an
# "AS IS" BASIS, WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied. See the License for the
# specific language governing permissions and limitations under the License.

"""Two-qubit gate operations for basic state vector simulator.

This module provides two-qubit quantum gate operations for the basic state vector simulator, including CNOT gates,
controlled gates, and other fundamental two-qubit quantum operations for entangling operations.
"""

from __future__ import annotations

import cmath
import math
from typing import TYPE_CHECKING

import numpy as np

from pecos.simulators.basic_sv.gates_one_qubit import H

if TYPE_CHECKING:
    from pecos.simulators.basic_sv.state import BasicSV
    from pecos.typing import SimulatorGateParams


def _apply_two_qubit_matrix(
    state: BasicSV,
    qubits: tuple[int, int],
    matrix: np.array,
) -> None:
    """Apply the matrix to the state.

    Args:
        state: An instance of BasicSV
        qubits: A tuple of two qubit indices where the gate is applied
        matrix: The matrix to be applied
    """
    if qubits[0] >= state.num_qubits or qubits[0] < 0:
        msg = f"Qubit {qubits[0]} out of range."
        raise ValueError(msg)
    if qubits[1] >= state.num_qubits or qubits[1] < 0:
        msg = f"Qubit {qubits[1]} out of range."
        raise ValueError(msg)

    # Reshape the matrix into an ndarray of shape (2,2,2,2)
    # Use positional argument for backward compatibility with NumPy < 2.0
    reshaped_matrix = np.reshape(matrix, (2, 2, 2, 2))

    # Use np.einsum to apply the gate to `qubit`.
    # To do so, we need to assign subscript labels to each array axis.
    if qubits[0] < qubits[1]:
        gate_subscripts = "LRlr"
        qubit_labels_before = ("l", "r")
        qubit_labels_after = ("L", "R")
    else:  # Implicit swap of gate subscripts
        gate_subscripts = "RLrl"
        qubit_labels_before = ("r", "l")
        qubit_labels_after = ("R", "L")

    subscripts = "".join(
        [
            state.subscript_string(qubits, qubit_labels_before),  # Current vector
            ",",
            gate_subscripts,  # Subscripts for the gate
            "->",
            state.subscript_string(qubits, qubit_labels_after),  # Resulting vector
        ],
    )
    # Update the state by applying the matrix
    state.internal_vector = np.einsum(
        subscripts,
        state.internal_vector,
        reshaped_matrix,
    )


def CX(state: BasicSV, qubits: tuple[int, int], **_params: SimulatorGateParams) -> None:
    """Apply controlled X gate.

    Args:
        state: An instance of BasicSV
        qubits: A tuple with the index of the qubits where the gate is applied
            The one at `qubits[0]` is the control qubit.
    """
    matrix = np.array(
        [
            [1, 0, 0, 0],
            [0, 1, 0, 0],
            [0, 0, 0, 1],
            [0, 0, 1, 0],
        ],
    )
    _apply_two_qubit_matrix(state, qubits, matrix)


def CY(state: BasicSV, qubits: tuple[int, int], **_params: SimulatorGateParams) -> None:
    """Apply controlled Y gate.

    Args:
        state: An instance of BasicSV
        qubits: A tuple with the index of the qubits where the gate is applied
            The one at `qubits[0]` is the control qubit.
    """
    matrix = np.array(
        [
            [1, 0, 0, 0],
            [0, 1, 0, 0],
            [0, 0, 0, -1j],
            [0, 0, 1j, 0],
        ],
    )
    _apply_two_qubit_matrix(state, qubits, matrix)


def CZ(state: BasicSV, qubits: tuple[int, int], **_params: SimulatorGateParams) -> None:
    """Apply controlled Z gate.

    Args:
        state: An instance of BasicSV
        qubits: A tuple with the index of the qubits where the gate is applied
            The one at `qubits[0]` is the control qubit.
    """
    matrix = np.array(
        [
            [1, 0, 0, 0],
            [0, 1, 0, 0],
            [0, 0, 1, 0],
            [0, 0, 0, -1],
        ],
    )
    _apply_two_qubit_matrix(state, qubits, matrix)


def RXX(
    state: BasicSV,
    qubits: tuple[int, int],
    angles: tuple[float],
    **_params: SimulatorGateParams,
) -> None:
    """Apply a rotation about XX.

    Args:
        state: An instance of BasicSV
        qubits: A tuple with the index of the qubits where the gate is applied
        angles: A tuple containing a single angle in radians
    """
    if len(angles) != 1:
        msg = "Gate must be given 1 angle parameter."
        raise ValueError(msg)
    theta = angles[0]

    matrix = np.array(
        [
            [math.cos(theta / 2), 0, 0, -1j * math.sin(theta / 2)],
            [0, math.cos(theta / 2), -1j * math.sin(theta / 2), 0],
            [0, -1j * math.sin(theta / 2), math.cos(theta / 2), 0],
            [-1j * math.sin(theta / 2), 0, 0, math.cos(theta / 2)],
        ],
    )
    _apply_two_qubit_matrix(state, qubits, matrix)


def RYY(
    state: BasicSV,
    qubits: tuple[int, int],
    angles: tuple[float],
    **_params: SimulatorGateParams,
) -> None:
    """Apply a rotation about YY.

    Args:
        state: An instance of BasicSV
        qubits: A tuple with the index of the qubits where the gate is applied
        angles: A tuple containing a single angle in radians
    """
    if len(angles) != 1:
        msg = "Gate must be given 1 angle parameter."
        raise ValueError(msg)
    theta = angles[0]

    matrix = np.array(
        [
            [math.cos(theta / 2), 0, 0, 1j * math.sin(theta / 2)],
            [0, math.cos(theta / 2), -1j * math.sin(theta / 2), 0],
            [0, -1j * math.sin(theta / 2), math.cos(theta / 2), 0],
            [1j * math.sin(theta / 2), 0, 0, math.cos(theta / 2)],
        ],
    )
    _apply_two_qubit_matrix(state, qubits, matrix)


def RZZ(
    state: BasicSV,
    qubits: tuple[int, int],
    angles: tuple[float],
    **_params: SimulatorGateParams,
) -> None:
    """Apply a rotation about ZZ.

    Args:
        state: An instance of BasicSV
        qubits: A tuple with the index of the qubits where the gate is applied
        angles: A tuple containing a single angle in radians
    """
    if len(angles) != 1:
        msg = "Gate must be given 1 angle parameter."
        raise ValueError(msg)
    theta = angles[0]

    matrix = np.array(
        [
            [cmath.exp(-1j * theta / 2), 0, 0, 0],
            [0, cmath.exp(1j * theta / 2), 0, 0],
            [0, 0, cmath.exp(1j * theta / 2), 0],
            [0, 0, 0, cmath.exp(-1j * theta / 2)],
        ],
    )
    _apply_two_qubit_matrix(state, qubits, matrix)


def R2XXYYZZ(
    state: BasicSV,
    qubits: tuple[int, int],
    angles: tuple[float, float, float],
    **_params: SimulatorGateParams,
) -> None:
    """Apply RXX*RYY*RZZ.

    Args:
        state: An instance of BasicSV
        qubits: A tuple with the index of the qubits where the gate is applied
        angles: A tuple containing three angles in radians, for XX, YY and ZZ, in that order
    """
    if len(angles) != 3:
        msg = "Gate must be given 3 angle parameters."
        raise ValueError(msg)

    RXX(state, qubits, (angles[0],))
    RYY(state, qubits, (angles[1],))
    RZZ(state, qubits, (angles[2],))


def SXX(
    state: BasicSV,
    qubits: tuple[int, int],
    **_params: SimulatorGateParams,
) -> None:
    """Apply a square root of XX gate.

    Args:
        state: An instance of BasicSV
        qubits: A tuple with the index of the qubits where the gate is applied
    """
    RXX(state, qubits, angles=(math.pi / 2,))


def SXXdg(
    state: BasicSV,
    qubits: tuple[int, int],
    **_params: SimulatorGateParams,
) -> None:
    """Apply adjoint of a square root of XX gate.

    Args:
        state: An instance of BasicSV
        qubits: A tuple with the index of the qubits where the gate is applied
    """
    RXX(state, qubits, angles=(-math.pi / 2,))


def SYY(
    state: BasicSV,
    qubits: tuple[int, int],
    **_params: SimulatorGateParams,
) -> None:
    """Apply a square root of YY gate.

    Args:
        state: An instance of BasicSV
        qubits: A tuple with the index of the qubits where the gate is applied
    """
    RYY(state, qubits, angles=(math.pi / 2,))


def SYYdg(
    state: BasicSV,
    qubits: tuple[int, int],
    **_params: SimulatorGateParams,
) -> None:
    """Apply adjoint of a square root of YY gate.

    Args:
        state: An instance of BasicSV
        qubits: A tuple with the index of the qubits where the gate is applied
    """
    RYY(state, qubits, angles=(-math.pi / 2,))


def SZZ(
    state: BasicSV,
    qubits: tuple[int, int],
    **_params: SimulatorGateParams,
) -> None:
    """Apply a square root of ZZ gate.

    Args:
        state: An instance of BasicSV
        qubits: A tuple with the index of the qubits where the gate is applied
    """
    RZZ(state, qubits, angles=(math.pi / 2,))


def SZZdg(
    state: BasicSV,
    qubits: tuple[int, int],
    **_params: SimulatorGateParams,
) -> None:
    """Apply adjoint of a square root of ZZ gate.

    Args:
        state: An instance of BasicSV
        qubits: A tuple with the index of the qubits where the gate is applied
    """
    RZZ(state, qubits, angles=(-math.pi / 2,))


def SWAP(
    state: BasicSV,
    qubits: tuple[int, int],
    **_params: SimulatorGateParams,
) -> None:
    """Apply a SWAP gate.

    Args:
        state: An instance of BasicSV
        qubits: A tuple with the index of the qubits where the gate is applied
    """
    matrix = np.array(
        [
            [1, 0, 0, 0],
            [0, 0, 1, 0],
            [0, 1, 0, 0],
            [0, 0, 0, 1],
        ],
    )
    _apply_two_qubit_matrix(state, qubits, matrix)


def G(state: BasicSV, qubits: tuple[int, int], **_params: SimulatorGateParams) -> None:
    """'G': (('I', 'H'), 'CNOT', ('H', 'H'), 'CNOT', ('I', 'H'))."""
    H(state, qubits[1])
    CX(state, qubits)
    H(state, qubits[0])
    H(state, qubits[1])
    CX(state, qubits)
    H(state, qubits[1])
