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

"""A simple wrapper for the ProjectQ simulator.

Compatibility checked for: ProjectQ version 0.5.1
"""

from __future__ import annotations

import contextlib
from typing import TYPE_CHECKING

import numpy as np
from projectq import MainEngine
from projectq.ops import All, Measure

from pecos.simulators.gate_syms import alt_symbols
from pecos.simulators.projectq import bindings
from pecos.simulators.projectq.helper import MakeFunc
from pecos.simulators.projectq.logical_sign import find_logical_signs
from pecos.simulators.sim_class_types import StateVector

if TYPE_CHECKING:
    from collections.abc import Callable

    from projectq.ops._basics import BasicGate

    from pecos.circuits import QuantumCircuit
    from pecos.typing import Location, SimulatorGateParams


class ProjectQSim(StateVector):
    """Initializes the stabilizer state.

    Args:
    ----
        num_qubits (int): Number of qubits being represented.
    """

    def __init__(self, num_qubits: int) -> None:
        """Initialize the ProjectQ quantum simulator state.

        Args:
            num_qubits: Number of qubits to simulate.

        Raises:
            TypeError: If num_qubits is not an integer.
        """
        if not isinstance(num_qubits, int):
            msg = f"`num_qubits` should be of type `int.` but got type: {type(num_qubits)} "
            raise TypeError(msg)

        super().__init__()

        self.bindings = bindings.gate_dict
        for k, v in alt_symbols.items():
            if v in self.bindings:
                self.bindings[k] = self.bindings[v]

        self.num_qubits = num_qubits
        self.eng = MainEngine()

        self.qureg = self.eng.allocate_qureg(num_qubits)
        self.qs = list(self.qureg)
        self.qids = dict(enumerate(self.qs))
        self.gate_dict = {}

    def reset(self) -> ProjectQSim:
        """Reset the quantum state to all 0 for another run without reinitializing."""
        self.eng.flush()
        amps = [0] * 2**self.num_qubits
        amps[0] = 1
        self.eng.backend.set_wavefunction(amps, self.qureg)
        return self

    def logical_sign(self, logical_op: QuantumCircuit) -> int:
        """Find the sign of a logical operator.

        Args:
            logical_op (QuantumCircuit): The logical operator circuit.
        """
        return find_logical_signs(self, logical_op)

    def add_gate(
        self,
        symbol: str,
        gate_obj: (
            BasicGate
            | type[BasicGate]
            | Callable[[ProjectQSim, Location, SimulatorGateParams], None]
        ),
        *,
        make_func: bool = True,
    ) -> None:
        """Adds a new gate on the fly to this Simulator.

        Args:
        ----
            symbol: The symbol/name for the gate
            gate_obj: The gate object to add
            make_func: Whether to wrap the gate object with MakeFunc
        """
        if symbol in self.gate_dict:
            print("WARNING: Can not add gate as the symbol has already been taken.")
        elif make_func:
            self.gate_dict[symbol] = MakeFunc(gate_obj).func
        else:
            self.gate_dict[symbol] = gate_obj

    def get_probs(self, key_basis: list[str] | None = None) -> dict[str, float]:
        """Get measurement probabilities for computational basis states.

        Args:
            key_basis: Optional list of basis states to get probabilities for.
                      If None, returns probabilities for all 2^n basis states.

        Returns:
            Dictionary mapping basis state strings to their probabilities.
        """
        self.eng.flush()

        if key_basis:
            probs_dict = {}
            for b in key_basis:
                # ProjectQ uses reversed bit order
                b_reversed = b[::-1]
                p = self.eng.backend.get_probability(b_reversed, self.qureg)
                probs_dict[b] = p
            return probs_dict

        probs_dict = {}
        for i in range(np.power(2, self.num_qubits)):
            b_str = format(i, f"0{self.num_qubits}b")
            p = self.eng.backend.get_probability(b_str, self.qureg)
            # Store with reversed bit order for consistent output
            b_key = b_str[::-1]
            probs_dict[b_key] = p

        return probs_dict

    def get_amps(self, key_basis: list[str] | None = None) -> dict[str, complex]:
        """Get probability amplitudes for computational basis states.

        Args:
            key_basis: Optional list of basis states to get amplitudes for.
                      If None, returns amplitudes for all 2^n basis states.

        Returns:
            Dictionary mapping basis state strings to their complex amplitudes.
        """
        self.eng.flush()

        if key_basis:
            amps_dict = {}
            for b in key_basis:
                # ProjectQ uses reversed bit order
                b_reversed = b[::-1]
                p = self.eng.backend.get_amplitude(b_reversed, self.qureg)
                amps_dict[b] = p
            return amps_dict

        amp_dict = {}
        for i in range(np.power(2, self.num_qubits)):
            b_str = format(i, f"0{self.num_qubits}b")
            a = self.eng.backend.get_amplitude(b_str, self.qureg)
            # Store with reversed bit order for consistent output
            b_key = b_str[::-1]
            amp_dict[b_key] = a

        return amp_dict

    def __del__(self) -> None:
        """Clean up ProjectQ engine and deallocate qubits when the object is destroyed."""
        self.eng.flush()
        All(Measure) | self.qureg  # Requirement by ProjectQ...

        with contextlib.suppress(KeyError):
            self.eng.flush(deallocate_qubits=True)
