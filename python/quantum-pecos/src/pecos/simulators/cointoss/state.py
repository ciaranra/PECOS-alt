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

"""State representation for coin toss quantum simulator.

This module provides the quantum state representation for the coin toss simulator, implementing a minimal state
model that tracks qubit count without maintaining actual quantum state information for rapid simulation.
"""

from __future__ import annotations

import random
from typing import TYPE_CHECKING

from pecos.simulators.cointoss import bindings
from pecos.simulators.default_simulator import DefaultSimulator

if TYPE_CHECKING:
    # Handle Python 3.10 compatibility for Self type
    import sys

    if sys.version_info >= (3, 11):
        from typing import Self
    else:
        from typing import TypeVar

        Self = TypeVar("Self", bound="CoinToss")


class CoinToss(DefaultSimulator):
    """Ignore all quantum operations and toss a coin to decide measurement outcomes.

    Meant for stochastical debugging of the classical branches.
    """

    def __init__(
        self,
        num_qubits: int,
        prob: float = 0.5,
        seed: int | None = None,
    ) -> None:
        """Initialization is trivial, since there is no state.

        Args:
            num_qubits (int): Number of qubits being represented.
            prob (float): Probability of measurements returning |1>.
                Default value is 0.5.
            seed (int): Seed for randomness.
        """
        if not isinstance(num_qubits, int):
            msg = "``num_qubits`` should be of type ``int``."
            raise TypeError(msg)
        if not (prob >= 0 and prob <= 1):
            msg = "``prob`` should be a real number in [0,1]."
            raise ValueError(msg)
        random.seed(seed)

        super().__init__()

        self.bindings = bindings.gate_dict
        self.num_qubits = num_qubits
        self.prob = prob

    def reset(self) -> Self:
        """Reset the quantum state for another run without reinitializing."""
        # Do nothing, this simulator does not keep a state!
        return self
