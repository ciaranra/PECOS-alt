#!/usr/bin/env python3
"""Debug full register measurement."""

from pecos.slr import *
from pecos.qeclib import qubit
from pecos.qeclib.qubit.measures import Measure

# Simple test with just measurement
prog = Main(
    q := QReg("q", 3),
    c := CReg("c", 3),
    Measure(q) > c,
)

print("Simple full register measurement:")
print(SlrConverter(prog).guppy())

# Now let's check what the Measure operation looks like
print("\nMeasure operation details:")
meas = prog.ops[0]
print(f"Type: {type(meas).__name__}")
print(f"qargs: {meas.qargs}")
print(f"qargs[0] type: {type(meas.qargs[0])}")
print(f"Has size? {hasattr(meas.qargs[0], 'size')}")
if hasattr(meas.qargs[0], 'size'):
    print(f"Size: {meas.qargs[0].size}")
print(f"cout: {meas.cout}")
print(f"cout[0] type: {type(meas.cout[0])}")
print(f"Has size? {hasattr(meas.cout[0], 'size')}")
if hasattr(meas.cout[0], 'size'):
    print(f"Size: {meas.cout[0].size}")