#!/usr/bin/env python3
"""Test standard QIR vs HUGR-generated QIR"""

import sys
import os
sys.path.append("python/quantum-pecos/src")

def test_standard_qir():
    """Test standard QIR files (bell.ll, qprog.ll)"""
    print("=== Testing Standard QIR ===")
    
    qir_files = [
        "examples/qir/bell.ll",
        "examples/qir/qprog.ll"
    ]
    
    for qir_file in qir_files:
        if not os.path.exists(qir_file):
            print(f"❌ {qir_file} not found")
            continue
            
        print(f"\n--- Testing {qir_file} ---")
        
        try:
            from pecos_rslib import execute_qir, reset_qir_runtime
            
            # Reset runtime
            reset_qir_runtime()
            
            # Execute with standard QIR convention
            print(f"Executing {qir_file} with standard QIR convention...")
            result = execute_qir(
                qir_file,
                1,  # shots
                42, # seed
                None,  # noise_probability
                None,  # workers
                llvm_convention="qir"  # Standard QIR convention
            )
            
            print(f"✅ SUCCESS: {qir_file}")
            print(f"Result: {result}")
            return True
            
        except Exception as e:
            print(f"❌ FAILED: {qir_file}")
            print(f"Error: {e}")
            import traceback
            traceback.print_exc()
            return False
    
    return False

def test_hugr_qir():
    """Test HUGR-generated QIR"""
    print("\n=== Testing HUGR-generated QIR ===")
    
    qir_file = "/tmp/pecos_guppy_rust_7c0k0azv/guppy_func.ll"
    
    if not os.path.exists(qir_file):
        print(f"❌ {qir_file} not found")
        return False
    
    try:
        from pecos_rslib import execute_qir, reset_qir_runtime
        
        # Reset runtime
        reset_qir_runtime()
        
        # Execute with HUGR convention
        print(f"Executing {qir_file} with HUGR convention...")
        result = execute_qir(
            qir_file,
            1,  # shots
            42, # seed
            None,  # noise_probability
            None,  # workers
            llvm_convention="hugr"  # HUGR convention
        )
        
        print(f"✅ SUCCESS: HUGR QIR")
        print(f"Result: {result}")
        return True
        
    except Exception as e:
        print(f"❌ FAILED: HUGR QIR")
        print(f"Error: {e}")
        import traceback
        traceback.print_exc()
        return False

if __name__ == "__main__":
    print("Testing different QIR formats to isolate the issue...")
    
    standard_works = test_standard_qir()
    hugr_works = test_hugr_qir()
    
    print(f"\n=== Summary ===")
    print(f"Standard QIR works: {standard_works}")
    print(f"HUGR QIR works: {hugr_works}")
    
    if standard_works and not hugr_works:
        print("\n🎯 CONCLUSION: Issue is specific to HUGR-generated LLVM-IR")
        print("Standard QIR works fine, problem is in the HUGR→LLVM-IR pipeline")
    elif not standard_works and not hugr_works:
        print("\n🚨 CONCLUSION: Issue affects all QIR execution")
    elif standard_works and hugr_works:
        print("\n✅ CONCLUSION: Both formats work!")
    else:
        print("\n🤔 CONCLUSION: Only HUGR works, standard QIR fails")