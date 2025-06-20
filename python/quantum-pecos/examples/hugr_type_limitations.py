#!/usr/bin/env python3
"""Demonstrate HUGR type limitations and workarounds.

This example shows what types currently work and don't work in the
Guppy -> HUGR -> LLVM compilation pipeline.
"""

from guppylang import guppy
from guppylang.std.quantum import h, measure, qubit
from pecos.compilation_pipeline import compile_guppy_to_hugr, compile_hugr_to_llvm
from pecos.hugr_types import HugrTypeError, create_quantum_example


def test_unsupported_types() -> None:
    """Show examples of unsupported types."""
    print("Testing Unsupported Types")
    print("=" * 50)

    # Example 1: Integer return type
    @guppy
    def return_int() -> int:
        return 42

    try:
        hugr = compile_guppy_to_hugr(return_int)
        compile_hugr_to_llvm(hugr)
        print("✗ This should have failed!")
    except HugrTypeError as e:
        print("✓ Expected error caught:")
        print(f"  {e}")
        print()

    # Example 2: Classical computation
    @guppy
    def add_numbers(x: int, y: int) -> int:
        return x + y

    try:
        hugr = compile_guppy_to_hugr(add_numbers)
        compile_hugr_to_llvm(hugr)
        print("✗ This should have failed!")
    except HugrTypeError as e:
        print("✓ Expected error caught:")
        print(f"  Type: {e.unsupported_type}")
        print()


def test_supported_quantum_operations() -> None:
    """Show examples that work."""
    print("\nTesting Supported Quantum Operations")
    print("=" * 50)

    # Example 1: Quantum coin (returns measurement)
    @guppy
    def quantum_coin() -> bool:
        q = qubit()
        h(q)
        return measure(q)

    try:
        hugr = compile_guppy_to_hugr(quantum_coin)
        print("✓ Quantum coin compiled to HUGR")
        print(f"  HUGR size: {len(hugr)} bytes")

        # This might still fail due to bool type issues, but let's try
        compile_hugr_to_llvm(hugr)
        print("✓ HUGR compiled to LLVM!")
    except HugrTypeError as e:
        print(f"✗ Type limitation: {e.unsupported_type}")
    except RuntimeError as e:
        print(f"✗ Other error: {e}")


def show_workarounds() -> None:
    """Show how to work around type limitations."""
    print("\n\nWorkarounds for Type Limitations")
    print("=" * 50)

    print("1. Use quantum operations instead of classical:")
    print("   - Instead of returning int, return measurement results")
    print("   - Use quantum gates for computation")

    print("\n2. Separate classical and quantum parts:")
    print("   - Do classical preprocessing in Python")
    print("   - Use Guppy only for quantum operations")
    print("   - Do classical postprocessing in Python")

    print("\n3. Example of working code:")
    print(create_quantum_example())


def main() -> None:
    """Run all demonstrations."""
    print("HUGR Type Limitations Demo")
    print("=" * 70)
    print()

    test_unsupported_types()
    test_supported_quantum_operations()
    show_workarounds()

    print("\nSummary:")
    print("- Classical types (int, float, etc.) are not yet supported")
    print("- Focus on quantum operations for now")
    print("- Type support will improve in future versions")


if __name__ == "__main__":
    main()
