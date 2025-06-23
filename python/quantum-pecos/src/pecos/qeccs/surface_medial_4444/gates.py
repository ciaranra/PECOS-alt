"""Logical gate implementations for the surface-medial-4444 code.

This module provides logical gate implementations for the surface-medial-4444 code,
a medial lattice variant of the surface code with specific geometric
arrangements and stabilizer patterns for quantum error correction.
"""

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

from __future__ import annotations

from typing import TYPE_CHECKING

from pecos.qeccs.default_logical_gate import DefaultLogicalGate
from pecos.qeccs.helper_functions import expected_params

if TYPE_CHECKING:
    from pecos.protocols import QECCProtocol
    from pecos.typing import QECCGateParams


class GateIdentity(DefaultLogicalGate):
    """Logical Identity.

    This is equivalent to ``distance`` number of syndrome of extraction rounds.
    """

    def __init__(
        self,
        qecc: QECCProtocol,
        symbol: str,
        **gate_params: QECCGateParams,
    ) -> None:
        """Initialize the GateSynExtract with the given parameters.

        Args:
        ----
            qecc(QECC): The quantum error correcting code instance
            symbol(str): Symbol identifier for the gate
            **gate_params(dict): kwargs including keys: 'num_syn_extract' (default: qecc.distance).
        """
        super().__init__(qecc, symbol, **gate_params)

        expected_params(
            gate_params,
            {"num_syn_extract", "error_free", "forced_outcome"},
        )

        self.num_syn_extract = gate_params.get("num_syn_extract", qecc.distance)

        # This specifies the logical instructions used for the gate.
        self.instr_symbols = ["instr_syn_extract"] * self.num_syn_extract


class GateInitZero(DefaultLogicalGate):
    """Initialize logical state zero."""

    def __init__(
        self,
        qecc: QECCProtocol,
        symbol: str,
        **gate_params: QECCGateParams,
    ) -> None:
        """Initialize the GateInitZero with the given parameters.

        Args:
        ----
            qecc(QECC): The quantum error correcting code instance
            symbol(str): Symbol identifier for the gate
            **gate_params(dict): kwargs including keys: 'num_syn_extract' (default: 0).
        """
        super().__init__(qecc, symbol, **gate_params)

        expected_params(
            gate_params,
            {"num_syn_extract", "error_free", "forced_outcome"},
        )

        self.num_syn_extract = gate_params.get("num_syn_extract", 0)

        # This specifies the logical instructions used for the gate.
        self.instr_symbols = ["instr_init_zero"]
        syn_extract = ["instr_syn_extract"] * self.num_syn_extract
        self.instr_symbols.extend(syn_extract)


class GateInitPlus(DefaultLogicalGate):
    """Initialize logical state plus."""

    def __init__(
        self,
        qecc: QECCProtocol,
        symbol: str,
        **gate_params: QECCGateParams,
    ) -> None:
        """Initialize the GateInitPlus with the given parameters.

        Args:
        ----
            qecc(QECC): The quantum error correcting code instance
            symbol(str): Symbol identifier for the gate
            **gate_params(dict): kwargs including keys: 'num_syn_extract' (default: 0).
        """
        super().__init__(qecc, symbol, **gate_params)

        expected_params(
            gate_params,
            {"num_syn_extract", "error_free", "forced_outcome"},
        )

        self.num_syn_extract = gate_params.get("num_syn_extract", 0)

        # This specifies the logical instructions used for the gate.
        self.instr_symbols = ["instr_init_plus"]
        syn_extract = ["instr_syn_extract"] * self.num_syn_extract
        self.instr_symbols.extend(syn_extract)
