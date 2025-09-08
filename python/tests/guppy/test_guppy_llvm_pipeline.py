#!/usr/bin/env python3
"""Test the complete Guppy → HUGR → Standard QIR → PECOS pipeline.

This tests the new Standard QIR+ architecture implementation.
"""

import sys
from pathlib import Path


def decode_integer_results(results: list[int], n_bits: int) -> list[tuple[bool, ...]]:
    """Decode integer-encoded results back to tuples of booleans."""
    decoded = []
    for val in results:
        bits = []
        for i in range(n_bits):
            bits.append(bool(val & (1 << i)))
        decoded.append(tuple(bits))
    return decoded


sys.path.append("python/quantum-pecos/src")

from pecos.frontends import get_guppy_backends, sim
from pecos.frontends.guppy_frontend import GuppyFrontend
from pecos_rslib import state_vector


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

    # External tools are no longer tracked - only Rust backend is used
    print("[OK] Using Rust backend for compilation")
    print()


def test_guppy_frontend() -> None:
    """Test the GuppyFrontend class directly."""
    print("=== Testing GuppyFrontend ===")

    try:
        frontend = GuppyFrontend()
        info = frontend.get_backend_info()
        print(f"Frontend backend info: {info}")
        print(f"[OK] Using backend: {info['backend']}")
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
            # Test using sim() API
            result = sim(bell_state).qubits(10).quantum(state_vector()).run(10)
            print("\n[PASS] Execution completed!")
            print("  Function: bell_state")
            print("  Backend: Unified sim() API with state_vector")
            measurements = result.get(
                "measurements",
                result.get("measurement_1", result.get("result", [])),
            )
            print(f"  Results (first 10): {measurements[:10]}")

            # Check correlation
            if measurements:
                # For Bell state, check if measurements are correlated
                if "measurement_1" in result and "measurement_2" in result:
                    # Tuple returns
                    correlated = sum(
                        1
                        for i in range(len(result["measurement_1"]))
                        if result["measurement_1"][i] == result["measurement_2"][i]
                    )
                    total = len(result["measurement_1"])
                    print(
                        f"  Correlation: {correlated}/{total} = {correlated/total:.2%}",
                    )
                elif isinstance(measurements[0], tuple):
                    # Tuple format
                    correlated = sum(1 for m in measurements if m[0] == m[1])
                    print(
                        f"  Correlation: {correlated}/{len(measurements)} = {correlated/len(measurements):.2%}",
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
        # Check if pecos-llvm-runtime compiled with hugr
        result = subprocess.run(
            [
                "cargo",
                "check",
                "-p",
                "pecos-llvm-runtime",
                "--features",
                "hugr",
            ],
            capture_output=True,
            text=True,
            cwd=Path(__file__).resolve().parent,
            check=False,
        )

        if result.returncode == 0:
            print("[PASS] pecos-llvm-runtime compiles with hugr feature")
        else:
            print("[FAIL] pecos-llvm-runtime compilation failed:")
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
