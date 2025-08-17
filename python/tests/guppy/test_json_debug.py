"""Debug HUGR JSON format."""

import json
import pytest


def test_debug_json():
    """Debug JSON structure."""
    try:
        from guppylang import guppy
        from guppylang.std.quantum import h, measure, qubit
    except ImportError:
        pytest.skip("guppylang not available")
    
    @guppy
    def simple() -> bool:
        q = qubit()
        h(q)
        return measure(q)
    
    # Compile to HUGR
    hugr = simple.compile()
    hugr_json = hugr.to_json()
    hugr_dict = json.loads(hugr_json)
    
    # Print full structure
    print("\n=== Full JSON structure ===")
    print(json.dumps(hugr_dict, indent=2))
    
    # Create minimal Package structure
    minimal_package = {
        "modules": hugr_dict.get("modules", []),
        "extensions": hugr_dict.get("extensions", [])
    }
    
    print("\n=== Minimal Package ===")
    print(json.dumps(minimal_package, indent=2))


if __name__ == "__main__":
    test_debug_json()