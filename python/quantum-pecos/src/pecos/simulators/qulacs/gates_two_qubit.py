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

"""Two-qubit gate operations for Qulacs simulator.

This module provides two-qubit quantum gate operations for the Qulacs simulator, including CNOT gates,
controlled gates, and other fundamental two-qubit operations using Qulacs framework.
"""

from __future__ import annotations

from typing import TYPE_CHECKING

import numpy as np
import qulacs.gate as qgate

from pecos.simulators.qulacs.gates_one_qubit import SZ, H, SZdg

if TYPE_CHECKING:
    from pecos.simulators.qulacs import Qulacs
    from pecos.typing import SimulatorGateParams


def CX(state: Qulacs, qubits: tuple[int, int], **_params: SimulatorGateParams) -> None:
    """Apply controlled X gate.

    Args:
        state: An instance of Qulacs
        qubits: A tuple with the index of the qubits where the gate is applied
            The one at `qubits[0]` is the control qubit.
    """
    # Qulacs uses qubit index 0 as the least significant bit
    control = state.num_qubits - qubits[0] - 1
    target = state.num_qubits - qubits[1] - 1

    qgate.CNOT(control, target).update_quantum_state(state.qulacs_state)


def CY(state: Qulacs, qubits: tuple[int, int], **_params: SimulatorGateParams) -> None:
    """Apply controlled Y gate.

    Args:
        state: An instance of Qulacs
        qubits: A tuple with the index of the qubits where the gate is applied
            The one at `qubits[0]` is the control qubit.
    """
    SZdg(state, qubits[1])
    CX(state, qubits)
    SZ(state, qubits[1])


def CZ(state: Qulacs, qubits: tuple[int, int], **_params: SimulatorGateParams) -> None:
    """Apply controlled Z gate.

    Args:
        state: An instance of Qulacs
        qubits: A tuple with the index of the qubits where the gate is applied
            The one at `qubits[0]` is the control qubit.
    """
    # Qulacs uses qubit index 0 as the least significant bit
    control = state.num_qubits - qubits[0] - 1
    target = state.num_qubits - qubits[1] - 1

    qgate.CZ(control, target).update_quantum_state(state.qulacs_state)


def RXX(
    state: Qulacs,
    qubits: tuple[int, int],
    angles: tuple[float],
    **_params: SimulatorGateParams,
) -> None:
    """Apply a rotation about XX.

    Args:
        state: An instance of Qulacs
        qubits: A tuple with the index of the qubits where the gate is applied
        angles: A tuple containing a single angle in radians
    """
    if len(angles) != 1:
        msg = "Gate must be given 1 angle parameter."
        raise ValueError(msg)
    theta = angles[0]

    # Qulacs uses qubit index 0 as the least significant bit
    idxs = [state.num_qubits - q - 1 for q in qubits]

    qgate.PauliRotation(
        index_list=idxs,
        pauli_ids=[1, 1],  # Paulis: [I, X, Y, Z]
        angle=-theta,  # Negative angle in the exponent
    ).update_quantum_state(state.qulacs_state)


def RYY(
    state: Qulacs,
    qubits: tuple[int, int],
    angles: tuple[float],
    **_params: SimulatorGateParams,
) -> None:
    """Apply a rotation about YY.

    Args:
        state: An instance of Qulacs
        qubits: A tuple with the index of the qubits where the gate is applied
        angles: A tuple containing a single angle in radians
    """
    if len(angles) != 1:
        msg = "Gate must be given 1 angle parameter."
        raise ValueError(msg)
    theta = angles[0]

    # Qulacs uses qubit index 0 as the least significant bit
    idxs = [state.num_qubits - q - 1 for q in qubits]

    qgate.PauliRotation(
        index_list=idxs,
        pauli_ids=[2, 2],  # Paulis: [I, X, Y, Z]
        angle=-theta,  # Negative angle in the exponent
    ).update_quantum_state(state.qulacs_state)


