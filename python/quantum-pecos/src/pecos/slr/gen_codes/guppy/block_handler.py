"""Handler for SLR blocks - converts blocks to control flow or functions."""

from __future__ import annotations

from typing import TYPE_CHECKING, ClassVar

if TYPE_CHECKING:
    from pecos.slr import Block
    from pecos.slr.gen_codes.guppy.generator import GuppyGenerator

from pecos.slr.gen_codes.guppy.naming import get_function_name


class BlockHandler:
    """Handles conversion of SLR blocks to Guppy code."""

    # Core blocks that should remain as control flow
    CORE_BLOCKS: ClassVar[set[str]] = {"If", "Repeat", "While", "Main", "Block"}

    def __init__(self, generator: GuppyGenerator):
        self.generator = generator
        # Track which block functions have been generated
        self.generated_functions: set[str] = set()
        # Map from block type to function name
        self.block_to_function_name: dict[type, str] = {}

    def handle_block(self, block: Block) -> None:
        """Handle a block of operations."""
        previous_scope = self.generator.enter_block(block)

        block_name = type(block).__name__
        # print(f"DEBUG: handle_block called for {block_name}")

        # Check if this block has a custom handler
        handler_method = f"_handle_{block_name.lower()}_block"
        if hasattr(self, handler_method):
            # print(f"DEBUG: Using custom handler {handler_method}")
            getattr(self, handler_method)(block)
        else:
            # print(f"DEBUG: Using generic handler for {block_name}")
            # Default handling for unknown blocks
            self._handle_generic_block(block)

        self.generator.exit_block(previous_scope)

    def _handle_main_block(self, block) -> None:
        """Handle Main block - generates the main function."""
        self.generator.write("@guppy")
        self.generator.write("def main() -> None:")
        self.generator.indent()

        # Analyze measurement patterns before generating code
        self.generator.measurement_info = (
            self.generator.measurement_analyzer.analyze_block(
                block,
                self.generator.variable_context,
            )
        )

        # Generate variable declarations and track in context
        for var in block.vars:
            self._generate_var_declaration(var)
            # Track variable in context for dependency analysis
            if hasattr(var, "sym"):
                self.generator.variable_context[var.sym] = var

        # Generate operations
        if block.ops:
            # print(f"DEBUG: Main block has {len(block.ops)} operations")
            for i, op in enumerate(block.ops):
                # print(f"DEBUG: Main op {i}: type={type(op).__name__}, has block_name={hasattr(op, 'block_name')}")
                self.generator.operation_handler.generate_op(op, position=i)

        # Handle repacking of measured values if needed
        self._handle_measurement_results(block)

        # Handle unconsumed quantum resources
        self._handle_unconsumed_qubits(block)

        # Generate result() call with all classical registers
        creg_names = []
        self._collect_all_cregs(block.vars, creg_names)

        if creg_names:
            # Generate result() calls with string labels
            for creg_name in creg_names:
                # Get the actual variable name (might be renamed)
                actual_var_name = creg_name
                if (
                    hasattr(self.generator, "renamed_vars")
                    and creg_name in self.generator.renamed_vars
                ):
                    actual_var_name = self.generator.renamed_vars[creg_name]

                # Use original name as label, actual variable name in the call
                self.generator.write(f'result("{creg_name}", {actual_var_name})')
        elif not block.ops:
            # Empty function body needs pass
            self.generator.write("pass")

        self.generator.dedent()
        # Add blank line after main function if there are pending functions
        if self.generator.pending_functions:
            self.generator.write("")

    def _handle_if_block(self, block) -> None:
        """Handle If block - generates conditional with resource tracking."""
        from pecos.slr.gen_codes.guppy.conditional_handler import (
            ConditionalResourceTracker,
        )

        tracker = ConditionalResourceTracker(self.generator)
        cond = self.generator.expression_handler.generate_condition(block.cond)

        # Analyze resource consumption in both branches
        then_only, else_only = tracker.ensure_branches_consume_same_resources(block)

        # Generate if statement
        self.generator.write(f"if {cond}:")
        self.generator.indent()

        if not block.ops:
            self.generator.write("pass")
        else:
            for op in block.ops:
                self.generator.operation_handler.generate_op(op)

        # Add cleanup for resources not consumed in then branch
        if then_only:
            tracker.generate_resource_cleanup(then_only)

        self.generator.dedent()

        # Generate else block if needed
        if hasattr(block, "else_block") and block.else_block:
            self.generator.write("else:")
            self.generator.indent()

            has_ops = block.else_block.ops if block.else_block.ops else False
            if has_ops:
                for op in block.else_block.ops:
                    self.generator.operation_handler.generate_op(op)

            # Add cleanup for resources not consumed in else branch
            cleanup_generated = False
            if else_only:
                cleanup_generated = tracker.generate_resource_cleanup(else_only)

            # If no ops and no cleanup, add pass
            if not has_ops and not cleanup_generated:
                self.generator.write("pass")

            self.generator.dedent()
        elif else_only:
            # No explicit else block but we need to consume resources
            self.generator.write("else:")
            self.generator.indent()

            # Generate cleanup and check if anything was generated
            cleanup_generated = tracker.generate_resource_cleanup(else_only)

            # If no cleanup was generated, add pass
            if not cleanup_generated:
                self.generator.write("pass")

            self.generator.dedent()

    def _handle_repeat_block(self, block) -> None:
        """Handle Repeat block - generates for loop."""
        # Repeat blocks store their count in cond
        limit = block.cond if hasattr(block, "cond") else 1
        self.generator.write(f"for _ in range({limit}):")
        self.generator.indent()

        if not block.ops:
            self.generator.write("pass")
        else:
            for op in block.ops:
                self.generator.operation_handler.generate_op(op)

        self.generator.dedent()

    def _handle_block_block(self, block) -> None:
        """Handle plain Block - just inline the operations."""
        if hasattr(block, "ops"):
            for op in block.ops:
                self.generator.operation_handler.generate_op(op)

    def _handle_generic_block(self, block) -> None:
        """Handle generic/unknown blocks by converting to function calls."""
        block_type = type(block)
        block_name = block_type.__name__

        # Use preserved block name if available
        original_block_name = getattr(block, "block_name", block_name)
        original_block_module = getattr(block, "block_module", block_type.__module__)

        # Debug: print block info
        # print(f"DEBUG: Handling block {block_name}, original: {original_block_name}, module: {original_block_module}")

        # Check if this is a core block that should be inlined
        if original_block_name in self.CORE_BLOCKS:
            # Process inline for core blocks
            if hasattr(block, "ops"):
                for op in block.ops:
                    self.generator.operation_handler.generate_op(op)
            return

        # For non-core blocks, generate a function call
        # Create a composite key using original block info
        # For Parallel blocks, include content hash to differentiate blocks with different operations
        if original_block_name == "Parallel" and hasattr(block, "ops"):
            content_hash = self._get_block_content_hash(block)
            block_key = (original_block_name, original_block_module, content_hash)
        else:
            block_key = (original_block_name, original_block_module)
        func_name = self._get_or_create_function_name_by_info(
            block_key,
            original_block_name,
            original_block_module,
        )

        # Generate the function if it hasn't been generated yet
        # Use block_key for deduplication to handle Parallel blocks with different content
        if block_key not in self.generated_functions:
            self._generate_block_function_by_info(
                block_key,
                func_name,
                block,
                original_block_name,
            )
            self.generated_functions.add(block_key)

        # Generate the function call
        # DEBUG: print(f"DEBUG: Generating call to function: {func_name}")
        self._generate_function_call(func_name, block)

    def _generate_var_declaration(self, var) -> None:
        """Generate variable declarations."""
        var_type = type(var).__name__

        # Reserved names that shouldn't be used as variables
        reserved_names = {"result", "array", "quantum", "guppy", "owned"}

        # Get the variable name, potentially with suffix to avoid conflicts
        var_name = var.sym
        if var_name in reserved_names:
            var_name = f"{var.sym}_reg"
            # Store mapping for later use
            if not hasattr(self.generator, "renamed_vars"):
                self.generator.renamed_vars = {}
            self.generator.renamed_vars[var.sym] = var_name

        if var_type == "QReg":
            self.generator.var_types[var_name] = "quantum"
            self.generator.write(
                f"{var_name} = array(quantum.qubit() for _ in range({var.size}))",
            )
        elif var_type == "CReg":
            self.generator.var_types[var_name] = "classical"
            self.generator.write(
                f"{var_name} = array(False for _ in range({var.size}))",
            )
        else:
            # For any other variable types, check if they have standard attributes
            if hasattr(var, "vars"):
                # This is a complex type with sub-variables (like Steane)
                # Generate declarations for all sub-variables
                for sub_var in var.vars:
                    self._generate_var_declaration(sub_var)
            else:
                # Unknown variable type
                var_name = var.sym if hasattr(var, "sym") else str(var)
                self.generator.write(
                    f"# TODO: Initialize {var_type} instance '{var_name}'",
                )
                self.generator.write(f"# Unknown variable type: {var_type}")

    def _get_or_create_function_name(self, block_type: type) -> str:
        """Get or create a function name for a block type."""
        if block_type not in self.block_to_function_name:
            func_name = get_function_name(block_type, use_module_prefix=True)
            self.block_to_function_name[block_type] = func_name
        return self.block_to_function_name[block_type]

    def _generate_block_function(
        self,
        block_type: type,
        func_name: str,
        sample_block: Block,
    ) -> None:
        """Generate a function definition for a block type."""
        # Add the function to pending functions to be generated later
        self.generator.pending_functions.append((block_type, func_name, sample_block))

    def _get_or_create_function_name_by_info(
        self,
        block_key: tuple,
        block_name: str,
        block_module: str,
    ) -> str:
        """Get or create a function name using block info."""
        if block_key not in self.block_to_function_name:
            # Use the naming utility directly with the block name
            from pecos.slr.gen_codes.guppy.naming import (
                class_to_function_name,
                get_module_prefix,
            )

            # Get base function name
            base_name = class_to_function_name(block_name)

            # Get module prefix if needed
            # Create a mock class just for module prefix extraction
            class MockBlockClass:
                __name__ = block_name
                __module__ = block_module

            prefix = get_module_prefix(MockBlockClass)
            func_name = (
                prefix + base_name
                if prefix and not base_name.startswith(prefix.rstrip("_"))
                else base_name
            )

            # For Parallel blocks with content hash, append the hash to make unique names
            if len(block_key) > 2 and block_name == "Parallel":
                content_hash = block_key[2]
                # Create a more readable suffix from the hash
                # e.g., "H_H" becomes "_h", "X_X" becomes "_x"
                if content_hash:
                    gates = content_hash.split("_")
                    if all(g == gates[0] for g in gates):
                        # All gates are the same type
                        func_name += f"_{gates[0].lower()}"
                    else:
                        # Mixed gates - use first letter of each
                        suffix = "_".join(g[0].lower() for g in gates[:3])  # Limit to 3
                        func_name += f"_{suffix}"

            self.block_to_function_name[block_key] = func_name
        return self.block_to_function_name[block_key]

    def _generate_block_function_by_info(
        self,
        block_key: tuple,
        func_name: str,
        sample_block: Block,
        block_name: str,
    ) -> None:
        """Generate a function definition using block info."""
        # Add the function to pending functions to be generated later
        self.generator.pending_functions.append(
            (block_key, func_name, sample_block, block_name),
        )

    def _generate_function_call(self, func_name: str, block: Block) -> None:
        """Generate a function call for a block."""
        # Use dependency analyzer to find all required arguments
        dep_info = self.generator.dependency_analyzer.analyze_block(block)

        args = []
        args_set = set()

        # Get arguments based on used variables (same logic as parameter detection)
        for var_name in sorted(dep_info.used_variables):
            if var_name in self.generator.variable_context and var_name not in args_set:
                args.append(var_name)
                args_set.add(var_name)

        # Analyze quantum resource flow to see what will be returned
        consumed_qregs, live_qregs = self.generator.analyze_quantum_resource_flow(
            block,
        )

        # Mark consumed quantum resources as consumed in the current scope too
        for qreg_name, indices in consumed_qregs.items():
            if qreg_name not in self.generator.consumed_qubits:
                self.generator.consumed_qubits[qreg_name] = set()
            self.generator.consumed_qubits[qreg_name].update(indices)

        # Generate the function call with return value handling
        call_expr = f"{func_name}({', '.join(args)})" if args else f"{func_name}()"

        if live_qregs:
            # Function returns quantum resources - need to capture them
            return_vars = []
            for qreg_name in sorted(live_qregs.keys()):
                live_indices = live_qregs[qreg_name]
                if qreg_name in self.generator.variable_context:
                    var = self.generator.variable_context[qreg_name]
                    if hasattr(var, "size"):
                        # Check if partial or full return
                        if len(live_indices) == var.size:
                            # Full return - use same variable name
                            return_vars.append(qreg_name)
                        else:
                            # Partial return - create new variable name
                            partial_var_name = f"{qreg_name}_remaining"
                            return_vars.append(partial_var_name)

            if len(return_vars) == 1:
                # Single return value
                self.generator.write(f"{return_vars[0]} = {call_expr}")

                # If this was a partial return, we need to handle the remaining qubits
                if return_vars[0].endswith("_remaining"):
                    # The original array name
                    return_vars[0].replace("_remaining", "")
                    # We'll need to update references to the unconsumed indices
                    # This is complex and needs more work
            else:
                # Multiple return values
                self.generator.write(f"{', '.join(return_vars)} = {call_expr}")
        else:
            # No return value
            self.generator.write(call_expr)

    def _collect_register_args(self, block, args: list, args_set: set) -> None:
        """Recursively collect register arguments from a block."""
        if hasattr(block, "ops"):
            for op in block.ops:
                # Check for qubit arguments
                if hasattr(op, "qargs"):
                    for qarg in op.qargs:
                        if hasattr(qarg, "reg") and hasattr(qarg.reg, "sym"):
                            reg_name = qarg.reg.sym
                            if reg_name not in args_set:
                                args.append(reg_name)
                                args_set.add(reg_name)
                # Check for classical bit arguments
                if hasattr(op, "cargs"):
                    for carg in op.cargs:
                        if hasattr(carg, "reg") and hasattr(carg.reg, "sym"):
                            reg_name = carg.reg.sym
                            if reg_name not in args_set:
                                args.append(reg_name)
                                args_set.add(reg_name)
                # Recurse into nested blocks
                if hasattr(op, "ops"):
                    self._collect_register_args(op, args, args_set)

    def _handle_measurement_results(self, block) -> None:
        """Handle packing of individual measurement results into CReg arrays if needed."""
        # Check if we have any individual measurements to pack
        if not hasattr(self.generator.operation_handler, "individual_measurements"):
            return

        individual_measurements = (
            self.generator.operation_handler.individual_measurements
        )
        if not individual_measurements:
            return

        # Get CReg info from block variables
        creg_info = {}
        for var in block.vars:
            if type(var).__name__ == "CReg" and hasattr(var, "sym"):
                creg_info[var.sym] = var.size if hasattr(var, "size") else 1

        # Check which CRegs were handled by measure_array
        handled_by_measure_array = set()
        for unpacked_info in self.generator.unpacked_arrays.values():
            if isinstance(unpacked_info, str) and unpacked_info.startswith(
                "__measure_array",
            ):
                # Find the associated CReg (this is a simplification - in practice might need better tracking)
                # For now, we'll skip packing for any CReg that might have been handled
                handled_by_measure_array.update(creg_info.keys())

        # Generate packing code for each CReg that had individual measurements
        has_packing = False
        for creg_name, measurements in individual_measurements.items():
            if creg_name in creg_info and creg_name not in handled_by_measure_array:
                creg_size = creg_info[creg_name]
                # Check if we have all measurements for this CReg
                if len(measurements) == creg_size:
                    if not has_packing:
                        self.generator.write("")
                        self.generator.write("# Pack measurement results")
                        has_packing = True

                    # Sort by index to ensure correct order
                    sorted_vars = []
                    for i in range(creg_size):
                        if i in measurements:
                            sorted_vars.append(measurements[i])
                        else:
                            # This shouldn't happen if analysis is correct
                            sorted_vars.append("False")  # Default value

                    self.generator.write(
                        f"{creg_name} = array({', '.join(sorted_vars)})",
                    )

    def _get_block_content_hash(self, block) -> str:
        """Get a hash of block operations for differentiation.

        This is used to differentiate Parallel blocks with different operations.
        """
        ops_summary = []
        if hasattr(block, "ops"):
            for op in block.ops:
                op_type = type(op).__name__
                # Include gate types to differentiate
                ops_summary.append(op_type)

        # Create a simple hash from operation types
        return "_".join(sorted(ops_summary)) if ops_summary else "empty"

    def _handle_unconsumed_qubits(self, block) -> None:
        """Handle qubits that haven't been consumed (measured) by end of main."""
        # Only needed for Main block
        if type(block).__name__ != "Main":
            return

        # Find all QRegs declared in the block
        all_qregs = {}
        for var in block.vars:
            if type(var).__name__ == "QReg":
                all_qregs[var.sym] = var

        # Group unconsumed qubits by register
        unconsumed_by_reg = {}

        for qreg_name, qreg in all_qregs.items():
            # Get the consumed indices for this register
            consumed_indices = self.generator.consumed_qubits.get(qreg_name, set())
            # Check each qubit in the register
            unconsumed_indices = [
                i for i in range(qreg.size) if i not in consumed_indices
            ]

            if unconsumed_indices:
                unconsumed_by_reg[qreg_name] = unconsumed_indices

        # If there are unconsumed qubits, handle them efficiently
        if unconsumed_by_reg:
            self.generator.write("")
            self.generator.write("# Consume remaining qubits to satisfy linearity")

            for qreg_name, indices in sorted(unconsumed_by_reg.items()):
                qreg = all_qregs[qreg_name]

                # If all qubits in the register are unconsumed, use measure_array
                if len(indices) == qreg.size and set(indices) == set(range(qreg.size)):
                    # Check if already unpacked
                    if qreg_name in self.generator.unpacked_arrays:
                        unpacked_info = self.generator.unpacked_arrays[qreg_name]
                        if isinstance(unpacked_info, list):
                            # Already unpacked - measure individually
                            for i in indices:
                                if i < len(unpacked_info):
                                    self.generator.write(
                                        f"_ = quantum.measure({unpacked_info[i]})",
                                    )
                                else:
                                    self.generator.write(
                                        f"_ = quantum.measure({qreg_name}[{i}])",
                                    )
                        elif isinstance(
                            unpacked_info,
                            str,
                        ) and unpacked_info.startswith("__measure_array"):
                            # Already handled by measure_array
                            continue
                        else:
                            # Use measure_array for efficiency
                            self.generator.write(
                                f"_ = quantum.measure_array({qreg_name})",
                            )
                    else:
                        # Not unpacked - use measure_array for efficiency
                        self.generator.write(f"_ = quantum.measure_array({qreg_name})")
                else:
                    # Partial consumption - handle individually
                    for i in indices:
                        if qreg_name in self.generator.unpacked_arrays:
                            unpacked_info = self.generator.unpacked_arrays[qreg_name]
                            if isinstance(unpacked_info, list) and i < len(
                                unpacked_info,
                            ):
                                self.generator.write(
                                    f"_ = quantum.measure({unpacked_info[i]})",
                                )
                            else:
                                self.generator.write(
                                    f"_ = quantum.measure({qreg_name}[{i}])",
                                )
                        else:
                            self.generator.write(
                                f"_ = quantum.measure({qreg_name}[{i}])",
                            )

    def _collect_all_cregs(self, vars_list, creg_names: list) -> None:
        """Recursively collect all classical registers, including nested ones."""
        for var in vars_list:
            var_type = type(var).__name__
            if var_type == "CReg":
                creg_names.append(var.sym)
            elif hasattr(var, "vars"):
                # This variable has sub-variables (like Steane)
                # Recursively collect CRegs from sub-variables
                self._collect_all_cregs(var.vars, creg_names)
