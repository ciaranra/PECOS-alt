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

"""Quantum measurement operations for MPS PyTket simulator.

This module provides quantum measurement operations for the Matrix Product State PyTket simulator, including
projective measurements with MPS state collapse and sampling from low-entanglement quantum states.
"""

from __future__ import annotations

from typing import TYPE_CHECKING

if TYPE_CHECKING:
    from pecos.simulators.mps_pytket.state import MPS
    from pecos.typing import SimulatorGateParams

from pytket import Qubit


def meas_z(state: MPS, qubit: int, **_params: SimulatorGateParams) -> int:
    """Measure in the Z-basis, collapse and normalise.

    Notes:
        The number of qubits in the state remains the same.

    Args:
        state: An instance of MPS
        qubit: The index of the qubit to be measured

    Returns:
        The outcome of the measurement, either 0 or 1.
    """
    if qubit >= state.num_qubits or qubit < 0:
        msg = f"Qubit {qubit} out of range."
        raise ValueError(msg)

    result = state.mps.measure({Qubit(qubit)}, destructive=False)

    return result[Qubit(qubit)]
