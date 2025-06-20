#!/usr/bin/env python3
"""Debug the transformation process."""

import json

from guppylang import guppy
from pecos.compilation_pipeline import compile_guppy_to_hugr


@guppy
def add_numbers(x: int, y: int) -> int:
    """Add two numbers together."""
    return x + y


# Compile to HUGR
print("Compiling to HUGR...")
hugr = compile_guppy_to_hugr(add_numbers)

# Extract JSON part
json_start = hugr.find(b"{")
json_bytes = hugr[json_start:]
json_obj = json.loads(json_bytes)

print("\n=== Original HUGR types ===")


# Find a few type instances
def find_types(obj: dict | list, path: str = "") -> int:
    """Find and print arithmetic int types in the HUGR structure."""
    count = 0
    if isinstance(obj, dict):
        if obj.get("t") == "Opaque" and obj.get("extension") == "arithmetic.int.types":
            count += 1
            if count <= 3:  # Show first 3
                print(f"\nAt {path}:")
                print(json.dumps(obj, indent=2))
        for k, v in obj.items():
            count += find_types(v, f"{path}.{k}")
    elif isinstance(obj, list):
        for i, item in enumerate(obj):
            count += find_types(item, f"{path}[{i}]")
    return count


type_count = find_types(json_obj)
print(f"\nTotal arithmetic.int.types found: {type_count}")

# Now let's manually test the transformation
print("\n=== Testing transformation ===")
try:
    # Import and run the transformer directly
    import sys

    sys.path.append("crates/pecos-qir/src/hugr")

    # Since we can't import Rust directly, let's simulate what should happen
    test_type = {
        "t": "Opaque",
        "extension": "arithmetic.int.types",
        "id": "int",
        "args": [{"tya": "BoundedNat", "n": 6}],
        "bound": "C",
    }

    print("\nOriginal type:")
    print(json.dumps(test_type, indent=2))

    print("\nWhat it should transform to:")
    print(json.dumps({"t": "I64"}, indent=2))

except Exception as e:  # noqa: BLE001
    print(f"Error: {e}")

# Try to compile with the actual system
print("\n=== Attempting compilation ===")
try:
    from pecos.compilation_pipeline import compile_hugr_to_llvm

    llvm = compile_hugr_to_llvm(hugr)
    print("SUCCESS! Compilation worked!")
except Exception as e:  # noqa: BLE001
    print(f"Failed with: {e}")
    if "Unknown type" in str(e):
        print("\nThe transformation is not being applied correctly.")
        print("Need to check if the transformer is actually being called.")
