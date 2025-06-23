#!/usr/bin/env python3
"""Test the complete Guppy → HUGR → Standard QIR → PECOS pipeline.

This tests the new Standard QIR+ architecture implementation.
"""

import sys
from pathlib import Path

sys.path.append("python/quantum-pecos/src")

from pecos.frontends.guppy_frontend import GuppyFrontend
from pecos.frontends.run_guppy import get_guppy_backends, run_guppy


def test_backend_availability() -> None:
    """Test that backends are properly detected."""
    print("=== Testing Backend Availability ===")
    backends = get_guppy_backends()
    print(f"Available backends: {backends}")

    if backends["guppy_available"]:
        print("[PASS] Guppy is available")
    else:
        print("[FAIL] Guppy is not available - install with: pip install guppylang")

    if backends["rust_backend"]:
        print("[PASS] Rust HUGR backend is available")
    else:
        print(
            f"[FAIL] Rust HUGR backend is not available: {backends.get('rust_message', 'Unknown')}",
        )

    print(f"[OK] External tools available: {backends['external_tools']}")
    print()


def test_guppy_frontend() -> None:
    """Test the GuppyFrontend class directly."""
    print("=== Testing GuppyFrontend ===")

    try:
        frontend = GuppyFrontend(llvm_convention="qir")
        info = frontend.get_backend_info()
        print(f"Frontend backend info: {info}")
        print(f"[OK] Using backend: {info['backend']}")
        print(f"[OK] LLVM convention: {info['llvm_convention']}")
        print()
        # Test passed
    except (ImportError, RuntimeError) as e:
        print(f"[FAIL] Failed to create GuppyFrontend: {e}")
        msg = f"Failed to create GuppyFrontend: {e}"
        raise AssertionError(msg) from e


def test_simple_guppy_function() -> None:
    """Test with a simple Guppy function (if available)."""
    print("=== Testing Simple Guppy Function ===")

    try:
        from guppylang import guppy
        from guppylang.std.quantum import h, measure, qubit

        @guppy
        def random_bit() -> bool:
            """Generate a random bit using quantum superposition."""
            q = qubit()
            h(q)
            return measure(q)

        print("[PASS] Guppy function defined successfully")

        # Test compilation only (not execution)
        try:
            frontend = GuppyFrontend()
            qir_file = frontend.compile_function(random_bit)
            print(f"[PASS] Compiled to QIR: {qir_file}")

            # Read and display part of the QIR
            with Path(qir_file).open() as f:
                qir_content = f.read()
                print("\nGenerated QIR (first 500 chars):")
                print(qir_content[:500])
                print("...")

        except (RuntimeError, FileNotFoundError) as e:
            print(f"[FAIL] Compilation failed: {e}")

    except ImportError:
        print("[SKIP] Guppy not available - skipping function test")
        print("  Install with: pip install guppylang")
    except RuntimeError as e:
        print(f"[FAIL] Test failed: {e}")


def test_bell_state_function() -> None:
    """Test with a Bell state function (if Guppy available)."""
    print("\n=== Testing Bell State Function ===")

    try:
        from guppylang import guppy
        from guppylang.std.quantum import cx, h, measure, qubit

        @guppy
        def bell_state() -> tuple[bool, bool]:
            """Create a Bell state and measure both qubits."""
            q0, q1 = qubit(), qubit()
            h(q0)
            cx(q0, q1)
            return measure(q0), measure(q1)

        print("[PASS] Bell state function defined")

        try:
            # Test using run_guppy API
            result = run_guppy(bell_state, shots=10, verbose=True)
            print("\n[PASS] Execution completed!")
            print(f"  Function: {result['function_name']}")
            print(f"  Backend used: {result['backend_used']}")
            print(f"  Results (first 10): {result['results'][:10]}")
            print(f"  QIR file: {result['qir_file']}")

            # Check correlation
            if result["results"]:
                correlated = sum(1 for (a, b) in result["results"] if a == b)
                print(
                    f"  Correlation: {correlated}/{len(result['results'])} = {correlated/len(result['results']):.2%}",
                )

        except (RuntimeError, ImportError) as e:
            print(f"[FAIL] Execution failed: {e}")
            import traceback

            traceback.print_exc()

    except ImportError:
        print("[SKIP] Guppy not available - skipping Bell state test")
    except RuntimeError as e:
        print(f"[FAIL] Test failed: {e}")


def test_rust_compilation() -> None:
    """Test Rust compilation status."""
    print("\n=== Testing Rust Compilation ===")

    import subprocess

    try:
        # Check if pecos-qir compiled with hugr
        result = subprocess.run(  # noqa: S603
            ["cargo", "check", "-p", "pecos-qir", "--features", "hugr"],  # noqa: S607
            capture_output=True,
            text=True,
            cwd=Path(__file__).resolve().parent,
            check=False,
        )

        if result.returncode == 0:
            print("[PASS] pecos-qir compiles with hugr feature")
        else:
            print("[FAIL] pecos-qir compilation failed:")
            print(result.stderr[:500])

    except (subprocess.SubprocessError, FileNotFoundError) as e:
        print(f"[FAIL] Could not check Rust compilation: {e}")


def main() -> None:
    """Run all tests."""
    print("Testing Guppy → HUGR → Standard QIR → PECOS Pipeline")
    print("=" * 60)

    test_backend_availability()

    test_guppy_frontend()
    test_simple_guppy_function()
    test_bell_state_function()

    test_rust_compilation()

    print("\n" + "=" * 60)
    print("Testing complete!")


if __name__ == "__main__":
    main()
