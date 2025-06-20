"""HUGR type support and error handling.

This module provides utilities for understanding and handling HUGR type limitations.
"""

import re
from typing import Any, TypeVar

T = TypeVar("T")


class HugrTypeError(RuntimeError):
    """Error raised when HUGR compilation encounters unsupported types."""

    def __init__(self, original_error: str) -> None:
        """Initialize HugrTypeError with the original error message."""
        self.original_error = original_error
        self.unsupported_type = self._extract_type(original_error)
        super().__init__(self._create_message())

    def _extract_type(self, error: str) -> str | None:
        """Extract the unsupported type from the error message."""
        # Pattern: "Unknown type: int(6)" or "Unknown type: bool"
        match = re.search(r"Unknown type: (\w+)(?:\((\d+)\))?", error)
        if match:
            type_name = match.group(1)
            width = match.group(2)
            if width:
                return f"{type_name}({width})"
            return type_name
        return None

    def _create_message(self) -> str:
        """Create a helpful error message."""
        base_msg = f"HUGR compilation failed: {self.original_error}"

        if self.unsupported_type:
            if self.unsupported_type.startswith("int"):
                return (
                    f"{base_msg}\n\n"
                    "Classical integer types are not yet supported in the HUGR→LLVM compiler.\n"
                    "Workarounds:\n"
                    "1. Use quantum operations that return measurement results (bool)\n"
                    "2. Perform classical computations outside the Guppy function\n"
                    "3. Wait for future updates to support classical types"
                )
            if self.unsupported_type == "bool":
                return (
                    f"{base_msg}\n\n"
                    "Direct boolean returns are not yet fully supported.\n"
                    "Workarounds:\n"
                    "1. Return measurement results from quantum operations\n"
                    "2. Use the function for quantum state preparation only"
                )

        return base_msg


# Supported and unsupported types
SUPPORTED_TYPES = {
    "qubit": "Quantum bit type",
    "measurement": "Measurement result type",
    "array[bool]": "Array of measurement results",
}

UNSUPPORTED_TYPES = {
    "int": "Classical integer types",
    "float": "Floating point types",
    "string": "String types",
    "complex": "Complex number types",
    "bool": "Direct boolean values (use measurements instead)",
}


def check_type_support(guppy_function: T) -> dict[str, Any]:
    """Check if a Guppy function uses supported types.

    Args:
        guppy_function: A function decorated with @guppy

    Returns:
        Dictionary with type support information
    """
    # This would need actual type inspection in a full implementation
    # For now, return a placeholder
    del guppy_function  # Mark as intentionally unused
    return {
        "supported": True,
        "warnings": [],
        "unsupported_types": [],
    }


def create_quantum_example() -> str:
    """Return example code that works with current type support."""
    return '''
from guppylang import guppy
from guppylang.std.quantum import qubit, h, measure, cx

@guppy
def bell_state() -> tuple[bool, bool]:
    """Create a Bell state and measure both qubits.

    This works because:
    - Uses quantum types (qubit)
    - Returns measurement results (bool from measure())
    - No classical integer computations
    """
    q0 = qubit()
    q1 = qubit()
    h(q0)
    cx(q0, q1)
    return measure(q0), measure(q1)

@guppy
def quantum_coin() -> bool:
    """Simple quantum random bit generator.

    This works because it returns a measurement result.
    """
    q = qubit()
    h(q)
    return measure(q)

# These would NOT work currently:

@guppy
def classical_add(x: int, y: int) -> int:
    """This fails - classical integer operations not supported."""
    return x + y

@guppy
def return_constant() -> int:
    """This fails - returning integer literals not supported."""
    return 42
'''
