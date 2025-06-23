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

"""Qubit initialization operations for ProjectQ simulator.

This module provides quantum state initialization operations for the ProjectQ simulator, including functions to
initialize qubits to computational basis states using the ProjectQ quantum computing framework.
"""

from __future__ import annotations

from typing import TYPE_CHECKING

if TYPE_CHECKING:
    from pecos.simulators.projectq.state import ProjectQSim
    from pecos.typing import SimulatorGateParams

from pecos.simulators.projectq.gates_meas import meas_z
from pecos.simulators.projectq.gates_one_qubit import H2, H5, H6, H, X


def init_zero(state: ProjectQSim, qubit: int, **_params: SimulatorGateParams) -> None:
    """Initialize qubit to zero state.

    Args:
        state: The ProjectQ state instance.
        qubit: The qubit index to initialize.
        **_params: Unused additional parameters (kept for interface compatibility).
    """
    result = meas_z(state, qubit)

    if result:
        X(state, qubit)


def init_one(state: ProjectQSim, qubit: int, **_params: SimulatorGateParams) -> None:
    """Initialize qubit in state |1>.

    :param state:
    :param qubit:
    :return:
    """
    init_zero(state, qubit)
    X(state, qubit)


def init_plus(state: ProjectQSim, qubit: int, **_params: SimulatorGateParams) -> None:
    """Initialize qubit in state |+>.

    :param gens:
    :param qubit:
    :return:
    """
    init_zero(state, qubit)
    H(state, qubit)


def init_minus(state: ProjectQSim, qubit: int, **_params: SimulatorGateParams) -> None:
    """Initialize qubit in state |->.

    :param gens:
    :param qubit:
    :return:
    """
    init_zero(state, qubit)
    H2(state, qubit)


def init_plusi(state: ProjectQSim, qubit: int, **_params: SimulatorGateParams) -> None:
    """Initialize qubit in state |+i>.

    :param gens:
    :param qubit:
    :return:
    """
    init_zero(state, qubit)
    H5(state, qubit)


def init_minusi(state: ProjectQSim, qubit: int, **_params: SimulatorGateParams) -> None:
    """Initialize qubit in state |-i>.

    Args:
    ----
        state: The ProjectQ state instance
        qubit: The qubit index to initialize
    """
    init_zero(state, qubit)
    H6(state, qubit)
