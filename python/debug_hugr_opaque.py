#!/usr/bin/env python3
"""Debug HUGR opaque types."""

import json

from guppylang import guppy
from pecos.compilation_pipeline import compile_guppy_to_hugr

# Type alias for JSON-like data structures
JSONType = dict | list | str | int | float | bool | None


@guppy
def add_numbers(x: int, y: int) -> int:
    """Add two numbers together."""
    return x + y


# Compile to HUGR
hugr = compile_guppy_to_hugr(add_numbers)

# Parse JSON part
json_start = hugr.find(b"{")
json_part = hugr[json_start:].decode("utf-8", errors="ignore")
json_obj = json.loads(json_part)

print("=== Looking for Opaque types with arithmetic.int.types extension ===")


def find_opaque_types(obj: JSONType, path: str = "") -> None:
    """Find opaque types in the object hierarchy."""
    if isinstance(obj, dict):
        if obj.get("t") == "Opaque" and obj.get("extension") == "arithmetic.int.types":
            print(f"\nFound Opaque type at {path}:")
            print(f"  Full object: {json.dumps(obj, indent=2)}")
            # Check if there's more info like args or params
            if "args" in obj:
                print(f"  Args: {obj['args']}")
            if "id" in obj:
                print(f"  ID: {obj['id']}")
            if "params" in obj:
                print(f"  Params: {obj['params']}")

        for k, v in obj.items():
            find_opaque_types(v, f"{path}.{k}")
    elif isinstance(obj, list):
        for i, item in enumerate(obj):
            find_opaque_types(item, f"{path}[{i}]")


find_opaque_types(json_obj)

# Also check if there's an extensions section
print("\n=== Looking for extension definitions ===")
if "extensions" in json_obj:
    print("Found extensions section!")
    print(json.dumps(json_obj["extensions"], indent=2))

# Check modules for extension info
for i, module in enumerate(json_obj.get("modules", [])):
    if "extensions" in module:
        print(f"\nModule {i} extensions:")
        print(json.dumps(module["extensions"], indent=2))
