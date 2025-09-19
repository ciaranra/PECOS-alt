"""Analyzer for measurement patterns to optimize Guppy code generation."""

from __future__ import annotations

from dataclasses import dataclass, field
from typing import TYPE_CHECKING, Any

if TYPE_CHECKING:
    from pecos.slr import Block


@dataclass
class MeasurementInfo:
    """Information about measurements on a quantum register."""

    qreg_name: str
    qreg_size: int
    measured_indices: set[int] = field(default_factory=set)
    measurement_positions: list[int] = field(default_factory=list)  # Operation indices
    all_measured_together: bool = False
    first_measurement_pos: int = -1
    last_operation_pos: int = -1  # Last operation on this qreg

    def is_fully_measured(self) -> bool:
        """Check if all qubits in the register are measured."""
        return len(self.measured_indices) == self.qreg_size

    def are_measurements_consecutive(self, ops_list) -> bool:
        """Check if all measurements happen consecutively at the end."""
        if not self.measurement_positions:
            return False

        # If measurements are individual (not full register), don't use measure_array
        # This avoids consuming the entire array when we need individual elements
        for pos in self.measurement_positions:
            op = ops_list[pos]
            if hasattr(op, "qargs") and op.qargs:
                for qarg in op.qargs:
                    # If any measurement is on an individual qubit, not the full register
                    if hasattr(qarg, "index"):
                        return False

        # Find the position of first measurement
        first_meas = self.measurement_positions[0]

        # Check if all operations after first measurement are also measurements
        for i in range(first_meas, len(ops_list)):
            op = ops_list[i]
            # Check if this operation involves the quantum register
            if self._is_operation_on_qreg_static(
                op,
                self.qreg_name,
            ) and not self._is_measurement_static(op):
                return False

        return self.is_fully_measured()

    @staticmethod
    def _is_operation_on_qreg_static(op, qreg_name: str) -> bool:
        """Check if an operation involves a specific quantum register."""
        if hasattr(op, "qargs") and op.qargs:
            for qarg in op.qargs:
                if (
                    hasattr(qarg, "reg")
                    and hasattr(qarg.reg, "sym")
                    and qarg.reg.sym == qreg_name
                ):
                    return True
        return False

    @staticmethod
    def _is_measurement_static(op) -> bool:
        """Check if an operation is a measurement."""
        op_type = type(op).__name__
        return op_type == "Measure" or (
            hasattr(op, "is_measurement") and op.is_measurement
        )


