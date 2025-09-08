"""Check HUGR format from guppylang."""

import json

import pytest


def test_check_hugr_format() -> None:
    """Check what HUGR format guppylang produces."""
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

    # Check binary format
    hugr_bytes = hugr.to_bytes()
    print(f"\nBinary format: {hugr_bytes[:20]}...")
    print(f"Header: {hugr_bytes[:8]}")
    print(f"Format byte: {hugr_bytes[8] if len(hugr_bytes) > 8 else 'N/A'}")

    # Check JSON format
    hugr_json = hugr.to_json()
    hugr_dict = json.loads(hugr_json)

    print(f"\nJSON keys: {list(hugr_dict.keys())}")

    # Check if it's a single HUGR or a Package
    if "modules" in hugr_dict:
        print("Format: HUGR Package")
        print(f"Number of modules: {len(hugr_dict['modules'])}")
    elif "nodes" in hugr_dict:
        print("Format: Single HUGR")
        print(f"Number of nodes: {len(hugr_dict['nodes'])}")

    # Save JSON for inspection
    import tempfile

    with tempfile.NamedTemporaryFile(mode="w", suffix=".json", delete=False) as f:
        json.dump(hugr_dict, f, indent=2)
        print(f"\nSaved full JSON to: {f.name}")


if __name__ == "__main__":
    test_check_hugr_format()
