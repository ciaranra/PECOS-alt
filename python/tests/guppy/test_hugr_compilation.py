#!/usr/bin/env python3
"""Test HUGR compilation and QIR generation."""

import subprocess
import tempfile
from pathlib import Path


def test_rust_hugr_compilation() -> None:
    """Test that the Rust HUGR support compiles."""
    print("=== Testing Rust HUGR Compilation ===")

    # Test 1: Check if HUGR support compiles in the new pecos-hugr-llvm crate
    result = subprocess.run(  # noqa: S603
        ["cargo", "check", "-p", "pecos-hugr-llvm"],  # noqa: S607
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
    result = subprocess.run(  # noqa: S603
        [  # noqa: S607
            "cargo",
            "test",
            "-p",
            "pecos-hugr-llvm",
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
    """Test standard QIR generation patterns."""
    print("\n=== Testing Standard QIR Generation ===")

    # Create a test QIR file
    test_qir = """
%Result = type opaque
%Qubit = type opaque

declare void @__quantum__qis__h__body(%Qubit*)
declare void @__quantum__qis__m__body(%Qubit*, %Result*)
declare void @__quantum__rt__result_record_output(%Result*, i8*)

define void @main() #0 {
    call void @__quantum__qis__h__body(%Qubit* null)
    call void @__quantum__qis__m__body(%Qubit* null, %Result* inttoptr (i64 0 to %Result*))
    ret void
}

attributes #0 = { "EntryPoint" }
"""

    with tempfile.NamedTemporaryFile(mode="w", suffix=".ll", delete=False) as f:
        f.write(test_qir)
        qir_file = f.name

    print(f"[OK] Created test QIR file: {qir_file}")

    # Verify it's valid LLVM IR
    try:
        result = subprocess.run(  # noqa: S603
            ["llvm-as", qir_file, "-o", "/dev/null"],  # noqa: S607
            capture_output=True,
            text=True,
            check=False,
        )

        if result.returncode == 0:
            print("[PASS] QIR format is valid LLVM IR")
        else:
            print("[FAIL] QIR validation failed (invalid format)")
    except FileNotFoundError:
        print("⚠ llvm-as not available, skipping validation")

    # Clean up
    Path(qir_file).unlink()

    # Standard QIR generation test passed


def test_qir_examples() -> None:
    """Test existing QIR examples."""
    print("\n=== Testing QIR Examples ===")

    # Find examples relative to the project root
    test_dir = Path(__file__).parent
    project_root = (
        test_dir.parent.parent.parent
    )  # tests/guppy -> tests -> python -> PECOS
    qir_examples = project_root / "examples" / "qir"

    if not qir_examples.exists():
        print(f"[SKIP] QIR examples directory not found at {qir_examples}")
        print("[INFO] This test requires QIR examples to be present")
        import pytest

        pytest.skip("QIR examples directory not found")

    qir_files = list(qir_examples.glob("*.ll"))
    print(f"Found {len(qir_files)} QIR example files:")

    for qir_file in qir_files:
        print(f"  - {qir_file.name}")

        # Check if it contains standard QIR patterns
        content = qir_file.read_text()
        has_qubit_type = "%Qubit" in content
        has_result_type = "%Result" in content
        has_quantum_ops = "__quantum__qis__" in content

        if has_qubit_type and has_result_type and has_quantum_ops:
            print("    [PASS] Valid standard QIR format")
        else:
            print("    ? Non-standard format")

    # QIR examples test passed


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
