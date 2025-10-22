"""Main Guppy generator class."""

from __future__ import annotations

from typing import TYPE_CHECKING

from pecos.slr.gen_codes.generator import Generator
from pecos.slr.gen_codes.guppy.block_handler import BlockHandler
from pecos.slr.gen_codes.guppy.dependency_analyzer import DependencyAnalyzer
from pecos.slr.gen_codes.guppy.expression_handler import ExpressionHandler
from pecos.slr.gen_codes.guppy.measurement_analyzer import MeasurementAnalyzer
from pecos.slr.gen_codes.guppy.operation_handler import OperationHandler

if TYPE_CHECKING:
    from pecos.slr import Block


class GuppyGenerator(Generator):
    """Generator that converts SLR programs to Guppy code."""

    def __init__(self):
        """Initialize the Guppy generator."""
        self.output = []
        self.indent_level = 0
        self.current_scope = None
        self.quantum_ops_used = set()
        self.var_types = {}  # Track variable types
        self.pending_functions = []  # Track functions to be generated

        # Initialize handlers
        self.block_handler = BlockHandler(self)
        self.operation_handler = OperationHandler(self)
        self.expression_handler = ExpressionHandler(self)
        self.dependency_analyzer = DependencyAnalyzer()
        self.measurement_analyzer = MeasurementAnalyzer()

        # Track variable context for dependency analysis
        self.variable_context = {}

        # Track array unpacking state
        self.unpacked_arrays = {}  # qreg_name -> list of unpacked var names
        self.measurement_info = {}  # Measurement analysis results

        # Track consumed quantum resources globally
        self.consumed_qubits = {}  # qreg_name -> set of consumed indices

        # Track array transformations from function returns
        self.array_replacements = {}  # original_name -> replacement_name
        self.partial_returns = {}  # Maps function returns to original arrays

    def write(self, line: str) -> None:
        """Write a line with proper indentation."""
        if line:
            self.output.append("    " * self.indent_level + line)
        else:
            self.output.append("")

    def indent(self) -> None:
        """Increase indentation level."""
        self.indent_level += 1

    def dedent(self) -> None:
        """Decrease indentation level."""
        self.indent_level = max(0, self.indent_level - 1)

    def get_output(self) -> str:
        """Get the generated Guppy code."""
        # Generate any pending functions
        while self.pending_functions:
            item = self.pending_functions.pop(0)
            if len(item) == 3:
                # Old format: (block_type, func_name, sample_block)
                block_type, func_name, sample_block = item
                self._generate_function_definition(block_type, func_name, sample_block)
            else:
                # New format: (block_key, func_name, sample_block, block_name)
                _block_key, func_name, sample_block, block_name = item
                self._generate_function_definition_by_info(
                    func_name,
                    sample_block,
                    block_name,
                )

        # Add imports at the beginning
        imports = [
            "from __future__ import annotations",
            "",
            "from guppylang.decorator import guppy",
            "from guppylang.std import quantum",
            "from guppylang.std.builtins import array, owned, result",
        ]

        # Add any additional imports needed
        if self.quantum_ops_used:
            imports.append("")

        return "\n".join([*imports, "", "", *self.output])

    def generate_block(self, block: Block) -> None:
        """Generate Guppy code for a block."""
        self.block_handler.handle_block(block)

    def enter_block(self, block) -> tuple:
        """Enter a new block scope."""
        previous_scope = self.current_scope
        previous_unpacked = self.unpacked_arrays.copy()
        previous_measurement_info = self.measurement_info.copy()
        previous_consumed = self.consumed_qubits.copy()

        self.current_scope = block
        # Clear unpacked arrays for new scope
        self.unpacked_arrays = {}
        self.measurement_info = {}

        # Don't clear consumed_qubits for If/Else blocks - we want to track globally
        block_type = type(block).__name__
        if block_type not in ["If", "Else"]:
            # For functions, clear consumed qubits
            self.consumed_qubits = {}

        return (
            previous_scope,
            previous_unpacked,
            previous_measurement_info,
            previous_consumed,
        )

    def exit_block(self, previous_state) -> None:
        """Exit the current block scope."""
        if isinstance(previous_state, tuple):
            if len(previous_state) == 4:
                (
                    previous_scope,
                    previous_unpacked,
                    previous_measurement_info,
                    previous_consumed,
                ) = previous_state
                self.current_scope = previous_scope
                self.unpacked_arrays = previous_unpacked
                self.measurement_info = previous_measurement_info
                # Restore consumed qubits for functions, but merge for If/Else
                current_block_type = (
                    type(self.current_scope).__name__ if self.current_scope else None
                )
                if current_block_type not in ["If", "Else", "Main"]:
                    self.consumed_qubits = previous_consumed
            else:
                # Old format
                previous_scope, previous_unpacked, previous_measurement_info = (
                    previous_state
                )
                self.current_scope = previous_scope
                self.unpacked_arrays = previous_unpacked
                self.measurement_info = previous_measurement_info
        else:
            # Backward compatibility
            self.current_scope = previous_state

    def _generate_function_definition(
        self,
        block_type: type,
        func_name: str,
        sample_block: Block,
    ) -> None:
        """Generate a function definition for a block type."""
        _ = block_type  # Reserved for future use (e.g., type-specific generation)
        # Add spacing before function
        self.write("")
        self.write("")
        self.write("@guppy")

        # Determine function parameters from the sample block
        params = self._get_function_parameters(sample_block)
        param_str = ", ".join(params) if params else ""

        self.write(f"def {func_name}({param_str}) -> None:")
        self.indent()

        # Generate the function body from the block's operations
        if hasattr(sample_block, "ops") and sample_block.ops:
            for op in sample_block.ops:
                self.operation_handler.generate_op(op)
        else:
            self.write("pass")

        self.dedent()

    def analyze_quantum_resource_flow(
        self,
        block: Block,
    ) -> tuple[dict[str, set[int]], dict[str, set[int]]]:
        """Analyze which quantum resources are consumed and which need to be returned.

        Returns:
            (consumed_qubits, live_qubits) - dicts mapping qreg_name -> set of indices
        """
        consumed_qubits = {}  # qreg_name -> set of consumed indices
        used_qubits = {}  # qreg_name -> set of used indices

        # First, check which quantum registers are parameters by looking at variable context
        # We need to mark all input quantum array qubits as "used"
        dep_info = self.dependency_analyzer.analyze_block(block)
        for var_name in dep_info.used_variables:
            if var_name in self.variable_context:
                var = self.variable_context[var_name]
                if type(var).__name__ == "QReg" and hasattr(var, "size"):
                    if var_name not in used_qubits:
                        used_qubits[var_name] = set()
                    # Mark all qubits in the array as used
                    for i in range(var.size):
                        used_qubits[var_name].add(i)

        def analyze_op(op):
            op_type = type(op).__name__

            # Track quantum register usage
            if hasattr(op, "qargs") and op.qargs:
                for qarg in op.qargs:
                    if hasattr(qarg, "reg") and hasattr(qarg.reg, "sym"):
                        qreg_name = qarg.reg.sym
                        if qreg_name not in used_qubits:
                            used_qubits[qreg_name] = set()

                        # Track specific indices if available
                        if hasattr(qarg, "index"):
                            used_qubits[qreg_name].add(qarg.index)
                        elif hasattr(qarg, "size"):
                            # Full register usage
                            for i in range(qarg.size):
                                used_qubits[qreg_name].add(i)

            # Track measurements (consumption)
            if op_type == "Measure" and hasattr(op, "qargs") and op.qargs:
                for qarg in op.qargs:
                    # Handle full register measurement (qarg is the register itself)
                    if hasattr(qarg, "sym") and hasattr(qarg, "size"):
                        qreg_name = qarg.sym
                        if qreg_name not in consumed_qubits:
                            consumed_qubits[qreg_name] = set()
                        # Mark all qubits as consumed
                        for i in range(qarg.size):
                            consumed_qubits[qreg_name].add(i)
                    # Handle individual qubit measurement
                    elif hasattr(qarg, "reg") and hasattr(qarg.reg, "sym"):
                        qreg_name = qarg.reg.sym
                        if qreg_name not in consumed_qubits:
                            consumed_qubits[qreg_name] = set()

                        # Track specific indices if available
                        if hasattr(qarg, "index"):
                            consumed_qubits[qreg_name].add(qarg.index)

            # Recursively analyze nested blocks
            if hasattr(op, "ops"):
                for nested_op in op.ops:
                    analyze_op(nested_op)

        # Analyze all operations
        if hasattr(block, "ops"):
            for op in block.ops:
                analyze_op(op)

        # Calculate live qubits (used but not consumed)
        live_qubits = {}
        for qreg_name, used_indices in used_qubits.items():
            consumed_indices = consumed_qubits.get(qreg_name, set())
            live_indices = used_indices - consumed_indices
            if live_indices:
                live_qubits[qreg_name] = live_indices

        return consumed_qubits, live_qubits

    def _get_function_parameters(self, block: Block) -> list[str]:
        """Determine function parameters from a block using dependency analysis."""
        # Use dependency analyzer to find all variables used in the block
        dep_info = self.dependency_analyzer.analyze_block(block)

        # Analyze quantum resource flow
        consumed_qubits, live_qubits = self._analyze_quantum_resource_flow(block)

        params = []
        param_set = set()

        # Get parameters based on used variables
        for var_name in sorted(dep_info.used_variables):
            if var_name in self.variable_context:
                var = self.variable_context[var_name]
                var_type_name = type(var).__name__

                if var_type_name == "QReg":
                    size = var.size if hasattr(var, "size") else 1
                    # Add @owned if this QReg is modified (used at all means modified in quantum)
                    if var_name in consumed_qubits or var_name in live_qubits:
                        params.append(
                            f"{var_name}: array[quantum.qubit, {size}] @owned",
                        )
                    else:
                        params.append(f"{var_name}: array[quantum.qubit, {size}]")
                    param_set.add(var_name)
                elif var_type_name == "CReg":
                    size = var.size if hasattr(var, "size") else 1
                    params.append(f"{var_name}: array[bool, {size}]")
                    param_set.add(var_name)
                else:
                    params.append(f"{var_name}: {var_type_name}")
                    param_set.add(var_name)

        # Also check if the block has a parent object for additional context
        # NOTE: We access _parent_obj which is a private attribute from pecos.slr
        # This is necessary to get the full context of nested blocks, but should
        # be replaced with a public API if one becomes available
        if hasattr(block, "_parent_obj"):
            parent = getattr(block, "_parent_obj")
            if hasattr(parent, "vars"):
                for var in parent.vars:
                    if hasattr(var, "sym") and var.sym not in param_set:
                        # Add type annotation based on variable type
                        var_type_name = type(var).__name__
                        if var_type_name == "QReg":
                            size = var.size if hasattr(var, "size") else 1
                            params.append(f"{var.sym}: array[quantum.qubit, {size}]")
                            param_set.add(var.sym)
                        elif var_type_name == "CReg":
                            size = var.size if hasattr(var, "size") else 1
                            params.append(f"{var.sym}: array[bool, {size}]")
                            param_set.add(var.sym)
                        else:
                            params.append(var.sym)
                            param_set.add(var.sym)

        # If no parent object, analyze the operations to find used registers
        if not params and hasattr(block, "ops"):
            for op in block.ops:
                # Check for qubit arguments in operations
                if hasattr(op, "qargs"):
                    for qarg in op.qargs:
                        if hasattr(qarg, "reg") and hasattr(qarg.reg, "sym"):
                            reg_name = qarg.reg.sym
                            if reg_name not in param_set:
                                # Try to get size from the register
                                size = qarg.reg.size if hasattr(qarg.reg, "size") else 1
                                params.append(
                                    f"{reg_name}: array[quantum.qubit, {size}]",
                                )
                                param_set.add(reg_name)
                # Check for classical bit arguments (e.g., in Measure operations)
                if hasattr(op, "cargs"):
                    for carg in op.cargs:
                        if hasattr(carg, "reg") and hasattr(carg.reg, "sym"):
                            reg_name = carg.reg.sym
                            if reg_name not in param_set:
                                # Try to get size from the register
                                size = carg.reg.size if hasattr(carg.reg, "size") else 1
                                params.append(f"{reg_name}: array[bool, {size}]")
                                param_set.add(reg_name)
                # Recursively check nested blocks (like If blocks)
                if hasattr(op, "ops"):
                    nested_block_params = self._get_function_parameters(op)
                    for param in nested_block_params:
                        param_name = param.split(":")[0].strip()
                        if param_name not in param_set:
                            params.append(param)
                            param_set.add(param_name)

        return params

    def _generate_function_definition_by_info(
        self,
        func_name: str,
        sample_block: Block,
        block_name: str,
    ) -> None:
        """Generate a function definition using block info."""
        # Add spacing before function
        self.write("")
        self.write("")
        self.write("@guppy")

        # Determine function parameters from the sample block
        params = self._get_function_parameters(sample_block)
        param_str = ", ".join(params) if params else ""

        # Analyze quantum resource flow to determine return type
        _consumed_qubits, live_qubits = self._analyze_quantum_resource_flow(
            sample_block,
        )
        # Debug output
        # print(f"DEBUG: Function {func_name} - consumed: {consumed_qubits}, live: {live_qubits}")

        # Build return type based on what quantum resources need to be returned
        return_types = []
        return_info = []  # Track what needs to be returned

        for qreg_name in sorted(live_qubits.keys()):
            if qreg_name in self.variable_context:
                var = self.variable_context[qreg_name]
                if hasattr(var, "size"):
                    qreg_size = var.size
                    live_indices = live_qubits[qreg_name]

                    # Check if entire register needs to be returned
                    if len(live_indices) == qreg_size:
                        # Return entire array
                        return_types.append(f"array[quantum.qubit, {qreg_size}]")
                        return_info.append((qreg_name, "full"))
                    else:
                        # For partial arrays, return only the unconsumed qubits
                        num_live = len(live_indices)
                        if num_live > 0:
                            return_types.append(f"array[quantum.qubit, {num_live}]")
                            return_info.append((qreg_name, "partial", live_indices))

        if return_types:
            return_type = (
                return_types[0]
                if len(return_types) == 1
                else f"tuple[{', '.join(return_types)}]"
            )
        else:
            return_type = "None"

        self.write(f"def {func_name}({param_str}) -> {return_type}:")
        self.indent()
        self.write(f'"""Generated from {block_name} block."""')

        # Enter the function scope
        prev_state = self.enter_block(sample_block)

        # Set up variable context for function parameters
        # This is needed for measurement analysis and unpacking
        for param in params:
            if ":" in param:
                var_name = param.split(":")[0].strip()
                # Try to find the variable in the global context
                if var_name in self.variable_context:
                    # Keep the variable reference for this function scope
                    pass  # Variable context is already shared

        # Analyze measurement patterns for this function
        self.measurement_info = self.measurement_analyzer.analyze_block(
            sample_block,
            self.variable_context,
        )

        # Generate the function body from the block's operations
        if hasattr(sample_block, "ops") and sample_block.ops:
            for i, op in enumerate(sample_block.ops):
                self.operation_handler.generate_op(op, position=i)
        else:
            self.write("pass")

        # Exit the function scope
        self.exit_block(prev_state)

        # Generate return statement for live quantum resources
        if return_info:
            return_values = []
            for info in return_info:
                if len(info) == 2:
                    qreg_name, return_type = info
                    return_values.append(qreg_name)
                else:
                    qreg_name, return_type, live_indices = info
                    # For partial consumption, construct array with only live qubits
                    sorted_indices = sorted(live_indices)

                    # Check if we have unpacked the array
                    if qreg_name in self.unpacked_arrays:
                        unpacked_names = self.unpacked_arrays[qreg_name]
                        if isinstance(unpacked_names, list):
                            # Build array from the live unpacked variables
                            live_vars = [
                                unpacked_names[i]
                                for i in sorted_indices
                                if i < len(unpacked_names)
                            ]
                            if live_vars:
                                array_expr = f"array({', '.join(live_vars)})"
                                return_values.append(array_expr)
                            else:
                                # Fallback to array indexing
                                elements = [f"{qreg_name}[{i}]" for i in sorted_indices]
                                array_expr = f"array({', '.join(elements)})"
                                return_values.append(array_expr)
                        else:
                            # Use array indexing
                            elements = [f"{qreg_name}[{i}]" for i in sorted_indices]
                            array_expr = f"array({', '.join(elements)})"
                            return_values.append(array_expr)
                    else:
                        # Use array indexing
                        elements = [f"{qreg_name}[{i}]" for i in sorted_indices]
                        array_expr = f"array({', '.join(elements)})"
                        return_values.append(array_expr)

            if len(return_values) == 1:
                self.write(f"return {return_values[0]}")
            else:
                self.write(f"return {', '.join(return_values)}")

        self.dedent()
