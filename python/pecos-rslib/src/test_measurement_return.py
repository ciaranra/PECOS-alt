"""Test that measurement results are returned correctly from qmain."""

from guppylang import guppy
from guppylang.std.quantum import qubit, h, measure
import pecos_rslib


def test_single_measurement_return():
    """Test that a single measurement is returned correctly."""

    @guppy
    def single_hadamard() -> bool:
        q = qubit()
        h(q)
        return measure(q)

    hugr = single_hadamard.compile()
    llvm_ir = pecos_rslib.compile_hugr_to_llvm_rust(hugr.to_json().encode())

    # Check that qmain returns i32
    assert "define i32 @qmain" in llvm_ir, "qmain should return i32"

    # Check that we return the measurement result
    lines = llvm_ir.split("\n")
    for i, line in enumerate(lines):
        if "ret i32" in line:
            # Get the returned variable
            ret_var = line.strip().split()[-1]
            # Find its definition
            for j in range(i - 1, max(0, i - 10), -1):
                if (
                    ret_var in lines[j]
                    and "trunc" in lines[j]
                    and "lazy_measure" in lines[j]
                ):
                    print(
                        f"✓ Correctly returning truncated measurement: {lines[j].strip()}"
                    )
                    return True

    raise AssertionError("qmain doesn't return the measurement result")


if __name__ == "__main__":
    test_single_measurement_return()
    print("✓ Test passed: Single measurement is returned correctly")
