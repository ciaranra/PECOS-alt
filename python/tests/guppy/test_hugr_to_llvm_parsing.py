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
    # Get string format (uses to_str if available, falls back to to_json)
    if hasattr(hugr, "to_str"):
        hugr_str = hugr.to_str()
        # Check if it's the envelope format with header
        if hugr_str.startswith("HUGRiHJv"):
            # Skip header and find JSON start
            json_start = hugr_str.find("{", 9)
            if json_start != -1:
                hugr_str = hugr_str[json_start:]
            else:
                msg = "Could not find JSON start in HUGR envelope"
                raise ValueError(msg)
    else:
        hugr_str = hugr.to_json()
    print(f"\nHUGR length: {len(hugr_str)}")
    print(f"First 100 chars: {hugr_str[:100]}")
    hugr_bytes = hugr_str.encode("utf-8")

    # Compile HUGR to LLVM using pecos-selene
    llvm_ir = compile_hugr_to_llvm(hugr_bytes)

    print("\n=== Generated LLVM IR ===")
    print(llvm_ir)

    # Verify basic structure - updated for new LLVM format
    # The new implementation uses i64 for qubits instead of opaque types
    assert (
        "@__quantum__rt__qubit_allocate()" in llvm_ir
        or "@__quantum__qis__qalloc()" in llvm_ir
    )
    assert "@__quantum__qis__h__body" in llvm_ir

    # Check if we found the main function (entry point)
    assert "define void @main()" in llvm_ir or "bell_state" in llvm_ir


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
    # Get string format (uses to_str if available, falls back to to_json)
    if hasattr(hugr, "to_str"):
        hugr_str = hugr.to_str()
        # Check if it's the envelope format with header
        if hugr_str.startswith("HUGRiHJv"):
            # Skip header and find JSON start
            json_start = hugr_str.find("{", 9)
            if json_start != -1:
                hugr_str = hugr_str[json_start:]
            else:
                msg = "Could not find JSON start in HUGR envelope"
                raise ValueError(msg)
    else:
        hugr_str = hugr.to_json()
    hugr_bytes = hugr_str.encode("utf-8")

    # Compile HUGR to LLVM
    llvm_ir = compile_hugr_to_llvm(hugr_bytes)

    print("\n=== Hadamard Circuit LLVM IR ===")
    print(llvm_ir)

    # Verify operations - updated for new LLVM format
    assert (
        "@__quantum__rt__qubit_allocate()" in llvm_ir
        or "@__quantum__qis__qalloc()" in llvm_ir
    )
    assert "@__quantum__qis__h__body" in llvm_ir
    assert "@__quantum__qis__mz__body" in llvm_ir


if __name__ == "__main__":
    test_hugr_to_llvm_compilation()
    test_simple_hadamard_circuit()
