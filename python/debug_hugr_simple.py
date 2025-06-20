#!/usr/bin/env python3
"""Debug HUGR type representations - simplified."""

import json
import re

from guppylang import guppy
from pecos.compilation_pipeline import compile_guppy_to_hugr

# Type alias for JSON-like data structures
JSONType = dict | list | str | int | float | bool | None


@guppy
def add_numbers(x: int, y: int) -> int:
    """Add two numbers together."""
    return x + y


# Compile to HUGR
print("Compiling add_numbers...")
hugr = compile_guppy_to_hugr(add_numbers)
print(f"HUGR size: {len(hugr)} bytes")

# Decode and parse
hugr_str = hugr.decode("utf-8")

# Look for "int(" pattern
if "int(" in hugr_str:
    print("\nFound 'int(' in HUGR!")
    # Find all occurrences
    matches = re.findall(r"int\(\d+\)", hugr_str)
    print(f"Integer types found: {matches}")

    # Show context around first occurrence
    idx = hugr_str.find("int(")
    if idx != -1:
        start = max(0, idx - 50)
        end = min(len(hugr_str), idx + 50)
        print("\nContext around 'int(':")
        print(hugr_str[start:end])

# Also check the JSON structure
hugr_json = json.loads(hugr_str)

# Simplified type search
print("\n=== Searching for type information ===")


def search_for_types(obj: JSONType, path: str = "") -> None:
    """Search for type information in the object hierarchy."""
    if isinstance(obj, dict):
        for k, v in obj.items():
            if isinstance(v, str) and "int(" in v:
                print(f"Found at {path}.{k}: {v}")
            search_for_types(v, f"{path}.{k}")
    elif isinstance(obj, list):
        for i, item in enumerate(obj):
            search_for_types(item, f"{path}[{i}]")


search_for_types(hugr_json)
