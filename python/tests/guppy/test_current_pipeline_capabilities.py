#!/usr/bin/env python3
"""Test current capabilities of both HUGR-LLVM and PMIR pipelines.

This test systematically determines what Guppy programs both pipelines 
can currently handle successfully.
"""

import sys
from pathlib import Path

import pytest

sys.path.append("python/quantum-pecos/src")

try:
    from guppylang import guppy
    from guppylang.std.quantum import h, measure, qubit, cx, x, y, z
    GUPPY_AVAILABLE = True
except ImportError:
    GUPPY_AVAILABLE = False

try:
    from pecos.frontends.run_guppy import run_guppy, get_guppy_backends
    PECOS_FRONTEND_AVAILABLE = True
except ImportError:
    PECOS_FRONTEND_AVAILABLE = False


def test_pipeline_capabilities():
    """Test what both pipelines can currently handle."""
    if not GUPPY_AVAILABLE or not PECOS_FRONTEND_AVAILABLE:
        pytest.skip("Dependencies not available")
    
    print("\n" + "="*80)
    print("CURRENT GUPPY PIPELINE CAPABILITIES TEST")
    print("="*80)
    
    backends = get_guppy_backends()
    print(f"Available backends: {backends}")
    
    # Test cases - ordered from simple to complex
    test_cases = []
    
    # 1. Basic single-qubit operations
    @guppy
    def test_hadamard() -> bool:
        q = qubit()
        h(q)
        return measure(q)
    test_cases.append(("Hadamard Gate", test_hadamard))
    
    @guppy
    def test_identity() -> bool:
        q = qubit()
        return measure(q)
    test_cases.append(("Identity (no gates)", test_identity))
    
    @guppy
    def test_pauli_x() -> bool:
        q = qubit()
        x(q)
        return measure(q)
    test_cases.append(("Pauli X Gate", test_pauli_x))
    
    # 2. Two-qubit operations
    @guppy
    def test_bell_state() -> tuple[bool, bool]:
        q0, q1 = qubit(), qubit()
        h(q0)
        cx(q0, q1)
        return measure(q0), measure(q1)
    test_cases.append(("Bell State", test_bell_state))
    
    # 3. Three-qubit operations  
    @guppy
    def test_ghz_state() -> tuple[bool, bool, bool]:
        q0, q1, q2 = qubit(), qubit(), qubit()
        h(q0)
        cx(q0, q1)
        cx(q1, q2)
        return measure(q0), measure(q1), measure(q2)
    test_cases.append(("GHZ State", test_ghz_state))
    
    # 4. Sequential operations
    @guppy
    def test_sequential_gates() -> bool:
        q = qubit()
        h(q)
        x(q)
        h(q)
        return measure(q)
    test_cases.append(("Sequential Gates", test_sequential_gates))
    
    # 5. Multiple qubits, independent operations
    @guppy
    def test_parallel_hadamard() -> tuple[bool, bool]:
        q0, q1 = qubit(), qubit()
        h(q0)
        h(q1)
        return measure(q0), measure(q1)
    test_cases.append(("Parallel Hadamard", test_parallel_hadamard))
    
    # Run tests on both pipelines
    results = {}
    
    for test_name, test_func in test_cases:
        print(f"\n📋 Testing: {test_name}")
        results[test_name] = {}
        
        # Test HUGR-LLVM pipeline
        if backends.get("rust_backend", False):
            try:
                result = run_guppy(test_func, shots=10, backend="rust", verbose=False)
                results[test_name]["hugr_llvm"] = {
                    "success": True,
                    "compilation_time": result.get("compilation_time", 0),
                    "backend": result.get("backend_used"),
                    "sample_results": result.get("results", [])[:5]
                }
                print(f"  ✅ HUGR-LLVM: {result.get('compilation_time', 0):.4f}s compilation")
            except Exception as e:
                results[test_name]["hugr_llvm"] = {
                    "success": False,
                    "error": str(e)
                }
                print(f"  ❌ HUGR-LLVM: {str(e)[:80]}")
        
        # Test PMIR pipeline  
        try:
            result = run_guppy(test_func, shots=10, backend="external", verbose=False)
            results[test_name]["pmir"] = {
                "success": True,
                "compilation_time": result.get("compilation_time", 0),
                "backend": result.get("backend_used"),
                "sample_results": result.get("results", [])[:5]
            }
            print(f"  ✅ PMIR: {result.get('compilation_time', 0):.4f}s compilation")
        except Exception as e:
            results[test_name]["pmir"] = {
                "success": False,
                "error": str(e)
            }
            print(f"  ❌ PMIR: {str(e)[:80]}")
    
    # Generate summary
    print("\n" + "="*80)
    print("PIPELINE CAPABILITY SUMMARY")
    print("="*80)
    
    print(f"{'Test Case':<25} {'HUGR-LLVM':<15} {'PMIR':<15}")
    print("-" * 80)
    
    hugr_success_count = 0
    pmir_success_count = 0
    
    for test_name, test_results in results.items():
        hugr_status = "✅ PASS" if test_results.get("hugr_llvm", {}).get("success", False) else "❌ FAIL"
        pmir_status = "✅ PASS" if test_results.get("pmir", {}).get("success", False) else "❌ FAIL"
        
        if test_results.get("hugr_llvm", {}).get("success", False):
            hugr_success_count += 1
        if test_results.get("pmir", {}).get("success", False):
            pmir_success_count += 1
            
        print(f"{test_name:<25} {hugr_status:<15} {pmir_status:<15}")
    
    total_tests = len(test_cases)
    print("-" * 80)
    print(f"{'TOTALS':<25} {hugr_success_count}/{total_tests} PASS{'':<6} {pmir_success_count}/{total_tests} PASS")
    
    # Detailed error analysis
    print("\n" + "="*80)
    print("ERROR ANALYSIS")
    print("="*80)
    
    for test_name, test_results in results.items():
        print(f"\n🔍 {test_name}:")
        
        hugr_result = test_results.get("hugr_llvm", {})
        if not hugr_result.get("success", False):
            print(f"  HUGR-LLVM Error: {hugr_result.get('error', 'Unknown')}")
        else:
            print(f"  HUGR-LLVM: Success, sample results: {hugr_result.get('sample_results', [])}")
            
        pmir_result = test_results.get("pmir", {})
        if not pmir_result.get("success", False):
            print(f"  PMIR Error: {pmir_result.get('error', 'Unknown')}")
        else:
            print(f"  PMIR: Success, sample results: {pmir_result.get('sample_results', [])}")
    
    # Final assessment
    print("\n" + "="*80)
    print("FINAL ASSESSMENT")
    print("="*80)
    
    if hugr_success_count == total_tests and pmir_success_count == total_tests:
        print("🎉 EXCELLENT: Both pipelines handle all tested Guppy programs!")
    elif hugr_success_count == pmir_success_count:
        print(f"✅ GOOD: Both pipelines have identical capabilities ({hugr_success_count}/{total_tests} tests pass)")
    elif hugr_success_count > pmir_success_count:
        print(f"📊 HUGR-LLVM has better coverage: {hugr_success_count}/{total_tests} vs {pmir_success_count}/{total_tests}")
    else:
        print(f"📊 PMIR has better coverage: {pmir_success_count}/{total_tests} vs {hugr_success_count}/{total_tests}")
    
    print("\nFor the current set of Guppy quantum programs:")
    working_programs = [name for name, results in results.items() 
                       if results.get("hugr_llvm", {}).get("success", False) 
                       and results.get("pmir", {}).get("success", False)]
    
    if working_programs:
        print("✅ Both pipelines can handle:")
        for program in working_programs:
            print(f"   - {program}")
    
    hugr_only = [name for name, results in results.items() 
                if results.get("hugr_llvm", {}).get("success", False) 
                and not results.get("pmir", {}).get("success", False)]
    
    pmir_only = [name for name, results in results.items() 
                if results.get("pmir", {}).get("success", False) 
                and not results.get("hugr_llvm", {}).get("success", False)]
    
    if hugr_only:
        print("🦀 Only HUGR-LLVM can handle:")
        for program in hugr_only:
            print(f"   - {program}")
    
    if pmir_only:
        print("🔧 Only PMIR can handle:")
        for program in pmir_only:
            print(f"   - {program}")
    
    neither = [name for name, results in results.items() 
              if not results.get("hugr_llvm", {}).get("success", False) 
              and not results.get("pmir", {}).get("success", False)]
    
    if neither:
        print("❌ Neither pipeline can handle:")
        for program in neither:
            print(f"   - {program}")
    
    print(f"\n📊 Coverage: {len(working_programs)}/{total_tests} programs work on both pipelines")
    
    # Performance comparison
    if working_programs:
        print("\n" + "="*80)
        print("PERFORMANCE COMPARISON")
        print("="*80)
        
        hugr_times = []
        pmir_times = []
        
        for program in working_programs:
            hugr_time = results[program]["hugr_llvm"].get("compilation_time", 0)
            pmir_time = results[program]["pmir"].get("compilation_time", 0)
            hugr_times.append(hugr_time)
            pmir_times.append(pmir_time)
            print(f"{program:<25} HUGR: {hugr_time:.4f}s  PMIR: {pmir_time:.4f}s")
        
        if hugr_times and pmir_times:
            avg_hugr = sum(hugr_times) / len(hugr_times)
            avg_pmir = sum(pmir_times) / len(pmir_times)
            print("-" * 80)
            print(f"{'AVERAGE':<25} HUGR: {avg_hugr:.4f}s  PMIR: {avg_pmir:.4f}s")
            
            if avg_hugr < avg_pmir:
                speedup = avg_pmir / avg_hugr
                print(f"🚀 HUGR-LLVM is {speedup:.1f}x faster on average")
            elif avg_pmir < avg_hugr:
                speedup = avg_hugr / avg_pmir
                print(f"🚀 PMIR is {speedup:.1f}x faster on average")
            else:
                print("⚖️  Both pipelines have similar performance")


if __name__ == "__main__":
    test_pipeline_capabilities()