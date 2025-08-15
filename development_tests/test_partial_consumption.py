#!/usr/bin/env python3
"""Test partial array consumption patterns common in QEC."""

from pecos.slr import *
from pecos.qeclib import qubit
from pecos.qeclib.qubit.measures import Measure

print("=== Testing Partial Array Consumption ===\n")

# Test 1: Function that measures ancillas but returns data
print("Test 1: Measure ancillas, return data qubits")

class MeasureAncillas(Block):
    """Measure ancilla qubits but keep data qubits."""
    def __init__(self, data, ancilla, syndrome):
        super().__init__()
        self.data = data
        self.ancilla = ancilla
        self.syndrome = syndrome
        self.ops = [
            # Measure all ancillas
            Measure(ancilla[0]) > syndrome[0],
            Measure(ancilla[1]) > syndrome[1],
            Measure(ancilla[2]) > syndrome[2],
            Measure(ancilla[3]) > syndrome[3],
            # Data qubits remain unmeasured
        ]

prog1 = Main(
    data := QReg("data", 7),
    ancilla := QReg("ancilla", 4),
    syndrome := CReg("syndrome", 4),
    
    # Prepare some state
    qubit.H(data[0]),
    qubit.CX(data[0], ancilla[0]),
    
    # Measure ancillas but keep data
    MeasureAncillas(data, ancilla, syndrome),
    
    # Continue using data
    qubit.X(data[0]),
    
    # Eventually measure data
    data_result := CReg("data_result", 7),
    Measure(data) > data_result,
)

print("Generated Guppy:")
print(SlrConverter(prog1).guppy())

print("\n" + "="*50 + "\n")

# Test 2: Function that consumes some qubits from an array
print("Test 2: Consume subset of qubits")

class MeasureFirstHalf(Block):
    """Measure first half of a qubit array."""
    def __init__(self, qubits, results):
        super().__init__()
        self.qubits = qubits
        self.results = results
        self.ops = [
            Measure(qubits[0]) > results[0],
            Measure(qubits[1]) > results[1],
            Measure(qubits[2]) > results[2],
            # qubits[3], [4], [5] remain unmeasured
        ]

prog2 = Main(
    q := QReg("q", 6),
    c_first := CReg("c_first", 3),
    c_second := CReg("c_second", 3),
    
    # Prepare
    qubit.H(q[0]),
    qubit.CX(q[0], q[1]),
    
    # Measure first half
    MeasureFirstHalf(q, c_first),
    
    # Continue with second half
    qubit.H(q[3]),
    Measure(q[3]) > c_second[0],
    Measure(q[4]) > c_second[1],
    Measure(q[5]) > c_second[2],
)

print("Generated Guppy:")
print(SlrConverter(prog2).guppy())

print("\n" + "="*50 + "\n")

# Test 3: Mixed quantum/classical returns
print("Test 3: Function returning both quantum and classical")

class StabilizerMeasurement(Block):
    """Measure stabilizer, return data qubits and syndrome."""
    def __init__(self, data, ancilla, syndrome):
        super().__init__()
        self.data = data
        self.ancilla = ancilla
        self.syndrome = syndrome
        # In real QEC, we'd have stabilizer operations here
        self.ops = [
            # Stabilizer circuit
            qubit.H(ancilla[0]),
            qubit.CX(data[0], ancilla[0]),
            qubit.CX(data[1], ancilla[0]),
            qubit.H(ancilla[0]),
            
            # Measure ancilla to get syndrome
            Measure(ancilla[0]) > syndrome[0],
        ]

prog3 = Main(
    data := QReg("data", 2),
    ancilla := QReg("ancilla", 1),
    syndrome := CReg("syndrome", 1),
    
    # Run stabilizer measurement
    StabilizerMeasurement(data, ancilla, syndrome),
    
    # Continue with data
    qubit.Z(data[0]),
    
    # Final measurements
    final := CReg("final", 2),
    Measure(data) > final,
)

print("Generated Guppy:")
try:
    print(SlrConverter(prog3).guppy())
except Exception as e:
    print(f"Error: {e}")

print("\nAnalysis:")
print("-" * 50)
print("Key requirements for partial consumption:")
print("1. Functions must be able to return unconsumed quantum arrays")
print("2. Functions should return tuples of (quantum, classical) when needed")
print("3. Function calls must properly capture returned quantum resources")
print("4. Type signatures must reflect what's consumed vs returned")