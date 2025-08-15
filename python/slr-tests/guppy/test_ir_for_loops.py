"""Test For loop implementation in IR generator."""

import pytest
from pecos.slr import *
from pecos.qeclib import qubit
from pecos.qeclib.qubit.measures import Measure
from pecos.slr.gen_codes.guppy.ir_generator import IRGuppyGenerator


def test_for_loop_range_basic():
    """Test basic for loop with range."""
    prog = Main(
        q := QReg("q", 5),
        
        # Apply H gate to each qubit using For loop
        For("i", 0, 5).Do(
            Comment("Apply H to qubit i"),
            # In real implementation, we'd use: qubit.H(q[i])
            # For now, just apply to q[0] as placeholder
            qubit.H(q[0]),
        ),
        
        Measure(q) > CReg("results", 5),
    )
    
    gen = IRGuppyGenerator()
    gen.generate_block(prog)
    code = gen.get_output()
    
    print("IR-generated code with For loop (range):")
    print(code)
    
    # Check that for loop is generated
    assert "for i in range(0, 5):" in code
    assert "# Apply H to qubit i" in code


def test_for_loop_range_with_step():
    """Test for loop with custom step."""
    prog = Main(
        q := QReg("q", 10),
        
        # Apply X to every other qubit
        For("i", 0, 10, 2).Do(
            Comment("Apply X to even-indexed qubits"),
            qubit.X(q[0]),  # Placeholder
        ),
        
        Measure(q) > CReg("results", 10),
    )
    
    gen = IRGuppyGenerator()
    gen.generate_block(prog)
    code = gen.get_output()
    
    print("\nIR-generated code with For loop (step):")
    print(code)
    
    # Check step parameter
    assert "for i in range(0, 10, 2):" in code


def test_for_loop_iterable():
    """Test for loop over an iterable."""
    prog = Main(
        q := QReg("q", 3),
        indices := CReg("indices", 3),
        
        # For loop over a collection (conceptual)
        For("idx", "indices").Do(
            Comment("Process index from collection"),
            qubit.Y(q[0]),
        ),
        
        Measure(q) > CReg("results", 3),
    )
    
    gen = IRGuppyGenerator()
    gen.generate_block(prog)
    code = gen.get_output()
    
    print("\nIR-generated code with For loop (iterable):")
    print(code)
    
    # Check iterable pattern
    assert "for idx in indices:" in code


def test_nested_for_loops():
    """Test nested for loops."""
    prog = Main(
        q := QReg("q", 9),  # 3x3 grid
        
        # Nested loops for 2D pattern
        For("i", 0, 3).Do(
            Comment("Outer loop"),
            For("j", 0, 3).Do(
                Comment("Inner loop"),
                Comment("Would apply operation to q[i*3 + j]"),
                qubit.H(q[0]),  # Placeholder
            ),
        ),
        
        Measure(q) > CReg("results", 9),
    )
    
    gen = IRGuppyGenerator()
    gen.generate_block(prog)
    code = gen.get_output()
    
    print("\nIR-generated code with nested For loops:")
    print(code)
    
    # Check nested structure
    assert "for i in range(0, 3):" in code
    assert "for j in range(0, 3):" in code
    assert "# Outer loop" in code
    assert "# Inner loop" in code


def test_for_loop_with_quantum_operations():
    """Test for loop with quantum operations inside."""
    prog = Main(
        data := QReg("data", 4),
        ancilla := QReg("ancilla", 1),
        
        # Initialize data qubits
        For("i", 0, 4).Do(
            Comment("Initialize data qubit"),
            qubit.H(data[0]),  # Would be data[i]
        ),
        
        # Entangle with ancilla
        For("i", 0, 4).Do(
            Comment("Entangle with ancilla"),
            qubit.CX(data[0], ancilla[0]),  # Would be data[i]
        ),
        
        Measure(data) > CReg("data_results", 4),
        Measure(ancilla) > CReg("ancilla_result", 1),
    )
    
    gen = IRGuppyGenerator()
    gen.generate_block(prog)
    code = gen.get_output()
    
    print("\nIR-generated code with For loops and quantum ops:")
    print(code)
    
    # Check multiple for loops
    assert code.count("for i in range(0, 4):") >= 2
    assert "quantum.h" in code
    assert "quantum.cx" in code


def test_for_loop_limitations():
    """Test current limitations of For loop implementation."""
    # The main limitation is that we can't use the loop variable
    # to index into quantum registers yet
    
    prog = Main(
        q := QReg("q", 5),
        
        For("i", 0, 5).Do(
            Comment("TODO: Need to support q[i] indexing"),
            Comment("Currently would need to unpack array first"),
        ),
        
        Measure(q) > CReg("c", 5),
    )
    
    gen = IRGuppyGenerator()
    gen.generate_block(prog)
    code = gen.get_output()
    
    print("\nFor loop limitations:")
    print(code)
    
    # Document the limitation
    assert "for i in range(0, 5):" in code
    assert "TODO" in code


def test_for_error_in_qasm():
    """Test that For loops raise error in QASM generator."""
    from pecos.slr.gen_codes.gen_qasm import QASMGenerator
    
    prog = Main(
        q := QReg("q", 3),
        
        For("i", 0, 3).Do(
            qubit.H(q[0]),
        ),
        
        Measure(q) > CReg("c", 3),
    )
    
    gen = QASMGenerator()
    
    # Should raise NotImplementedError
    with pytest.raises(NotImplementedError) as exc_info:
        gen.generate_block(prog)
    
    assert "For loops are not supported in QASM" in str(exc_info.value)


def test_for_loop_syntax_examples():
    """Document For loop syntax and patterns."""
    
    print("\n=== For Loop Syntax Examples ===")
    
    print("\nSLR Syntax:")
    print("  For('i', 0, 5).Do(...)           # range(0, 5)")
    print("  For('i', 0, 10, 2).Do(...)       # range(0, 10, 2)")
    print("  For('item', collection).Do(...)   # for item in collection")
    
    print("\nGenerated Guppy:")
    print("  for i in range(0, 5):")
    print("      # loop body")
    print("")
    print("  for i in range(0, 10, 2):")
    print("      # loop body")
    print("")
    print("  for item in collection:")
    print("      # loop body")
    
    print("\nFuture enhancement - indexed access:")
    print("  For('i', 0, n).Do(")
    print("      qubit.H(q[i]),  # Would need special handling")
    print("  )")
    
    # Always passes
    assert True


if __name__ == "__main__":
    test_for_loop_range_basic()
    test_for_loop_range_with_step()
    test_for_loop_iterable()
    test_nested_for_loops()
    test_for_loop_with_quantum_operations()
    test_for_loop_limitations()
    test_for_error_in_qasm()
    test_for_loop_syntax_examples()
    print("\nAll For loop tests completed!")