class MeasurementAnalyzer:
    """Analyzes measurement patterns in SLR blocks for optimal Guppy generation."""

    def __init__(self):
        self.qreg_info: dict[str, MeasurementInfo] = {}
        self.used_var_names: set[str] = set()

    def analyze_block(
        self,
        block: Block,
        variable_context: dict[str, Any] | None = None,
    ) -> dict[str, MeasurementInfo]:
        """Analyze measurement patterns in a block.

        Args:
            block: The block to analyze
            variable_context: Optional context with variable definitions from parent scope
        """
        self.qreg_info.clear()

        # First, collect all QReg declarations from block vars
        if hasattr(block, "vars"):
            for var in block.vars:
                if type(var).__name__ == "QReg":
                    self.qreg_info[var.sym] = MeasurementInfo(
                        qreg_name=var.sym,
                        qreg_size=var.size,
                    )
                    # Track variable name as used
                    self.used_var_names.add(var.sym)

        # Also check variable context for QRegs used in this block
        if variable_context:
            # Scan operations to find which QRegs are used
            used_qregs = set()
            if hasattr(block, "ops"):
                for op in block.ops:
                    if hasattr(op, "qargs") and op.qargs:
                        for qarg in op.qargs:
                            if hasattr(qarg, "reg") and hasattr(qarg.reg, "sym"):
                                used_qregs.add(qarg.reg.sym)

            # Add QReg info from context for used registers
            for qreg_name in used_qregs:
                if qreg_name in variable_context and qreg_name not in self.qreg_info:
                    var = variable_context[qreg_name]
                    if type(var).__name__ == "QReg" and hasattr(var, "size"):
                        self.qreg_info[qreg_name] = MeasurementInfo(
                            qreg_name=qreg_name,
                            qreg_size=var.size,
                        )
                        self.used_var_names.add(qreg_name)

        # Then analyze operations
        if hasattr(block, "ops"):
            for i, op in enumerate(block.ops):
                self._analyze_operation(op, i)

        # Determine if measurements are all together
        for info in self.qreg_info.values():
            if info.is_fully_measured():
                info.all_measured_together = info.are_measurements_consecutive(
                    block.ops,
                )
                # Debug output
                # print(f"DEBUG: {info.qreg_name} all_measured_together="
                #        f"{info.all_measured_together}, measured_indices="
                #        f"{info.measured_indices}, positions={info.measurement_positions}")

        return self.qreg_info

    def _analyze_operation(self, op, position: int) -> None:
        """Analyze a single operation."""
        op_type = type(op).__name__

        # Check if it's a measurement
        if self._is_measurement(op):
            # Extract quantum register and index
            if hasattr(op, "qargs") and op.qargs:
                for qarg in op.qargs:
                    if hasattr(qarg, "reg") and hasattr(qarg.reg, "sym"):
                        qreg_name = qarg.reg.sym
                        if qreg_name in self.qreg_info:
                            info = self.qreg_info[qreg_name]
                            if hasattr(qarg, "index"):
                                info.measured_indices.add(qarg.index)
                            info.measurement_positions.append(position)
                            if info.first_measurement_pos == -1:
                                info.first_measurement_pos = position
                            info.last_operation_pos = position
        # Track any operation on quantum registers
        elif hasattr(op, "qargs") and op.qargs:
            for qarg in op.qargs:
                if hasattr(qarg, "reg") and hasattr(qarg.reg, "sym"):
                    qreg_name = qarg.reg.sym
                    if qreg_name in self.qreg_info:
                        self.qreg_info[qreg_name].last_operation_pos = position

        # Recurse into nested blocks
        if hasattr(op, "ops"):
            # This is a nested block - analyze it too
            for nested_op in op.ops:
                self._analyze_operation(nested_op, position)

        # Also check else blocks for If statements
        if (
            op_type == "If"
            and hasattr(op, "else_block")
            and op.else_block
            and hasattr(op.else_block, "ops")
        ):
            for nested_op in op.else_block.ops:
                self._analyze_operation(nested_op, position)

    def _is_measurement(self, op) -> bool:
        """Check if an operation is a measurement."""
        op_type = type(op).__name__
        return op_type == "Measure" or (
            hasattr(op, "is_measurement") and op.is_measurement
        )

    def _is_operation_on_qreg(self, op, qreg_name: str) -> bool:
        """Check if an operation involves a specific quantum register."""
        if hasattr(op, "qargs") and op.qargs:
            for qarg in op.qargs:
                if (
                    hasattr(qarg, "reg")
                    and hasattr(qarg.reg, "sym")
                    and qarg.reg.sym == qreg_name
                ):
                    return True
        return False

    def generate_unique_var_name(self, base_name: str, index: int) -> str:
        """Generate a unique variable name that doesn't conflict with existing names."""
        # Start with the pattern: base_name + index
        candidate = f"{base_name}{index}"

        # If it conflicts, add underscores
        while candidate in self.used_var_names:
            candidate = f"_{candidate}"

        self.used_var_names.add(candidate)
        return candidate

    def get_unpacked_var_names(self, qreg_name: str, size: int) -> list[str]:
        """Generate variable names for unpacked qubits."""
        names = []
        for i in range(size):
            name = self.generate_unique_var_name(f"{qreg_name}_", i)
            names.append(name)
        return names
