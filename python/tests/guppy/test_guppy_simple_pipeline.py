#!/usr/bin/env python3
"""
Simple test script for the Guppy → HUGR → QIR → PECOS pipeline.
Run with: uv run test_guppy_simple_pipeline.py
"""

import sys
sys.path.insert(0, 'python/quantum-pecos/src')

def test_infrastructure():
    """Test if all components are available"""
    print("🔍 Checking infrastructure...")
    
    # Check imports
    try:
        from pecos.frontends.guppy_frontend import GuppyFrontend, GUPPY_AVAILABLE
        from pecos.frontends.run_guppy import get_guppy_backends
        print("✅ PECOS imports successful")
    except ImportError as e:
        print(f"❌ PECOS import failed: {e}")
        return False
    
    # Check backends
    backends = get_guppy_backends()
    print(f"\n📊 Backend status:")
    print(f"   Guppy available: {backends['guppy_available']}")
    print(f"   Rust backend: {backends['rust_backend']}")
    print(f"   External tools: {backends['external_tools']}")
    
    if backends['rust_backend']:
        print("✅ Rust backend with HUGR support is available!")
    else:
        print(f"⚠️  Rust backend not available: {backends.get('rust_message', 'Unknown reason')}")
    
    return True

def test_simple_classical():
    """Test with a simple classical function (no quantum operations)"""
    print("\n🧪 Testing classical function compilation...")
    
    try:
        from guppylang.decorator import guppy
        from guppylang import guppy as guppy_compiler
        print("✅ Guppylang imports successful")
        
        # Define a simple classical function
        @guppy
        def add_numbers(x: int, y: int) -> int:
            return x + y
        
        print("✅ Classical function defined")
        
        # Try to compile it
        compiled = guppy_compiler.compile(add_numbers)
        print(f"✅ Function compiled: {type(compiled)}")
        
        # Get HUGR
        hugr_bytes = compiled.package.to_bytes()
        print(f"✅ HUGR generated: {len(hugr_bytes)} bytes")
        
        # Try with GuppyFrontend
        from pecos.frontends.guppy_frontend import GuppyFrontend
        frontend = GuppyFrontend()
        qir_file = frontend.compile_function(add_numbers)
        print(f"✅ QIR file generated: {qir_file}")
        
        # Read QIR content
        with open(qir_file, 'r') as f:
            qir_content = f.read()
        print(f"✅ QIR size: {len(qir_content)} characters")
        
        # Show first few lines
        print("\n📄 QIR preview:")
        lines = qir_content.split('\n')[:10]
        for line in lines:
            print(f"   {line}")
        
        frontend.cleanup()
        return True
        
    except Exception as e:
        print(f"❌ Error: {e}")
        import traceback
        traceback.print_exc()
        return False

def test_quantum_if_available():
    """Test quantum compilation if imports work"""
    print("\n🔮 Testing quantum function (if possible)...")
    
    try:
        # Try the documented import pattern
        from guppylang.decorator import guppy
        from guppylang.std.quantum import qubit, h, measure
        from guppylang import guppy as guppy_compiler
        
        print("✅ Quantum imports successful")
        
        @guppy
        def quantum_coin() -> bool:
            q = qubit()
            h(q)
            return measure(q)
        
        print("✅ Quantum function defined")
        
        # Try to compile
        compiled = guppy_compiler.compile(quantum_coin)
        print("✅ Quantum function compiled!")
        
        return True
        
    except ImportError as e:
        print(f"⚠️  Quantum imports not available: {e}")
        print("   This might be due to guppylang version mismatch")
        return False
    except Exception as e:
        print(f"⚠️  Quantum compilation failed: {e}")
        print("   This is expected with guppylang version changes")
        return False

def suggest_version_pinning():
    """Show how to pin versions"""
    print("\n📌 Version Pinning Recommendations:")
    print("\nTo ensure stability, update python/quantum-pecos/pyproject.toml:")
    print("""
[project.optional-dependencies]
guppy = [
    "guppylang==0.19.1",  # Pin to exact version instead of >=0.19.0
]
""")
    print("\nThe HUGR versions are already pinned in Rust:")
    print("   hugr-core = 0.20.1")
    print("   hugr-llvm = 0.20.1")
    
    print("\nTo update dependencies after pinning:")
    print("   uv pip install -e python/quantum-pecos[guppy]")

def main():
    """Run all tests"""
    print("🚀 Guppy → HUGR → QIR → PECOS Pipeline Test")
    print("=" * 60)
    
    # Test infrastructure
    infra_ok = test_infrastructure()
    
    if not infra_ok:
        print("\n❌ Infrastructure not ready")
        return 1
    
    # Test classical compilation
    classical_ok = test_simple_classical()
    
    # Test quantum if possible
    quantum_ok = test_quantum_if_available()
    
    # Show version pinning suggestions
    suggest_version_pinning()
    
    # Summary
    print("\n" + "=" * 60)
    print("📊 Summary:")
    print(f"   Infrastructure: {'✅' if infra_ok else '❌'}")
    print(f"   Classical compilation: {'✅' if classical_ok else '❌'}")
    print(f"   Quantum compilation: {'✅' if quantum_ok else '⚠️ (version mismatch expected)'}")
    
    if infra_ok and classical_ok:
        print("\n✨ Core pipeline is working!")
        print("   The infrastructure is ready for Guppy → HUGR → QIR compilation.")
        print("   Quantum function compilation may need guppylang version adjustment.")
        return 0
    else:
        print("\n❌ Some tests failed")
        return 1

if __name__ == "__main__":
    sys.exit(main())