"""Handler for conditional blocks with resource tracking."""

from __future__ import annotations

from typing import TYPE_CHECKING

if TYPE_CHECKING:
    from pecos.slr import Block
    from pecos.slr.gen_codes.guppy.generator import GuppyGenerator


class ConditionalResourceTracker:
    """Tracks quantum resource consumption across conditional branches."""

    def __init__(self, generator: GuppyGenerator):
        self.generator = generator

    def analyze_if_block_resources(
        self,
        if_block: Block,
    ) -> tuple[dict[str, set[int]], dict[str, set[int]], dict[str, set[int]]]:
        """Analyze resource consumption in If and Else branches.

        Returns:
            (then_consumed, else_consumed, all_used) - dicts mapping qreg_name -> set of indices
        """
        # Analyze Then branch
        then_consumed, then_used = self._analyze_branch_resources(if_block)

        # Analyze Else branch if it exists
        else_consumed = {}
        else_used = {}
        if hasattr(if_block, "else_block") and if_block.else_block:
            else_consumed, else_used = self._analyze_branch_resources(
                if_block.else_block,
            )

        # Combine all used resources
        all_used = {}
        for qreg_name in set(then_used.keys()) | set(else_used.keys()):
            all_used[qreg_name] = then_used.get(qreg_name, set()) | else_used.get(
                qreg_name,
                set(),
            )

        return then_consumed, else_consumed, all_used

    def _analyze_branch_resources(
        self,
        block: Block,
    ) -> tuple[dict[str, set[int]], dict[str, set[int]]]:
        """Analyze resource consumption in a single branch."""
        consumed_qubits = {}
        used_qubits = {}

        if hasattr(block, "ops"):
            for op in block.ops:
                self._analyze_op_resources(op, consumed_qubits, used_qubits)

        return consumed_qubits, used_qubits

    def _analyze_op_resources(
        self,
        op,
        consumed_qubits: dict[str, set[int]],
        used_qubits: dict[str, set[int]],
    ) -> None:
        """Analyze resource usage in a single operation."""
        op_type = type(op).__name__

        # Track quantum register usage
        if hasattr(op, "qargs") and op.qargs:
            for qarg in op.qargs:
                if hasattr(qarg, "reg") and hasattr(qarg.reg, "sym"):
                    qreg_name = qarg.reg.sym
                    if qreg_name not in used_qubits:
                        used_qubits[qreg_name] = set()

                    # Track specific indices
                    if hasattr(qarg, "index"):
                        used_qubits[qreg_name].add(qarg.index)
                    elif hasattr(qarg, "size"):
                        # Full register usage
                        for i in range(qarg.size):
                            used_qubits[qreg_name].add(i)

        # Track measurements (consumption)
        if op_type == "Measure" and hasattr(op, "qargs") and op.qargs:
            for qarg in op.qargs:
                if hasattr(qarg, "reg") and hasattr(qarg.reg, "sym"):
                    qreg_name = qarg.reg.sym
                    if qreg_name not in consumed_qubits:
                        consumed_qubits[qreg_name] = set()

                    # Track specific indices
                    if hasattr(qarg, "index"):
                        consumed_qubits[qreg_name].add(qarg.index)
                    elif hasattr(qarg, "size"):
                        # Full register measurement
                        for i in range(qarg.size):
                            consumed_qubits[qreg_name].add(i)

        # Handle nested If blocks specially - they also need resource balancing
        if op_type == "If":
            # Recursively analyze the If block's branches
            if hasattr(op, "ops"):
                for nested_op in op.ops:
                    self._analyze_op_resources(nested_op, consumed_qubits, used_qubits)
            if (
                hasattr(op, "else_block")
                and op.else_block
                and hasattr(op.else_block, "ops")
            ):
                for nested_op in op.else_block.ops:
                    self._analyze_op_resources(nested_op, consumed_qubits, used_qubits)
        # Recursively analyze other nested blocks
        elif hasattr(op, "ops"):
            for nested_op in op.ops:
                self._analyze_op_resources(nested_op, consumed_qubits, used_qubits)

    def generate_resource_cleanup(self, missing_consumed: dict[str, set[int]]) -> bool:
        """Generate code to consume resources that were not consumed in a branch.

        Returns:
            True if any cleanup code was generated, False otherwise.
        """
        if not missing_consumed:
            return False

        # Filter out already globally consumed qubits
        actually_missing = {}
        for qreg_name, indices in missing_consumed.items():
            already_consumed = self.generator.consumed_qubits.get(qreg_name, set())
            remaining = indices - already_consumed
            if remaining:
                actually_missing[qreg_name] = remaining

        if not actually_missing:
            return False

        self.generator.write("# Consume qubits to maintain linearity across branches")

        for qreg_name in sorted(actually_missing.keys()):
            indices = sorted(actually_missing[qreg_name])

            # Mark these as consumed
            if qreg_name not in self.generator.consumed_qubits:
                self.generator.consumed_qubits[qreg_name] = set()
            self.generator.consumed_qubits[qreg_name].update(indices)

            # Check if we need to consume the entire array
            qreg = self.generator.variable_context.get(qreg_name)
            if (
                qreg
                and hasattr(qreg, "size")
                and len(indices) == qreg.size
                and set(indices) == set(range(qreg.size))
            ):
                # Check if register is already unpacked
                if qreg_name in self.generator.unpacked_arrays:
                    unpacked_info = self.generator.unpacked_arrays[qreg_name]
                    if isinstance(unpacked_info, list):
                        # Already unpacked - measure individually
                        for idx in indices:
                            if idx < len(unpacked_info):
                                self.generator.write(
                                    f"_ = quantum.measure({unpacked_info[idx]})",
                                )
                            else:
                                self.generator.write(
                                    f"_ = quantum.measure({qreg_name}[{idx}])",
                                )
                    else:
                        # Use measure_array
                        self.generator.write(
                            f"_ = quantum.measure_array({qreg_name})",
                        )
                else:
                    # Not unpacked - use measure_array for efficiency
                    self.generator.write(f"_ = quantum.measure_array({qreg_name})")
                continue

            # Partial consumption - need to handle individual qubits
            # Check if this register is unpacked
            if qreg_name in self.generator.unpacked_arrays:
                unpacked_names = self.generator.unpacked_arrays[qreg_name]
                if isinstance(unpacked_names, list):
                    # Use unpacked names
                    for idx in indices:
                        if idx < len(unpacked_names):
                            self.generator.write(
                                f"_ = quantum.measure({unpacked_names[idx]})",
                            )
                        else:
                            self.generator.write(
                                f"_ = quantum.measure({qreg_name}[{idx}])",
                            )
                # Check if we need to unpack first
                elif not unpacked_names.startswith("__measure_array"):
                    # Not a special marker - use standard indexing
                    for idx in indices:
                        self.generator.write(
                            f"_ = quantum.measure({qreg_name}[{idx}])",
                        )
                else:
                    # This was marked for measure_array but we need partial
                    # We need to unpack it first
                    self._unpack_for_partial_access(qreg_name, indices)
            # Not unpacked - check if we should unpack for partial access
            elif self._should_unpack_for_cleanup(qreg_name, indices):
                self._unpack_for_partial_access(qreg_name, indices)
            else:
                # Use standard array indexing
                for idx in indices:
                    self.generator.write(f"_ = quantum.measure({qreg_name}[{idx}])")

        return True

    def _should_unpack_for_cleanup(self, qreg_name: str, indices: list) -> bool:
        """Check if we should unpack an array for cleanup access."""
        _ = qreg_name  # Reserved for future use
        _ = indices  # Reserved for future use
        # For now, don't unpack in cleanup - let HUGR handle it or fail clearly
        # This avoids the MoveOutOfSubscriptError
        return False

    def _unpack_for_partial_access(self, qreg_name: str, indices: list) -> None:
        """Unpack an array for partial access and measure specific indices."""
        qreg = self.generator.variable_context.get(qreg_name)
        if not qreg or not hasattr(qreg, "size"):
            # Fallback to individual access
            for idx in indices:
                self.generator.write(f"_ = quantum.measure({qreg_name}[{idx}])")
            return

        # Generate unpacking
        size = qreg.size
        unpacked_names = [f"{qreg_name}_{i}" for i in range(size)]

        self.generator.write(f"# Unpack {qreg_name} for partial measurement")
        if len(unpacked_names) == 1:
            self.generator.write(f"{unpacked_names[0]}, = {qreg_name}")
        else:
            self.generator.write(f"{', '.join(unpacked_names)} = {qreg_name}")

        # Store unpacking info
        self.generator.unpacked_arrays[qreg_name] = unpacked_names

        # Now measure the specific indices
        for idx in indices:
            if idx < len(unpacked_names):
                self.generator.write(f"_ = quantum.measure({unpacked_names[idx]})")

    def ensure_branches_consume_same_resources(self, if_block: Block) -> None:
        """Ensure both branches of an If block consume the same quantum resources."""
        # Analyze resource consumption
        then_consumed, else_consumed, _all_used = self.analyze_if_block_resources(
            if_block,
        )

        # Find resources consumed in one branch but not the other
        then_only = {}
        else_only = {}

        for qreg_name in set(then_consumed.keys()) | set(else_consumed.keys()):
            then_indices = then_consumed.get(qreg_name, set())
            else_indices = else_consumed.get(qreg_name, set())

            # Resources consumed in then but not else
            diff = then_indices - else_indices
            if diff:
                else_only[qreg_name] = diff

            # Resources consumed in else but not then
            diff = else_indices - then_indices
            if diff:
                then_only[qreg_name] = diff

        return then_only, else_only
