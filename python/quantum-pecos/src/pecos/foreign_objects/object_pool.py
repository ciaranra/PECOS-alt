"""Object pool management for foreign object lifecycle.

This module provides object pool management functionality for handling the lifecycle of foreign objects within the
PECOS framework, enabling efficient resource management and reuse of external computational resources such as
WebAssembly modules and other foreign language integrations.
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

if TYPE_CHECKING:
    from collections.abc import Sequence

    from pecos.protocols import ForeignObjectProtocol


class NamedObjectPool:
    """A collection of objections that can be access via this class."""

    def __init__(self, **objects: ForeignObjectProtocol) -> None:
        """Initialize the NamedObjectPool.

        Args:
        ----
            **objects: Foreign objects to include in the pool, keyed by namespace name.
        """
        self.objs = objects
        self.default = objects.get("default")

    def new_instance(self) -> None:
        """Create new instance/internal state."""
        for obj in self.objs.values():
            obj.new_instance()

    def init(self) -> None:
        """Initialize object before running a series of experiments."""
        for obj in self.objs.values():
            obj.init()

    def shot_reinit(self) -> None:
        """Call before each shot to, e.g., reset variables."""
        for obj in self.objs.values():
            if "shot_reinit" in obj.get_funcs():
                obj.exec("shot_reinit", [])

    def add(self, namespace: str, obj: ForeignObjectProtocol) -> None:
        """Add a foreign object to the pool.

        Args:
            namespace: Name identifier for the object.
            obj: Foreign object to add to the pool.

        Raises:
            Exception: If an object with the same namespace already exists.
        """
        if namespace in self.objs:
            msg = f"Object named '{namespace}' already exists!"
            raise Exception(msg)
        self.objs[namespace] = obj

    def get_funcs(self) -> list[str]:
        """Get a list of function names available from the object."""
        return []

    def exec(
        self,
        func_name: str,
        args: Sequence,
        namespace: str | None = None,
    ) -> tuple:
        """Execute a function given a list of arguments."""
        if namespace is None:
            obj = self.default
        elif namespace not in self.objs:
            msg = f"Object named '{namespace}' not recognized!"
            raise Exception(msg)
        else:
            obj = self.objs[namespace]

        return obj.exec(func_name, args)
