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

"""Single-qubit gate operations for basic state vector simulator.

This module provides single-qubit quantum gate operations for the basic state vector simulator, including Pauli
gates, rotation gates, Hadamard gates, and other fundamental single-qubit quantum operations.
"""

from __future__ import annotations

import cmath
import math
from typing import TYPE_CHECKING

import numpy as np

if TYPE_CHECKING:
    from pecos.simulators.basic_sv.state import BasicSV
    from pecos.typing import SimulatorGateParams


def _apply_one_qubit_matrix(state: BasicSV, qubit: int, matrix: np.ndarray) -> None:
    """Apply the matrix to the state.

    Args:
        state: An instance of BasicSV
        qubit: The index of the qubit where the gate is applied
        matrix: The matrix to be applied
    """
    if qubit >= state.num_qubits or qubit < 0:
        msg = f"Qubit {qubit} out of range."
        raise ValueError(msg)

    # Use np.einsum to apply the gate to `qubit`.
    # To do so, we need to assign subscript labels to each array axis.
    subscripts = "".join(
        [
            state.subscript_string((qubit,), ("q",)),  # Current vector
            ",",
            "Qq",  # Subscripts for the gate, acting on `qubit` q
            "->",
            state.subscript_string(
                (qubit,),
                ("Q",),
            ),  # Resulting vector, with updated Q
        ],
    )
    # Update the state by applying the matrix
    state.internal_vector = np.einsum(subscripts, state.internal_vector, matrix)


def identity(state: BasicSV, qubit: int, **_params: SimulatorGateParams) -> None:
    """Identity gate.

    Args:
        state: An instance of BasicSV
        qubit: The index of the qubit where the gate is applied
    """


def X(state: BasicSV, qubit: int, **_params: SimulatorGateParams) -> None:
    """Pauli X gate.

    Args:
        state: An instance of BasicSV
        qubit: The index of the qubit where the gate is applied
    """
    matrix = np.array(
        [
            [0, 1],
            [1, 0],
        ],
    )
    _apply_one_qubit_matrix(state, qubit, matrix)


def Y(state: BasicSV, qubit: int, **_params: SimulatorGateParams) -> None:
    """Pauli Y gate.

    Args:
        state: An instance of BasicSV
        qubit: The index of the qubit where the gate is applied
    """
    matrix = np.array(
        [
            [0, -1j],
            [1j, 0],
        ],
    )
    _apply_one_qubit_matrix(state, qubit, matrix)


def Z(state: BasicSV, qubit: int, **_params: SimulatorGateParams) -> None:
    """Pauli Z gate.

    Args:
        state: An instance of BasicSV
        qubit: The index of the qubit where the gate is applied
    """
    matrix = np.array(
        [
            [1, 0],
            [0, -1],
        ],
    )
    _apply_one_qubit_matrix(state, qubit, matrix)


def RX(
    state: BasicSV,
    qubit: int,
    angles: tuple[float],
    **_params: SimulatorGateParams,
) -> None:
    """Apply an RX gate.

    Args:
        state: An instance of BasicSV
        qubit: The index of the qubit where the gate is applied
        angles: A tuple containing a single angle in radians
    """
    if len(angles) != 1:
        msg = "Gate must be given 1 angle parameter."
        raise ValueError(msg)
    theta = angles[0]

    matrix = np.array(
        [
            [math.cos(theta / 2), -1j * math.sin(theta / 2)],
            [-1j * math.sin(theta / 2), math.cos(theta / 2)],
        ],
    )
    _apply_one_qubit_matrix(state, qubit, matrix)


def RY(
    state: BasicSV,
    qubit: int,
    angles: tuple[float],
    **_params: SimulatorGateParams,
) -> None:
    """Apply an RY gate.

    Args:
        state: An instance of BasicSV
        qubit: The index of the qubit where the gate is applied
        angles: A tuple containing a single angle in radians
    """
    if len(angles) != 1:
        msg = "Gate must be given 1 angle parameter."
        raise ValueError(msg)
    theta = angles[0]

    matrix = np.array(
        [
            [math.cos(theta / 2), -math.sin(theta / 2)],
            [math.sin(theta / 2), math.cos(theta / 2)],
        ],
    )
    _apply_one_qubit_matrix(state, qubit, matrix)


