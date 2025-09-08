"""Test HUGR 0.13 to LLVM parsing in pecos-selene."""

import pytest


def test_hugr_to_llvm_compilation() -> None:
    """Test actual HUGR to LLVM compilation in Rust."""
    try:
        from guppylang import guppy
        from guppylang.std.quantum import cx, h, measure, qubit
        from pecos_rslib import compile_hugr_to_llvm
    except ImportError as e:
        pytest.skip(f"Required imports not available: {e}")

    @guppy
    def bell_state() -> tuple[bool, bool]:
        q1, q2 = qubit(), qubit()
        h(q1)
        cx(q1, q2)
        return measure(q1), measure(q2)

    # Compile to HUGR
    hugr = bell_state.compile()
    # Get JSON format instead of binary
    hugr_json = hugr.to_json()
    print(f"\nJSON length: {len(hugr_json)}")
    print(f"First 100 chars: {hugr_json[:100]}")
    hugr_bytes = hugr_json.encode("utf-8")

    # Compile HUGR to LLVM using pecos-selene
    llvm_ir = compile_hugr_to_llvm(hugr_bytes)

    print("\n=== Generated LLVM IR ===")
    print(llvm_ir)

    # Verify basic structure
    assert "%Qubit = type opaque" in llvm_ir
    assert "%Result = type opaque" in llvm_ir
    assert "@__quantum__qis__qalloc()" in llvm_ir
    assert "@__quantum__qis__h__body" in llvm_ir
    assert "EntryPoint" in llvm_ir

    # Check if we found the bell_state function
    assert "bell_state" in llvm_ir or "main" in llvm_ir


def test_simple_hadamard_circuit() -> None:
    """Test simple Hadamard circuit compilation."""
    try:
        from guppylang import guppy
        from guppylang.std.quantum import h, measure, qubit
        from pecos_rslib import compile_hugr_to_llvm
    except ImportError as e:
        pytest.skip(f"Required imports not available: {e}")

    @guppy
    def hadamard_test() -> bool:
        q = qubit()
        h(q)
        return measure(q)

    # Compile to HUGR
    hugr = hadamard_test.compile()
    # Get JSON format instead of binary
    hugr_json = hugr.to_json()
    hugr_bytes = hugr_json.encode("utf-8")

    # Compile HUGR to LLVM
    llvm_ir = compile_hugr_to_llvm(hugr_bytes)

    print("\n=== Hadamard Circuit LLVM IR ===")
    print(llvm_ir)

    # Verify operations
    assert "@__quantum__qis__qalloc()" in llvm_ir
    assert "@__quantum__qis__h__body" in llvm_ir
    assert "@__quantum__qis__mz__body" in llvm_ir
    assert "@__quantum__qis__qfree" in llvm_ir


if __name__ == "__main__":
    test_hugr_to_llvm_compilation()
    test_simple_hadamard_circuit()