def RZZ(
    state: Qulacs,
    qubits: tuple[int, int],
    angles: tuple[float],
    **_params: SimulatorGateParams,
) -> None:
    """Apply a rotation about ZZ.

    Args:
        state: An instance of Qulacs
        qubits: A tuple with the index of the qubits where the gate is applied
        angles: A tuple containing a single angle in radians
    """
    if len(angles) != 1:
        msg = "Gate must be given 1 angle parameter."
        raise ValueError(msg)
    theta = angles[0]

    # Qulacs uses qubit index 0 as the least significant bit
    idxs = [state.num_qubits - q - 1 for q in qubits]

    qgate.PauliRotation(
        index_list=idxs,
        pauli_ids=[3, 3],  # Paulis: [I, X, Y, Z]
        angle=-theta,  # Negative angle in the exponent
    ).update_quantum_state(state.qulacs_state)


def R2XXYYZZ(
    state: Qulacs,
    qubits: tuple[int, int],
    angles: tuple[float, float, float],
    **_params: SimulatorGateParams,
) -> None:
    """Apply RXX*RYY*RZZ.

    Args:
        state: An instance of Qulacs
        qubits: A tuple with the index of the qubits where the gate is applied
        angles: A tuple containing three angles in radians, for XX, YY and ZZ, in that order
    """
    if len(angles) != 3:
        msg = "Gate must be given 3 angle parameters."
        raise ValueError(msg)

    RXX(state, qubits, (angles[0],))
    RYY(state, qubits, (angles[1],))
    RZZ(state, qubits, (angles[2],))


def SXX(state: Qulacs, qubits: tuple[int, int], **_params: SimulatorGateParams) -> None:
    """Apply a square root of XX gate.

    Args:
        state: An instance of Qulacs
        qubits: A tuple with the index of the qubits where the gate is applied
    """
    RXX(state, qubits, angles=(np.pi / 2,))


def SXXdg(
    state: Qulacs,
    qubits: tuple[int, int],
    **_params: SimulatorGateParams,
) -> None:
    """Apply adjoint of a square root of XX gate.

    Args:
        state: An instance of Qulacs
        qubits: A tuple with the index of the qubits where the gate is applied
    """
    RXX(state, qubits, angles=(-np.pi / 2,))


def SYY(state: Qulacs, qubits: tuple[int, int], **_params: SimulatorGateParams) -> None:
    """Apply a square root of YY gate.

    Args:
        state: An instance of Qulacs
        qubits: A tuple with the index of the qubits where the gate is applied
    """
    RYY(state, qubits, angles=(np.pi / 2,))


def SYYdg(
    state: Qulacs,
    qubits: tuple[int, int],
    **_params: SimulatorGateParams,
) -> None:
    """Apply adjoint of a square root of YY gate.

    Args:
        state: An instance of Qulacs
        qubits: A tuple with the index of the qubits where the gate is applied
    """
    RYY(state, qubits, angles=(-np.pi / 2,))


def SZZ(state: Qulacs, qubits: tuple[int, int], **_params: SimulatorGateParams) -> None:
    """Apply a square root of ZZ gate.

    Args:
        state: An instance of Qulacs
        qubits: A tuple with the index of the qubits where the gate is applied
    """
    RZZ(state, qubits, angles=(np.pi / 2,))


def SZZdg(
    state: Qulacs,
    qubits: tuple[int, int],
    **_params: SimulatorGateParams,
) -> None:
    """Apply adjoint of a square root of ZZ gate.

    Args:
        state: An instance of Qulacs
        qubits: A tuple with the index of the qubits where the gate is applied
    """
    RZZ(state, qubits, angles=(-np.pi / 2,))


def SWAP(
    state: Qulacs,
    qubits: tuple[int, int],
    **_params: SimulatorGateParams,
) -> None:
    """Apply a SWAP gate.

    Args:
        state: An instance of Qulacs
        qubits: A tuple with the index of the qubits where the gate is applied
    """
    # Qulacs uses qubit index 0 as the least significant bit
    idxs = [state.num_qubits - q - 1 for q in qubits]

    qgate.SWAP(idxs[0], idxs[1]).update_quantum_state(state.qulacs_state)


def G(state: Qulacs, qubits: tuple[int, int], **_params: SimulatorGateParams) -> None:
    """'G': (('I', 'H'), 'CNOT', ('H', 'H'), 'CNOT', ('I', 'H'))."""
    H(state, qubits[1])
    CX(state, qubits)
    H(state, qubits[0])
    H(state, qubits[1])
    CX(state, qubits)
    H(state, qubits[1])
