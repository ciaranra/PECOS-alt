#!/usr/bin/env python3
"""Debug measurement analysis."""

from pecos.slr import *
from pecos.qeclib import qubit
from pecos.qeclib.qubit.measures import Measure
from pecos.slr.gen_codes.guppy.measurement_analyzer import MeasurementAnalyzer

# Test 3: Operations between measurements
prog3 = Main(
    q := QReg("q", 4),
    c := CReg("c", 4),
    qubit.H(q[0]),
    Measure(q[0]) > c[0],
    qubit.CX(q[1], q[2]),  # Operation between measurements
    Measure(q[1]) > c[1],
    Measure(q[2]) > c[2],
    Measure(q[3]) > c[3],
)

analyzer = MeasurementAnalyzer()
info = analyzer.analyze_block(prog3)

print("Measurement Analysis for Test 3:")
for qreg_name, meas_info in info.items():
    print(f"\nQReg: {qreg_name}")
    print(f"  Size: {meas_info.qreg_size}")
    print(f"  Measured indices: {sorted(meas_info.measured_indices)}")
    print(f"  Measurement positions: {meas_info.measurement_positions}")
    print(f"  All measured together: {meas_info.all_measured_together}")
    print(f"  First measurement pos: {meas_info.first_measurement_pos}")

print("\nOperations:")
for i, op in enumerate(prog3.ops):
    print(f"{i}: {type(op).__name__}")
    if hasattr(op, 'qargs'):
        print(f"   qargs: {[str(q) for q in op.qargs]}")