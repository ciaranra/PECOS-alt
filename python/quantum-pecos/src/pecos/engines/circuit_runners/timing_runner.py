# Copyright 2018 The PECOS Developers
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

"""Timing circuit runner for performance measurement.

This module provides a circuit runner with timing capabilities to measure
the execution performance of quantum circuits in the PECOS framework.
"""

from __future__ import annotations

from time import perf_counter as default_timer
from typing import TYPE_CHECKING

from pecos.engines.circuit_runners.standard import Standard

if TYPE_CHECKING:
    from collections.abc import Callable
    from typing import Any

    from pecos.circuits import QuantumCircuit
    from pecos.protocols import SimulatorProtocol


class TimingRunner(Standard):
    """This class represents a standard model for running quantum circuits and adding in errors."""

    def __init__(
        self,
        seed: int | bool | None = None,
        timer: Callable[[], float] | None = None,
    ) -> None:
        """Initialize timing runner with optional seed and timer.

        Args:
        ----
            seed: Random seed for reproducibility. Can be bool True for random seed, int for specific seed, or None.
            timer: Timer function to use for performance measurements. Defaults to perf_counter if None.
        """
        super().__init__(seed)

        self.total_time = 0.0
        self.num_gates = 0

        if timer is None:
            self.timer = default_timer
        else:
            self.timer = timer

    def reset_time(self) -> None:
        """Used to clear the time data in `total_time`."""
        self.total_time = 0.0
        self.num_gates = 0

    def run_gates(
        self,
        state: SimulatorProtocol,
        gates: QuantumCircuit,
        removed_locations: set[int] | None = None,
    ) -> dict[str, Any]:
        """Directly apply a collection of quantum gates to a state.

        Args:
        ----
            state: The quantum state to apply gates to.
            gates: Collection of gates to apply.
            removed_locations: Set of qubit locations to skip when applying gates.

        """
        timer = self.timer

        if removed_locations is None:
            removed_locations = set()

        gate_results = {}
        for symbol, physical_gate_locations, gate_kwargs in gates.items():
            ti = timer()
            gate_results = state.run_gate(
                symbol,
                physical_gate_locations - removed_locations,
                **gate_kwargs,
            )
            tf = timer()
            self.total_time += tf - ti
            self.num_gates += len(physical_gate_locations - removed_locations)

        return gate_results
