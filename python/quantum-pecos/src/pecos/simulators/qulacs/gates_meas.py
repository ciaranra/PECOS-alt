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

"""Quantum measurement operations for Qulacs simulator.

This module provides quantum measurement operations for the Qulacs simulator, including projective measurements
with proper state collapse using Qulacs quantum simulation framework.
"""

from __future__ import annotations

from typing import TYPE_CHECKING

from qulacs.gate import Measurement

if TYPE_CHECKING:
    from pecos.simulators.qulacs.state import Qulacs
    from pecos.typing import SimulatorGateParams


def meas_z(state: Qulacs, qubit: int, **_params: SimulatorGateParams) -> int:
    """Measure in the Z-basis, collapse and normalise.

    Notes:
        The number of qubits in the state remains the same.

    Args:
        state: An instance of Qulacs
        qubit: The index of the qubit to be measured

    Returns:
        The outcome of the measurement, either 0 or 1.
    """
    # Qulacs uses qubit index 0 as the least significant bit
    idx = state.num_qubits - qubit - 1

    m = Measurement(index=idx, register=idx)
    m.update_quantum_state(state.qulacs_state)
    return state.qulacs_state.get_classical_value(idx)
