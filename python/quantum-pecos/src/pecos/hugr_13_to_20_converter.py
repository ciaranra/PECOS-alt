"""Convert HUGR 0.13 types to HUGR 0.20 format.

This module provides functions to convert HUGR packages from version 0.13
(used by guppylang) to version 0.20 (used by PECOS/Selene).
"""

import json
from typing import Any


def convert_list_to_array(value: Any) -> None:
    """Recursively convert List types to Array types in a JSON structure.

    This modifies the structure in-place.
    """
    if isinstance(value, dict):
        # Check if this is a List type - different fields might contain it
        # Handle "variant" field
        if value.get("variant") == "List":
            value["variant"] = "Array"
            # Update extension if present
            if "extension" in value and isinstance(value["extension"], str):
                value["extension"] = value["extension"].replace("list", "array")

        # Handle "tya" field (type alias?)
        if value.get("tya") == "List":
            value["tya"] = "Array"

        # Handle "tp" field (type?)
        if value.get("tp") == "List":
            value["tp"] = "Array"

        # Handle any string value that is exactly "List"
        for key, val in list(value.items()):
            if val == "List":
                value[key] = "Array"
            elif isinstance(val, str) and "List" in val:
                # Check for compound types like "List<T>"
                value[key] = val.replace("List", "Array")

        # Recursively process all values
        for v in value.values():
            convert_list_to_array(v)

    elif isinstance(value, list):
        for item in value:
            convert_list_to_array(item)


def fix_hugr_13_to_20(package) -> None:
    """Fix HUGR 0.13 to 0.20 compatibility issues in a Package object.

    This modifies the package in-place.

    Args:
        package: A hugr.package.Package object
    """
    # Convert to JSON (use to_str if available, otherwise to_json)
    json_str = package.to_str() if hasattr(package, "to_str") else package.to_json()
    json_obj = json.loads(json_str)

    # Apply conversions
    convert_list_to_array(json_obj)

    # Convert back to package
    fixed_json = json.dumps(json_obj)

    # Update the package in-place by replacing its modules
    from hugr.package import Package

    fixed_package = Package.from_json(fixed_json)

    # Replace the modules
    package.modules.clear()
    package.modules.extend(fixed_package.modules)

    # Replace extensions if any
    if hasattr(package, "extensions"):
        package.extensions.clear()
        package.extensions.extend(fixed_package.extensions)


def compile_guppy_to_hugr_fixed(guppy_function) -> bytes:
    """Compile a Guppy function to HUGR bytes with type fixes.

    This is a wrapper around the standard compilation that fixes
    HUGR 0.13 to 0.20 compatibility issues.

    Args:
        guppy_function: A function decorated with @guppy

    Returns:
        HUGR package as bytes (compatible with HUGR 0.20)
    """
    from guppylang import guppy as guppy_module

    # Check if this is a Guppy function
    is_guppy = (
        hasattr(guppy_function, "_guppy_compiled")
        or hasattr(guppy_function, "name")
        or str(type(guppy_function)).find("GuppyDefinition") != -1
        or str(type(guppy_function)).find("GuppyFunctionDefinition") != -1
    )

    if not is_guppy:
        msg = "Function must be decorated with @guppy"
        raise ValueError(msg)

    # Compile the function
    compiled = (
        guppy_function.compile()
        if hasattr(guppy_function, "compile")
        else guppy_module.compile(guppy_function)
    )

    # Get the package
    if hasattr(compiled, "package"):
        package = compiled.package
    elif hasattr(compiled, "to_package"):
        package = compiled.to_package()
    else:
        package = compiled

    # Fix HUGR 0.13 to 0.20 compatibility
    fix_hugr_13_to_20(package)

    # Return as bytes
    return package.to_bytes()
