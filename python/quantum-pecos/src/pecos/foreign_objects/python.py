"""Python foreign object integration for PECOS.

This module provides Python-based foreign object integration capabilities, enabling the execution of Python code and
functions within the PECOS quantum error correction framework for classical computations and custom logic implementation
in quantum algorithms.
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

import inspect
from typing import TYPE_CHECKING

if TYPE_CHECKING:
    from collections.abc import Sequence


class PythonObj:
    """A Python object with an interface consistent with "foreign objects."."""

    def get_funcs(self) -> list[str]:
        """Get list of method names available in this Python object.

        Returns:
            List of method names that can be called.
        """
        return [attr for attr in dir(self) if inspect.ismethod(getattr(self, attr))]

    def exec(self, func_name: str, args: Sequence) -> tuple:
        """Execute a method on this Python object.

        Args:
            func_name: Name of the method to execute.
            args: Sequence of arguments to pass to the method.

        Returns:
            Result of the method call.
        """
        return getattr(self, func_name)(*args)
