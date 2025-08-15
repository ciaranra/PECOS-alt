#!/usr/bin/env python3
"""Debug measurement patterns."""

from pecos.slr import *
from pecos.qeclib import qubit
from pecos.qeclib.qubit.measures import Measure

# Test single measurement of entire register
prog = Main(
    q := QReg("q", 4),
    c := CReg("c", 4),
    qubit.H(q[0]),
    # Measure entire register at once
    Measure(q) > c,
)

print("Generated Guppy code:")
print("-" * 50)
print(SlrConverter(prog).guppy())