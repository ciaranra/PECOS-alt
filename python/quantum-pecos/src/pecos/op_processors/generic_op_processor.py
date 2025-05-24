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

from pecos.reps.pypmir import types as pt

if TYPE_CHECKING:
    from pecos.protocols import ErrorModelProtocol, MachineProtocol


class GenericOpProc:
    def __init__(
        self,
        machine: MachineProtocol | None = None,
        error_model: ErrorModelProtocol | None = None,
    ) -> None:
        """Initialize the GenericOpProc.

        Args:
        ----
            machine: Optional machine protocol for processing machine operations.
            error_model: Optional error model protocol for applying noise.
        """
        self.machine = machine
        self.error_model = error_model

    def reset(self) -> None:
        """Reset state to initialization state."""

    def init(self) -> None:
        pass

    def attach_machine(self, machine: MachineProtocol) -> None:
        self.machine = machine

    def attach_error_model(self, error_model: ErrorModelProtocol) -> None:
        self.error_model = error_model

    def shot_reinit(self) -> None:
        pass

    def process(self, buffered_ops: list) -> list:
        buffered_noisy_qops = []
        for op in buffered_ops:
            if isinstance(op, pt.opt.MOp):
                ops = self.machine.process([op])
                noisy_ops = self.error_model.process(ops)
            elif isinstance(op, pt.opt.QOp):
                noisy_ops = self.error_model.process([op])
            else:
                msg = f"This operation processor only handles MOps and QOps! Received type: {type(op)}"
                raise TypeError(msg)

            if noisy_ops:
                buffered_noisy_qops.extend(noisy_ops)

        return buffered_noisy_qops

    def process_meas(self, measurements: list[dict]) -> list[dict]:
        return measurements
