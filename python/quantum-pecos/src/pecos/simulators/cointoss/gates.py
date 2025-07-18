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

"""Gate operations for coin toss quantum simulator.

This module provides gate operations for the coin toss quantum simulator, implementing a simplified quantum model
where all quantum gates are treated as no-ops and measurements return random classical outcomes.
"""

from __future__ import annotations

from typing import TYPE_CHECKING

import numpy as np

if TYPE_CHECKING:
    from pecos.simulators.cointoss.state import CoinToss
    from pecos.typing import SimulatorGateParams


def ignore_gate(state: CoinToss, _qubits: int, **_params: SimulatorGateParams) -> None:
    """Ignore the gate.

    Args:
        state: An instance of ``CoinToss``.
        _qubits: The qubits the gate was applied to.
    """


def measure(state: CoinToss, _qubits: int, **_params: SimulatorGateParams) -> int:
    """Return |1> with probability ``state.prob`` or |0> otherwise.

    Args:
        state: An instance of ``CoinToss``.
        _qubits: The qubit the measurement is applied to.
    """
    return 1 if np.random.random() < state.prob else 0
