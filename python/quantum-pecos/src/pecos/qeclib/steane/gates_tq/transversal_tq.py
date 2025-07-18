# Copyright 2025 The PECOS Developers
#
# Licensed under the Apache License, Version 2.0 (the "License"); you may not use this file except in compliance with
# the License.You may obtain a copy of the License at
#
#     https://www.apache.org/licenses/LICENSE-2.0
#
# Unless required by applicable law or agreed to in writing, software distributed under the License is distributed on an
# "AS IS" BASIS, WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied. See the License for the
# specific language governing permissions and limitations under the License.

from pecos.qeclib import qubit as qb
from pecos.slr import Block, QReg
from pecos.qeclib.generic.transversal import transversal_tq


class CX(Block):
    """Transversal logical CX gate for Color code."""

    def __init__(self, q1: QReg, q2: QReg) -> None:
        """Initialize a transversal logical CX gate on two Steane code logical qubits.

        Args:
            q1: First quantum register (control).
            q2: Second quantum register (target).
        """
        super().__init__()

        self.extend(
            transversal_tq(qb.CX, q1, q2)
        )

class CY(Block):
    """Transversal logical CX gate for Color code."""

    def __init__(self, q1: QReg, q2: QReg) -> None:
        """Initialize a transversal logical CY gate on two Steane code logical qubits.

        Args:
            q1: First quantum register (control).
            q2: Second quantum register (target).
        """
        super().__init__()

        self.extend(
            transversal_tq(qb.CY, q1, q2)
        )

class CZ(Block):
    """Transversal logical CX gate for Color code."""

    def __init__(self, q1: QReg, q2: QReg) -> None:
        """Initialize a transversal logical CZ gate on two Steane code logical qubits.

        Args:
            q1: First quantum register (control).
            q2: Second quantum register (target).
        """
        super().__init__()

        self.extend(
            transversal_tq(qb.CZ, q1, q2)
        )

class SZZ(Block):
    """Transversal logical CX gate for Color code."""

    def __init__(self, q1: QReg, q2: QReg) -> None:
        """Initialize a transversal logical SZZ gate on two Steane code logical qubits.

        Args:
            q1: First quantum register (control).
            q2: Second quantum register (target).
        """
        super().__init__()

        self.extend(
            transversal_tq(qb.SZZ, q1, q2)
        )
