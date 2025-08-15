#!/usr/bin/env python3
"""Debug measurement patterns in detail."""

from pecos.slr import *
from pecos.qeclib import qubit
from pecos.qeclib.qubit.measures import Measure

# Test single measurement of entire register
prog = Main(
    q := QReg("q", 4),
    c := CReg("c", 4),
    # Measure entire register at once
    Measure(q) > c,
)

# Let's examine the measurement operation
print(f"Number of operations: {len(prog.ops)}")
for i, op in enumerate(prog.ops):
    print(f"Op {i}: {type(op).__name__}")

meas_op = prog.ops[-1]  # The last operation should be Measure
print(f"Measurement operation type: {type(meas_op).__name__}")
print(f"meas.qargs: {meas_op.qargs}")
print(f"meas.cout: {meas_op.cout}")
print(f"len(meas.qargs): {len(meas_op.qargs)}")
print(f"len(meas.cout): {len(meas_op.cout)}")

if len(meas_op.qargs) > 0:
    print(f"\nFirst qarg: {meas_op.qargs[0]}")
    print(f"Has size? {hasattr(meas_op.qargs[0], 'size')}")
    if hasattr(meas_op.qargs[0], 'size'):
        print(f"Size: {meas_op.qargs[0].size}")
    print(f"Has sym? {hasattr(meas_op.qargs[0], 'sym')}")
    if hasattr(meas_op.qargs[0], 'sym'):
        print(f"Sym: {meas_op.qargs[0].sym}")

if len(meas_op.cout) > 0:
    print(f"\nFirst cout: {meas_op.cout[0]}")
    print(f"Has size? {hasattr(meas_op.cout[0], 'size')}")
    if hasattr(meas_op.cout[0], 'size'):
        print(f"Size: {meas_op.cout[0].size}")
    print(f"Has sym? {hasattr(meas_op.cout[0], 'sym')}")
    if hasattr(meas_op.cout[0], 'sym'):
        print(f"Sym: {meas_op.cout[0].sym}")