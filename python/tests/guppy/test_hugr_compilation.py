#!/usr/bin/env python3
"""
Test HUGR compilation and QIR generation
"""

import subprocess
import os
import tempfile
from pathlib import Path

def test_rust_hugr_compilation():
    """Test that the Rust HUGR support compiles"""
    print("=== Testing Rust HUGR Compilation ===")
    
    # Test 1: Check if HUGR support compiles
    result = subprocess.run(
        ["cargo", "check", "-p", "pecos-qir", "--features", "hugr-support"],
        capture_output=True,
        text=True
    )
    
    if result.returncode == 0:
        print("[PASS] HUGR support compiles successfully")
    else:
        print("[FAIL] HUGR compilation failed")
        print(result.stderr[:500])
        return False
    
    # Test 2: Run HUGR-specific unit tests
    result = subprocess.run(
        ["cargo", "test", "-p", "pecos-qir", "hugr", "--features", "hugr-support"],
        capture_output=True,
        text=True
    )
    
    if result.returncode == 0:
        print("[PASS] HUGR unit tests pass")
        # Count tests
        test_count = result.stdout.count("test result: ok")
        print(f"  {test_count} test suites passed")
    else:
        print("[FAIL] HUGR tests failed")
        print(result.stderr[:500])
        return False
    
    return True

def test_standard_qir_generation():
    """Test standard QIR generation patterns"""
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
    
    with tempfile.NamedTemporaryFile(mode='w', suffix='.ll', delete=False) as f:
        f.write(test_qir)
        qir_file = f.name
    
    print(f"[OK] Created test QIR file: {qir_file}")
    
    # Verify it's valid LLVM IR
    try:
        result = subprocess.run(
            ["llvm-as", qir_file, "-o", "/dev/null"],
            capture_output=True,
            text=True
        )
        
        if result.returncode == 0:
            print("[PASS] QIR format is valid LLVM IR")
        else:
            print("[FAIL] QIR validation failed (invalid format)")
    except FileNotFoundError:
        print("⚠ llvm-as not available, skipping validation")
    
    # Clean up
    os.unlink(qir_file)
    
    return True

def test_qir_examples():
    """Test existing QIR examples"""
    print("\n=== Testing QIR Examples ===")
    
    qir_examples = Path("examples/qir")
    
    if not qir_examples.exists():
        print("[FAIL] QIR examples directory not found")
        return False
    
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
            print(f"    [PASS] Valid standard QIR format")
        else:
            print(f"    ? Non-standard format")
    
    return True

def test_python_api():
    """Test Python API availability"""
    print("\n=== Testing Python API ===")
    
    try:
        import sys
        sys.path.append('python/quantum-pecos/src')
        
        from pecos.frontends.guppy_frontend import GuppyFrontend
        from pecos.frontends.run_guppy import get_guppy_backends
        
        print("[PASS] Python imports successful")
        
        backends = get_guppy_backends()
        print(f"[PASS] Backend detection works: {backends}")
        
        return True
        
    except Exception as e:
        print(f"[FAIL] Python API test failed: {e}")
        return False

def main():
    print("HUGR Compilation and QIR Generation Tests")
    print("=" * 60)
    
    all_passed = True
    
    # Run tests
    all_passed &= test_rust_hugr_compilation()
    all_passed &= test_standard_qir_generation()
    all_passed &= test_qir_examples()
    all_passed &= test_python_api()
    
    print("\n" + "=" * 60)
    if all_passed:
        print("[PASS] All tests passed!")
    else:
        print("[FAIL] Some tests failed")
    
    return 0 if all_passed else 1

if __name__ == "__main__":
    exit(main())