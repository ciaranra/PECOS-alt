#!/usr/bin/env python3
"""Debug HUGR type representations."""

import json

from guppylang import guppy
from pecos.compilation_pipeline import compile_guppy_to_hugr

# Type alias for JSON-like data structures
JSONType = dict | list | str | int | float | bool | None


@guppy
def simple_int() -> int:
    """Return a simple integer."""
    return 42


@guppy
def add_numbers(x: int, y: int) -> int:
    """Add two numbers together."""
    return x + y


@guppy
def bool_func() -> bool:
    """Return a boolean value."""
    return True


# Compile to HUGR
print("Compiling simple_int...")
hugr1 = compile_guppy_to_hugr(simple_int)
print(f"HUGR size: {len(hugr1)} bytes")

print("\nCompiling add_numbers...")
hugr2 = compile_guppy_to_hugr(add_numbers)
print(f"HUGR size: {len(hugr2)} bytes")

print("\nCompiling bool_func...")
hugr3 = compile_guppy_to_hugr(bool_func)
print(f"HUGR size: {len(hugr3)} bytes")

# Pretty print the HUGR to see the type representations
print("\n=== HUGR for simple_int ===")
# HUGR is bytes, need to decode first
hugr1_str = hugr1.decode("utf-8")
hugr1_json = json.loads(hugr1_str)
print(json.dumps(hugr1_json, indent=2))

print("\n=== HUGR for add_numbers (showing types) ===")
hugr2_str = hugr2.decode("utf-8")
hugr2_json = json.loads(hugr2_str)


# Look for type information
def find_types_in_dict(d: JSONType, path: str = "") -> None:
    """Recursively find type information in the HUGR structure."""
    if isinstance(d, dict):
        for k, v in d.items():
            if k in ["type", "t", "extension", "name"] or "type" in str(k).lower():
                print(f"{path}.{k}: {v}")
            find_types_in_dict(v, f"{path}.{k}")
    elif isinstance(d, list):
        for i, item in enumerate(d):
            find_types_in_dict(item, f"{path}[{i}]")


print("\nType information found in add_numbers HUGR:")
find_types_in_dict(hugr2_json)
