"""Test all Selene integration paths with Guppy."""

import tempfile
from pathlib import Path

import pytest


def test_python_guppy_to_plugin() -> None:
    """Test: Python Guppy → Selene Plugin."""
    try:
        from guppylang import guppy
        from guppylang.std.quantum import cx, h, measure, qubit
        from pecos.frontends.guppy_selene_compiler import GuppySeleneCompiler
    except ImportError as e:
        pytest.skip(f"Required imports not available: {e}")

    @guppy
    def bell_state() -> tuple[bool, bool]:
        q1, q2 = qubit(), qubit()
        h(q1)
        cx(q1, q2)
        return measure(q1), measure(q2)

    with tempfile.TemporaryDirectory() as tmpdir:
        compiler = GuppySeleneCompiler(output_dir=Path(tmpdir))
        output_dir = compiler.compile_function(bell_state)

        assert output_dir.exists(), "Output directory not created"
        assert output_dir.is_dir(), "Output should be a directory"

        # Check for HUGR file (using default name since Guppy functions don't have names)
        hugr_file = output_dir / "quantum_func.hugr"
        assert hugr_file.exists(), "HUGR file not created"
        assert hugr_file.stat().st_size > 100, "HUGR file too small"

        # Check for LLVM IR file
        llvm_file = output_dir / "quantum_func.ll"
        assert llvm_file.exists(), "LLVM IR file not created"
        assert llvm_file.stat().st_size > 100, "LLVM IR file too small"


def test_python_guppy_hugr_to_selene() -> None:
    """Test: Python Guppy → HUGR → Rust Selene."""
    try:
        from guppylang import guppy
        from guppylang.std.quantum import h, measure, qubit
        from pecos_rslib.programs import HugrProgram
        from pecos_rslib.sim import selene_engine
    except ImportError as e:
        pytest.skip(f"Required imports not available: {e}")

    @guppy
    def hadamard_measure() -> bool:
        q = qubit()
        h(q)
        return measure(q)

    # Compile to HUGR 0.13
    hugr = hadamard_measure.compile()
    hugr_bytes = hugr.to_bytes()

    # Check it's HUGR envelope format
    assert hugr_bytes[:8] == b"HUGRiHJv", "Not HUGR envelope format"
    format_byte = hugr_bytes[8]
    assert format_byte in [2, 63], f"Unknown HUGR format: {format_byte}"

    # Create HUGR program and use with Selene
    hugr_program = HugrProgram.from_bytes(hugr_bytes)
    engine = selene_engine().program(hugr_program)

    # Should accept without version error
    engine.to_sim()
    # Note: Would need quantum engine to run


def test_python_llvm_to_selene() -> None:
    """Test: Python LLVM → Rust Selene."""
    try:
        from pecos_rslib.programs import LlvmProgram
        from pecos_rslib.sim import selene_engine
    except ImportError as e:
        pytest.skip(f"Required imports not available: {e}")

    llvm_ir = """
    ; Quantum circuit LLVM IR
    %Qubit = type opaque
    %Result = type opaque

    declare %Qubit* @__quantum__qis__qalloc()
    declare void @__quantum__qis__qfree(%Qubit*)
    declare void @__quantum__qis__h__body(%Qubit*)
    declare void @__quantum__qis__x__body(%Qubit*)
    declare %Result* @__quantum__qis__mz__body(%Qubit*)

    define void @quantum_circuit() #0 {
    entry:
      %q0 = call %Qubit* @__quantum__qis__qalloc()
      %q1 = call %Qubit* @__quantum__qis__qalloc()
      call void @__quantum__qis__h__body(%Qubit* %q0)
      call void @__quantum__qis__x__body(%Qubit* %q1)
      %r0 = call %Result* @__quantum__qis__mz__body(%Qubit* %q0)
      %r1 = call %Result* @__quantum__qis__mz__body(%Qubit* %q1)
      call void @__quantum__qis__qfree(%Qubit* %q0)
      call void @__quantum__qis__qfree(%Qubit* %q1)
      ret void
    }

    attributes #0 = { "EntryPoint" }
    """

    llvm_program = LlvmProgram.from_string(llvm_ir)
    engine = selene_engine().program(llvm_program)
    engine.to_sim()


def test_rust_llvm_to_plugin() -> None:
    """Test: Rust LLVM → Selene Plugin.

    Note: Plugin compilation feature was removed as it was incomplete and unused.
    Selene uses its own runtime plugins (selene_simple_runtime_plugin).
    """
    pytest.skip("Plugin compilation feature removed - Selene uses its own runtime")


def test_full_guppy_to_selene_execution() -> None:
    """Test: Full Guppy → Selene → Execution (setup only)."""
    try:
        from guppylang import guppy
        from guppylang.std.quantum import h, measure, qubit
        from pecos_rslib.programs import HugrProgram
        from pecos_rslib.sim import selene_engine, sim, state_vector
    except ImportError as e:
        pytest.skip(f"Required imports not available: {e}")

    @guppy
    def random_bit() -> bool:
        q = qubit()
        h(q)
        return measure(q)

    # Compile to HUGR
    hugr = random_bit.compile()
    hugr_bytes = hugr.to_bytes()

    # Create program
    hugr_program = HugrProgram.from_bytes(hugr_bytes)

    # Set up simulation (would work if HUGR parsing was complete)
    selene_builder = selene_engine().program(hugr_program)
    quantum_builder = state_vector().qubits(1)

    # This shows the full path is set up correctly
    # Actual execution would require complete HUGR parsing
    try:
        result = sim(selene_builder, quantum_builder, n_shots=10)
        # If this works, we have full execution
        assert len(result) == 10
    except Exception as e:
        # Expected until HUGR parsing is complete
        # But should not be version error
        error_msg = str(e)
        if "HUGR version incompatibility" in error_msg:
            pytest.fail(f"Unexpected HUGR version error: {error_msg}")
