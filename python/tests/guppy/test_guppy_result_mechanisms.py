"""Test different ways Guppy programs can output results and how they appear in HUGR/LLVM.

This test explores:
1. Using result() function with string labels
2. Direct returns from functions
3. How these compile to HUGR and LLVM
4. What we should expect in Selene's result stream
"""

import json
import tempfile
from pathlib import Path
from typing import Any

from guppylang import guppy
from guppylang.std.builtins import result  # The result function is here!
from guppylang.std.quantum import cx, h, measure, qubit
from pecos.compilation_pipeline import compile_guppy_to_hugr, compile_hugr_to_llvm


def test_result_function_vs_return() -> None:
    """Compare programs using result() vs return statements."""
    print("=" * 60)
    print("TESTING RESULT() VS RETURN MECHANISMS")
    print("=" * 60)

    # Program 1: Using result() to tag outputs
    @guppy
    def bell_with_result_tags() -> None:
        """Bell state using result() to tag measurements."""
        q0, q1 = qubit(), qubit()
        h(q0)
        cx(q0, q1)

        m0 = measure(q0)
        m1 = measure(q1)

        # Tag individual results
        result("alice_measurement", m0)
        result("bob_measurement", m1)
        result("correlation", m0 == m1)

    # Program 2: Using return statement
    @guppy
    def bell_with_return() -> tuple[bool, bool]:
        """Bell state returning measurements."""
        q0, q1 = qubit(), qubit()
        h(q0)
        cx(q0, q1)

        return measure(q0), measure(q1)

    # Program 3: Mix of both
    @guppy
    def bell_mixed_output() -> bool:
        """Bell state with both result() and return."""
        q0, q1 = qubit(), qubit()
        h(q0)
        cx(q0, q1)

        m0 = measure(q0)
        m1 = measure(q1)

        # Tag one result
        result("alice", m0)

        # Return the other
        return m1

    programs = [
        ("bell_with_result_tags", bell_with_result_tags),
        ("bell_with_return", bell_with_return),
        ("bell_mixed_output", bell_mixed_output),
    ]

    for name, prog in programs:
        print(f"\n{'='*40}")
        print(f"Program: {name}")
        print(f"{'='*40}")
        analyze_program(name, prog)


def analyze_program(name: str, program: Any) -> None:
    """Analyze a Guppy program through compilation stages."""
    # Step 1: Compile to HUGR
    hugr_bytes = compile_guppy_to_hugr(program)
    print(f"\n1. HUGR: {len(hugr_bytes)} bytes")

    hugr_json = json.loads(hugr_bytes.decode("utf-8"))

    # Look for interesting operations in HUGR
    print("\n2. HUGR Operations Analysis:")
    analyze_hugr_ops(hugr_json)

    # Step 2: Compile to LLVM
    try:
        llvm_ir = compile_hugr_to_llvm(hugr_bytes)
        print(f"\n3. LLVM IR: {len(llvm_ir)} bytes")

        # Analyze LLVM for result calls
        print("\n4. LLVM Result Calls:")
        analyze_llvm_results(llvm_ir)

        # Save for inspection
        with tempfile.TemporaryDirectory() as tmpdir:
            llvm_file = Path(tmpdir) / f"{name}.ll"
            llvm_file.write_text(llvm_ir)
            print(f"\n5. Saved to: {llvm_file}")

    except Exception as e:
        print(f"\n3. LLVM compilation failed: {e}")


def analyze_hugr_ops(hugr: dict) -> None:
    """Find result/output operations in HUGR."""
    result_ops = []
    output_ops = []
    io_ops = []

    def search(obj, path="") -> None:
        if isinstance(obj, dict):
            if "op" in obj:
                op = str(obj["op"])
                # Check for different types of operations
                if "result" in op.lower():
                    result_ops.append((path, op))
                elif "output" in op.lower():
                    output_ops.append((path, op))
                elif "io" in op.lower() or "print" in op.lower():
                    io_ops.append((path, op))

                # Also check for Extension operations that might be I/O
                if op == "Extension" and "extension" in obj:
                    ext = obj["extension"]
                    if any(
                        term in str(ext).lower() for term in ["io", "result", "print"]
                    ):
                        io_ops.append((path, f"Extension: {ext}"))

            for key, value in obj.items():
                search(value, f"{path}.{key}" if path else key)
        elif isinstance(obj, list):
            for i, item in enumerate(obj):
                search(item, f"{path}[{i}]")

    search(hugr)

    print(f"  Result operations: {len(result_ops)}")
    for path, op in result_ops[:3]:  # Show first 3
        print(f"    - {path}: {op}")

    print(f"  Output operations: {len(output_ops)}")
    for path, op in output_ops[:3]:
        print(f"    - {path}: {op}")

    print(f"  I/O operations: {len(io_ops)}")
    for path, op in io_ops[:3]:
        print(f"    - {path}: {op}")


