"""Test to understand HUGR 0.13 structure from guppylang."""

import json
import tempfile
from pathlib import Path

import pytest


def test_hugr_json_structure():
    """Examine HUGR JSON structure from guppylang."""
    try:
        from guppylang import guppy
        from guppylang.std.quantum import h, measure, qubit
    except ImportError:
        pytest.skip("guppylang not available")
    
    @guppy
    def simple_circuit() -> bool:
        q = qubit()
        h(q)
        return measure(q)
    
    # Compile to HUGR
    hugr = simple_circuit.compile()
    
    # Get JSON representation
    hugr_json = hugr.to_json()
    hugr_dict = json.loads(hugr_json)
    
    print("\n=== HUGR JSON Structure ===")
    print(f"Keys: {list(hugr_dict.keys())}")
    
    if "modules" in hugr_dict:
        print(f"\nNumber of modules: {len(hugr_dict['modules'])}")
        for i, module in enumerate(hugr_dict['modules']):
            print(f"\nModule {i}:")
            print(f"  Keys: {list(module.keys())}")
            if "nodes" in module:
                print(f"  Number of nodes: {len(module['nodes'])}")
                # Print first few nodes
                for j, node in enumerate(module['nodes'][:5]):
                    print(f"  Node {j}: {node}")
    
    # Save to file for inspection
    with tempfile.NamedTemporaryFile(mode='w', suffix='.json', delete=False) as f:
        json.dump(hugr_dict, f, indent=2)
        print(f"\nSaved HUGR JSON to: {f.name}")


if __name__ == "__main__":
    test_hugr_json_structure()