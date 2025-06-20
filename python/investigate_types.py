#!/usr/bin/env python3
"""Investigate what types work in HUGR compilation."""

import json

from guppylang import guppy
from guppylang.std.quantum import h, measure, qubit
from pecos.compilation_pipeline import compile_guppy_to_hugr


# Test quantum-only function to see what types work
@guppy
def quantum_only() -> bool:
    """Test quantum-only function to see what types work."""
    q = qubit()
    h(q)
    return measure(q)


print("=== Analyzing working quantum types ===")
hugr = compile_guppy_to_hugr(quantum_only)

# Parse and analyze the working HUGR
json_start = hugr.find(b"{")
json_obj = json.loads(hugr[json_start:])


def find_all_types(obj: object, path: str = "") -> list[tuple[str, dict]]:
    """Find all type definitions in the HUGR.

    Args:
        obj: The object to search for type definitions.
        path: The current path in the object hierarchy.

    Returns:
        List of tuples containing (path, type_definition).
    """
    types = []
    if isinstance(obj, dict):
        if "t" in obj and isinstance(obj["t"], str):
            types.append((path, obj.copy()))
        for k, v in obj.items():
            types.extend(find_all_types(v, f"{path}.{k}"))
    elif isinstance(obj, list):
        for i, item in enumerate(obj):
            types.extend(find_all_types(item, f"{path}[{i}]"))
    return types


all_types = find_all_types(json_obj)

print(f"Found {len(all_types)} type definitions in working quantum function:")
for path, type_def in all_types:
    print(f"  {path}: {type_def}")

print("\n=== Valid type variants from previous error ===")
print("Q, I, G, Sum, Opaque, Alias, V, R")

print("\n=== Let's see what 'usize' might map to ===")
# Based on the variants, usize might be:
# - I: Integer type
# - G: Generic type
# - V: Variable type
# Let's check what parameters these might need

print("Trying different transformations...")

# Test different type representations
test_types = [
    {"t": "I"},
    {"t": "G"},
    {"t": "V"},
    {"t": "Sum", "s": "Unit", "size": 2},  # Like bool
    {"t": "Alias", "name": "usize"},
]

for i, test_type in enumerate(test_types):
    print(f"\nOption {i+1}: {json.dumps(test_type)}")
