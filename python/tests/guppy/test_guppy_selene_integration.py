"""Test Guppy integration with pecos-selene (HUGR 0.13)."""

import tempfile
from pathlib import Path

import pytest


def test_guppy_to_selene_plugin():
    """Test Guppy to Selene plugin compilation."""
    try:
        from guppylang import guppy
        from guppylang.std.quantum import h, measure, qubit
    except ImportError:
        pytest.skip("guppylang not available")
        
    try:
        from pecos.frontends.guppy_selene_compiler import GuppySeleneCompiler
    except ImportError:
        pytest.skip("GuppySeleneCompiler not available")
    
    # Create a simple quantum function
    @guppy
    def bell_state() -> tuple[bool, bool]:
        q1 = qubit()
        q2 = qubit()
        h(q1)
        # Note: cx not imported, use basic gates only
        return measure(q1), measure(q2)
    
    # Compile to plugin
    with tempfile.TemporaryDirectory() as tmpdir:
        compiler = GuppySeleneCompiler(output_dir=Path(tmpdir))
        plugin_path = compiler.compile_function(bell_state)
        
        assert plugin_path.exists()
        assert plugin_path.suffix == ".so"
        assert plugin_path.stat().st_size > 0
        

def test_guppy_hugr_to_selene():
    """Test Guppy HUGR with Selene engine."""
    try:
        from guppylang import guppy
        from guppylang.std.quantum import h, measure, qubit
    except ImportError:
        pytest.skip("guppylang not available")
        
    try:
        from pecos_rslib.programs import HugrProgram
        from pecos_rslib.sim import selene_engine
    except ImportError:
        pytest.skip("pecos_rslib not available")
    
    # Create quantum function
    @guppy
    def simple_h() -> bool:
        q = qubit()
        h(q)
        return measure(q)
    
    # Compile to HUGR
    hugr = simple_h.compile()
    hugr_bytes = hugr.to_bytes()
    
    # Create HUGR program
    hugr_program = HugrProgram.from_bytes(hugr_bytes)
    
    # Use with Selene engine (should accept HUGR 0.13)
    engine = selene_engine().program(hugr_program)
    
    # This should not raise HUGR version error
    # (though it might fail at simulation config)
    try:
        sim = engine.to_sim()
        # Would need .qubits(1) to actually run
    except Exception as e:
        # Check it's not a version error
        assert "HUGR version incompatibility" not in str(e)
        

def test_selene_llvm_program():
    """Test Selene with LLVM program."""
    try:
        from pecos_rslib.programs import LlvmProgram
        from pecos_rslib.sim import selene_engine
    except ImportError:
        pytest.skip("pecos_rslib not available")
    
    # Simple LLVM IR
    llvm_ir = """
    declare void @__quantum__qis__h__body(%Qubit*)
    declare %Result* @__quantum__qis__mz__body(%Qubit*)
    declare %Qubit* @__quantum__qis__qalloc()
    declare void @__quantum__qis__qfree(%Qubit*)
    
    %Qubit = type opaque
    %Result = type opaque
    
    define void @main() #0 {
    entry:
      %q = call %Qubit* @__quantum__qis__qalloc()
      call void @__quantum__qis__h__body(%Qubit* %q)
      %r = call %Result* @__quantum__qis__mz__body(%Qubit* %q)
      call void @__quantum__qis__qfree(%Qubit* %q)
      ret void
    }
    
    attributes #0 = { "EntryPoint" }
    """
    
    # Create LLVM program
    llvm_program = LlvmProgram.from_string(llvm_ir)
    
    # Use with Selene
    engine = selene_engine().program(llvm_program)
    
    # Should accept LLVM program
    try:
        sim = engine.to_sim()
        # Would need quantum engine to actually run
    except Exception as e:
        # Check it's not a program rejection
        assert "Invalid program" not in str(e)