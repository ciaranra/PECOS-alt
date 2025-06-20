#!/usr/bin/env python3
"""Debug HUGR format."""

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

# Check what the HUGR looks like
print(f"\nFirst 100 bytes: {hugr[:100]}")
print(f"\nFirst 100 bytes as string: {hugr[:100]!r}")

# Try to find the string "int(" in the binary
if b"int(" in hugr:
    print("\nFound b'int(' in HUGR bytes!")
    idx = hugr.find(b"int(")
    print(f"Position: {idx}")
    print(f"Context: {hugr[idx-20:idx+30]}")

# Check if it starts with a known header
if hugr.startswith(b"HUGR"):
    print("\nHUGR starts with 'HUGR' header")
    # Try to find JSON after header
    json_start = hugr.find(b"{")
    if json_start != -1:
        print(f"Found JSON at position {json_start}")
        try:
            json_part = hugr[json_start:].decode("utf-8", errors="ignore")
            json_obj = json.loads(json_part)
            print("Successfully parsed JSON part!")

            # Search for int( in the JSON
            json_str = json.dumps(json_obj, indent=2)
            if "int(" in json_str:
                print("\nFound 'int(' in JSON!")
                matches = re.findall(r"int\(\d+\)", json_str)
                print(f"Integer types: {matches}")

                # Find and show context
                for match in matches:
                    idx = json_str.find(match)
                    start = max(0, idx - 100)
                    end = min(len(json_str), idx + 100)
                    print(f"\nContext for {match}:")
                    print(json_str[start:end])
            else:
                # Maybe it's in a different format, let's search more broadly
                print("\nSearching for type information...")

                def find_types(obj: JSONType, path: str = "") -> None:
                    """Find type information in the object hierarchy."""
                    if isinstance(obj, dict):
                        for k, v in obj.items():
                            if k in ["t", "type", "extension", "name"] and isinstance(
                                v,
                                str,
                            ):
                                print(f"{path}.{k} = {v}")
                            find_types(v, f"{path}.{k}")
                    elif isinstance(obj, list):
                        for i, item in enumerate(obj):
                            find_types(item, f"{path}[{i}]")

                find_types(json_obj)
        except json.JSONDecodeError as e:
            print(f"Failed to parse JSON: {e}")
