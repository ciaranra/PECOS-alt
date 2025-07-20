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

"""Qubit initialization operations for cuStateVec simulator.

This module provides GPU-accelerated quantum state initialization operations for the NVIDIA cuStateVec simulator,
including functions to initialize qubits to computational basis states using CUDA acceleration.
"""

from __future__ import annotations

from typing import TYPE_CHECKING

if TYPE_CHECKING:
    from pecos.simulators.custatevec.state import CuStateVec
    from pecos.typing import SimulatorGateParams

from pecos.simulators.custatevec.gates_meas import meas_z
from pecos.simulators.custatevec.gates_one_qubit import X


def init_zero(state: CuStateVec, qubit: int, **_params: SimulatorGateParams) -> None:
    """Initialise or reset the qubit to state |0>.

    Args:
        state: An instance of CuStateVec
        qubit: The index of the qubit to be initialised
    """
    result = meas_z(state, qubit)

    if result:
        X(state, qubit)


def init_one(state: CuStateVec, qubit: int, **_params: SimulatorGateParams) -> None:
    """Initialise or reset the qubit to state |1>.

    Args:
        state: An instance of CuStateVec
        qubit: The index of the qubit to be initialised
    """
    result = meas_z(state, qubit)

    if not result:
        X(state, qubit)
