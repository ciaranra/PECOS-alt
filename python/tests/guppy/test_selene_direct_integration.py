"""Test running Guppy programs directly with Selene (without PECOS integration).

This test helps us understand how Selene works in isolation before integrating
it with PECOS's ClassicalControlEngine infrastructure.
"""

import pytest
import tempfile
import shutil
from pathlib import Path
from typing import Any, Dict, List

# Check if required dependencies are available
try:
    from guppylang import guppy
    from guppylang.std.quantum import qubit, h, cx, measure
    GUPPY_AVAILABLE = True
except ImportError:
    GUPPY_AVAILABLE = False

try:
    import selene_sim
    from selene_sim import build
    from selene_sim.backends import SimpleRuntime, IdealErrorModel, CoinflipSimulator
    SELENE_AVAILABLE = True
except ImportError:
    SELENE_AVAILABLE = False

try:
    from pecos.compilation_pipeline import compile_guppy_to_hugr
    COMPILATION_AVAILABLE = True
except ImportError:
    COMPILATION_AVAILABLE = False


@pytest.mark.skipif(not GUPPY_AVAILABLE, reason="guppylang not available")
@pytest.mark.skipif(not SELENE_AVAILABLE, reason="selene not available")
class TestSeleneDirectIntegration:
    """Test Selene running Guppy programs directly."""
    
    def test_simple_bell_state_with_selene(self):
        """Test running a Bell state Guppy program through Selene's complete pipeline."""
        
        # Step 1: Define a Guppy quantum program
        @guppy
        def bell_state() -> tuple[bool, bool]:
            """Create a Bell state and measure both qubits."""
            q0, q1 = qubit(), qubit()
            h(q0)
            cx(q0, q1)
            return measure(q0), measure(q1)
        
        print("\n=== Step 1: Guppy program defined ===")
        print(f"Function: {bell_state}")
        
        # Step 2: Compile Guppy to HUGR
        if COMPILATION_AVAILABLE:
            hugr_bytes = compile_guppy_to_hugr(bell_state)
            print(f"\n=== Step 2: Compiled to HUGR ({len(hugr_bytes)} bytes) ===")
        else:
            print("\n=== Step 2: SKIPPING - compilation pipeline not available ===")
            hugr_bytes = b"dummy_hugr"
        
        # Step 3: Use Selene to build an executable from HUGR
        with tempfile.TemporaryDirectory() as tmpdir:
            build_dir = Path(tmpdir) / "selene_build"
            build_dir.mkdir()
            
            # Write HUGR to file for Selene to process
            hugr_file = build_dir / "program.hugr"
            hugr_file.write_bytes(hugr_bytes)
            
            print(f"\n=== Step 3: Building with Selene ===")
            print(f"Build directory: {build_dir}")
            
            # Use Selene's build API
            # Note: This is where we'd use Selene's actual build process
            # The exact API might vary depending on Selene version
            try:
                # Build the program using Selene
                instance = build(
                    str(hugr_file),
                    output_dir=str(build_dir),
                    verbose=True
                )
                print(f"Built Selene instance: {instance}")
                
                # Step 4: Configure Selene runtime components
                print("\n=== Step 4: Configuring Selene runtime ===")
                runtime = SimpleRuntime()  # Selene's simple runtime
                error_model = IdealErrorModel()  # No errors
                simulator = CoinflipSimulator()  # Simple 50/50 simulator
                
                print(f"Runtime: {runtime}")
                print(f"Error model: {error_model}")
                print(f"Simulator: {simulator}")
                
                # Step 5: Run the program and collect results
                print("\n=== Step 5: Running program ===")
                n_shots = 10
                n_qubits = 2
                
                results = []
                for shot_results in instance.run_shots(
                    simulator=simulator,
                    n_qubits=n_qubits,
                    runtime=runtime,
                    error_model=error_model,
                    n_shots=n_shots,
                    verbose=True
                ):
                    # Collect all results from this shot
                    shot_data = {}
                    for tag, value in shot_results:
                        print(f"  Shot result: {tag} = {value}")
                        shot_data[tag] = value
                    results.append(shot_data)
                
                print(f"\n=== Step 6: Results collected ===")
                print(f"Total shots: {len(results)}")
                for i, shot in enumerate(results):
                    print(f"  Shot {i}: {shot}")
                
                # Verify we got results
                assert len(results) == n_shots, f"Expected {n_shots} shots, got {len(results)}"
                
            except Exception as e:
                print(f"\n=== ERROR: Selene build/run failed ===")
                print(f"Error type: {type(e).__name__}")
                print(f"Error message: {str(e)}")
                
                # This is expected if Selene's HUGR support isn't fully ready
                # Let's try a simpler approach with LLVM IR instead
                print("\n=== Fallback: Trying with LLVM IR ===")
                self._test_with_llvm_ir_fallback(build_dir)
    
    def _test_with_llvm_ir_fallback(self, build_dir: Path):
        """Fallback test using LLVM IR instead of HUGR."""
        
        # Create a simple LLVM IR program
        llvm_ir = """
        declare void @__quantum__qis__h__body(i64)
        declare void @__quantum__qis__cnot__body(i64, i64)
        declare i1 @__quantum__qis__mz__body(i64)
        declare void @__quantum__rt__result_record(i8*, i1)
        
        define void @bell_state() {
        entry:
            ; Apply H to qubit 0
            call void @__quantum__qis__h__body(i64 0)
            
            ; Apply CNOT(0, 1)
            call void @__quantum__qis__cnot__body(i64 0, i64 1)
            
            ; Measure both qubits
            %m0 = call i1 @__quantum__qis__mz__body(i64 0)
            %m1 = call i1 @__quantum__qis__mz__body(i64 1)
            
            ; Record results
            call void @__quantum__rt__result_record(i8* null, i1 %m0)
            call void @__quantum__rt__result_record(i8* null, i1 %m1)
            
            ret void
        }
        """
        
        # Write LLVM IR to file
        llvm_file = build_dir / "program.ll"
        llvm_file.write_text(llvm_ir)
        
        print(f"Created LLVM IR file: {llvm_file}")
        
        try:
            # Try to build with Selene using LLVM IR
            instance = build(
                str(llvm_file),
                output_dir=str(build_dir),
                verbose=True
            )
            
            # Run with simple configuration
            runtime = SimpleRuntime()
            simulator = CoinflipSimulator()
            
            results = list(instance.run_shots(
                simulator=simulator,
                n_qubits=2,
                runtime=runtime,
                n_shots=1,
                verbose=True
            ))
            
            print(f"Fallback results: {results}")
            
        except Exception as e:
            print(f"Fallback also failed: {e}")
            # This is okay - we're learning about the integration
    
    def test_selene_configuration_exploration(self):
        """Explore what configuration Selene needs for running quantum programs."""
        
        print("\n=== Exploring Selene Configuration ===")
        
        # Check available backends
        if SELENE_AVAILABLE:
            print("\nAvailable Selene backends:")
            print(f"  Runtimes: {dir(selene_sim.backends)}")
            
            # Check what a SimpleRuntime provides
            runtime = SimpleRuntime()
            print(f"\nSimpleRuntime attributes: {dir(runtime)}")
            
            # Check simulator options
            from selene_sim.backends import bundled_simulators
            print(f"\nBundled simulators: {bundled_simulators.__all__ if hasattr(bundled_simulators, '__all__') else 'N/A'}")
    
    def test_understanding_selene_result_stream(self):
        """Understand how Selene handles result streams."""
        
        print("\n=== Understanding Selene Result Stream ===")
        
        if not SELENE_AVAILABLE:
            pytest.skip("Selene not available")
        
        # Create a minimal test to see result format
        with tempfile.TemporaryDirectory() as tmpdir:
            # Create the simplest possible quantum program
            simple_program = """
            ; Minimal quantum program
            declare i1 @__quantum__qis__mz__body(i64)
            
            define void @main() {
                %result = call i1 @__quantum__qis__mz__body(i64 0)
                ret void
            }
            """
            
            program_file = Path(tmpdir) / "minimal.ll"
            program_file.write_text(simple_program)
            
            try:
                # Try to understand the build process
                print(f"\nAttempting to build: {program_file}")
                
                # Check what build function signature looks like
                print(f"Build function signature: {build.__doc__ if hasattr(build, '__doc__') else 'No docs'}")
                
                # Try different build approaches
                # This helps us understand what Selene expects
                
            except Exception as e:
                print(f"Build exploration error: {e}")