def RZ(
    state: BasicSV,
    qubit: int,
    angles: tuple[float],
    **_params: SimulatorGateParams,
) -> None:
    """Apply an RZ gate.

    Args:
        state: An instance of BasicSV
        qubit: The index of the qubit where the gate is applied
        angles: A tuple containing a single angle in radians
    """
    if len(angles) != 1:
        msg = "Gate must be given 1 angle parameter."
        raise ValueError(msg)
    theta = angles[0]

    matrix = np.array(
        [
            [cmath.exp(-1j * theta / 2), 0],
            [0, cmath.exp(1j * theta / 2)],
        ],
    )
    _apply_one_qubit_matrix(state, qubit, matrix)


def R1XY(
    state: BasicSV,
    qubit: int,
    angles: tuple[float, float],
    **_params: SimulatorGateParams,
) -> None:
    """Apply an R1XY gate.

    Args:
        state: An instance of BasicSV
        qubit: The index of the qubit where the gate is applied
        angles: A tuple containing two angles in radians
    """
    if len(angles) != 2:
        msg = "Gate must be given 2 angle parameters."
        raise ValueError(msg)
    theta = angles[0]
    phi = angles[1]

    # Gate is equal to RZ(phi-pi/2)*RY(theta)*RZ(-phi+pi/2)
    RZ(state, qubit, angles=(-phi + math.pi / 2,))
    RY(state, qubit, angles=(theta,))
    RZ(state, qubit, angles=(phi - math.pi / 2,))


def SX(state: BasicSV, qubit: int, **_params: SimulatorGateParams) -> None:
    """Apply a square-root of X.

    Args:
        state: An instance of BasicSV
        qubit: The index of the qubit where the gate is applied
    """
    RX(state, qubit, angles=(math.pi / 2,))


def SXdg(state: BasicSV, qubit: int, **_params: SimulatorGateParams) -> None:
    """Apply adjoint of the square-root of X.

    Args:
        state: An instance of BasicSV
        qubit: The index of the qubit where the gate is applied
    """
    RX(state, qubit, angles=(-math.pi / 2,))


def SY(state: BasicSV, qubit: int, **_params: SimulatorGateParams) -> None:
    """Apply a square-root of Y.

    Args:
        state: An instance of BasicSV
        qubit: The index of the qubit where the gate is applied
    """
    RY(state, qubit, angles=(math.pi / 2,))


def SYdg(state: BasicSV, qubit: int, **_params: SimulatorGateParams) -> None:
    """Apply adjoint of the square-root of Y.

    Args:
        state: An instance of BasicSV
        qubit: The index of the qubit where the gate is applied
    """
    RY(state, qubit, angles=(-math.pi / 2,))


def SZ(state: BasicSV, qubit: int, **_params: SimulatorGateParams) -> None:
    """Apply a square-root of Z.

    Args:
        state: An instance of BasicSV
        qubit: The index of the qubit where the gate is applied
    """
    RZ(state, qubit, angles=(math.pi / 2,))


def SZdg(state: BasicSV, qubit: int, **_params: SimulatorGateParams) -> None:
    """Apply adjoint of the square-root of Z.

    Args:
        state: An instance of BasicSV
        qubit: The index of the qubit where the gate is applied
    """
    RZ(state, qubit, angles=(-math.pi / 2,))


def H(state: BasicSV, qubit: int, **_params: SimulatorGateParams) -> None:
    """Apply Hadamard gate.

    Args:
        state: An instance of BasicSV
        qubit: The index of the qubit where the gate is applied
    """
    matrix = (
        1
        / np.sqrt(2)
        * np.array(
            [
                [1, 1],
                [1, -1],
            ],
        )
    )
    _apply_one_qubit_matrix(state, qubit, matrix)


