"""Type annotations for shot result types."""

from typing import List, Dict

class ShotVec:
    """A collection of quantum measurement shot results.

    This is the primary result type returned by quantum simulations.
    It stores measurement data for multiple shots in a row-oriented format.
    """

    @property
    def len(self) -> int:
        """Number of shots in the collection."""
        ...

    def is_empty(self) -> bool:
        """Check if the collection is empty."""
        ...

    def to_shot_map(self) -> "ShotMap":
        """Convert to columnar format for efficient access by register.

        Returns:
            ShotMap: A columnar representation of the shot data

        Raises:
            RuntimeError: If conversion fails
        """
        ...

    def to_dict(self) -> Dict[str, List[int]]:
        """Convert to a Python dictionary with integer values.

        This is the default format, where bit vectors are converted to integers.

        Returns:
            Dict mapping register names to lists of integer values
        """
        ...

    def to_binary_dict(self) -> Dict[str, List[str]]:
        """Convert to a Python dictionary with binary string values.

        Bit vectors are formatted as binary strings (e.g., "0101").

        Returns:
            Dict mapping register names to lists of binary strings
        """
        ...

    def __len__(self) -> int:
        """Number of shots in the collection."""
        ...

    def __repr__(self) -> str:
        """String representation."""
        ...

class ShotMap:
    """Columnar representation of quantum measurement results.

    This format organizes shot data by register, making it efficient
    to access all values for a specific register.
    """

    @property
    def register_names(self) -> List[str]:
        """List of all register names in the shot data."""
        ...

    @property
    def shots(self) -> int:
        """Number of shots in the data."""
        ...

    def get_integers(self, register: str) -> List[int]:
        """Get values from a register as integers.

        Args:
            register: Name of the register

        Returns:
            List of integer values

        Raises:
            RuntimeError: If register doesn't exist or contains non-integer data
        """
        ...

    def get_binary_strings(self, register: str) -> List[str]:
        """Get values from a register as binary strings.

        Args:
            register: Name of the register

        Returns:
            List of binary string values (e.g., ["0101", "1010"])

        Raises:
            RuntimeError: If register doesn't exist or contains non-bit data
        """
        ...

    def get_decimal_strings(self, register: str) -> List[str]:
        """Get values from a register as decimal strings.

        Args:
            register: Name of the register

        Returns:
            List of decimal string values

        Raises:
            RuntimeError: If register doesn't exist or contains non-bit data
        """
        ...

    def get_hex_strings(self, register: str) -> List[str]:
        """Get values from a register as hexadecimal strings.

        Args:
            register: Name of the register

        Returns:
            List of hex string values

        Raises:
            RuntimeError: If register doesn't exist or contains non-bit data
        """
        ...

    def to_dict(self) -> Dict[str, List[int]]:
        """Convert to a Python dictionary with integer values.

        Returns:
            Dict mapping register names to lists of integer values
        """
        ...

    def to_binary_dict(self) -> Dict[str, List[str]]:
        """Convert to a Python dictionary with binary string values.

        Returns:
            Dict mapping register names to lists of binary strings
        """
        ...

    def __repr__(self) -> str:
        """String representation."""
        ...