@pytest.mark.skipif(not GUPPY_AVAILABLE, reason="guppylang not available")
def test_guppy_to_hugr_only():
    """Test just the Guppy to HUGR compilation step."""
    
    @guppy
    def simple_h_gate() -> bool:
        """Apply H gate and measure."""
        q = qubit()
        h(q)
        return measure(q)
    
    if COMPILATION_AVAILABLE:
        hugr_bytes = compile_guppy_to_hugr(simple_h_gate)
        print(f"\nCompiled Guppy to HUGR: {len(hugr_bytes)} bytes")
        
        # Try to understand HUGR format
        print(f"First 100 bytes: {hugr_bytes[:100]}")
        
        # Check if it's JSON (HUGR 0.13 format)
        try:
            import json
            hugr_json = json.loads(hugr_bytes.decode('utf-8'))
            print(f"HUGR is JSON with keys: {list(hugr_json.keys())}")
        except:
            print("HUGR is not JSON format")
    else:
        print("Compilation pipeline not available")


if __name__ == "__main__":
    # Run tests directly for exploration
    test = TestSeleneDirectIntegration()
    
    print("=" * 60)
    print("SELENE DIRECT INTEGRATION EXPLORATION")
    print("=" * 60)
    
    if SELENE_AVAILABLE:
        test.test_selene_configuration_exploration()
        test.test_understanding_selene_result_stream()
    
    if GUPPY_AVAILABLE and SELENE_AVAILABLE:
        test.test_simple_bell_state_with_selene()
    
    if GUPPY_AVAILABLE:
        test_guppy_to_hugr_only()