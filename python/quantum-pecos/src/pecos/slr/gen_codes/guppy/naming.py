"""Utilities for converting block names to function names."""

import re


def class_to_function_name(class_name: str) -> str:
    """Convert a PascalCase class name to snake_case function name.

    Examples:
        PrepareGHZ -> prepare_ghz
        ApplyXCorrection -> apply_x_correction
        QPEStep -> qpe_step
        PrepareLogical0 -> prepare_logical_0
    """
    # First, handle the sequence of capital letters followed by lowercase (e.g., 'GHZ' -> 'ghz')
    s1 = re.sub("(.)([A-Z][a-z]+)", r"\1_\2", class_name)
    # Handle lowercase (or number) followed by capital letter
    s2 = re.sub("([a-z0-9])([A-Z])", r"\1_\2", s1)
    # Handle letter followed by number
    s3 = re.sub(r"([a-zA-Z])(\d)", r"\1_\2", s2)
    return s3.lower()


def get_module_prefix(block_class) -> str | None:
    """Get module-based prefix for a block class.

    Examples:
        pecos.qeclib.steane.PrepareLogical0 -> steane_
        pecos.qeclib.surface.MeasureStabilizers -> surface_
        mypackage.circuits.teleport.BellPair -> teleport_
    """
    module = getattr(block_class, "__module__", "")
    if not module:
        return None

    # Look for qeclib patterns
    if "qeclib" in module:
        parts = module.split(".")
        try:
            qeclib_idx = parts.index("qeclib")
            if qeclib_idx + 1 < len(parts):
                # Return the module after qeclib (e.g., 'steane', 'surface')
                return parts[qeclib_idx + 1] + "_"
        except (ValueError, IndexError):
            pass

    # For other patterns, look for meaningful module names
    common_modules = {
        "blocks",
        "ops",
        "operations",
        "circuits",
        "components",
        "__main__",
    }
    parts = module.split(".")

    # Skip the class name itself if it's at the end
    if parts and parts[-1] == block_class.__name__:
        parts = parts[:-1]

    # Look backwards for a meaningful, specific module name
    for i in range(len(parts) - 1, -1, -1):
        part = parts[i]
        # Skip common structural names
        if part in common_modules:
            continue
        # Found a specific module name
        return part + "_"

    return None


def get_function_name(block_class, *, use_module_prefix: bool = True) -> str:
    """Get the full function name for a block class.

    Args:
        block_class: The block class
        use_module_prefix: Whether to include module-based prefix

    Returns:
        Function name like 'prepare_ghz' or 'steane_prepare_logical_0'
    """
    # Get base name from class
    class_name = block_class.__name__
    base_name = class_to_function_name(class_name)

    # Add module prefix if requested
    if use_module_prefix:
        prefix = get_module_prefix(block_class)
        if prefix and not base_name.startswith(prefix.rstrip("_")):
            return prefix + base_name

    return base_name


# Example usage and tests
if __name__ == "__main__":
    # Test class name conversion
    test_cases = [
        ("PrepareGHZ", "prepare_ghz"),
        ("ApplyXCorrection", "apply_x_correction"),
        ("QPEStep", "qpe_step"),
        ("PrepareLogical0", "prepare_logical_0"),
        ("CNOTGate", "cnot_gate"),
        ("Phase90", "phase_90"),
        ("TOffoli3", "t_offoli_3"),
    ]

    print("Class name conversions:")
    for input_name, expected in test_cases:
        result = class_to_function_name(input_name)
        status = "PASS" if result == expected else "FAIL"
        print(f"  {status} {input_name} -> {result} (expected: {expected})")

    # Test module prefix extraction
    print("\nModule prefix extraction:")

    class MockClass:
        pass

    # Test different module paths
    test_modules = [
        ("pecos.qeclib.steane.PrepareLogical0", "steane_"),
        ("pecos.qeclib.surface.MeasureStabilizers", "surface_"),
        ("pecos.qeclib.bacon_shor.ExtractSyndrome", "bacon_shor_"),
        ("mypackage.circuits.teleport.BellPair", "teleport_"),
        ("mypackage.circuits.BellPair", None),  # 'circuits' is common
        ("pecos.slr.blocks.CustomBlock", None),  # 'blocks' is common
        ("__main__.MyBlock", None),
    ]

    for module_path, expected_prefix in test_modules:
        MockClass.__module__ = module_path
        MockClass.__name__ = module_path.split(".")[-1]
        prefix = get_module_prefix(MockClass)
        status = "PASS" if prefix == expected_prefix else "FAIL"
        print(f"  {status} {module_path} -> {prefix} (expected: {expected_prefix})")
