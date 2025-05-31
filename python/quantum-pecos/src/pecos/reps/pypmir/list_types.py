"""List type definitions for PyPMIR intermediate representation.

This module defines specialized list types for PyPMIR (Python PECOS Medium-level Intermediate Representation) including
typed lists for instructions, operations, and other quantum circuit elements.
"""

from __future__ import annotations

from typing import TYPE_CHECKING

from pecos.reps.pypmir.instr_type import Instr
from pecos.reps.pypmir.op_types import Op, QOp
from pecos.typed_list import TypedList

if TYPE_CHECKING:
    from collections.abc import Iterable


class InstrList(TypedList):
    """A list of general Instructions include Ops, Blocks, and Data."""

    _type = Instr

    def __init__(self, data: Iterable[Instr] | None = None) -> None:
        """Initialize an InstrList.

        Args:
            data: Optional iterable of Instr objects to initialize the list.
        """
        super().__init__(self._type, data)
        self.metadata = None


class OpList(InstrList):
    """A list of Operations, e.g., QOp, MOp,EMOp, etc.."""

    _type = Op

    def __init__(self, data: Iterable[Op] | None = None) -> None:
        """Initialize an OpList.

        Args:
            data: Optional iterable of Op objects to initialize the list.
        """
        super().__init__(data)


class QOpList(OpList):
    """A list of just QOps."""

    _type = QOp

    def __init__(self, data: Iterable[QOp] | None = None) -> None:
        """Initialize a QOpList.

        Args:
            data: Optional iterable of QOp (quantum operation) objects to initialize the list.
        """
        super().__init__(data)
