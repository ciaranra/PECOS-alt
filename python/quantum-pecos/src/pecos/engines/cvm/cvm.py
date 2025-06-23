# Copyright 2022 The PECOS Developers
#
# Licensed under the Apache License, Version 2.0 (the "License"); you may not use this file except in compliance with
# the License.You may obtain a copy of the License at
#
#     https://www.apache.org/licenses/LICENSE-2.0
#
# Unless required by applicable law or agreed to in writing, software distributed under the License is distributed on an
# "AS IS" BASIS, WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied. See the License for the
# specific language governing permissions and limitations under the License.

"""Classical Virtual Machine (CVM) implementation for PECOS.

This module provides the main CVM implementation for executing classical
computations alongside quantum operations in hybrid quantum-classical algorithms.
"""

from __future__ import annotations

from typing import TYPE_CHECKING

if TYPE_CHECKING:
    from collections.abc import Callable
    from typing import Any, Protocol

    class CCOPVMProtocol(Protocol):
        """Protocol for CCOP VM objects."""

        def exec(self, func_name: str, args: list) -> int | None:
            """Execute a function in the classical co-processor VM.

            Args:
                func_name: Name of the function to execute.
                args: List of arguments to pass to the function.

            Returns:
                Optional integer result from the function execution.
            """
            ...

    class InterpreterProtocol(Protocol):
        """Protocol for interpreter objects."""

        # Without more context, we'll keep this minimal


class CVM:
    """Classical Virtual Machine for executing classical functions and statements.

    The CVM manages the execution of classical code within the hybrid quantum-classical
    computing framework, providing interfaces for classical operations and interpreters.
    """

    def __init__(
        self,
        ccop_vm: CCOPVMProtocol | None = None,
        cinterpreter: InterpreterProtocol | None = None,
        sim_debug: dict[str, Callable[..., Any]] | None = None,
    ) -> None:
        """Classical Virtual Machine, which is responsible for executing classical functions and statements.

        Attributes:
        ----------
            ccop_vm: A VM representing the computing environment of a classical co-processor. This generally provides
                external classical functions that are usually problem specific.
            cinterpreter: Provides an interpreter for generic classical statements. For example, boolean operations,
                assignments, comparisons, etc.
            sim_debug: A collection of functions used in the simulation environment to provide additional information
                that may not typically be available to a physical quantum device.
        """
        self.state = {}

        self.ccop_vm = ccop_vm
        self.cinterpreter = cinterpreter
        self.sim_debug = sim_debug

    def reset_state(self) -> None:
        """Reset the CVM state to its initial empty state."""
        self.state = ()

    def exec(self, func_name: str, args: list[Any]) -> None:
        """Execute a function in the classical virtual machine.

        Args:
            func_name: Name of the function to execute.
            args: List of arguments to pass to the function.
        """
