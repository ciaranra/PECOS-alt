"""Single-qubit depolarizing noise with leakage.

This module provides depolarizing noise models for single-qubit operations
that include leakage to states outside the computational subspace,
providing more realistic error modeling for quantum systems.
"""

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

from __future__ import annotations

from typing import TYPE_CHECKING

import numpy as np

from pecos.reps.pypmir.op_types import QOp

if TYPE_CHECKING:
    from pecos.protocols import MachineProtocol


def noise_sq_depolarizing_leakage(
    op: QOp,
    p: float,
    noise_dict: dict,
    machine: MachineProtocol,
) -> list[QOp] | None:
    """Apply single-qubit depolarizing noise with leakage support.

    Applies depolarizing noise to quantum operations while handling
    leaked qubits through the machine protocol. Leaked qubits are
    excluded from noise application.

    Args:
        op: Quantum operation to apply noise to.
        p: Probability of noise occurring on each non-leaked qubit.
        noise_dict: Dictionary mapping fault types to their probabilities.
        machine: Machine protocol handling qubit leakage states.

    Returns:
        List of quantum operations including modified operation and noise,
        or None if no noise or leakage occurs.
    """
    args = set(op.args)
    leaked = machine.leaked_qubits & args

    if leaked:
        not_leaked = args - leaked
        noisy_op = QOp(name=op.name, args=list(not_leaked), metadata=dict(op.metadata))
    else:
        noisy_op = op

    rand_nums = np.random.random(len(noisy_op.args)) <= p

    noise = {}
    if np.any(rand_nums):
        for r, loc in zip(rand_nums, noisy_op.args, strict=False):
            if r:
                rand = np.random.random()
                p_tot = 0.0
                for fault1, prob in noise_dict.items():
                    p_tot += prob

                    if p_tot >= rand:
                        noise.setdefault(fault1, []).append(loc)
                        break

    if noise or leaked:
        buffered_ops = []

        if noise:
            for sym, args in noise.items():
                if sym == "L":
                    leak_ops = machine.leak(set(noise["L"]))
                    buffered_ops.extend(leak_ops)
                else:
                    buffered_ops.extend(
                        (noisy_op, QOp(name=sym, args=args, metadata={})),
                    )

        else:
            buffered_ops.append(noisy_op)

        return buffered_ops

    return None
