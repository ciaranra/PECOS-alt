#!/usr/bin/env python3
"""Test manual transformation to see what works."""

import json
import tempfile
from pathlib import Path

from guppylang import guppy
from pecos.compilation_pipeline import compile_guppy_to_hugr


@guppy
def add_numbers(x: int, y: int) -> int:
    """Add two numbers together."""
    return x + y


# Compile to HUGR
print("Compiling to HUGR...")
hugr = compile_guppy_to_hugr(add_numbers)

# Manual transformation
json_start = hugr.find(b"{")
header = hugr[:json_start]
json_bytes = hugr[json_start:]
json_obj = json.loads(json_bytes)

print("Original arithmetic types found - transforming...")


def transform_types(obj: dict | list) -> int:
    """Manually transform types."""
    count = 0
    if isinstance(obj, dict):
        # Transform arithmetic int types
        if (
            obj.get("t") == "Opaque"
            and obj.get("extension") == "arithmetic.int.types"
            and obj.get("id") == "int"
        ):

            # Try different transformations
            print(f"Transforming type instance {count}")

            # Option 1: Simple unit type (no parameters)
            obj.clear()
            obj.update({"t": "Tuple", "ts": []})  # Empty tuple
            count += 1

        # Transform boolean types
        elif obj.get("t") == "Opaque" and obj.get("extension") == "tket2.bool":
            obj.clear()
            obj.update({"t": "Sum", "s": "Unit", "size": 2})
            count += 1

        # Recurse
        for v in obj.values():
            count += transform_types(v)

    elif isinstance(obj, list):
        for item in obj:
            count += transform_types(item)

    return count


# Apply transformation
transform_count = transform_types(json_obj)
print(f"Transformed {transform_count} type instances")

# Rebuild HUGR
new_json = json.dumps(json_obj, separators=(",", ":"))
new_hugr = header + new_json.encode("utf-8")

print(f"Original HUGR size: {len(hugr)} bytes")
print(f"Transformed HUGR size: {len(new_hugr)} bytes")

# Write to temporary file and try to compile
with tempfile.NamedTemporaryFile(suffix=".hugr", delete=False) as f:
    f.write(new_hugr)
    temp_path = f.name

try:
    print("\nTrying to compile transformed HUGR...")
    from pecos.compilation_pipeline import compile_hugr_to_llvm

    llvm = compile_hugr_to_llvm(new_hugr)
    print("SUCCESS! Transformation worked!")
    print(f"LLVM IR length: {len(llvm)} characters")
    print("First 500 characters:")
    print(llvm[:500])
except Exception as e:  # noqa: BLE001
    print(f"Failed: {e}")
    if "Unknown type" in str(e):
        print("Still getting Unknown type error. Need different transformation.")
    else:
        print("Different error - might be progress!")

finally:
    Path(temp_path).unlink()