def analyze_llvm_results(llvm_ir: str) -> None:
    """Find result recording calls in LLVM IR."""
    lines = llvm_ir.split("\n")

    # Look for different result patterns
    result_patterns = [
        "__quantum__rt__result_record",
        "__quantum__rt__tuple_record",
        "__quantum__rt__string_record",
        "__quantum__rt__bool_record",
        "__quantum__rt__integer_record",
        "result",
        "print",
        "@output",
    ]

    found_calls = {}
    for pattern in result_patterns:
        found_calls[pattern] = []
        for i, line in enumerate(lines):
            if pattern in line:
                found_calls[pattern].append((i, line.strip()))

    for pattern, calls in found_calls.items():
        if calls:
            print(f"  {pattern}: {len(calls)} calls")
            for line_no, line in calls[:2]:  # Show first 2
                print(f"    Line {line_no}: {line[:80]}...")


def test_expected_selene_output() -> None:
    """Document what we expect Selene to output for each case."""
    print("\n" + "=" * 60)
    print("EXPECTED SELENE RESULT STREAM OUTPUT")
    print("=" * 60)

    print(
        """
For bell_with_result_tags():
    Expected in result stream:
    - ("USER:BOOL:alice_measurement", True/False)
    - ("USER:BOOL:bob_measurement", True/False)
    - ("USER:BOOL:correlation", True)

    After parsing by Selene:
    - ("alice_measurement", True/False)
    - ("bob_measurement", True/False)
    - ("correlation", True)

For bell_with_return():
    Expected in result stream:
    - ("USER:TUPLE:result", (True/False, True/False))
    OR
    - ("USER:BOOL:result_0", True/False)
    - ("USER:BOOL:result_1", True/False)

    After parsing:
    - ("result", (True/False, True/False))
    OR
    - ("result_0", True/False)
    - ("result_1", True/False)

For bell_mixed_output():
    Expected in result stream:
    - ("USER:BOOL:alice", True/False)  # From result()
    - ("USER:BOOL:result", True/False)  # From return

    After parsing:
    - ("alice", True/False)
    - ("result", True/False)
    """,
    )


def test_simple_result_examples() -> None:
    """Test simpler examples to understand the pattern."""
    print("\n" + "=" * 60)
    print("SIMPLE RESULT EXAMPLES")
    print("=" * 60)

    # Simplest case: just a result call
    @guppy
    def just_result() -> None:
        """Just call result with a constant."""
        result("test_value", 42)

    # Simple measurement with result
    @guppy
    def measure_and_result() -> None:
        """Measure and use result()."""
        q = qubit()
        h(q)
        m = measure(q)
        result("measurement", m)

    # Multiple results
    @guppy
    def multiple_results() -> None:
        """Multiple result calls."""
        result("first", 1)
        result("second", 2.5)
        result("third", True)

    simple_programs = [
        ("just_result", just_result),
        ("measure_and_result", measure_and_result),
        ("multiple_results", multiple_results),
    ]

    for name, prog in simple_programs:
        print(f"\nProgram: {name}")
        try:
            hugr_bytes = compile_guppy_to_hugr(prog)
            print(f"  HUGR size: {len(hugr_bytes)} bytes")

            # Try LLVM compilation
            try:
                llvm_ir = compile_hugr_to_llvm(hugr_bytes)

                # Look specifically for result calls
                if "__quantum__rt__" in llvm_ir:
                    print("  LLVM has __quantum__rt__ calls ✓")
                else:
                    print("  LLVM missing __quantum__rt__ calls ✗")

                # Check what the entry point looks like
                for line in llvm_ir.split("\n"):
                    if "define" in line and "void @" in line:
                        print(f"  Entry point: {line.strip()[:60]}...")
                        break

            except Exception as e:
                print(f"  LLVM compilation error: {e}")

        except Exception as e:
            print(f"  HUGR compilation error: {e}")


if __name__ == "__main__":
    print("GUPPY RESULT MECHANISM ANALYSIS")
    print("=" * 60)

    # First check that result is available
    print(f"result function available: {result}")
    print(
        f"result docstring: {result.__doc__ if hasattr(result, '__doc__') else 'No docs'}",
    )

    # Run tests
    test_simple_result_examples()
    test_result_function_vs_return()
    test_expected_selene_output()
