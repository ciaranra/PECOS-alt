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

"""Wasmer WebAssembly runtime integration.

This module provides integration with the Wasmer WebAssembly runtime for
executing compiled classical functions in the PECOS framework.
"""

from __future__ import annotations

import contextlib
import sys
from pathlib import Path
from typing import TYPE_CHECKING

from pecos.engines.cvm.sim_func import sim_funcs

if TYPE_CHECKING:
    from collections.abc import Sequence
    from typing import Any

with contextlib.suppress(ImportError):
    from wasmer import Instance, Module, Store, engine

with contextlib.suppress(ImportError):
    from wasmer_compiler_cranelift import Compiler as CompilerCranelift

with contextlib.suppress(ImportError):
    from wasmer_compiler_llvm import Compiler as CompilerLLVM


class WasmerInstance:
    """Wrapper class to create a wasmer instance and access its functions."""

    def __init__(self, file: str | bytes, compiler: str = "wasm_cl") -> None:
        """Initialize a Wasmer WebAssembly instance.

        Args:
            file: Path to a WebAssembly file or raw WebAssembly bytes.
            compiler: The compiler backend to use. Options are 'wasm_cl' for
                Cranelift (default) or 'wasm_llvm' for LLVM.

        Raises:
            ImportError: If the wasmer module is not installed.
        """
        if "wasmer" not in sys.modules:
            msg = 'wasmer is being called but not installed! Install "wasmer"'
            raise ImportError(msg)
        if isinstance(file, str):
            with Path.open(file, "rb") as f:
                wasm_b = f.read()
        else:
            wasm_b = file

        store = (
            Store(engine.JIT(CompilerLLVM))
            if compiler == "wasm_llvm"
            else Store(engine.JIT(CompilerCranelift))
        )

        module = Module(store, wasm_b)
        instance = Instance(module)

        self.wasm = instance
        self.module = module

    def get_funcs(self) -> list[str]:
        """Get list of available function names from the WASM module.

        Returns:
            List of function names that can be executed.
        """
        return [
            str(f.name)
            for f in self.module.exports
            if str(f.type).startswith("FunctionType")
        ]

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

        method = getattr(self.wasm.exports, func_name)
        args = [int(b) for _, b in args]
        return method(*args)

    def teardown(self) -> None:
        """Clean up resources (no-op for Wasmer)."""
        # Only needed for wasmtime


def read_wasmer(path: str | bytes, compiler: str = "wasm_cl") -> WasmerInstance:
    """Helper method to create a wasmer instance."""
    return WasmerInstance(path, compiler)
