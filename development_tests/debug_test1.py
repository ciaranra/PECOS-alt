#!/usr/bin/env python3
"""Debug test 1 issue."""

from pecos.slr import *
from pecos.qeclib import qubit
from pecos.qeclib.qubit.measures import Measure

class PrepareGHZ(Block):
    """Prepare a GHZ state."""
    def __init__(self, q):
        super().__init__()
        self.q = q
        self.ops = [
            qubit.H(q[0]),
            qubit.CX(q[0], q[1]),
            qubit.CX(q[1], q[2]),
        ]

prog1 = Main(
    q := QReg("q", 3),
    c := CReg("c", 3),
    PrepareGHZ(q),
    Measure(q) > c,
)

print("Generated Guppy code:")
print(SlrConverter(prog1).guppy())