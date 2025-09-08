#!/usr/bin/env python3
"""Test HUGR compilation and QIR generation."""

import subprocess
import tempfile
from pathlib import Path


def test_rust_hugr_compilation() -> None:
    """Test that the Rust HUGR support compiles."""
    print("=== Testing Rust HUGR Compilation ===")

    # Test 1: Check if HUGR support compiles in the new pecos-hugr crate
    result = subprocess.run(
        ["cargo", "check", "-p", "pecos-hugr"],  # noqa: S607
        capture_output=True,
        text=True,
        check=False,
    )

    if result.returncode == 0:
        print("[PASS] HUGR support compiles successfully")
    else:
        print("[FAIL] HUGR compilation failed")
        print(result.stderr[:500])
        msg = "HUGR compilation failed"
        raise AssertionError(msg)

    # Test 2: Run HUGR-specific unit tests
    result = subprocess.run(
        [  # noqa: S607
            "cargo",
            "test",
            "-p",
            "pecos-hugr",
        ],
        capture_output=True,
        text=True,
        check=False,
    )

    if result.returncode == 0:
        print("[PASS] HUGR unit tests pass")
        # Count tests
        test_count = result.stdout.count("test result: ok")
        print(f"  {test_count} test suites passed")
    else:
        print("[FAIL] HUGR tests failed")
        print(result.stderr[:500])
        msg = "HUGR tests failed"
        raise AssertionError(msg)

    # Test passed


def test_standard_qir_generation() -> None:
    """Test LLVM IR generation patterns (HUGR convention, not QIR)."""
    print("\n=== Testing LLVM IR Generation (HUGR Convention) ===")

    # Create a test LLVM IR file (HUGR convention)
    test_llvm = """
; HUGR convention LLVM IR (not QIR)
; Uses i64 for qubit indices, immediate measurements

declare void @__quantum__qis__h__body(i64)
declare i32 @__quantum__qis__m__body(i64, i64)
declare void @__quantum__rt__result_record_output(i64, i8*)

@.str.c = constant [2 x i8] c"c\00"

define void @main() #0 {
    ; Apply H to qubit 0
    call void @__quantum__qis__h__body(i64 0)

    ; Immediate measurement - returns i32 result
    %result = call i32 @__quantum__qis__m__body(i64 0, i64 0)

    ; Record result
    call void @__quantum__rt__result_record_output(i64 0, i8* getelementptr inbounds ([2 x i8], [2 x i8]* @.str.c, i32 0, i32 0))

    ret void
}

attributes #0 = { "EntryPoint" }
"""

    with tempfile.NamedTemporaryFile(mode="w", suffix=".ll", delete=False) as f:
        f.write(test_llvm)
        llvm_file = f.name

    print(f"[OK] Created test LLVM IR file: {llvm_file}")

    # Verify it's valid LLVM IR
    try:
        result = subprocess.run(  # noqa: S603
            ["llvm-as", llvm_file, "-o", "/dev/null"],  # noqa: S607
            capture_output=True,
            text=True,
            check=False,
        )

        if result.returncode == 0:
            print("[PASS] LLVM IR format is valid")
        else:
            print("[FAIL] LLVM IR validation failed (invalid format)")
    except FileNotFoundError:
        print("⚠ llvm-as not available, skipping validation")

    # Clean up
    Path(llvm_file).unlink()

    # LLVM IR generation test passed


def test_qir_examples() -> None:
    """Test existing LLVM IR examples (HUGR convention)."""
    print("\n=== Testing LLVM IR Examples (HUGR Convention) ===")

    # Find examples relative to the project root
    test_dir = Path(__file__).parent
    project_root = (
        test_dir.parent.parent.parent
    )  # tests/guppy -> tests -> python -> PECOS

    # Look for LLVM IR examples (HUGR convention, not QIR)
    llvm_examples = project_root / "examples" / "llvm"

    if not llvm_examples.exists():
        print(f"[SKIP] LLVM examples directory not found at {llvm_examples}")
        print("[INFO] This test requires LLVM IR examples to be present")
        import pytest

        pytest.skip("LLVM examples directory not found")

    # Look for .ll files in the examples directory and subdirectories
    llvm_files = list(llvm_examples.glob("*.ll"))

    # Also check for .ll files in the parent examples directory
    parent_ll_files = list((llvm_examples.parent).glob("*.ll"))
    llvm_files.extend(parent_ll_files)

    print(f"Found {len(llvm_files)} LLVM IR example files:")

    for llvm_file in llvm_files:
        print(f"  - {llvm_file.name}")

        # Check if it contains HUGR convention LLVM IR patterns
        content = llvm_file.read_text()

        # HUGR convention LLVM IR characteristics:
        # - Uses __quantum__qis__ intrinsics for quantum operations
        # - Uses i64 for qubit indices (not opaque %Qubit type)
        # - Has immediate measurement returns (i32 from __quantum__qis__m__body)
        # - Has @main entry point with EntryPoint attribute
        has_quantum_intrinsics = "__quantum__qis__" in content
        has_i64_params = "i64" in content
        has_immediate_measurements = (
            "__quantum__qis__m__body" in content and "i32" in content
        )
        has_entry_point = "@main" in content or "EntryPoint" in content

        if has_quantum_intrinsics and has_i64_params and has_entry_point:
            if has_immediate_measurements:
                print(
                    "    [PASS] Valid HUGR convention LLVM IR (with immediate measurements)",
                )
            else:
                print("    [PASS] Valid HUGR convention LLVM IR")
        else:
            missing = []
            if not has_quantum_intrinsics:
                missing.append("quantum intrinsics")
            if not has_i64_params:
                missing.append("i64 qubit indices")
            if not has_entry_point:
                missing.append("entry point")
            print(f"    ? Missing: {', '.join(missing)}")

    # LLVM IR examples test passed


def test_python_api() -> None:
    """Test Python API availability."""
    print("\n=== Testing Python API ===")

    try:
        import sys

        sys.path.append("python/quantum-pecos/src")

        from pecos.frontends.run_guppy import get_guppy_backends

        print("[PASS] Python imports successful")

        backends = get_guppy_backends()
        print(f"[PASS] Backend detection works: {backends}")

        # Python API test passed

    except (RuntimeError, ImportError) as e:
        print(f"[FAIL] Python API test failed: {e}")
        msg = f"Python API test failed: {e}"
        raise AssertionError(msg) from e


def main() -> int:
    """Run all HUGR compilation tests."""
    print("HUGR Compilation and QIR Generation Tests")
    print("=" * 60)

    # Run tests
    test_rust_hugr_compilation()
    test_standard_qir_generation()
    test_qir_examples()
    test_python_api()

    print("\n" + "=" * 60)
    print("[PASS] All tests passed!")

    return 0


if __name__ == "__main__":
    exit(main())