def F(state: BasicSV, qubit: int, **_params: SimulatorGateParams) -> None:
    """Apply face rotation of an octahedron #1 (X->Y->Z->X).

    Args:
        state: An instance of BasicSV
        qubit: The index of the qubit where the gate is applied
    """
    RX(state, qubit, angles=(math.pi / 2,))
    RZ(state, qubit, angles=(math.pi / 2,))


def Fdg(state: BasicSV, qubit: int, **_params: SimulatorGateParams) -> None:
    """Apply adjoint of face rotation of an octahedron #1 (X<-Y<-Z<-X).

    Args:
        state: An instance of BasicSV
        qubit: The index of the qubit where the gate is applied
    """
    RZ(state, qubit, angles=(-math.pi / 2,))
    RX(state, qubit, angles=(-math.pi / 2,))


def T(state: BasicSV, qubit: int, **_params: SimulatorGateParams) -> None:
    """Apply a T gate.

    Args:
        state: An instance of BasicSV
        qubit: The index of the qubit where the gate is applied
    """
    RZ(state, qubit, angles=(math.pi / 4,))


def Tdg(state: BasicSV, qubit: int, **_params: SimulatorGateParams) -> None:
    """Apply adjoint of a T gate.

    Args:
        state: An instance of BasicSV
        qubit: The index of the qubit where the gate is applied
    """
    RZ(state, qubit, angles=(-math.pi / 4,))


def H2(state: BasicSV, qubit: int, **_params: SimulatorGateParams) -> None:
    """'H2': ('S', 'S', 'H', 'S', 'S')."""
    Z(state, qubit)
    H(state, qubit)
    Z(state, qubit)


def H3(state: BasicSV, qubit: int, **_params: SimulatorGateParams) -> None:
    """'H3': ('H', 'S', 'S', 'H', 'S',)."""
    X(state, qubit)
    SZ(state, qubit)


def H4(state: BasicSV, qubit: int, **_params: SimulatorGateParams) -> None:
    """'H4': ('H', 'S', 'S', 'H', 'S', 'S', 'S',)."""
    X(state, qubit)
    SZdg(state, qubit)


def H5(state: BasicSV, qubit: int, **_params: SimulatorGateParams) -> None:
    """'H5': ('S', 'S', 'S', 'H', 'S')."""
    SZdg(state, qubit)
    H(state, qubit)
    SZ(state, qubit)


def H6(state: BasicSV, qubit: int, **_params: SimulatorGateParams) -> None:
    """'H6': ('S', 'H', 'S', 'S', 'S',)."""
    SZ(state, qubit)
    H(state, qubit)
    SZdg(state, qubit)


def F2(state: BasicSV, qubit: int, **_params: SimulatorGateParams) -> None:
    """'F2': ('S', 'S', 'H', 'S')."""
    Z(state, qubit)
    H(state, qubit)
    SZ(state, qubit)


def F2d(state: BasicSV, qubit: int, **_params: SimulatorGateParams) -> None:
    """'F2d': ('S', 'S', 'S', 'H', 'S', 'S')."""
    SZdg(state, qubit)
    H(state, qubit)
    Z(state, qubit)


def F3(state: BasicSV, qubit: int, **_params: SimulatorGateParams) -> None:
    """'F3': ('S', 'H', 'S', 'S')."""
    SZ(state, qubit)
    H(state, qubit)
    Z(state, qubit)


def F3d(state: BasicSV, qubit: int, **_params: SimulatorGateParams) -> None:
    """'F3d': ('S', 'S', 'H', 'S', 'S', 'S')."""
    Z(state, qubit)
    H(state, qubit)
    SZdg(state, qubit)


def F4(state: BasicSV, qubit: int, **_params: SimulatorGateParams) -> None:
    """'F4': ('H', 'S', 'S', 'S')."""
    H(state, qubit)
    SZdg(state, qubit)


def F4d(state: BasicSV, qubit: int, **_params: SimulatorGateParams) -> None:
    """'F4d': ('S', 'H')."""
    SZ(state, qubit)
    H(state, qubit)
