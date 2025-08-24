"""Test to understand Selene's build process for HUGR programs.

This test explores how to use Selene's Python build() function to compile
HUGR from Guppy and create an executable that can be wrapped by SeleneExecutableEngine.
"""

import tempfile
import json
from pathlib import Path

try:
    from guppylang import guppy
    from guppylang.std.quantum import qubit, h, measure
    GUPPY_AVAILABLE = True
except ImportError:
    GUPPY_AVAILABLE = False

try:
    import selene_sim
    from selene_sim import SeleneInstance, build
    from selene_sim.backends import SimpleRuntime, Coinflip
    SELENE_AVAILABLE = True
except ImportError:
    SELENE_AVAILABLE = False

try:
    from pecos.compilation_pipeline import compile_guppy_to_hugr
    COMPILATION_AVAILABLE = True
except ImportError:
    COMPILATION_AVAILABLE = False


def test_selene_build_from_hugr():
    """Test building a Selene executable from HUGR."""
    
    if not all([GUPPY_AVAILABLE, SELENE_AVAILABLE, COMPILATION_AVAILABLE]):
        print("Missing dependencies")
        return
    
    # Create a simple Guppy program
    @guppy
    def simple_h() -> bool:
        q = qubit()
        h(q)
        return measure(q)
    
    # Compile to HUGR
    hugr_bytes = compile_guppy_to_hugr(simple_h)
    print(f"\nStep 1: Compiled to HUGR ({len(hugr_bytes)} bytes)")
    
    # Parse HUGR to understand structure
    hugr_json = json.loads(hugr_bytes.decode('utf-8'))
    print(f"HUGR structure: modules={len(hugr_json.get('modules', []))}, "
          f"extensions={len(hugr_json.get('extensions', []))}")
    
    with tempfile.TemporaryDirectory() as tmpdir:
        build_dir = Path(tmpdir)
        
        # Save HUGR to file
        hugr_file = build_dir / "program.hugr"
        hugr_file.write_bytes(hugr_bytes)
        print(f"\nStep 2: Saved HUGR to {hugr_file}")
        
        # Try to build with Selene
        print("\nStep 3: Attempting Selene build...")
        try:
            # Use Selene's build function
            instance = build(
                src=str(hugr_file),  # Input HUGR file
                name="test_hugr_program",  # Name for the executable
                build_dir=str(build_dir),  # Build directory
                verbose=True  # Show build process
            )
            
            print(f"\nStep 4: Build successful!")
            print(f"Instance type: {type(instance)}")
            print(f"Instance attributes: {dir(instance)}")
            
            # Check what was created
            print(f"\nBuild artifacts:")
            for item in build_dir.rglob("*"):
                if item.is_file():
                    print(f"  - {item.relative_to(build_dir)}")
            
            # Try to run the instance
            print("\nStep 5: Running instance...")
            runtime = SimpleRuntime()
            simulator = Coinflip()
            
            # Run one shot
            results = list(instance.run(
                simulator=simulator,
                n_qubits=1,
                runtime=runtime,
                verbose=True
            ))
            
            print(f"\nResults: {results}")
            
            # This shows us:
            # 1. What files Selene creates (executable, libraries, etc.)
            # 2. How to run the instance
            # 3. What format results come in
            
            return instance
            
        except Exception as e:
            print(f"\nBuild failed with error: {e}")
            print(f"Error type: {type(e).__name__}")
            
            # Check if it's a known issue
            if "hugr" in str(e).lower():
                print("\nThis might be a HUGR version compatibility issue")
                print("Selene might expect a different HUGR format")
            
            return None


def test_selene_build_from_llvm():
    """Test building from LLVM IR as a comparison."""
    
    if not SELENE_AVAILABLE:
        print("Selene not available")
        return
    
    # Create simple LLVM IR
    llvm_ir = """
    ; ModuleID = 'simple_measure'
    
    declare i1 @__quantum__qis__mz__body(i64)
    declare void @__quantum__qis__h__body(i64)
    declare void @__quantum__rt__result_record_output(i64, i8*)
    
    define void @main() #0 {
    entry:
        call void @__quantum__qis__h__body(i64 0)
        %result = call i1 @__quantum__qis__mz__body(i64 0)
        ret void
    }
    
    attributes #0 = { "entry_point" }
    """
    
    with tempfile.TemporaryDirectory() as tmpdir:
        build_dir = Path(tmpdir)
        
        # Save LLVM IR
        llvm_file = build_dir / "program.ll"
        llvm_file.write_text(llvm_ir)
        print(f"\nSaved LLVM IR to {llvm_file}")
        
        try:
            # Build with Selene
            instance = build(
                src=str(llvm_file),
                name="test_llvm_program",
                build_dir=str(build_dir),
                verbose=True
            )
            
            print(f"\nLLVM build successful!")
            print(f"Build artifacts:")
            for item in build_dir.rglob("*"):
                if item.is_file():
                    print(f"  - {item.relative_to(build_dir)}")
            
            return instance
            
        except Exception as e:
            print(f"\nLLVM build failed: {e}")
            return None


def explore_selene_instance_api():
    """Explore what a SeleneInstance provides."""
    
    if not SELENE_AVAILABLE:
        return
    
    print("\n=== SeleneInstance API ===")
    print(f"SeleneInstance attributes: {dir(SeleneInstance)}")
    
    # Check docstrings
    if hasattr(SeleneInstance, '__init__'):
        print(f"\n__init__ docs: {SeleneInstance.__init__.__doc__}")
    if hasattr(SeleneInstance, 'run'):
        print(f"\nrun docs: {SeleneInstance.run.__doc__}")
    if hasattr(SeleneInstance, 'run_shots'):
        print(f"\nrun_shots docs: {SeleneInstance.run_shots.__doc__}")


def main():
    """Run all exploration tests."""
    
    print("=" * 60)
    print("SELENE BUILD PROCESS EXPLORATION")
    print("=" * 60)
    
    # First understand the API
    explore_selene_instance_api()
    
    # Try building from HUGR
    print("\n" + "=" * 60)
    print("TEST: Build from HUGR")
    print("=" * 60)
    hugr_instance = test_selene_build_from_hugr()
    
    # Try building from LLVM for comparison
    print("\n" + "=" * 60)
    print("TEST: Build from LLVM")
    print("=" * 60)
    llvm_instance = test_selene_build_from_llvm()
    
    # Summary
    print("\n" + "=" * 60)
    print("SUMMARY")
    print("=" * 60)
    print(f"HUGR build: {'SUCCESS' if hugr_instance else 'FAILED'}")
    print(f"LLVM build: {'SUCCESS' if llvm_instance else 'FAILED'}")


if __name__ == "__main__":
    main()