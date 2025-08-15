"""Compare IR generator output with original generator."""

import pytest
from pecos.slr import *
from pecos.qeclib import qubit
from pecos.qeclib.qubit.measures import Measure
from pecos.slr.gen_codes.guppy.ir_generator import IRGuppyGenerator


def test_compare_simple_measurements():
    """Compare outputs for simple measurements."""
    prog = Main(
        q := QReg("q", 2),
        c := CReg("c", 2),
        
        Measure(q[0]) > c[0],
        Measure(q[1]) > c[1],
    )
    
    # Generate with original
    original = SlrConverter(prog).guppy()
    
    # Generate with IR
    gen = IRGuppyGenerator()
    gen.generate_block(prog)
    ir_output = gen.get_output()
    
    print("Original generator output:")
    print(original)
    print("\nIR generator output:")
    print(ir_output)
    
    # Both should have the basic structure
    assert "@guppy" in original
    assert "@guppy" in ir_output
    assert "def main() -> None:" in original
    assert "def main() -> None:" in ir_output
    
    # Both should measure the qubits
    assert "quantum.measure" in original
    assert "quantum.measure" in ir_output


def test_compare_quantum_gates():
    """Compare outputs for quantum gates."""
    prog = Main(
        q := QReg("q", 3),
        
        qubit.H(q[0]),
        qubit.CX(q[0], q[1]),
        qubit.CZ(q[1], q[2]),
        
        Measure(q) > CReg("c", 3),
    )
    
    # Generate with original
    original = SlrConverter(prog).guppy()
    
    # Generate with IR
    gen = IRGuppyGenerator()
    gen.generate_block(prog)
    ir_output = gen.get_output()
    
    print("\n\nQuantum gates comparison:")
    print("Original:")
    print(original)
    print("\nIR:")
    print(ir_output)
    
    # Both should have the gates
    assert "quantum.h" in original
    assert "quantum.h" in ir_output
    assert "quantum.cx" in original
    assert "quantum.cx" in ir_output
    assert "quantum.cz" in original
    assert "quantum.cz" in ir_output


def test_compare_conditionals():
    """Compare conditional handling."""
    prog = Main(
        q := QReg("q", 2),
        flag := CReg("flag", 1),
        
        Measure(q[0]) > flag[0],
        
        If(flag[0]).Then(
            qubit.X(q[1])
        ),
    )
    
    # Generate with original
    original = SlrConverter(prog).guppy()
    
    # Generate with IR
    gen = IRGuppyGenerator()
    gen.generate_block(prog)
    ir_output = gen.get_output()
    
    print("\n\nConditional comparison:")
    print("Original:")
    print(original)
    print("\nIR:")
    print(ir_output)
    
    # Both should have conditional
    assert "if flag[0]:" in original
    assert "if flag[0]:" in ir_output
    
    # Both should handle unconsumed resources with discard_array
    assert "quantum.discard_array(q)" in original
    assert "quantum.discard_array(q)" in ir_output
    
    # Both should have similar structure
    assert "quantum.x(q_1)" in original
    assert "quantum.x(q_1)" in ir_output


def test_compare_array_operations():
    """Compare array handling."""
    prog = Main(
        q := QReg("q", 4),
        c := CReg("c", 4),
        
        # Mix of operations
        qubit.H(q[0]),
        qubit.H(q[2]),
        
        # Individual measurements
        Measure(q[1]) > c[1],
        Measure(q[3]) > c[3],
        
        # Remaining qubits
        Measure(q[0]) > c[0],
        Measure(q[2]) > c[2],
    )
    
    # Generate with original
    original = SlrConverter(prog).guppy()
    
    # Generate with IR
    gen = IRGuppyGenerator()
    gen.generate_block(prog)
    ir_output = gen.get_output()
    
    print("\n\nArray operations comparison:")
    print("Original:")
    print(original)
    print("\nIR:")
    print(ir_output)
    
    # Check that both handle the operations
    assert "quantum.h" in original
    assert "quantum.h" in ir_output
    assert original.count("quantum.measure") >= 4
    assert ir_output.count("quantum.measure") >= 4


if __name__ == "__main__":
    test_compare_simple_measurements()
    test_compare_quantum_gates()
    test_compare_conditionals()
    test_compare_array_operations()
    print("\nAll comparison tests completed!")