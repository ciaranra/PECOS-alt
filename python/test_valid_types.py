#!/usr/bin/env python3
"""Test what types are valid in HUGR."""

import json

from guppylang import guppy
from pecos.compilation_pipeline import compile_guppy_to_hugr


# Test different simple types
@guppy
def quantum_only() -> bool:
    """Create and measure a quantum bit in superposition."""
    from guppylang.std.quantum import h, measure, qubit

    q = qubit()
    h(q)
    return measure(q)


print("Testing quantum-only function...")
hugr = compile_guppy_to_hugr(quantum_only)

json_start = hugr.find(b"{")
json_obj = json.loads(hugr[json_start:])

print("Types found in quantum-only function:")


def find_types(obj: dict | list, path: str = "") -> None:
    """Find and print type information in HUGR structure."""
    if isinstance(obj, dict):
        if "t" in obj and isinstance(obj["t"], str):
            t_val = obj["t"]
            if t_val in ["Q", "I", "G", "Sum", "Opaque", "Alias", "V", "R"]:
                print(f"At {path}: {json.dumps(obj, indent=2)}")
        for k, v in obj.items():
            find_types(v, f"{path}.{k}")
    elif isinstance(obj, list):
        for i, item in enumerate(obj):
            find_types(item, f"{path}[{i}]")


find_types(json_obj)

print("\n" + "=" * 50)
print("Valid type variants from error: Q, I, G, Sum, Opaque, Alias, V, R")
print("Let's try using these instead...")


# Now let's test with manual transformations
@guppy
def simple_add(x: int, y: int) -> int:
    """Add two integers."""
    return x + y


hugr2 = compile_guppy_to_hugr(simple_add)
json_start2 = hugr2.find(b"{")
json_obj2 = json.loads(hugr2[json_start2:])


def test_transform(obj: dict | list) -> int:
    """Test different transformations."""
    count = 0
    if isinstance(obj, dict):
        if obj.get("t") == "Opaque" and obj.get("extension") == "arithmetic.int.types":

            print(f"\nOriginal: {json.dumps(obj, indent=2)}")

            # Try different valid types
            # Option 1: Use "I" (might be integer)
            obj.clear()
            obj.update({"t": "I"})
            count += 1

            print(f"Transformed to: {json.dumps(obj, indent=2)}")

        for v in obj.values():
            count += test_transform(v)
    elif isinstance(obj, list):
        for item in obj:
            count += test_transform(item)
    return count


print("\nTransforming with 'I' type...")
transform_count = test_transform(json_obj2)
print(f"Transformed {transform_count} instances")

# Test compilation
new_json = json.dumps(json_obj2, separators=(",", ":"))
new_hugr = hugr2[:json_start2] + new_json.encode("utf-8")

try:
    from pecos.compilation_pipeline import compile_hugr_to_llvm

    llvm = compile_hugr_to_llvm(new_hugr)
    print("SUCCESS with 'I' type!")
except Exception as e:  # noqa: BLE001
    print(f"Failed with 'I' type: {e}")

    # Try with Sum type (like bool)
    print("\nTrying with Sum type...")
    json_obj3 = json.loads(hugr2[json_start2:])

    def transform_to_sum(obj: dict | list) -> int:
        """Transform arithmetic types to Sum types."""
        count = 0
        if isinstance(obj, dict):
            if (
                obj.get("t") == "Opaque"
                and obj.get("extension") == "arithmetic.int.types"
            ):
                obj.clear()
                obj.update(
                    {"t": "Sum", "s": "Unit", "size": 256},
                )  # 256 variants for int
                count += 1
            for v in obj.values():
                count += transform_to_sum(v)
        elif isinstance(obj, list):
            for item in obj:
                count += transform_to_sum(item)
        return count

    transform_to_sum(json_obj3)
    new_json3 = json.dumps(json_obj3, separators=(",", ":"))
    new_hugr3 = hugr2[:json_start2] + new_json3.encode("utf-8")

    try:
        llvm = compile_hugr_to_llvm(new_hugr3)
        print("SUCCESS with Sum type!")
    except Exception as e2:  # noqa: BLE001
        print(f"Failed with Sum type: {e2}")
