#!/usr/bin/env python3
"""Test a simple case to debug the scoping issue."""

from pecos.slr import *
from pecos.qeclib import qubit
from pecos.qeclib.qubit.measures import Measure

class MeasureFirst(Block):
    """Measure only the first qubit of a register."""
    def __init__(self, q, c):
        super().__init__()
        self.q = q
        self.c = c
        self.ops = [
            Measure(q[0]) > c[0],
        ]

prog = Main(
    q := QReg("q", 3),
    c := CReg("c", 3),
    MeasureFirst(q, c),
)

print("Generated code:")
print(SlrConverter(prog).guppy())

# Let's also check what the Measure operation looks like
meas_block = MeasureFirst(prog.vars[0], prog.vars[1])
print("\nMeasure operation in MeasureFirst:")
print(f"Type: {type(meas_block.ops[0])}")
print(f"qargs: {meas_block.ops[0].qargs}")
print(f"cout: {meas_block.ops[0].cout}")