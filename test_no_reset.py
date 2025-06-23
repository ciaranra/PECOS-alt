#!/usr/bin/env python3
"""Test QIR without calling reset"""

import sys
sys.path.append("python/quantum-pecos/src")

def test_no_reset():
    print("=== Test QIR Without Reset ===")
    
    try:
        # Import the setup function directly
        from pecos_qir import setup_qir_engine
        print("✅ Imported setup_qir_engine from Rust")
        
        # Try to create engine without any reset
        print("Creating QIR engine...")
        engine = setup_qir_engine("examples/qir/bell.ll", None)
        print(f"✅ Created engine: {type(engine)}")
        
    except ImportError:
        # Try through pecos
        print("Trying through pecos module...")
        from pecos.engines.hybrid_engine import MonteCarloEngine
        from pecos_qir import QirEngine
        
        print("Creating QIR classical engine...")
        classical_engine = QirEngine("examples/qir/bell.ll")
        print(f"✅ Created classical engine: {type(classical_engine)}")
        
        print("Creating Monte Carlo engine...")
        engine = MonteCarloEngine(classical_engine)
        print(f"✅ Created Monte Carlo engine: {type(engine)}")
        
    except Exception as e:
        print(f"❌ Error: {type(e).__name__}: {e}")
        import traceback
        traceback.print_exc()

if __name__ == "__main__":
    test_no_reset()