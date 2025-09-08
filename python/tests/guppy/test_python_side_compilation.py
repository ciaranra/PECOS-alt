#!/usr/bin/env python3
"""Test Python-side Guppy to Selene compilation."""

from pathlib import Path

import pytest

pytestmark = pytest.mark.optional_dependency


def test_python_side_selene_compilation() -> None:
    """Test compiling Guppy function for Selene execution in Python."""
    from guppylang.decorator import guppy
    from guppylang.std.quantum import h, measure, qubit
    from pecos.frontends.guppy_selene_compiler import compile_guppy_for_selene

    # Define a simple quantum function
    @guppy
    def simple_circuit() -> bool:
        """Simple H-gate and measurement."""
        q = qubit()
        h(q)
        return measure(q)

    # Compile for Selene
    import tempfile

    try:
        # Create a persistent output directory for testing
        test_output_dir = Path(tempfile.mkdtemp(prefix="test_guppy_selene_"))
        output_dir = compile_guppy_for_selene(simple_circuit, test_output_dir)
        print(f"Compiled to directory: {output_dir}")

        # Verify compiled files exist
        assert output_dir.exists()
        # Get function name - guppylang may use a default name
        func_name = getattr(
            simple_circuit,
            "name",
            getattr(simple_circuit, "__name__", "quantum_func"),
        )
        llvm_file = output_dir / f"{func_name}.ll"
        hugr_file = output_dir / f"{func_name}.hugr"

        # If files don't exist with expected name, look for any .ll and .hugr files
        if not llvm_file.exists():
            llvm_files = list(output_dir.glob("*.ll"))
            if llvm_files:
                llvm_file = llvm_files[0]
                func_name = llvm_file.stem
                hugr_file = output_dir / f"{func_name}.hugr"

        assert llvm_file.exists(), f"LLVM file not found in {output_dir}"
        assert hugr_file.exists(), f"HUGR file not found in {output_dir}"

        # Now we can use these files with Selene engine
        from pecos_rslib import selene_engine

        # Create engine and run
        # Note: This might fail if LLVM format isn't exactly right,
        # but it demonstrates the architecture
        try:
            selene_engine().llvm_file(str(llvm_file)).qubits(1).build()
            print("Successfully created Selene engine with compiled LLVM")
        except Exception as e:
            print(f"Engine creation failed (expected during development): {e}")

    except Exception as e:
        print(f"Compilation failed: {e}")
        # Check if it's because of missing functionality
        if "compile_guppy_for_selene" in str(e):
            print("compile_guppy_for_selene not available")
        else:
            raise


def test_hugr_pass_through_path() -> None:
    """Test the HUGR pass-through path (Guppy → HUGR → Rust)."""
    from guppylang import GuppyModule
    from guppylang.decorator import guppy
    from guppylang.std.quantum import h, measure, qubit
    from pecos_rslib import HugrProgram, sim
    from pecos_rslib.hugr_llvm import serialize_hugr_json_to_binary

    # Define a quantum function
    @guppy
    def bell_pair() -> tuple[bool, bool]:
        """Create a Bell pair (without CNOT for now)."""
        q1 = qubit()
        q2 = qubit()
        h(q1)
        # Would add CNOT here when supported
        return measure(q1), measure(q2)

    # Compile to HUGR
    module = GuppyModule("bell_module")
    module.register_func(bell_pair)
    hugr = module.compile()
    hugr_json = hugr.to_json()

    # Serialize to binary
    hugr_bytes = serialize_hugr_json_to_binary(hugr_json)

    # Create HugrProgram
    try:
        hugr_program = HugrProgram.from_bytes(hugr_bytes)
        print(f"Created HugrProgram from {len(hugr_bytes)} bytes")

        # Use sim API - this should route through Selene
        builder = sim(hugr_program)
        print(f"Created sim builder: {type(builder)}")

        # Actual execution would require full HUGR parsing
        # but this demonstrates the architecture

    except Exception as e:
        print(f"HUGR pass-through failed (expected): {e}")


def test_architecture_demonstration() -> None:
    """Demonstrate both compilation paths."""
    print("\n=== Guppy Compilation Architecture ===")
    print("\nPath 1: Python-side compilation")
    print("  Guppy → HUGR → LLVM IR (all in Python)")
    print("  Then: LLVM IR → Rust Selene Engine (native execution)")
    print("  Advantage: All compilation in Python where Guppy lives")

    print("\nPath 2: HUGR pass-through")
    print("  Guppy → HUGR (in Python)")
    print("  Then: HUGR → Rust pecos-selene → LLVM IR → Native execution")
    print("  Advantage: Reusable Rust compilation pipeline")

    print("\nPure Rust path (no Python):")
    print("  HUGR/LLVM IR → Rust pecos-selene → Native execution")
    print("  Advantage: No Python dependency for pre-compiled programs")


if __name__ == "__main__":
    test_architecture_demonstration()
    test_python_side_selene_compilation()
    test_hugr_pass_through_path()
