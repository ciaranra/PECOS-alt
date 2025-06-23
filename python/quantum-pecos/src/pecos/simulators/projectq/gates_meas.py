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

"""Quantum measurement operations for ProjectQ simulator.

This module provides quantum measurement operations for the ProjectQ simulator, including projective measurements
with proper state collapse and sampling using the ProjectQ quantum computing framework.
"""

from __future__ import annotations

from typing import TYPE_CHECKING

if TYPE_CHECKING:
    from pecos.simulators.projectq.state import ProjectQSim
    from pecos.typing import SimulatorGateParams

from projectq.ops import Measure

from pecos.simulators.projectq.gates_one_qubit import H5, H


def force_output(
    _state: ProjectQSim,
    _qubit: int,
    forced_output: int = -1,
    **_params: SimulatorGateParams,
) -> int:
    """Outputs value.

    Used for error generators to generate outputs when replacing measurements.

    Args:
    ----
        _state: Unused state parameter (kept for interface compatibility)
        _qubit: Unused qubit parameter (kept for interface compatibility)
        forced_output: The value to output
        **_params: Unused additional parameters (kept for interface compatibility)
    """
    return forced_output


def meas_z(
    state: ProjectQSim,
    qubit: int,
    forced_outcome: int = -1,
    **_params: SimulatorGateParams,
) -> int:
    """Measurement in the Z-basis.

    Args:
        state: The ProjectQ state instance
        qubit: The qubit index to measure
        forced_outcome: If 0 or 1, forces the measurement outcome to that value when the measurement would
            otherwise be non-deterministic
        **_params: Unused additional parameters (kept for interface compatibility)
    """
    q = state.qids[qubit]

    state.eng.flush()

    if forced_outcome in {0, 1}:
        # project the qubit to the desired state ("randomly" chooses the value `forced_outcome`)
        state.eng.backend.collapse_wavefunction([q], [forced_outcome])
        # Note: this will raise an error if the probability of collapsing to this state is close to 0.0

        return forced_outcome

    Measure | q
    state.eng.flush()

    return int(q)


def meas_y(
    state: ProjectQSim,
    qubit: int,
    forced_outcome: int = -1,
    **_params: SimulatorGateParams,
) -> int:
    """Measurement in the Y-basis.

    Args:
    ----
        state: The ProjectQ state instance
        qubit: The qubit index to measure
        forced_outcome: If 0 or 1, forces the measurement outcome to that value when the measurement would
            otherwise be non-deterministic
        **_params: Unused additional parameters (kept for interface compatibility)
    """
    H5(state, qubit)
    meas_outcome = meas_z(state, qubit, forced_outcome)
    H5(state, qubit)

    return meas_outcome


def meas_x(
    state: ProjectQSim,
    qubit: int,
    forced_outcome: int = -1,
    **_params: SimulatorGateParams,
) -> int:
    """Measurement in the X-basis.

    Args:
    ----
        state: The ProjectQ state instance
        qubit: The qubit index to measure
        forced_outcome: If 0 or 1, forces the measurement outcome to that value when the measurement would
            otherwise be non-deterministic
        **_params: Unused additional parameters (kept for interface compatibility)
    """
    H(state, qubit)
    meas_outcome = meas_z(state, qubit, forced_outcome)
    H(state, qubit)

    return meas_outcome
