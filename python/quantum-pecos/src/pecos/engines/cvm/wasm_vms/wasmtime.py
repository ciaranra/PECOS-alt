# Copyright 2024 The PECOS Developers
#
# Licensed under the Apache License, Version 2.0 (the "License"); you may not use this file except in compliance with
# the License.You may obtain a copy of the License at
#
#     https://www.apache.org/licenses/LICENSE-2.0
#
# Unless required by applicable law or agreed to in writing, software distributed under the License is distributed on an
# "AS IS" BASIS, WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied. See the License for the
# specific language governing permissions and limitations under the License.

"""Wasmtime WebAssembly runtime integration.

This module provides integration with the Wasmtime WebAssembly runtime for
executing compiled classical functions in the PECOS framework.
"""

from __future__ import annotations

import contextlib
from typing import TYPE_CHECKING

from pecos.engines.cvm.sim_func import sim_funcs

with contextlib.suppress(ImportError):
    from pecos.foreign_objects.wasmtime import WasmtimeObj

if TYPE_CHECKING:
    from collections.abc import Sequence
    from typing import Any


class WASM:
    """Helper class to provide the same interface as other Wasm objects."""

    def __init__(self, _path: str | bytes) -> None:
        """Initialize a WASM instance using the Wasmtime runtime.

        Args:
            _path: Path to a WebAssembly file or raw WebAssembly bytes.
        """
        self.wasmtime = WasmtimeObj(_path)
        self.wasmtime.init()

    def get_funcs(self) -> list[str]:
        """Get list of available function names from the WASM module.

        Returns:
            List of function names that can be executed.
        """
        return self.wasmtime.get_funcs()

    def exec(
        self,
        func_name: str,
        args: Sequence[tuple[Any, int]],
        *,
        debug: bool = False,
    ) -> int:
        """Execute a WASM function with given arguments.

        Args:
            func_name: Name of the function to execute.
            args: Sequence of (type, value) tuples for arguments.
            debug: Whether to use debug simulation functions.

        Returns:
            Integer result from the function execution.
        """
        if debug and func_name.startswith("sim_"):
            method = sim_funcs[func_name]
            return method(*args)

        args = [int(b) for _, b in args]
        return self.wasmtime.exec(func_name, args)

    def teardown(self) -> None:
        """Clean up wasmtime resources."""
        self.wasmtime.teardown()


def read_wasmtime(path: str | bytes) -> WASM:
    """Helper method to create a wasmtime instance."""
    return WASM(path)
