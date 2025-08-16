"""Track quantum array consumption and transformations."""

from __future__ import annotations

from dataclasses import dataclass, field


@dataclass
class ArrayState:
    """Track the state of a quantum array."""

    original_name: str
    current_name: str
    size: int
    consumed_indices: set[int] = field(default_factory=set)
    # Maps original indices to new indices after partial return
    index_mapping: dict[int, int] | None = None
    is_replaced: bool = False  # True if array was replaced by function return


class QuantumArrayTracker:
    """Track quantum array consumption and transformations through function calls."""

    def __init__(self):
        # Map from array name to its current state
        self.arrays: dict[str, ArrayState] = {}
        # Track array replacements: old_name -> new_name
        self.replacements: dict[str, str] = {}

    def register_array(self, name: str, size: int) -> None:
        """Register a new quantum array."""
        self.arrays[name] = ArrayState(
            original_name=name,
            current_name=name,
            size=size,
        )

    def mark_consumed(self, array_name: str, indices: set[int]) -> None:
        """Mark indices as consumed in an array."""
        if array_name in self.arrays:
            self.arrays[array_name].consumed_indices.update(indices)

    def register_partial_return(
        self,
        original_array: str,
        new_array: str,
        remaining_indices: list[int],
    ) -> None:
        """Register that a function returned a partial array.

        Args:
            original_array: Name of the input array
            new_array: Name of the returned array
            remaining_indices: Which indices from original are in the new array
        """
        if original_array not in self.arrays:
            return

        # Mark the original array as replaced
        self.arrays[original_array].is_replaced = True
        self.replacements[original_array] = new_array

        # Create new array state with index mapping
        index_mapping = {
            old_idx: new_idx for new_idx, old_idx in enumerate(remaining_indices)
        }

        self.arrays[new_array] = ArrayState(
            original_name=original_array,
            current_name=new_array,
            size=len(remaining_indices),
            index_mapping=index_mapping,
        )

    def get_current_reference(self, array_name: str, index: int) -> tuple[str, int]:
        """Get the current reference for an array element.

        Returns:
            (current_array_name, current_index)
        """
        # Check if array was replaced
        current_name = array_name
        if array_name in self.replacements:
            current_name = self.replacements[array_name]

        if current_name not in self.arrays:
            return array_name, index

        state = self.arrays[current_name]

        # If there's an index mapping, use it
        if state.index_mapping and index in state.index_mapping:
            return current_name, state.index_mapping[index]

        return current_name, index

    def is_index_consumed(self, array_name: str, index: int) -> bool:
        """Check if a specific index has been consumed."""
        # Follow replacements
        current_name = array_name
        if array_name in self.replacements:
            current_name = self.replacements[array_name]

        if current_name in self.arrays:
            return index in self.arrays[current_name].consumed_indices

        return False

    def get_unconsumed_indices(self, array_name: str) -> set[int]:
        """Get indices that haven't been consumed yet."""
        if array_name not in self.arrays:
            return set()

        state = self.arrays[array_name]
        all_indices = set(range(state.size))
        return all_indices - state.consumed_indices
