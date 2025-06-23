# Copyright 2019 The PECOS Developers
#
# Licensed under the Apache License, Version 2.0 (the "License"); you may not use this file except in compliance with
# the License.You may obtain a copy of the License at
#
#     https://www.apache.org/licenses/LICENSE-2.0
#
# Unless required by applicable law or agreed to in writing, software distributed under the License is distributed on an
# "AS IS" BASIS, WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied. See the License for the
# specific language governing permissions and limitations under the License.

"""Helper utilities for ProjectQ simulator.

This module provides helper utilities and utility functions for the ProjectQ simulator, including common operations
and support functions used across the ProjectQ-based quantum simulation components.
"""

from __future__ import annotations

from typing import TYPE_CHECKING

if TYPE_CHECKING:
    from projectq.ops._basics import BasicGate

    from pecos.simulators.projectq.state import ProjectQSim
    from pecos.typing import Location, SimulatorGateParams


class MakeFunc:
    """Converts ProjectQ gate to a function."""

    def __init__(
        self,
        gate: BasicGate | type[BasicGate],
        *,
        angle: bool = False,
    ) -> None:
        """Initialize MakeFunc with a gate.

        Args:
            gate: The ProjectQ gate to wrap.
            angle (bool): Whether the gate takes an angle parameter.
        """
        self.gate = gate
        self.angle = angle

    def func(
        self,
        state: ProjectQSim,
        qubits: Location,
        **params: SimulatorGateParams,
    ) -> None:
        """Apply the wrapped ProjectQ gate to the quantum state.

        Args:
            state: The ProjectQ simulator state.
            qubits: Qubit location(s) to apply the gate to.
            **params: Additional gate parameters (e.g., angles).
        """
        if isinstance(qubits, int):
            qs = state.qids[qubits]
        else:
            qs = []
            for q in qubits:
                qs.append(state.qids[q])

        if self.angle:
            self.gate(params["angle"]) | qs
        else:
            self.gate | qs
