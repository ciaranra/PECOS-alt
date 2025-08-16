"""Builder for converting SLR operations to IR."""

from __future__ import annotations

from typing import TYPE_CHECKING, Any, ClassVar

if TYPE_CHECKING:
    from pecos.slr import Block as SLRBlock
    from pecos.slr.gen_codes.guppy.ir import IRNode
    from pecos.slr.gen_codes.guppy.ir_analyzer import UnpackingPlan

from pecos.slr.gen_codes.guppy.allocation_optimizer import (
    AllocationOptimizer,
    AllocationStrategy,
)
from pecos.slr.gen_codes.guppy.ir import (
    ArrayAccess,
    ArrayUnpack,
    Assignment,
    BinaryOp,
    Block,
    Comment,
    Expression,
    FieldAccess,
    ForStatement,
    Function,
    FunctionCall,
    IfStatement,
    Literal,
    Measurement,
    Module,
    ResourceState,
    ReturnStatement,
    ScopeContext,
    Statement,
    TupleExpression,
    UnaryOp,
    VariableInfo,
    VariableRef,
    WhileStatement,
)
from pecos.slr.gen_codes.guppy.qubit_usage_analyzer import QubitRole, QubitUsageAnalyzer
from pecos.slr.gen_codes.guppy.scope_manager import (
    ResourceUsage,
    ScopeManager,
    ScopeType,
)


class IRBuilder:
    """Builds IR from SLR operations."""

    # Core blocks that should remain as control flow (not converted to functions)
    CORE_BLOCKS: ClassVar[set[str]] = {
        "If",
        "Repeat",
        "While",
        "For",
        "Main",
        "Block",
        "Comment",
        "Barrier",
    }

    def __init__(
        self,
        unpacking_plan: UnpackingPlan,
        *,
        include_optimization_report: bool = False,
    ):
        self.plan = unpacking_plan
        self.context = ScopeContext()
        self.scope_manager = ScopeManager()
        self.current_block: Block | None = None
        self.allocation_optimizer = AllocationOptimizer()
        self.allocation_decisions = {}
        self.include_optimization_report = include_optimization_report

        # Track blocks for function generation
        self.block_registry = {}  # Maps block signature to function name
        self.pending_functions = []  # Functions to be generated
        self.generated_functions = set()  # Functions already generated (actually built)
        self.discovered_functions = (
            set()
        )  # Functions discovered but maybe not built yet
        self.function_counter = 0  # For generating unique function names
        self.function_info = {}  # Track metadata about functions

        # Struct generation tracking
        self.struct_info = (
            {}
        )  # Maps prefix -> {fields: [(suffix, type, size)], struct_name: str}

    def build_module(self, main_block: SLRBlock, pending_functions: list) -> Module:
        """Build a complete module from SLR."""
        module = Module()

        # First, analyze allocation patterns
        self.allocation_decisions = self.allocation_optimizer.analyze_program(
            main_block,
        )

        # Analyze qubit usage to identify ancillas
        qubit_analyzer = QubitUsageAnalyzer()
        self.qubit_usage_stats = qubit_analyzer.analyze_block(main_block)

        # Detect and analyze struct patterns (will use qubit usage stats)
        self._detect_struct_patterns(main_block)

        # Add imports (including functional quantum operations for Array Unpacking Pattern)
        module.imports = [
            "from __future__ import annotations",
            "",
            "from typing import no_type_check",
            "",
            "from guppylang.decorator import guppy",
            "from guppylang.std import quantum",
            "from guppylang.std.quantum import qubit",
            "from guppylang.std.quantum.functional import ("
            "reset, h, x, y, z, s, t, sdg, tdg, cx, cy, cz"
            ")",
            "from guppylang.std.builtins import array, owned, result, py",
        ]

        # Generate struct definitions after imports
        if self.struct_info:
            module.imports.append("")
            struct_defs = self._generate_struct_definitions()
            module.imports.extend(struct_defs)

        # Add optimization report as comments (only if requested)
        if self.include_optimization_report and self.allocation_decisions:
            report = self.allocation_optimizer.generate_optimization_report(
                self.allocation_decisions,
            )
            module.imports.extend(
                [
                    "",
                    *["# " + line for line in report.split("\n") if line.strip()],
                ],
            )

        # Build main function
        main_func = self.build_main_function(main_block)
        module.functions.append(main_func)

        # Generate helper functions for structs
        for prefix, info in self.struct_info.items():
            # Generate decompose function (always needed for cleanup)
            decompose_func = self._generate_struct_decompose_function(prefix, info)
            if decompose_func:
                module.functions.append(decompose_func)

            # Also generate discard function (useful for other contexts)
            discard_func = self._generate_struct_discard_function(prefix, info)
            if discard_func:
                module.functions.append(discard_func)

        # Build any pending functions (from both parameter and internal tracking)
        all_pending = list(pending_functions) + self.pending_functions
        while all_pending:
            func_info = all_pending.pop(0)
            func = self.build_function(func_info)
            if func:
                module.functions.append(func)
                # Mark this function as generated
                if len(func_info) >= 2:
                    self.generated_functions.add(func_info[1])
                # Check if building this function added more pending functions
                # Add any new pending functions, avoiding duplicates
                for new_func in self.pending_functions:
                    new_block, new_name, new_sig = new_func
                    # Check if this function is already built or pending
                    already_pending = any(
                        f[1] == new_name for f in all_pending if len(f) >= 2
                    )
                    if new_name not in self.generated_functions and not already_pending:
                        all_pending.append(new_func)
                self.pending_functions = []

        return module

    def build_main_function(self, block: SLRBlock) -> Function:
        """Build the main function."""
        # Set current function name
        self.current_function_name = "main"

        # Analyze qubit usage patterns
        usage_analyzer = QubitUsageAnalyzer()
        usage_analyzer.analyze_block(block, self.struct_info)
        self.allocation_recommendations = (
            usage_analyzer.get_allocation_recommendations()
        )

        # Override allocation recommendations for struct fields to ensure they're pre-allocated
        # (struct constructors need all fields to be available)
        if self.struct_info:
            for prefix, info in self.struct_info.items():
                for suffix, _, _ in info["fields"]:
                    var_name = info["var_names"][suffix]
                    # Override the allocation recommendations system
                    if var_name in self.allocation_recommendations:
                        recommendation = self.allocation_recommendations[var_name]
                        if recommendation.get("allocation") == "dynamic":
                            # Override dynamic allocation for struct fields
                            self.allocation_recommendations[var_name] = {
                                "allocation": "pre_allocate",
                                "reason": "Struct field requires pre-allocation",
                                "keep_packed": recommendation.get("keep_packed", True),
                                "pre_allocate": True,
                            }

        body = Block()
        self.current_block = body

        # Track arrays consumed by @owned function calls
        self.consumed_arrays = set()

        # Add variable declarations
        if hasattr(block, "vars"):
            # First, add non-struct variables
            struct_vars = set()
            for prefix, info in self.struct_info.items():
                struct_vars.update(info["var_names"].values())

            # Get ancilla variables that were excluded from structs
            ancilla_vars = getattr(self, "ancilla_qubits", set())

            for var in block.vars:
                if hasattr(var, "sym"):
                    # Add if not in struct OR if it's an ancilla (which was excluded from struct)
                    if var.sym not in struct_vars or var.sym in ancilla_vars:
                        self._add_variable_declaration(var, block)

                    # Add to scope context for resource tracking
                    var_type = type(var).__name__
                    if var_type in ["QReg", "CReg"]:
                        is_quantum = var_type == "QReg"
                        size = getattr(var, "size", None)

                        var_info = VariableInfo(
                            name=var.sym,
                            original_name=var.sym,
                            var_type="quantum" if is_quantum else "classical",
                            size=size,
                            is_array=True,
                        )
                        self.context.add_variable(var_info)

            # Then, create struct instances
            for prefix, info in self.struct_info.items():
                self._add_struct_initialization(prefix, info, block)

        # Main function maintains natural SLR array semantics
        # Arrays are only unpacked internally when needed for selective measurements

        # Track unpacked vars for main
        self.unpacked_vars = {}

        # Add unpacking statements at the start if needed
        for array_name in self.plan.unpack_at_start:
            if array_name in self.plan.arrays_to_unpack:
                info = self.plan.arrays_to_unpack[array_name]

                # Skip unpacking for arrays that are struct fields
                # (already consumed by struct constructor)
                is_struct_field = False
                if self.struct_info:
                    for prefix, struct_info in self.struct_info.items():
                        if array_name in struct_info.get("var_names", {}).values():
                            is_struct_field = True
                            break

                if is_struct_field:
                    # Skip unpacking - array is consumed by struct constructor
                    # Individual elements can be accessed via struct decomposition
                    self.current_block.statements.append(
                        Comment(
                            f"Skip unpacking {array_name} - consumed by struct constructor",
                        ),
                    )
                    continue

                # For dynamically allocated arrays, we still need to unpack if the analyzer says so
                # This happens when there are selective measurements/operations
                if (
                    hasattr(self, "dynamic_allocations")
                    and array_name in self.dynamic_allocations
                ):
                    # Add comment explaining why we're unpacking a dynamic array
                    self.current_block.statements.append(
                        Comment(f"Unpack {array_name} for individual access"),
                    )
                elif not info.is_classical:
                    # Regular unpacking for quantum arrays
                    self.current_block.statements.append(
                        Comment(f"Unpack {array_name} for individual access"),
                    )
                # Don't skip classical arrays - they should be unpacked too
                self._add_array_unpacking(array_name, info.size)

        # Add operations
        if hasattr(block, "ops"):
            for op in block.ops:
                stmt = self._convert_operation(op)
                if stmt:
                    body.statements.append(stmt)

        # Handle struct decomposition, results, and cleanup
        self._add_final_handling(block)

        return Function(
            name="main",
            params=[],
            return_type="None",
            body=body,
            decorators=["guppy", "no_type_check"],
        )

    def build_function(self, func_info) -> Function | None:
        """Build a function from pending function info."""
        # Handle different formats of func_info
        if len(func_info) == 3:
            # New format from IR builder: (block, func_name, signature)
            sample_block, func_name, block_signature = func_info
        elif len(func_info) == 4:
            # Old format: (block_key, func_name, sample_block, block_name)
            block_key, func_name, sample_block, block_name = func_info
        else:
            return None

        # Analyze dependencies to determine parameters
        deps = self._analyze_block_dependencies(sample_block)

        # Build parameter list
        params = []
        param_mapping = {}  # Maps parameter names to original variable names

        # Check if we should use structs instead of individual arrays
        struct_params = set()  # Structs we've already added
        vars_in_structs = set()  # Variables that are part of structs

        # First pass: identify which variables are part of structs
        for prefix, info in self.struct_info.items():
            vars_in_this_struct = []
            for var in info["var_names"].values():
                if var in deps["quantum"] or var in deps["classical"]:
                    vars_in_structs.add(var)
                    vars_in_this_struct.append(var)

            # If any variable from this struct is used, add the struct as a parameter
            if vars_in_this_struct and prefix not in struct_params:
                # Add struct parameter
                struct_name = info["struct_name"]
                param_type = struct_name

                # Check if this struct contains quantum resources
                has_quantum = any(v in deps["quantum"] for v in vars_in_this_struct)
                if has_quantum and self._block_consumes_quantum(sample_block):
                    param_type = f"{param_type} @owned"

                params.append((prefix, param_type))
                param_mapping[prefix] = prefix
                struct_params.add(prefix)

        # Black Box Pattern: All functions that handle quantum arrays should use
        # functional pattern. This maintains SLR's global array semantics at
        # boundaries while using functional internals
        # BUT: Only unpack if the IR analyzer determined it's necessary
        # First, run the IR analyzer on this block to get unpacking plan
        from pecos.slr.gen_codes.guppy.ir_analyzer import IRAnalyzer

        analyzer = IRAnalyzer()
        block_plan = analyzer.analyze_block(sample_block, self.context.variables)

        # Only unpack if there are arrays that need unpacking according to the analyzer
        needs_unpacking = len(block_plan.arrays_to_unpack) > 0

        # Check if this function consumes its quantum arrays
        consumes_quantum = self._block_consumes_quantum(sample_block)

        # Add quantum parameters (skip those in structs UNLESS they're ancillas)
        for var in sorted(deps["quantum"] & deps["reads"]):
            # Check if this is an ancilla that was excluded from structs
            is_excluded_ancilla = (
                hasattr(self, "ancilla_qubits") and var in self.ancilla_qubits
            )

            if var in vars_in_structs and not is_excluded_ancilla:
                continue
            param_name = var  # Use the same name, no need for _param suffix
            param_mapping[param_name] = var
            # Determine type from context or default to qubit array
            var_info = self.context.lookup_variable(var)
            if var_info:
                if var_info.is_unpacked:
                    # This is an unpacked array - need the original array type
                    param_type = f"array[quantum.qubit, {var_info.size}]"
                else:
                    # Always use array type to maintain consistency with SLR semantics
                    param_type = f"array[quantum.qubit, {var_info.size}]"
            else:
                # Default assumption for quantum variables
                param_type = "array[quantum.qubit, 7]"

            # Add @owned annotation if this function consumes quantum resources
            if consumes_quantum:
                param_type = f"{param_type} @owned"

            params.append((param_name, param_type))

        # Add classical parameters (no ownership, but include written vars
        # since arrays are mutable)
        for var in sorted(deps["classical"] & (deps["reads"] | deps["writes"])):
            if var in vars_in_structs:
                continue
            param_name = var  # Use the same name, no need for _param suffix
            param_mapping[param_name] = var
            # Determine type from context
            var_info = self.context.lookup_variable(var)
            # Always use array type for consistency
            param_type = (
                f"array[bool, {var_info.size}]" if var_info else "array[bool, 32]"
            )
            params.append((param_name, param_type))

        # Create function body
        body = Block()
        prev_block = self.current_block
        prev_mapping = self.param_mapping if hasattr(self, "param_mapping") else {}
        self.current_block = body
        self.param_mapping = param_mapping

        # Create a variable remapping context for this function
        # This maps original variable names to their parameter names
        var_remapping = {}
        for param_name, original_name in param_mapping.items():
            var_remapping[original_name] = param_name

            # Also handle unpacked variables
            var_info = self.context.lookup_variable(original_name)
            if var_info and var_info.is_unpacked:
                # Map each unpacked element
                for i, unpacked_name in enumerate(var_info.unpacked_names):
                    var_remapping[unpacked_name] = f"{param_name}[{i}]"

        # Store current function context
        self.current_function_name = func_name
        self.current_function_params = params

        # Track if this function has @owned struct parameters
        has_owned_struct_params = any(
            "@owned" in param_type and param_name in self.struct_info
            for param_name, param_type in params
        )
        self.function_info[func_name] = {
            "has_owned_struct_params": has_owned_struct_params,
            "params": params,
        }

        # Store the remapping for use during conversion
        prev_var_remapping = getattr(self, "var_remapping", {})
        self.var_remapping = var_remapping

        # Track unpacked variables (only if needed)
        self.unpacked_vars = {}  # Maps array_name -> [element_names]
        self.replaced_qubits = {}  # Maps array_name -> set of replaced indices

        # Only add array unpacking for arrays that the analyzer determined need it
        # ALSO: Unpack ancilla arrays with @owned annotation to avoid MoveOutOfSubscriptError
        if needs_unpacking:
            for param_name, param_type in params:
                if (
                    "array[quantum.qubit," in param_type
                    and param_name in block_plan.arrays_to_unpack
                ):
                    # Extract array size
                    import re

                    match = re.search(r"array\[quantum\.qubit, (\d+)\]", param_type)
                    if match:
                        size = int(match.group(1))
                        # Generate unpacked variable names
                        element_names = [f"{param_name}_{i}" for i in range(size)]
                        self.unpacked_vars[param_name] = element_names

                        # Add unpacking statement to function body
                        unpacking_stmt = self._create_array_unpack_statement(
                            param_name,
                            element_names,
                        )
                        body.statements.append(unpacking_stmt)

        # Additionally, check for ancilla arrays with @owned that need unpacking
        for param_name, param_type in params:
            # Check if this is an ancilla array with @owned
            is_ancilla = (
                hasattr(self, "ancilla_qubits") and param_name in self.ancilla_qubits
            )
            if (
                is_ancilla
                and "@owned" in param_type
                and "array[quantum.qubit," in param_type
                and param_name not in self.unpacked_vars
            ):
                # This ancilla array needs unpacking to avoid MoveOutOfSubscriptError
                import re

                match = re.search(r"array\[quantum\.qubit, (\d+)\]", param_type)
                if match:
                    size = int(match.group(1))
                    # Generate unpacked variable names
                    element_names = [f"{param_name}_{i}" for i in range(size)]
                    self.unpacked_vars[param_name] = element_names

                    # Add comment explaining why we're unpacking
                    body.statements.append(
                        Comment(
                            f"Unpack ancilla array {param_name} to avoid "
                            "MoveOutOfSubscriptError with @owned",
                        ),
                    )

                    # Add unpacking statement to function body
                    unpacking_stmt = self._create_array_unpack_statement(
                        param_name,
                        element_names,
                    )
                    body.statements.append(unpacking_stmt)

        # Add struct unpacking for struct parameters
        struct_field_vars = (
            {}
        )  # Maps original var name to struct field path for @owned structs
        struct_reconstruction = (
            {}
        )  # Maps struct param name to list of field vars for reconstruction

        for param_name, param_type in params:
            if "@owned" in param_type and param_name in self.struct_info:
                # This is an @owned struct parameter
                # With @owned structs, we work functionally - no unpacking
                struct_info = self.struct_info[param_name]

                # Track that we have an owned struct
                if not hasattr(self, "owned_structs"):
                    self.owned_structs = set()
                self.owned_structs.add(param_name)

                # Map variables to use struct field access
                for suffix, field_type, field_size in sorted(struct_info["fields"]):
                    original_var = struct_info["var_names"].get(suffix)
                    if original_var:
                        # We'll handle these specially in variable references
                        struct_field_vars[original_var] = f"{param_name}.{suffix}"

                # Skip unpacking for @owned structs
                continue
            if param_name in self.struct_info:
                # Non-owned struct parameter - can unpack normally
                struct_info = self.struct_info[param_name]
                field_vars = []

                # Generate unpacking statement - use same order as struct
                # definition (sorted by suffix)
                unpack_targets = []
                for suffix, field_type, field_size in sorted(struct_info["fields"]):
                    field_var = f"{param_name}_{suffix}"
                    unpack_targets.append(field_var)
                    field_vars.append(field_var)

                    # Map the original variable name to this unpacked field variable
                    original_var = struct_info["var_names"].get(suffix)
                    if original_var:
                        struct_field_vars[original_var] = field_var
                        # Also update var_remapping to use field access directly
                        self.var_remapping[original_var] = field_var

                # Create the unpacking statement:
                # field1, field2, ... = struct.field1, struct.field2, ...
                # In Guppy, we need to unpack the entire struct at once -
                # use same order as struct definition
                unpack_stmt = Assignment(
                    target=TupleExpression(
                        [VariableRef(var) for var in unpack_targets],
                    ),
                    value=TupleExpression(
                        [
                            FieldAccess(VariableRef(param_name), field)
                            for field, _, _ in sorted(struct_info["fields"])
                        ],
                    ),
                )
                body.statements.append(unpack_stmt)

                # Store for reconstruction
                struct_reconstruction[param_name] = field_vars

        # Store struct field mappings for use in variable references
        self.struct_field_mapping = struct_field_vars

        # Add operations from the sample block
        if hasattr(sample_block, "ops"):
            for op in sample_block.ops:
                stmt = self._convert_operation(op)
                if stmt:
                    body.statements.append(stmt)

        # Restore previous remapping
        self.var_remapping = prev_var_remapping

        self.current_block = prev_block
        self.param_mapping = prev_mapping

        # Analyze what qubits were consumed in this function
        consumed_in_function = {}
        self._track_consumed_qubits(sample_block, consumed_in_function)

        # Initialize return type
        return_type = "None"

        # Black Box Pattern: functions that handle quantum arrays return modified arrays
        # BUT: if function consumes arrays (@owned), don't return them
        # Check if we have quantum arrays or structs to return (regardless of unpacking)
        has_quantum_arrays = any(
            "array[quantum.qubit," in ptype for name, ptype in params
        )
        has_structs = any(name in self.struct_info for name, ptype in params)

        if has_quantum_arrays or has_structs:
            # Array/struct return pattern: functions return reconstructed arrays or structs
            quantum_returns = []

            # Add structs first - even @owned structs can be returned if they're reconstructed
            for name, ptype in params:
                if name in self.struct_info:
                    # Remove @owned annotation from type for return type
                    return_type = ptype.replace(" @owned", "")
                    quantum_returns.append((name, return_type))

            # Then add individual arrays not in structs (including ancillas)
            for name, ptype in params:
                if "array[quantum.qubit," in ptype:
                    # Check if this array is part of a struct
                    in_struct = False
                    is_excluded_ancilla = False

                    for prefix, info in self.struct_info.items():
                        if name in info["var_names"].values():
                            in_struct = True
                            break

                    # Check if this is an ancilla that was excluded from structs
                    if hasattr(self, "ancilla_qubits") and name in self.ancilla_qubits:
                        is_excluded_ancilla = True

                    # Include if: not in struct OR is an excluded ancilla
                    if not in_struct or is_excluded_ancilla:
                        # Check if any elements remain unconsumed for ALL arrays
                        if name in consumed_in_function:
                            # Extract array size from type
                            import re

                            match = re.search(r"array\[quantum\.qubit, (\d+)\]", ptype)
                            if match:
                                original_size = int(match.group(1))
                                consumed_indices = consumed_in_function[name]

                                # Check if any consumed qubits were replaced
                                replaced_indices = set()
                                if (
                                    hasattr(self, "replaced_qubits")
                                    and name in self.replaced_qubits
                                ):
                                    replaced_indices = self.replaced_qubits[name]

                                # Only count as consumed if not replaced
                                actually_consumed = consumed_indices - replaced_indices
                                remaining_count = original_size - len(actually_consumed)

                                if remaining_count > 0:
                                    # Some qubits remain - return array
                                    # If qubits were replaced, return full array
                                    if replaced_indices:
                                        new_type = ptype.replace(" @owned", "")
                                    # Special case: ancilla arrays that are passed
                                    # between functions. In patterns like Steane code,
                                    # ancillas are measured and replaced
                                    # throughout multiple function calls, so return full array
                                    elif (
                                        hasattr(self, "ancilla_qubits")
                                        and name in self.ancilla_qubits
                                        and len(consumed_indices) > 0
                                    ):
                                        # Ancilla with some consumption - likely
                                        # replaced in called functions
                                        new_type = ptype.replace(" @owned", "")
                                    elif remaining_count < original_size:
                                        new_type = (
                                            f"array[quantum.qubit, {remaining_count}]"
                                        )
                                    else:
                                        new_type = ptype.replace(" @owned", "")
                                    quantum_returns.append((name, new_type))
                                # If all consumed, don't add to returns
                        else:
                            # No consumption tracked - return full array
                            # Remove @owned annotation from return type
                            return_type = ptype.replace(" @owned", "")
                            quantum_returns.append((name, return_type))

            if quantum_returns:
                # Add return statements
                if len(quantum_returns) == 1:
                    name, ptype = quantum_returns[0]

                    # Check if this is a partial return
                    if name in consumed_in_function and "array[quantum.qubit," in ptype:
                        # Need to return only unconsumed elements
                        import re

                        match = re.search(r"array\[quantum\.qubit, (\d+)\]", ptype)
                        if match:
                            int(match.group(1))
                            original_match = re.search(
                                r"array\[quantum\.qubit, (\d+)\]",
                                next(pt for n, pt in params if n == name),
                            )
                            if original_match:
                                original_size = int(original_match.group(1))
                                consumed_indices = consumed_in_function[name]

                                # Build array with only unconsumed elements
                                unconsumed_elements = []
                                for i in range(original_size):
                                    if i not in consumed_indices:
                                        if name in self.unpacked_vars:
                                            # Use unpacked element name
                                            element_name = self.unpacked_vars[name][i]
                                            unconsumed_elements.append(
                                                VariableRef(element_name),
                                            )
                                        else:
                                            # Use array indexing
                                            unconsumed_elements.append(
                                                ArrayAccess(array_name=name, index=i),
                                            )

                                # Create array construction with unconsumed elements
                                array_expr = FunctionCall(
                                    func_name="array",
                                    args=unconsumed_elements,
                                )
                                body.statements.append(
                                    ReturnStatement(value=array_expr),
                                )
                    elif name in self.unpacked_vars:
                        # Full array return - reconstruct from elements
                        element_names = self.unpacked_vars[name]
                        array_construction = self._create_array_construction(
                            element_names,
                        )
                        body.statements.append(
                            ReturnStatement(value=array_construction),
                        )
                    elif name in struct_reconstruction:
                        # Struct was unpacked - check if we can still use the unpacked variables
                        struct_info = self.struct_info[name]

                        # Check if the unpacked variables are still valid
                        # They're only valid if we haven't passed the struct
                        # to any @owned functions
                        unpacked_vars_valid = all(
                            struct_info["var_names"].get(suffix) in self.var_remapping
                            for suffix, _, _ in struct_info["fields"]
                        )

                        if unpacked_vars_valid:
                            # Create struct constructor call - use same order
                            # as struct definition (sorted by suffix)
                            constructor_args = []
                            for suffix, field_type, field_size in sorted(
                                struct_info["fields"],
                            ):
                                field_var = f"{name}_{suffix}"
                                constructor_args.append(VariableRef(field_var))

                            struct_constructor = FunctionCall(
                                func_name=struct_info["struct_name"],
                                args=constructor_args,
                            )
                            body.statements.append(
                                ReturnStatement(value=struct_constructor),
                            )
                        else:
                            # Unpacked variables are no longer valid - return the struct directly
                            body.statements.append(
                                ReturnStatement(value=VariableRef(name)),
                            )
                    else:
                        # Array/struct was not unpacked - return it directly
                        body.statements.append(ReturnStatement(value=VariableRef(name)))

                    # Set return type
                    return_type = ptype  # Use the potentially modified type
                else:
                    # Multiple arrays/structs - return tuple
                    return_exprs = []
                    return_types = []
                    for name, ptype in quantum_returns:
                        if name in self.unpacked_vars:
                            # Array was unpacked - reconstruct from elements
                            element_names = self.unpacked_vars[name]
                            array_construction = self._create_array_construction(
                                element_names,
                            )
                            return_exprs.append(array_construction)
                        elif name in struct_reconstruction:
                            # Struct was unpacked - check if we can still use
                            # the unpacked variables
                            struct_info = self.struct_info[name]

                            # Check if the unpacked variables are still valid
                            unpacked_vars_valid = all(
                                struct_info["var_names"].get(suffix)
                                in self.var_remapping
                                for suffix, _, _ in struct_info["fields"]
                            )

                            if unpacked_vars_valid:
                                # Create struct constructor call - use same order
                                # as struct definition (sorted by suffix)
                                constructor_args = []
                                for suffix, field_type, field_size in sorted(
                                    struct_info["fields"],
                                ):
                                    field_var = f"{name}_{suffix}"
                                    constructor_args.append(VariableRef(field_var))

                                struct_constructor = FunctionCall(
                                    func_name=struct_info["struct_name"],
                                    args=constructor_args,
                                )
                                return_exprs.append(struct_constructor)
                            else:
                                # Unpacked variables are no longer valid -
                                # return the struct directly
                                return_exprs.append(VariableRef(name))
                        else:
                            # Array/struct was not unpacked - return it directly
                            return_exprs.append(VariableRef(name))

                        # Add type to return types
                        return_types.append(ptype)

                    if return_exprs:
                        body.statements.append(
                            ReturnStatement(
                                value=TupleExpression(elements=return_exprs),
                            ),
                        )
                        return_type = f"tuple[{', '.join(return_types)}]"

        return Function(
            name=func_name,
            params=params,
            return_type=return_type,
            body=body,
            decorators=["guppy", "no_type_check"],
        )

    def _add_variable_declaration(self, var, block=None) -> None:
        """Add variable declaration to current block."""
        var_type = type(var).__name__
        var_name = var.sym

        # Check for renaming
        if var_name in self.plan.renamed_variables:
            var_name = self.plan.renamed_variables[var_name]

        if var_type == "QReg":
            # Get size for all cases
            size = var.size

            # Check allocation recommendation for this array
            recommendation = self.allocation_recommendations.get(var.sym, {})

            # Check allocation decision for this array
            decision = self.allocation_decisions.get(var.sym)

            # Check if this array needs unpacking (selective measurements)
            needs_unpacking = var.sym in self.plan.arrays_to_unpack

            # Check if this array is used in full array operations
            needs_full_array = self._array_needs_full_allocation(var.sym, block)

            # Check if this should be dynamically allocated based on usage patterns
            # But only if it doesn't need unpacking for selective measurements
            # AND not used in full array ops
            if (
                recommendation.get("allocation") == "dynamic"
                and not needs_unpacking
                and not needs_full_array
            ):
                # Check if this ancilla array is used as a function parameter
                # If so, we need to pre-allocate it despite being an ancilla
                is_function_param = False
                if hasattr(self, "ancilla_qubits") and var_name in self.ancilla_qubits:
                    # This is an ancilla that was excluded from structs
                    # It will be passed as a parameter to functions, so pre-allocate it
                    is_function_param = True

                if is_function_param:
                    # Pre-allocate the ancilla array since it's used as a function parameter
                    self.current_block.statements.append(
                        Comment(
                            f"Pre-allocate ancilla array {var_name} (used as function parameter)",
                        ),
                    )
                    init_expr = FunctionCall(
                        func_name="array",
                        args=[
                            FunctionCall(
                                func_name="quantum.qubit() for _ in range",
                                args=[Literal(size)],
                            ),
                        ],
                    )
                    assignment = Assignment(
                        target=VariableRef(var_name),
                        value=init_expr,
                    )
                    self.current_block.statements.append(assignment)
                else:
                    # For other ancillas, don't pre-allocate array
                    reason = recommendation.get("reason", "ancilla pattern")
                    self.current_block.statements.append(
                        Comment(
                            f"# {var_name} will be allocated dynamically ({reason})",
                        ),
                    )
                    # Track that this is dynamically allocated
                    if not hasattr(self, "dynamic_allocations"):
                        self.dynamic_allocations = set()
                    self.dynamic_allocations.add(var.sym)
            elif decision and decision.strategy == AllocationStrategy.LOCAL_ALLOCATE:
                # Don't pre-allocate - will be allocated when first used
                self.current_block.statements.append(
                    Comment(f"Qubits from {var_name} will be allocated locally"),
                )
            elif decision and decision.strategy == AllocationStrategy.FUNCTION_SCOPED:
                # Mixed strategy - pre-allocate some, allocate others locally
                # But only if the array doesn't need unpacking
                if needs_unpacking:
                    # Can't use FUNCTION_SCOPED with unpacking - fall back to full pre-allocation
                    init_expr = FunctionCall(
                        func_name="array",
                        args=[
                            FunctionCall(
                                func_name="quantum.qubit() for _ in range",
                                args=[Literal(size)],
                            ),
                        ],
                    )
                    assignment = Assignment(
                        target=VariableRef(var_name),
                        value=init_expr,
                    )
                    self.current_block.statements.append(assignment)
                    self.current_block.statements.append(
                        Comment(
                            f"Note: Full pre-allocation used because {var_name} needs unpacking",
                        ),
                    )
                elif decision.original_size - len(decision.local_elements) > 0:
                    pre_alloc_size = decision.original_size - len(
                        decision.local_elements,
                    )
                    init_expr = FunctionCall(
                        func_name="array",
                        args=[
                            FunctionCall(
                                func_name="quantum.qubit() for _ in range",
                                args=[Literal(pre_alloc_size)],
                            ),
                        ],
                    )
                    assignment = Assignment(
                        target=VariableRef(var_name),
                        value=init_expr,
                    )
                    self.current_block.statements.append(assignment)

                self.current_block.statements.append(
                    Comment(
                        f"Elements {sorted(decision.local_elements)} of "
                        f"{var_name} will be allocated locally",
                    ),
                )
            else:
                # Default: pre-allocate all qubits
                init_expr = FunctionCall(
                    func_name="array",
                    args=[
                        FunctionCall(
                            func_name="quantum.qubit() for _ in range",
                            args=[Literal(size)],
                        ),
                    ],
                )
                assignment = Assignment(
                    target=VariableRef(var_name),
                    value=init_expr,
                )
                self.current_block.statements.append(assignment)

            # Track in context
            var_info = VariableInfo(
                name=var_name,
                original_name=var.sym,
                var_type="quantum",
                size=size,
                is_array=True,
            )
            self.context.add_variable(var_info)
            self.scope_manager.current_context.add_variable(var_info)

        elif var_type == "CReg":
            # Create classical array
            size = var.size
            init_expr = FunctionCall(
                func_name="array",
                args=[
                    FunctionCall(
                        func_name="False for _ in range",
                        args=[Literal(size)],
                    ),
                ],
            )
            assignment = Assignment(
                target=VariableRef(var_name),
                value=init_expr,
            )
            self.current_block.statements.append(assignment)

            # Track in context
            var_info = VariableInfo(
                name=var_name,
                original_name=var.sym,
                var_type="classical",
                size=size,
                is_array=True,
            )
            self.context.add_variable(var_info)
            self.scope_manager.current_context.add_variable(var_info)

    def _block_consumes_quantum(self, block) -> bool:
        """Check if a block consumes ALL quantum resources.

        Only return True if the block consumes ALL its quantum inputs.
        Most SLR functions modify arrays in-place without consuming them.

        However, functions that access quantum fields within structs need @owned
        annotation to satisfy Guppy's linearity requirements.
        """
        # For now, be very conservative - assume functions don't consume
        # their parameters unless they're explicitly measurement blocks
        # that measure ALL qubits

        # Check the block name - only certain blocks truly consume all resources
        block_name = type(block).__name__
        if block_name in ["MeasureAll", "DiscardAll"]:
            return True

        # IMPORTANT: Functions that will access quantum fields within structs
        # need @owned annotation for Guppy's linearity system
        # Otherwise assume the function modifies in-place without consuming
        return self._block_accesses_struct_quantum_fields(block)

    def _block_accesses_struct_quantum_fields(self, block) -> bool:
        """Check if a block accesses quantum fields within structs.

        This is important because Guppy's linearity system requires @owned
        annotation for functions that access quantum fields within structs.
        """
        if not hasattr(block, "ops"):
            return False

        # If we have struct info, assume that functions accessing quantum operations
        # will need to access quantum fields within structs
        if self.struct_info:
            # Check if this block has quantum operations
            for op in block.ops:
                # Check for quantum operations (gates, measurements, etc.)
                op_name = type(op).__name__
                if op_name in [
                    "H",
                    "X",
                    "Y",
                    "Z",
                    "CX",
                    "CY",
                    "CZ",
                    "Reset",
                    "Measure",
                    "S",
                    "T",
                    "Sdg",
                    "Tdg",
                ]:
                    return True

                # Also check for nested quantum operations
                if hasattr(op, "ops") and self._block_accesses_struct_quantum_fields(
                    op,
                ):
                    return True

        return False

    def _needs_unpacking_workaround(self, block) -> bool:
        """Detect if a block needs the unpacking workaround for Guppy constraints."""
        if not hasattr(block, "ops"):
            return False

        # Check for patterns that cause MoveOutOfSubscriptError
        for op in block.ops:
            op_type = type(op).__name__

            # Reset operations on arrays are the main culprit
            if op_type == "Prep" and hasattr(op, "qargs"):
                for qarg in op.qargs:
                    # If it's an array operation, it might cause issues
                    if hasattr(qarg, "sym") and hasattr(qarg, "size") and qarg.size > 1:
                        return True

            # Multiple operations on the same array elements might cause issues
            # This is a more complex heuristic we could add later

            # Recursively check nested blocks
            if hasattr(op, "ops") and self._needs_unpacking_workaround(op):
                return True

        return False

    def _function_needs_unpacking(self, func_name: str) -> bool:
        """Check if a function uses the unpacking pattern by analyzing function behavior.

        This method analyzes the actual function operations rather than using hardcoded names,
        making it general for all QEC codes.
        """
        _ = func_name  # Currently not used, reserved for future use
        # Since this function is not currently used, return False for now
        # In the future, this could analyze the function's block to determine
        # if it performs operations that would benefit from unpacking
        return False

    def _function_consumes_parameters(self, func_name: str, block) -> bool:
        """Check if a function consumes its quantum parameters (has @owned)."""
        _ = func_name  # Currently not used, reserved for future use
        # Check if we already know about this function
        if hasattr(block, "ops"):
            return self._block_consumes_quantum(block)

        # Default: assume functions don't consume unless we know otherwise
        return False

    def _create_array_unpack_statement(
        self,
        array_name: str,
        element_names: list[str],
    ) -> Statement:
        """Create an array unpacking statement: q_0, q_1, q_2 = q"""

        class ArrayUnpackStatement(Statement):
            def __init__(self, targets, source):
                self.targets = targets
                self.source = source

            def analyze(self, context):
                _ = context  # Not used

            def render(self, context):
                _ = context  # Not used
                target_str = ", ".join(self.targets)
                return [f"{target_str} = {self.source}"]

        return ArrayUnpackStatement(element_names, array_name)

    def _create_array_construction(self, element_names: list[str]) -> Expression:
        """Create an array construction expression: array([q_0, q_1, q_2])"""

        class ArrayConstructionExpression(Expression):
            def __init__(self, elements):
                self.elements = elements

            def analyze(self, context):
                _ = context  # Not used

            def render(self, context):
                _ = context  # Not used
                element_str = ", ".join(self.elements)
                return [f"array({element_str})"]

        return ArrayConstructionExpression(element_names)

    def _create_struct_construction(
        self,
        struct_name: str,
        field_names: list[str],
        field_values: list[Expression],
    ) -> Expression:
        """Create a struct construction expression."""

        class StructConstructionExpression(Expression):
            def __init__(self, struct_name, field_names, field_values):
                self.struct_name = struct_name
                self.field_names = field_names
                self.field_values = field_values

            def analyze(self, context):
                for value in self.field_values:
                    value.analyze(context)

            def render(self, context):
                # Render as struct_name(value1, value2, ...) - positional args only
                # Guppy doesn't support keyword arguments in struct construction
                field_values_str = []
                for value in self.field_values:
                    value_str = value.render(context)[0]
                    field_values_str.append(value_str)
                return [f"{self.struct_name}({', '.join(field_values_str)})"]

        return StructConstructionExpression(struct_name, field_names, field_values)

    def _add_array_unpacking(self, array_name: str, size: int) -> None:
        """Add array unpacking statement."""
        # Get the actual variable name (might be renamed)
        actual_name = array_name
        if array_name in self.plan.renamed_variables:
            actual_name = self.plan.renamed_variables[array_name]

        # Generate unpacked names
        unpacked_names = [f"{array_name}_{i}" for i in range(size)]

        # Track unpacked vars in the builder
        self.unpacked_vars[array_name] = unpacked_names

        # Comment already added by caller, don't add another one

        # Add unpacking statement
        unpack = ArrayUnpack(
            targets=unpacked_names,
            source=actual_name,
        )
        self.current_block.statements.append(unpack)

        # Update variable info
        var = self.context.lookup_variable(actual_name)
        if var:
            var.is_unpacked = True
            var.unpacked_names = unpacked_names

    def _convert_operation(self, op) -> Statement | None:
        """Convert an SLR operation to IR statement."""
        op_type = type(op).__name__

        if op_type == "Measure":
            return self._convert_measurement(op)
        if op_type == "If":
            return self._convert_if(op)
        if op_type == "While":
            return self._convert_while(op)
        if op_type == "For":
            return self._convert_for(op)
        if op_type == "Repeat":
            return self._convert_repeat(op)
        if op_type == "Comment":
            return self._convert_comment(op)
        if op_type == "Permute":
            return self._convert_permute(op)
        if hasattr(op, "qargs"):
            stmt = self._convert_quantum_gate(op)
            # Handle case where quantum gate returns a Block
            if stmt and type(stmt).__name__ == "Block":
                # Add all statements from the block
                for s in stmt.statements:
                    self.current_block.statements.append(s)
                return None  # Already added
            return stmt
        if hasattr(op, "ops") and hasattr(op, "vars"):
            # This is a block - convert to function call
            return self._convert_block_call(op)
        if op_type == "SET":
            # Classical bit assignment
            return self._convert_set_operation(op)
        if op_type == "Barrier":
            # Barriers are just synchronization points, ignore in Guppy
            return None

        # Unknown operation
        return Comment(f"TODO: Handle {op_type}")

    def _convert_measurement(self, meas) -> Statement | None:
        """Convert measurement operation."""
        if not hasattr(meas, "qargs") or not meas.qargs:
            return None

        # Check if we're measuring a struct field qubit with @owned struct
        if hasattr(meas, "qargs") and len(meas.qargs) > 0:
            qarg = meas.qargs[0]
            if hasattr(qarg, "reg") and hasattr(qarg.reg, "sym"):
                array_name = qarg.reg.sym
                # Check if this is a struct field
                for info in self.struct_info.values():
                    if (
                        array_name in info["var_names"].values()
                        and hasattr(self, "function_info")
                        and hasattr(self, "current_function_name")
                    ):
                        func_info = self.function_info.get(
                            self.current_function_name,
                            {},
                        )
                        if func_info.get("has_owned_struct_params", False):
                            # This is a known limitation - add a warning comment
                            self.current_block.statements.append(
                                Comment(
                                    "WARNING: Measuring qubits from @owned struct arrays "
                                    "is not supported by guppylang",
                                ),
                            )
                            self.current_block.statements.append(
                                Comment(
                                    "This will cause a MoveOutOfSubscriptError "
                                    "during compilation",
                                ),
                            )

        # Check if we're in a function that takes and returns a struct
        # If so, we need to be careful about struct field access
        if hasattr(self, "current_function_params"):
            for param_name, param_type in self.current_function_params:
                if "_struct" in str(param_type) and "@owned" not in str(param_type):
                    break

        # Check if this is a full array measurement
        if (
            len(meas.qargs) == 1
            and hasattr(meas.qargs[0], "sym")
            and hasattr(meas.qargs[0], "size")
            and meas.qargs[0].size >= 1
        ):
            # Full array measurement
            qreg = meas.qargs[0]

            # Track full array consumption globally
            if not hasattr(self, "consumed_resources"):
                self.consumed_resources = {}
            if qreg.sym not in self.consumed_resources:
                self.consumed_resources[qreg.sym] = set()
            self.consumed_resources[qreg.sym].update(range(qreg.size))

            # Track in scope manager too
            self.scope_manager.track_resource_usage(
                qreg.sym,
                set(range(qreg.size)),
                consumed=True,
            )

            # Check if this array was dynamically allocated
            if (
                hasattr(self, "dynamic_allocations")
                and qreg.sym in self.dynamic_allocations
            ):
                # For dynamically allocated arrays, we need to handle this differently
                # Generate individual measurements
                stmts = []

                # Check for target
                if hasattr(meas, "cout") and meas.cout and len(meas.cout) == 1:
                    cout = meas.cout[0]
                    if hasattr(cout, "sym"):
                        creg_name = cout.sym
                        # Measure each individual qubit
                        for i in range(qreg.size):
                            ancilla_var = f"{qreg.sym}_{i}"
                            # Allocate if not already allocated
                            if not hasattr(self, "allocated_ancillas"):
                                self.allocated_ancillas = set()
                            if ancilla_var not in self.allocated_ancillas:
                                alloc_stmt = Assignment(
                                    target=VariableRef(ancilla_var),
                                    value=FunctionCall(
                                        func_name="quantum.qubit",
                                        args=[],
                                    ),
                                )
                                stmts.append(alloc_stmt)
                                self.allocated_ancillas.add(ancilla_var)

                            # Measure individual qubit
                            meas_call = FunctionCall(
                                func_name="quantum.measure",
                                args=[VariableRef(ancilla_var)],
                            )
                            creg_access = ArrayAccess(array_name=creg_name, index=i)
                            assign = Assignment(target=creg_access, value=meas_call)
                            stmts.append(assign)

                        # Return block with all statements
                        if len(stmts) == 1:
                            return stmts[0]
                        return Block(statements=stmts)
                else:
                    # No target - measure individual qubits without storing
                    for i in range(qreg.size):
                        ancilla_var = f"{qreg.sym}_{i}"
                        if not hasattr(self, "allocated_ancillas"):
                            self.allocated_ancillas = set()
                        if ancilla_var not in self.allocated_ancillas:
                            alloc_stmt = Assignment(
                                target=VariableRef(ancilla_var),
                                value=FunctionCall(func_name="quantum.qubit", args=[]),
                            )
                            stmts.append(alloc_stmt)
                            self.allocated_ancillas.add(ancilla_var)

                        # Measure and discard result
                        meas_call = FunctionCall(
                            func_name="quantum.measure",
                            args=[VariableRef(ancilla_var)],
                        )

                        class ExpressionStatement(Statement):
                            def __init__(self, expr):
                                self.expr = expr

                            def analyze(self, context):
                                self.expr.analyze(context)

                            def render(self, context):
                                return f"_ = {self.expr.render(context)}"

                        stmts.append(ExpressionStatement(meas_call))

                    if len(stmts) == 1:
                        return stmts[0]
                    return Block(statements=stmts)
            else:
                # Regular pre-allocated array - use measure_array
                qreg_ref = self._convert_qubit_ref(qreg)

                # Check for target
                if hasattr(meas, "cout") and meas.cout and len(meas.cout) == 1:
                    cout = meas.cout[0]
                    if hasattr(cout, "sym"):
                        creg_ref = VariableRef(cout.sym)
                        # Generate measure_array
                        call = FunctionCall(
                            func_name="quantum.measure_array",
                            args=[qreg_ref],
                        )
                        return Assignment(target=creg_ref, value=call)

                # No target - just measure
                call = FunctionCall(
                    func_name="quantum.measure_array",
                    args=[qreg_ref],
                )

                # Create expression statement wrapper
                class ExpressionStatement(Statement):
                    def __init__(self, expr):
                        self.expr = expr

                    def analyze(self, context):
                        self.expr.analyze(context)

                    def render(self, context):
                        return self.expr.render(context)

                return ExpressionStatement(call)

        # Handle single qubit measurement
        if len(meas.qargs) == 1:
            qarg = meas.qargs[0]
            qubit_ref = self._convert_qubit_ref(qarg)

            # Get target if specified
            target_ref = None
            if hasattr(meas, "cout") and meas.cout and len(meas.cout) == 1:
                cout = meas.cout[0]
                # For measurements, the target should use unpacked names if available
                # So we pass is_assignment_target=False to use unpacked names
                target_ref = self._convert_bit_ref(cout, is_assignment_target=False)

            # Track resource consumption for linearity checking
            if (
                hasattr(qarg, "reg")
                and hasattr(qarg.reg, "sym")
                and hasattr(qarg, "index")
            ):
                array_name = qarg.reg.sym
                qubit_index = qarg.index
                self.scope_manager.track_resource_usage(
                    array_name,
                    {qubit_index},
                    consumed=True,
                )

                # Also track globally for conditional resource balancing
                if not hasattr(self, "consumed_resources"):
                    self.consumed_resources = {}
                if array_name not in self.consumed_resources:
                    self.consumed_resources[array_name] = set()
                self.consumed_resources[array_name].add(qubit_index)

            # In the black box pattern, after measuring a qubit, we need to replace it
            # with a fresh qubit to maintain array structure for returns
            meas_stmt = Measurement(qubit=qubit_ref, target=target_ref)

            # If we're in a function with unpacked variables, replace measured qubit
            # But only if we're not in main (main doesn't return arrays)
            is_main = (
                hasattr(self, "current_function_name")
                and self.current_function_name == "main"
            )
            if (
                not is_main
                and hasattr(self, "unpacked_vars")
                and hasattr(qarg, "reg")
                and hasattr(qarg.reg, "sym")
                and hasattr(qarg, "index")
            ):
                array_name = qarg.reg.sym
                qubit_index = qarg.index

                # Check if this array is unpacked in current function
                if array_name in self.unpacked_vars:
                    element_names = self.unpacked_vars[array_name]
                    if qubit_index < len(element_names):
                        # Replace the measured qubit with a fresh one
                        replacement_stmt = Assignment(
                            target=VariableRef(element_names[qubit_index]),
                            value=FunctionCall(func_name="quantum.qubit", args=[]),
                        )

                        # Track that this qubit was replaced (not consumed)
                        if not hasattr(self, "replaced_qubits"):
                            self.replaced_qubits = {}
                        if array_name not in self.replaced_qubits:
                            self.replaced_qubits[array_name] = set()
                        self.replaced_qubits[array_name].add(qubit_index)

                        # Return a block with measurement followed by replacement
                        statements = [meas_stmt, replacement_stmt]
                        return Block(statements=statements)

            return meas_stmt

        # TODO: Handle multi-qubit measurements
        return Comment("TODO: Multi-qubit measurement")

    def _convert_qubit_ref(self, qarg) -> IRNode:
        """Convert a qubit reference to IR."""
        if hasattr(qarg, "reg") and hasattr(qarg.reg, "sym"):
            array_name = qarg.reg.sym
            original_array = array_name

            # Check if this array has been unpacked (for ancilla arrays with @owned)
            if (
                hasattr(self, "unpacked_vars")
                and array_name in self.unpacked_vars
                and hasattr(qarg, "index")
            ):
                # This array was unpacked - use the unpacked variable directly
                element_names = self.unpacked_vars[array_name]
                if qarg.index < len(element_names):
                    return VariableRef(element_names[qarg.index])

            # Check if this variable is mapped to a struct field (for @owned structs)
            if (
                hasattr(self, "struct_field_mapping")
                and original_array in self.struct_field_mapping
            ):
                struct_field_path = self.struct_field_mapping[original_array]
                if "." in struct_field_path:
                    struct_name, field_name = struct_field_path.split(".", 1)
                    if hasattr(qarg, "index"):
                        # Return struct.field[index]
                        field_access = FieldAccess(
                            obj=VariableRef(struct_name),
                            field=field_name,
                        )
                        return ArrayAccess(array=field_access, index=qarg.index)
                    # Return struct.field
                    return FieldAccess(obj=VariableRef(struct_name), field=field_name)

            # Check if this is a dynamically allocated array (ancilla)
            if (
                hasattr(self, "dynamic_allocations")
                and original_array in self.dynamic_allocations
                and hasattr(qarg, "index")
            ):
                # Create a variable name for this specific ancilla
                ancilla_var = f"{original_array}_{qarg.index}"

                # Check if we've already allocated this specific ancilla
                if not hasattr(self, "allocated_ancillas"):
                    self.allocated_ancillas = set()

                if ancilla_var not in self.allocated_ancillas:
                    # Allocate this ancilla now
                    alloc_stmt = Assignment(
                        target=VariableRef(ancilla_var),
                        value=FunctionCall(func_name="quantum.qubit", args=[]),
                    )
                    self.current_block.statements.append(alloc_stmt)
                    self.allocated_ancillas.add(ancilla_var)

                return VariableRef(ancilla_var)

            # Check if this variable is part of a struct and has been unpacked
            if hasattr(self, "var_remapping") and original_array in self.var_remapping:
                # Use the unpacked field variable
                unpacked_var_name = self.var_remapping[original_array]
                if hasattr(qarg, "index"):
                    # Array element access with unpacked variable: c_d[0]
                    return ArrayAccess(
                        array=VariableRef(unpacked_var_name),
                        index=qarg.index,
                    )
                # Full array access with unpacked variable: c_d
                return VariableRef(unpacked_var_name)

            # Check if this array is part of a struct (fallback)
            for prefix, info in self.struct_info.items():
                if array_name in info["var_names"].values():
                    # This is a struct field
                    suffix = next(
                        k for k, v in info["var_names"].items() if v == array_name
                    )

                    # Check if we're in a function that takes this struct as parameter
                    struct_param_name = prefix  # Default to the struct name
                    if hasattr(self, "param_mapping") and prefix in self.param_mapping:
                        struct_param_name = self.param_mapping[prefix]

                    if hasattr(qarg, "index"):
                        # Struct field element access: c.d[0]
                        field_access = FieldAccess(
                            obj=VariableRef(struct_param_name),
                            field=suffix,
                        )
                        return ArrayAccess(array=field_access, index=qarg.index)
                    # Full struct field access: c.d
                    return FieldAccess(obj=VariableRef(struct_param_name), field=suffix)

            # Check if we're inside a function and need to use remapped names
            if hasattr(self, "var_remapping") and original_array in self.var_remapping:
                array_name = self.var_remapping[original_array]

            # Check for renaming
            if array_name in self.plan.renamed_variables:
                array_name = self.plan.renamed_variables[array_name]

            if hasattr(qarg, "index"):
                # Array Unpacking Pattern: use unpacked variable names instead of array indexing
                # Check both the original name and any remapped name
                check_names = [original_array]
                if (
                    hasattr(self, "var_remapping")
                    and original_array in self.var_remapping
                ):
                    check_names.append(self.var_remapping[original_array])
                if array_name != original_array:
                    check_names.append(array_name)

                # Try each possible name for unpacked variables
                for check_name in check_names:
                    if (
                        hasattr(self, "unpacked_vars")
                        and check_name in self.unpacked_vars
                    ):
                        element_names = self.unpacked_vars[check_name]
                        if qarg.index < len(element_names):
                            return VariableRef(element_names[qarg.index])

                # Check if this element should be allocated locally
                decision = self.allocation_decisions.get(original_array)
                if decision and qarg.index in decision.local_elements:
                    # This element should be allocated locally
                    local_var_name = f"{original_array}_{qarg.index}_local"

                    # Add local allocation if not already done
                    if not hasattr(self, "_local_allocations"):
                        self._local_allocations = set()

                    if local_var_name not in self._local_allocations:
                        self._local_allocations.add(local_var_name)
                        # Add allocation statement
                        alloc_stmt = Assignment(
                            target=VariableRef(local_var_name),
                            value=FunctionCall(func_name="quantum.qubit", args=[]),
                        )
                        self.current_block.statements.append(alloc_stmt)

                    return VariableRef(local_var_name)

                # Array element access
                # Skip this shortcut - we need to check for unpacked vars first
                # The unpacking check above should handle function cases too

                # In main function, check if this array is unpacked
                if original_array in self.plan.arrays_to_unpack:
                    # This array should be unpacked, use unpacked name
                    info = self.plan.arrays_to_unpack[original_array]
                    if qarg.index < info.size:
                        # Check if the array is actually unpacked yet
                        var_info = self.context.lookup_variable(array_name)
                        if var_info and var_info.is_unpacked:
                            unpacked_name = f"{original_array}_{qarg.index}"
                            return VariableRef(unpacked_name)

                # Not unpacked or inside function, use array access
                return ArrayAccess(array_name=array_name, index=qarg.index)
            # Full array reference
            return VariableRef(array_name)
        if hasattr(qarg, "sym"):
            # Direct variable reference
            var_name = qarg.sym
            original_var = var_name

            # Check if we're inside a function and need to use remapped names
            if hasattr(self, "var_remapping") and original_var in self.var_remapping:
                var_name = self.var_remapping[original_var]

            # Check for renaming
            if var_name in self.plan.renamed_variables:
                var_name = self.plan.renamed_variables[var_name]
            return VariableRef(var_name)

        # Fallback
        return VariableRef(str(qarg))

    def _convert_bit_ref(self, carg, *, is_assignment_target: bool = False) -> IRNode:
        """Convert a classical bit reference to IR.

        Args:
            carg: The classical argument to convert
            is_assignment_target: If True, always use array indexing (for assignments)
        """
        if hasattr(carg, "reg") and hasattr(carg.reg, "sym"):
            array_name = carg.reg.sym
            original_array = array_name

            # Check if this variable is mapped to a struct field (for @owned structs)
            if (
                hasattr(self, "struct_field_mapping")
                and original_array in self.struct_field_mapping
            ):
                struct_field_path = self.struct_field_mapping[original_array]
                if "." in struct_field_path:
                    struct_name, field_name = struct_field_path.split(".", 1)
                    if hasattr(carg, "index"):
                        # Return struct.field[index]
                        field_access = FieldAccess(
                            obj=VariableRef(struct_name),
                            field=field_name,
                        )
                        return ArrayAccess(array=field_access, index=carg.index)
                    # Return struct.field
                    return FieldAccess(obj=VariableRef(struct_name), field=field_name)

            # Check if this variable is part of a struct and has been unpacked
            if hasattr(self, "var_remapping") and original_array in self.var_remapping:
                # Use the unpacked field variable
                unpacked_var_name = self.var_remapping[original_array]
                if hasattr(carg, "index"):
                    # Array element access with unpacked variable: c_verify_prep[0]
                    return ArrayAccess(
                        array=VariableRef(unpacked_var_name),
                        index=carg.index,
                    )
                # Full array access with unpacked variable: c_verify_prep
                return VariableRef(unpacked_var_name)

            # Check if this variable is part of a struct in main context (fallback)
            for prefix, info in self.struct_info.items():
                if original_array in info["var_names"].values():
                    # Find the field name
                    for suffix, var_name in info["var_names"].items():
                        if var_name == original_array:
                            # Check if we're in a function that receives the struct
                            struct_param_name = prefix
                            if (
                                hasattr(self, "param_mapping")
                                and prefix in self.param_mapping
                            ):
                                struct_param_name = self.param_mapping[prefix]

                            if hasattr(carg, "index"):
                                # Struct field element access: c.verify_prep[0]
                                field_access = FieldAccess(
                                    obj=VariableRef(struct_param_name),
                                    field=suffix,
                                )
                                return ArrayAccess(array=field_access, index=carg.index)
                            # Full struct field access: c.verify_prep
                            return FieldAccess(
                                obj=VariableRef(struct_param_name),
                                field=suffix,
                            )

            # Check if we're inside a function and need to use remapped names
            if hasattr(self, "var_remapping") and original_array in self.var_remapping:
                array_name = self.var_remapping[original_array]

            # Check for renaming
            if array_name in self.plan.renamed_variables:
                array_name = self.plan.renamed_variables[array_name]

            if hasattr(carg, "index"):
                # Check if this array is unpacked and we're not assigning
                var_info = self.context.lookup_variable(array_name)
                if (
                    not is_assignment_target
                    and var_info
                    and var_info.is_unpacked
                    and hasattr(var_info, "unpacked_names")
                ):
                    # Use unpacked variable name for reading
                    index = carg.index
                    if index < len(var_info.unpacked_names):
                        return VariableRef(var_info.unpacked_names[index])

                # Use array access for assignments or non-unpacked arrays
                return ArrayAccess(array_name=array_name, index=carg.index)
            # Full array reference
            return VariableRef(array_name)
        if hasattr(carg, "sym"):
            # Direct variable reference
            var_name = carg.sym
            # Check for renaming
            if var_name in self.plan.renamed_variables:
                var_name = self.plan.renamed_variables[var_name]
            return VariableRef(var_name)

        # Fallback
        return VariableRef(str(carg))

    def _convert_quantum_gate(self, gate) -> Statement | None:
        """Convert quantum gate operation."""
        gate_name = type(gate).__name__

        # Regular gate mapping for in-place operations
        gate_map = {
            "H": "quantum.h",
            "X": "quantum.x",
            "Y": "quantum.y",
            "Z": "quantum.z",
            "S": "quantum.s",
            "SZ": "quantum.s",
            "SZdg": "quantum.sdg",
            "T": "quantum.t",
            "Tdg": "quantum.tdg",
            "CX": "quantum.cx",
            "CY": "quantum.cy",
            "CZ": "quantum.cz",
            "Prep": "quantum.reset",
        }

        if gate_name not in gate_map:
            return Comment(f"Unknown gate: {gate_name}")

        func_name = gate_map[gate_name]

        # Convert qubit arguments
        args = []
        if hasattr(gate, "qargs") and gate.qargs:
            # Check if this is a single-qubit gate with multiple arguments
            if (
                gate_name in ["H", "X", "Y", "Z", "S", "SZ", "SZdg", "T", "Tdg", "Prep"]
                and len(gate.qargs) > 1
            ):
                # Single-qubit gate applied to multiple qubits
                # Check if all qargs are consecutive array elements from the same array
                if (
                    all(
                        hasattr(qarg, "reg") and hasattr(qarg, "index")
                        for qarg in gate.qargs
                    )
                    and len({qarg.reg.sym for qarg in gate.qargs}) == 1
                ):
                    # All from same array - check if consecutive
                    indices = [qarg.index for qarg in gate.qargs]
                    array_name = gate.qargs[0].reg.sym

                    if indices == list(range(min(indices), max(indices) + 1)):
                        # Consecutive indices - generate a loop
                        loop_var = "i"
                        start = min(indices)
                        stop = max(indices) + 1

                        # Create loop body
                        body_block = Block()

                        # Check if the array name needs remapping (for unpacked struct fields)
                        actual_array_name = array_name
                        if (
                            hasattr(self, "var_remapping")
                            and array_name in self.var_remapping
                        ):
                            actual_array_name = self.var_remapping[array_name]

                        array_ref = VariableRef(actual_array_name)
                        index_ref = VariableRef(loop_var)
                        elem_access = ArrayAccess(array=array_ref, index=index_ref)
                        call = FunctionCall(func_name=func_name, args=[elem_access])

                        # Create expression statement wrapper
                        class ExpressionStatement(Statement):
                            def __init__(self, expr):
                                self.expr = expr

                            def analyze(self, context):
                                self.expr.analyze(context)

                            def render(self, context):
                                return self.expr.render(context)

                        body_block.statements.append(ExpressionStatement(call))

                        # Create for loop
                        range_call = FunctionCall(
                            func_name="range",
                            args=[Literal(start), Literal(stop)],
                        )
                        return ForStatement(
                            loop_var=loop_var,
                            iterable=range_call,
                            body=body_block,
                        )

                # Not consecutive or not from same array - expand to individual calls
                stmts = []
                for qarg in gate.qargs:
                    qref = self._convert_qubit_ref(qarg)
                    call = FunctionCall(func_name=func_name, args=[qref])

                    # Create expression statement wrapper
                    class ExpressionStatement(Statement):
                        def __init__(self, expr):
                            self.expr = expr

                        def analyze(self, context):
                            self.expr.analyze(context)

                        def render(self, context):
                            return self.expr.render(context)

                    stmts.append(ExpressionStatement(call))
                # Return a block with all statements
                return Block(statements=stmts)
            # Handle multi-qubit gates with tuple arguments
            if gate_name in ["CX", "CY", "CZ"] and all(
                isinstance(arg, tuple) and len(arg) == 2 for arg in gate.qargs
            ):
                # Multiple (control, target) pairs - generate multiple statements
                stmts = []
                for ctrl, tgt in gate.qargs:
                    ctrl_ref = self._convert_qubit_ref(ctrl)
                    tgt_ref = self._convert_qubit_ref(tgt)
                    call = FunctionCall(func_name=func_name, args=[ctrl_ref, tgt_ref])

                    # Create expression statement wrapper
                    class ExpressionStatement(Statement):
                        def __init__(self, expr):
                            self.expr = expr

                        def analyze(self, context):
                            self.expr.analyze(context)

                        def render(self, context):
                            return self.expr.render(context)

                    stmts.append(ExpressionStatement(call))
                # Return a block with all statements
                return Block(statements=stmts)
            # Standard argument handling
            for qarg in gate.qargs:
                # Check if this is a full array (no index)
                if hasattr(qarg, "sym") and hasattr(qarg, "size") and qarg.size > 1:
                    # This is a full array - need to expand to individual gates
                    stmts = []
                    array_name = qarg.sym

                    # Check for renaming
                    if array_name in self.plan.renamed_variables:
                        array_name = self.plan.renamed_variables[array_name]

                    # Check if this array name needs remapping (for unpacked struct fields)
                    if (
                        hasattr(self, "var_remapping")
                        and array_name in self.var_remapping
                    ):
                        array_name = self.var_remapping[array_name]

                    # Apply gate to each element
                    # For operations on arrays, we need to expand to individual operations
                    # However, reset operations in functions with owned arrays
                    # need special handling

                    if (
                        gate_name == "Prep"
                        and hasattr(self, "var_remapping")
                        and self.var_remapping
                        and array_name in self.var_remapping
                    ):
                        # Array Unpacking Pattern: use unpacked variables with
                        # functional operations
                        stmts.append(Comment(f"Reset all qubits in {array_name}"))

                        if (
                            hasattr(self, "unpacked_vars")
                            and array_name in self.unpacked_vars
                        ):
                            # Use unpacked variables with functional assignments
                            element_names = self.unpacked_vars[array_name]
                            for i in range(min(qarg.size, len(element_names))):
                                elem_var = VariableRef(element_names[i])
                                call = FunctionCall(
                                    func_name=func_name,
                                    args=[elem_var],
                                )

                                # Functional assignment: q_i = reset(q_i)
                                assignment = Assignment(target=elem_var, value=call)
                                stmts.append(assignment)
                        else:
                            # Fallback to array indexing if no unpacking
                            for i in range(qarg.size):
                                elem_ref = ArrayAccess(array_name=array_name, index=i)
                                call = FunctionCall(
                                    func_name=func_name,
                                    args=[elem_ref],
                                )

                                # Create expression statement wrapper
                                class ExpressionStatement(Statement):
                                    def __init__(self, expr):
                                        self.expr = expr

                                    def analyze(self, context):
                                        self.expr.analyze(context)

                                    def render(self, context):
                                        return self.expr.render(context)

                                stmts.append(ExpressionStatement(call))
                    else:
                        # Regular case - generate a loop instead of expanding
                        # Check if this array is part of a struct
                        is_struct_field = False

                        # First check if we have a remapped variable (unpacked struct field)
                        # The key insight is that if we're in a function with
                        # @owned struct parameters
                        # and this array is a struct field that has been unpacked, we should use
                        # the unpacked variable name directly, not struct.field notation
                        use_unpacked = False
                        if (
                            hasattr(self, "var_remapping")
                            and array_name in self.var_remapping
                        ):
                            # Check if this is a struct field that has been unpacked
                            for prefix, info in self.struct_info.items():
                                if array_name in info["var_names"].values() and hasattr(
                                    self,
                                    "current_function_params",
                                ):
                                    # Check if the struct is an @owned parameter
                                    for (
                                        param_name,
                                        param_type,
                                    ) in self.current_function_params:
                                        if param_name == prefix and "@owned" in str(
                                            param_type,
                                        ):
                                            use_unpacked = True
                                            break
                                    if use_unpacked:
                                        break

                        if use_unpacked:
                            # Generate a loop using the unpacked variable
                            loop_var = "i"
                            body_block = Block()

                            # Use the remapped name from var_remapping
                            remapped_name = self.var_remapping.get(
                                array_name,
                                array_name,
                            )
                            elem_ref = ArrayAccess(
                                array=VariableRef(remapped_name),
                                index=VariableRef(loop_var),
                            )
                            call = FunctionCall(func_name=func_name, args=[elem_ref])

                            # Create expression statement wrapper
                            class ExpressionStatement(Statement):
                                def __init__(self, expr):
                                    self.expr = expr

                                def analyze(self, context):
                                    self.expr.analyze(context)

                                def render(self, context):
                                    return self.expr.render(context)

                            body_block.statements.append(ExpressionStatement(call))

                            # Create for loop
                            range_call = FunctionCall(
                                func_name="range",
                                args=[Literal(0), Literal(qarg.size)],
                            )
                            for_stmt = ForStatement(
                                loop_var=loop_var,
                                iterable=range_call,
                                body=body_block,
                            )
                            stmts.append(for_stmt)
                            is_struct_field = True  # Skip the struct field check below

                        if not is_struct_field:
                            for prefix, info in self.struct_info.items():
                                if qarg.sym in info["var_names"].values():
                                    # Find the field name
                                    for suffix, var_name in info["var_names"].items():
                                        if var_name == qarg.sym:
                                            # Check if we're in a function that receives the struct
                                            struct_param_name = prefix
                                            if (
                                                hasattr(self, "param_mapping")
                                                and prefix in self.param_mapping
                                            ):
                                                struct_param_name = self.param_mapping[
                                                    prefix
                                                ]

                                            # Generate a loop for struct field access
                                            loop_var = "i"
                                            body_block = Block()

                                            field_access = FieldAccess(
                                                obj=VariableRef(struct_param_name),
                                                field=suffix,
                                            )
                                            elem_ref = ArrayAccess(
                                                array=field_access,
                                                index=VariableRef(loop_var),
                                            )
                                            call = FunctionCall(
                                                func_name=func_name,
                                                args=[elem_ref],
                                            )

                                        # Create expression statement wrapper
                                        class ExpressionStatement(Statement):
                                            def __init__(self, expr):
                                                self.expr = expr

                                            def analyze(self, context):
                                                self.expr.analyze(context)

                                            def render(self, context):
                                                return self.expr.render(context)

                                        body_block.statements.append(
                                            ExpressionStatement(call),
                                        )

                                        # Create for loop
                                        range_call = FunctionCall(
                                            func_name="range",
                                            args=[Literal(0), Literal(qarg.size)],
                                        )
                                        for_stmt = ForStatement(
                                            loop_var=loop_var,
                                            iterable=range_call,
                                            body=body_block,
                                        )
                                        stmts.append(for_stmt)
                                        is_struct_field = True
                                        break
                                break

                        if not is_struct_field:
                            # Not in a struct - generate a loop
                            loop_var = "i"
                            body_block = Block()

                            # Check if the array name needs remapping (for unpacked struct fields)
                            actual_array_name = array_name
                            if (
                                hasattr(self, "var_remapping")
                                and array_name in self.var_remapping
                            ):
                                actual_array_name = self.var_remapping[array_name]

                            elem_ref = ArrayAccess(
                                array=VariableRef(actual_array_name),
                                index=VariableRef(loop_var),
                            )
                            call = FunctionCall(func_name=func_name, args=[elem_ref])

                            # Create expression statement wrapper
                            class ExpressionStatement(Statement):
                                def __init__(self, expr):
                                    self.expr = expr

                                def analyze(self, context):
                                    self.expr.analyze(context)

                                def render(self, context):
                                    return self.expr.render(context)

                            body_block.statements.append(ExpressionStatement(call))

                            # Create for loop
                            range_call = FunctionCall(
                                func_name="range",
                                args=[Literal(0), Literal(qarg.size)],
                            )
                            for_stmt = ForStatement(
                                loop_var=loop_var,
                                iterable=range_call,
                                body=body_block,
                            )
                            stmts.append(for_stmt)

                    # Return a block with all statements
                    return Block(statements=stmts)
                args.append(self._convert_qubit_ref(qarg))

        # If we get here, we have regular args (not arrays)
        if args:
            # Create function call expression
            call = FunctionCall(func_name=func_name, args=args)

            # No longer use functional operations - all gates are in-place

            # Create expression statement wrapper for non-functional operations
            class ExpressionStatement(Statement):
                def __init__(self, expr):
                    self.expr = expr

                def analyze(self, context):
                    self.expr.analyze(context)

                def render(self, context):
                    return self.expr.render(context)

            return ExpressionStatement(call)

        return None

    def _convert_if(self, if_block) -> Statement | None:
        """Convert If block."""
        # Check if this If block has struct field access in loop with @owned parameters
        if hasattr(if_block, "cond") and self._is_struct_field_in_loop_with_owned(
            if_block.cond,
        ):
            # Implement a proper fix by extracting the condition value before the conditional
            # This allows us to check the struct field without violating @owned constraints

            # Extract the struct field that's being tested
            condition_var = self._extract_condition_variable(if_block.cond)
            if condition_var:
                self.current_block.statements.append(
                    Comment(
                        "Extract condition variable to avoid @owned struct field access in loop",
                    ),
                )

                # Create a local variable to hold the condition value
                condition_stmt = Assignment(
                    target=VariableRef(condition_var["var_name"]),
                    value=self._convert_condition_value(if_block.cond),
                )
                self.current_block.statements.append(condition_stmt)

                # Convert then block first
                then_block = Block()
                if hasattr(if_block, "ops"):
                    # Enter a new scope for the If block
                    prev_block = self.current_block
                    self.current_block = then_block

                    for op in if_block.ops:
                        stmt = self._convert_operation(op)
                        if stmt:
                            then_block.statements.append(stmt)

                    self.current_block = prev_block

                # Now create the If statement using the extracted variable
                if condition_var["comparison"] == "EQUIV":
                    # For bool comparison with 1, convert to just the boolean variable
                    # Since verify_prep[0] is bool and we're checking == 1,
                    # this means "if verification failed" which is just the boolean value
                    if condition_var["compare_value"] == 1:
                        condition = VariableRef(condition_var["var_name"])
                    else:
                        # For other comparisons, use == operator with appropriate type
                        condition = BinaryOp(
                            left=VariableRef(condition_var["var_name"]),
                            op="==",
                            right=Literal(condition_var["compare_value"]),
                        )
                else:
                    condition = VariableRef(condition_var["var_name"])

                # Create and return the If statement
                return IfStatement(
                    condition=condition,
                    then_block=then_block,
                )
            # Fallback to the conservative approach if we can't extract the condition
            self.current_block.statements.append(
                Comment(
                    "Fallback: If condition with struct field access "
                    "simplified for @owned compatibility",
                ),
            )

            # Convert the If body operations unconditionally
            if hasattr(if_block, "ops"):
                for op in if_block.ops:
                    stmt = self._convert_operation(op)
                    if stmt:
                        self.current_block.statements.append(stmt)

            return None

        # Convert condition
        condition = self._convert_condition(if_block.cond)

        # Track what resources were consumed before this conditional
        # We need to ensure we don't try to re-consume them in else blocks
        consumed_before_if = {}
        if not hasattr(self, "consumed_resources"):
            self.consumed_resources = {}
        for res_name, indices in self.consumed_resources.items():
            consumed_before_if[res_name] = (
                indices.copy() if isinstance(indices, set) else set(indices)
            )

        # Convert then block with scope tracking
        then_block = Block()
        prev_block = self.current_block

        with self.scope_manager.enter_scope(ScopeType.IF_THEN) as then_scope:
            self.current_block = then_block

            if hasattr(if_block, "ops"):
                for op in if_block.ops:
                    stmt = self._convert_operation(op)
                    if stmt:
                        then_block.statements.append(stmt)

        # Convert else block if present
        else_block = None
        else_scope_info = None

        if hasattr(if_block, "else_block") and if_block.else_block:
            else_block = Block()

            with self.scope_manager.enter_scope(ScopeType.IF_ELSE) as else_scope:
                else_scope_info = else_scope
                self.current_block = else_block

                if hasattr(if_block.else_block, "ops"):
                    for op in if_block.else_block.ops:
                        stmt = self._convert_operation(op)
                        if stmt:
                            else_block.statements.append(stmt)

        # Check for resource balancing needs
        # Analyze resource consumption across branches
        unbalanced = self.scope_manager.analyze_conditional_branches(
            then_scope,
            else_scope_info,
            self.context,
        )

        # If there are unbalanced resources, we need to balance them
        if unbalanced:
            # Helper function to add resource consumption
            def add_resource_consumption(block, res_name, indices):
                # Filter out indices that were already consumed before the if statement
                if res_name in consumed_before_if:
                    already_consumed = consumed_before_if[res_name]
                    indices = indices - already_consumed

                if indices:
                    block.statements.append(
                        Comment("Consume qubits to maintain linearity"),
                    )
                    for idx in sorted(indices):
                        # Check if resource is unpacked
                        if res_name in self.unpacked_vars:
                            element_names = self.unpacked_vars[res_name]
                            if idx < len(element_names):
                                # Measure the unpacked qubit
                                meas_expr = FunctionCall(
                                    func_name="quantum.measure",
                                    args=[VariableRef(element_names[idx])],
                                )
                                block.statements.append(
                                    Assignment(
                                        target=VariableRef("_"),
                                        value=meas_expr,
                                    ),
                                )
                        else:
                            # Use array indexing
                            meas_expr = FunctionCall(
                                func_name="quantum.measure",
                                args=[ArrayAccess(array_name=res_name, index=idx)],
                            )
                            block.statements.append(
                                Assignment(target=VariableRef("_"), value=meas_expr),
                            )

            # If we have an else block, add balancing to both branches
            if else_block:
                # Add to then branch what else consumed
                for res_name, indices in unbalanced.items():
                    if res_name in then_scope.resource_usage:
                        then_usage = then_scope.resource_usage[res_name]
                        else_usage = else_scope_info.resource_usage.get(
                            res_name,
                            ResourceUsage(res_name, set()),
                        )
                        missing_in_then = else_usage.consumed - then_usage.consumed
                        if missing_in_then:
                            add_resource_consumption(
                                then_block,
                                res_name,
                                missing_in_then,
                            )

                # Add to else branch what then consumed
                for res_name in then_scope.resource_usage:
                    then_usage = then_scope.resource_usage[res_name]
                    else_usage = else_scope_info.resource_usage.get(
                        res_name,
                        ResourceUsage(res_name, set()),
                    )
                    missing_in_else = then_usage.consumed - else_usage.consumed
                    if missing_in_else:
                        add_resource_consumption(else_block, res_name, missing_in_else)
            else:
                # No else block - create one to consume resources
                else_block = Block()
                else_block.statements.append(
                    Comment("Auto-generated else block for linearity"),
                )

                for res_name, indices in unbalanced.items():
                    add_resource_consumption(else_block, res_name, indices)

        self.current_block = prev_block

        return IfStatement(
            condition=condition,
            then_block=then_block,
            else_block=else_block,
        )

    def _convert_while(self, while_block) -> Statement | None:
        """Convert While loop."""
        # Convert condition
        condition = self._convert_condition(while_block.cond)

        # Convert body with scope tracking
        body_block = Block()
        prev_block = self.current_block

        with self.scope_manager.enter_scope(ScopeType.LOOP):
            self.current_block = body_block

            if hasattr(while_block, "ops"):
                for op in while_block.ops:
                    stmt = self._convert_operation(op)
                    if stmt:
                        body_block.statements.append(stmt)

        self.current_block = prev_block

        return WhileStatement(
            condition=condition,
            body=body_block,
        )

    def _convert_for(self, for_block) -> Statement | None:
        """Convert For loop."""
        # Get loop variable and range
        loop_var = for_block.var

        # Determine the iteration pattern
        if hasattr(for_block, "iterable") and for_block.iterable:
            # For(i, iterable)
            return self._convert_for_iterable(for_block, loop_var)
        if hasattr(for_block, "start") and hasattr(for_block, "stop"):
            # For(i, start, stop, [step])
            return self._convert_for_range(for_block, loop_var)
        # Unknown pattern
        return Comment(f"TODO: Unsupported For loop pattern with variable {loop_var}")

    def _convert_for_range(self, for_block, loop_var) -> Statement | None:
        """Convert For loop with range pattern."""
        start = for_block.start
        stop = for_block.stop
        step = getattr(for_block, "step", 1)

        # Create range() call
        if step == 1:
            # range(start, stop)
            range_call = FunctionCall(
                func_name="range",
                args=[Literal(start), Literal(stop)],
            )
        else:
            # range(start, stop, step)
            range_call = FunctionCall(
                func_name="range",
                args=[Literal(start), Literal(stop), Literal(step)],
            )

        # Convert body with scope tracking
        body_block = Block()
        prev_block = self.current_block

        with self.scope_manager.enter_scope(ScopeType.LOOP):
            self.current_block = body_block

            if hasattr(for_block, "ops"):
                for op in for_block.ops:
                    stmt = self._convert_operation(op)
                    if stmt:
                        body_block.statements.append(stmt)

        self.current_block = prev_block

        return ForStatement(
            loop_var=str(loop_var),
            iterable=range_call,
            body=body_block,
        )

    def _convert_for_iterable(self, for_block, loop_var) -> Statement | None:
        """Convert For loop with iterable pattern."""
        # For now, just handle the iterable as a variable reference
        iterable = for_block.iterable

        # Try to convert it to an IR node
        if isinstance(iterable, str):
            iter_node = VariableRef(iterable)
        elif hasattr(iterable, "sym"):
            iter_node = VariableRef(iterable.sym)
        else:
            # Try to represent it somehow
            iter_node = Literal(str(iterable))

        # Convert body
        body_block = Block()
        prev_block = self.current_block

        with self.scope_manager.enter_scope(ScopeType.LOOP):
            self.current_block = body_block

            if hasattr(for_block, "ops"):
                for op in for_block.ops:
                    stmt = self._convert_operation(op)
                    if stmt:
                        body_block.statements.append(stmt)

        self.current_block = prev_block

        return ForStatement(
            loop_var=str(loop_var),
            iterable=iter_node,
            body=body_block,
        )

    def _convert_condition(self, cond) -> IRNode:
        """Convert condition expression."""
        cond_type = type(cond).__name__

        if cond_type == "Bit":
            # Bit reference
            return self._convert_bit_ref(cond)
        if cond_type == "EQUIV":
            # Equality comparison

            left = self._convert_condition(cond.left)
            right = self._convert_condition(cond.right)

            # Optimize boolean comparisons to 1
            if (
                isinstance(right, Literal)
                and right.value == 1
                and type(cond.left).__name__ == "Bit"
            ):
                # Just return the boolean value itself
                return left

            return BinaryOp(left=left, op="==", right=right)
        if cond_type == "LT":
            # Less than
            left = self._convert_condition(cond.left)
            right = self._convert_condition(cond.right)
            return BinaryOp(left=left, op="<", right=right)
        if cond_type == "GT":
            # Greater than
            left = self._convert_condition(cond.left)
            right = self._convert_condition(cond.right)
            return BinaryOp(left=left, op=">", right=right)
        if cond_type == "AND":
            # Bitwise AND (used as logical in conditions)
            left = self._convert_condition(cond.left)
            right = self._convert_condition(cond.right)
            return BinaryOp(left=left, op="&", right=right)
        if cond_type == "OR":
            # Bitwise OR (used as logical in conditions)
            left = self._convert_condition(cond.left)
            right = self._convert_condition(cond.right)
            return BinaryOp(left=left, op="|", right=right)
        if cond_type == "NOT":
            # Logical NOT
            operand = self._convert_condition(cond.value)
            return UnaryOp(op="not", operand=operand)
        if hasattr(cond, "value"):
            # Literal value
            return Literal(cond.value)
        if isinstance(cond, int | bool | str):
            # Direct literal
            return Literal(cond)

        # Default: try to convert as bit reference
        return self._convert_bit_ref(cond)

    def _convert_repeat(self, repeat_block) -> Statement | None:
        """Convert Repeat block to for loop."""
        # Repeat is essentially a for loop with an anonymous variable
        repeat_count = repeat_block.cond

        # Convert body
        body_block = Block()
        prev_block = self.current_block

        with self.scope_manager.enter_scope(ScopeType.LOOP):
            self.current_block = body_block

            if hasattr(repeat_block, "ops"):
                for op in repeat_block.ops:
                    stmt = self._convert_operation(op)
                    if stmt:
                        body_block.statements.append(stmt)

        self.current_block = prev_block

        # Create ForStatement with anonymous variable
        return ForStatement(
            loop_var="_",
            iterable=FunctionCall(func_name="range", args=[Literal(repeat_count)]),
            body=body_block,
        )

    def _convert_comment(self, comment) -> Statement | None:
        """Convert comment."""
        if hasattr(comment, "txt") and comment.txt:
            return Comment(comment.txt)
        return None  # Skip empty comments

    def _is_struct_field_in_loop_with_owned(self, cond) -> bool:
        """Check if a condition accesses a struct field in a problematic context.

        Returns True if:
        1. We're in a loop scope
        2. We're in a function with @owned struct parameters
        3. The condition accesses a struct field
        """
        # Check if we're in a loop
        if not hasattr(self, "scope_manager") or not self.scope_manager.is_in_loop():
            return False

        # Check if we're in a function with @owned struct parameters
        if not hasattr(self, "function_info") or self.current_function_name == "main":
            return False

        func_info = self.function_info.get(self.current_function_name, {})
        if not func_info.get("has_owned_struct_params", False):
            return False

        # Check if the condition accesses a struct field
        # Handle different condition types
        cond_type = type(cond).__name__

        if cond_type == "EQUIV":
            # For equality comparisons, check the left side
            if hasattr(cond, "left"):
                return self._is_struct_field_in_loop_with_owned(cond.left)
        elif hasattr(cond, "reg") and hasattr(cond.reg, "sym"):
            array_name = cond.reg.sym
            # Check if this variable is a struct field
            for info in self.struct_info.values():
                if array_name in info["var_names"].values():
                    return True

        return False

    def _extract_condition_variable(self, cond) -> dict | None:
        """Extract information about a condition variable that accesses a struct field.

        Returns a dict with:
        - var_name: suggested variable name for the extracted value
        - struct_field: the struct field being accessed (e.g., 'c.verify_prep[0]')
        - comparison: the comparison type (e.g., 'EQUIV')
        - compare_value: the value being compared against
        """
        cond_type = type(cond).__name__

        if cond_type == "EQUIV" and hasattr(cond, "left") and hasattr(cond, "right"):
            # Handle EQUIV(c_verify_prep[0], 1)
            left = cond.left
            right = cond.right

            # Check if left side is a struct field access
            if (
                hasattr(left, "reg")
                and hasattr(left.reg, "sym")
                and hasattr(left, "index")
            ):
                array_name = left.reg.sym
                index = left.index

                # Check if this is a struct field
                for prefix, info in self.struct_info.items():
                    if array_name in info["var_names"].values():
                        # Find the field name
                        field_name = None
                        for suffix, var_name in info["var_names"].items():
                            if var_name == array_name:
                                field_name = suffix
                                break

                        if field_name:
                            # Extract the comparison value
                            compare_value = (
                                getattr(right, "val", right)
                                if hasattr(right, "val")
                                else right
                            )

                            return {
                                "var_name": f"{field_name}_{index}_extracted",
                                "struct_field": f"{prefix}.{field_name}[{index}]",
                                "comparison": "EQUIV",
                                "compare_value": compare_value,
                            }

        return None

    def _convert_condition_value(self, cond) -> IRNode:
        """Convert the struct field access part of a condition to an IR node."""
        cond_type = type(cond).__name__

        if cond_type == "EQUIV" and hasattr(cond, "left"):
            # For EQUIV(c_verify_prep[0], 1), convert the left side (c_verify_prep[0])
            left = cond.left

            if (
                hasattr(left, "reg")
                and hasattr(left.reg, "sym")
                and hasattr(left, "index")
            ):
                array_name = left.reg.sym
                index = left.index

                # Check if this is a struct field and get the struct parameter name
                for prefix, info in self.struct_info.items():
                    if array_name in info["var_names"].values():
                        # Find the field name
                        field_name = None
                        for suffix, var_name in info["var_names"].items():
                            if var_name == array_name:
                                field_name = suffix
                                break

                        if field_name:
                            # Get the struct parameter name (e.g., 'c')
                            struct_param_name = prefix
                            if (
                                hasattr(self, "param_mapping")
                                and prefix in self.param_mapping
                            ):
                                struct_param_name = self.param_mapping[prefix]

                            # Create: c.verify_prep[0]
                            field_access = FieldAccess(
                                obj=VariableRef(struct_param_name),
                                field=field_name,
                            )
                            return ArrayAccess(array=field_access, index=index)

        # Fallback
        return Literal(0)

    def _convert_set_operation(self, set_op) -> Statement | None:
        """Convert SET operation for classical bits."""
        if not hasattr(set_op, "left") or not hasattr(set_op, "right"):
            return Comment("Invalid SET operation")

        # Convert left side (target) - use array indexing for assignments
        target = self._convert_bit_ref(set_op.left, is_assignment_target=True)

        # Convert right side (value)
        value = self._convert_set_value(set_op.right)

        return Assignment(target=target, value=value)

    def _convert_set_value(self, value, parent_op=None) -> IRNode:
        """Convert value in SET operation.

        Args:
            value: The value to convert
            parent_op: The parent operation type (if any) to determine if parens are needed
        """
        # Check if it's a literal
        if isinstance(value, int | bool):
            return Literal(bool(value))

        # Check if it's a bit reference
        value_type = type(value).__name__
        if value_type == "Bit":
            return self._convert_bit_ref(value)

        # Check for bitwise operations
        if value_type == "XOR":
            left = self._convert_set_value(value.left, parent_op=value_type)
            right = self._convert_set_value(value.right, parent_op=value_type)
            result = BinaryOp(left=left, op="^", right=right)
            # XOR has same precedence as AND, higher than OR
            # Only need parens if parent is AND (to clarify precedence)
            if parent_op == "AND":
                result.needs_parens = True
            return result
        if value_type == "AND":
            left = self._convert_set_value(value.left, parent_op=value_type)
            right = self._convert_set_value(value.right, parent_op=value_type)
            result = BinaryOp(left=left, op="&", right=right)
            # Mark as needing parens if it's a child of |
            if parent_op == "OR":
                result.needs_parens = True
            return result
        if value_type == "OR":
            left = self._convert_set_value(value.left, parent_op=value_type)
            right = self._convert_set_value(value.right, parent_op=value_type)
            return BinaryOp(left=left, op="|", right=right)
        if value_type == "NOT":
            # NOT might have 'operand' or be applied to first item
            if hasattr(value, "operand"):
                operand = self._convert_set_value(value.operand, parent_op=value_type)
            elif hasattr(value, "value"):
                operand = self._convert_set_value(value.value, parent_op=value_type)
            else:
                # Try to get the operand another way
                operand = Literal(value=True)
            return UnaryOp(op="not", operand=operand)

        # Unknown value type - generate function call as fallback
        args = []
        if hasattr(value, "left"):
            args.append(self._convert_set_value(value.left, parent_op=value_type))
        if hasattr(value, "right"):
            args.append(self._convert_set_value(value.right, parent_op=value_type))
        return FunctionCall(func_name=value_type, args=args)

    def _convert_permute(self, permute) -> Statement | None:
        """Convert Permute operation."""
        # Permute swaps registers or elements
        # In Guppy, we can implement this using Python's swap syntax

        if hasattr(permute, "elems_i") and hasattr(permute, "elems_f"):
            elems_i = permute.elems_i
            elems_f = permute.elems_f

            # Case 1: Simple register swap (a, b = b, a)
            if hasattr(elems_i, "sym") and hasattr(elems_f, "sym"):
                # Full register swap
                comment = Comment(f"Swap {elems_i.sym} and {elems_f.sym}")
                self.current_block.statements.append(comment)

                # In Guppy, we need to use a temporary variable
                temp_var = f"_temp_{elems_i.sym}"

                # temp = a
                self.current_block.statements.append(
                    Assignment(
                        target=VariableRef(temp_var),
                        value=VariableRef(elems_i.sym),
                    ),
                )

                # a = b
                self.current_block.statements.append(
                    Assignment(
                        target=VariableRef(elems_i.sym),
                        value=VariableRef(elems_f.sym),
                    ),
                )

                # b = temp
                self.current_block.statements.append(
                    Assignment(
                        target=VariableRef(elems_f.sym),
                        value=VariableRef(temp_var),
                    ),
                )

                return None  # Already added statements

            # Case 2: List of elements permutation
            if isinstance(elems_i, list) and isinstance(elems_f, list):
                if len(elems_i) != len(elems_f):
                    return Comment("ERROR: Permutation lists must have same length")

                # Analyze the permutation pattern
                permutation_map = self._analyze_permutation(elems_i, elems_f)

                if permutation_map is None:
                    return Comment("ERROR: Invalid permutation - elements don't match")

                # Generate permutation code based on the pattern
                return self._generate_permutation_code(
                    permutation_map,
                    elems_i,
                    elems_f,
                )

        # Fallback for unrecognized patterns
        return Comment("TODO: Implement complex permutation")

    def _analyze_permutation(self, elems_i, elems_f):
        """Analyze permutation to create a mapping."""
        # Create a set of all elements to ensure they match
        elems_i_set = set()
        elems_f_set = set()

        # Build element signatures for comparison
        for elem in elems_i:
            if hasattr(elem, "reg") and hasattr(elem, "index"):
                elems_i_set.add((elem.reg.sym, elem.index))
            elif hasattr(elem, "sym"):
                # Full register reference
                elems_i_set.add((elem.sym, None))

        for elem in elems_f:
            if hasattr(elem, "reg") and hasattr(elem, "index"):
                elems_f_set.add((elem.reg.sym, elem.index))
            elif hasattr(elem, "sym"):
                elems_f_set.add((elem.sym, None))

        # Check if the sets match (same elements, just reordered)
        if elems_i_set != elems_f_set:
            return None

        # Create the mapping: what goes to position i
        # If elems_f[i] == elems_i[j], then position i gets value from position j
        permutation_map = {}
        for i, elem_f in enumerate(elems_f):
            # Find which element in elems_i matches elem_f
            for j, elem_i in enumerate(elems_i):
                if self._elements_equal(elem_i, elem_f):
                    permutation_map[i] = j  # position i gets value from position j
                    break

        return permutation_map

    def _elements_equal(self, elem1, elem2):
        """Check if two elements refer to the same qubit."""
        # Both are register[index] references
        if (
            hasattr(elem1, "reg")
            and hasattr(elem1, "index")
            and hasattr(elem2, "reg")
            and hasattr(elem2, "index")
        ):
            return elem1.reg.sym == elem2.reg.sym and elem1.index == elem2.index
        # Both are full register references
        if hasattr(elem1, "sym") and hasattr(elem2, "sym"):
            return elem1.sym == elem2.sym
        return False

    def _generate_permutation_code(self, permutation_map, elems_i, elems_f):
        """Generate code for complex permutation patterns."""
        _ = elems_f  # Currently not used, reserved for future use
        # Identify cycles in the permutation
        cycles = self._find_permutation_cycles(permutation_map)

        if not cycles:
            return Comment("Identity permutation - no action needed")

        # Add comment describing the permutation
        self.current_block.statements.append(
            Comment(f"Permute {len(elems_i)} elements"),
        )

        # For each cycle, generate swap operations
        for cycle in cycles:
            if len(cycle) == 1:
                # Fixed point, no action needed
                continue
            if len(cycle) == 2:
                # Simple swap
                self._generate_swap(elems_i[cycle[0]], elems_i[cycle[1]])
            else:
                # Multi-element cycle: use temporary variables
                self._generate_cycle_permutation(cycle, elems_i)

        return None  # Statements already added

    def _find_permutation_cycles(self, permutation_map):
        """Find cycles in a permutation."""
        visited = set()
        cycles = []

        for start in permutation_map:
            if start in visited:
                continue

            cycle = []
            current = start
            while current not in visited:
                visited.add(current)
                cycle.append(current)
                current = permutation_map.get(current, current)

            if len(cycle) > 0 and (
                len(cycle) > 1 or cycle[0] != permutation_map.get(cycle[0], cycle[0])
            ):
                cycles.append(cycle)

        return cycles

    def _generate_swap(self, elem1, elem2):
        """Generate code to swap two elements."""
        ref1 = self._convert_qubit_ref(elem1)
        ref2 = self._convert_qubit_ref(elem2)

        # Use a temporary variable
        temp_var = "_temp_swap"

        self.current_block.statements.append(
            Assignment(target=VariableRef(temp_var), value=ref1),
        )
        self.current_block.statements.append(
            Assignment(target=ref1, value=ref2),
        )
        self.current_block.statements.append(
            Assignment(target=ref2, value=VariableRef(temp_var)),
        )

    def _generate_cycle_permutation(self, cycle, elements):
        """Generate code for a multi-element cycle permutation."""
        if len(cycle) < 2:
            return

        # Save the first element
        first_elem = elements[cycle[0]]
        first_ref = self._convert_qubit_ref(first_elem)
        temp_var = "_temp_cycle"

        self.current_block.statements.append(
            Assignment(target=VariableRef(temp_var), value=first_ref),
        )

        # Shift elements in the cycle
        for i in range(len(cycle) - 1):
            src_elem = elements[cycle[i + 1]]
            dst_elem = elements[cycle[i]]

            src_ref = self._convert_qubit_ref(src_elem)
            dst_ref = self._convert_qubit_ref(dst_elem)

            self.current_block.statements.append(
                Assignment(target=dst_ref, value=src_ref),
            )

        # Complete the cycle
        last_elem = elements[cycle[-1]]
        last_ref = self._convert_qubit_ref(last_elem)

        self.current_block.statements.append(
            Assignment(target=last_ref, value=VariableRef(temp_var)),
        )

    def _convert_block_call(self, block) -> Statement | None:
        """Convert a block to a function call or inline expansion."""
        block_type = type(block)
        block_name = block_type.__name__

        # Get original block info if preserved
        original_block_name = getattr(block, "block_name", block_name)
        original_block_module = getattr(block, "block_module", block_type.__module__)

        # Check if this is a core block that should be inlined
        if original_block_name in self.CORE_BLOCKS:
            # Inline core blocks
            if hasattr(block, "ops"):
                self.current_block.statements.append(
                    Comment(f"Begin {block_name} block"),
                )
                for op in block.ops:
                    stmt = self._convert_operation(op)
                    if stmt:
                        self.current_block.statements.append(stmt)
                self.current_block.statements.append(
                    Comment(f"End {block_name} block"),
                )
            return None

        # For non-core blocks, create a function
        block_signature = self._get_block_signature(block)

        # Check if we already have a function for this block type
        if block_signature not in self.block_registry:
            # Determine struct prefix if this block operates on a struct
            struct_prefix = None
            deps = self._analyze_block_dependencies(block)

            # Check if all variables belong to the same struct
            for prefix, info in self.struct_info.items():
                vars_in_this_struct = set()
                for var in info["var_names"].values():
                    if var in deps["quantum"] or var in deps["classical"]:
                        vars_in_this_struct.add(var)

                # If this block operates on variables from this struct, use
                # QEC code name if available
                if vars_in_this_struct:
                    # Use the QEC code name if we have it, otherwise use prefix
                    struct_prefix = info.get("qec_code_name", prefix)
                    break

            # Generate a unique function name with struct prefix
            # Include module name if not __main__
            base_name = original_block_name

            # For Parallel blocks with content hash, include the content info
            if len(block_signature) > 2 and original_block_name == "Parallel":
                content_hash = block_signature[2]
                # Create a more readable suffix from the hash
                # e.g., "H_H" becomes "_h", "X_X" becomes "_x"
                if content_hash:
                    gates = content_hash.split("_")
                    if all(g == gates[0] for g in gates):
                        # All gates are the same type
                        base_name += f"_{gates[0].lower()}"
                    else:
                        # Mixed gates - use first letter of each
                        suffix = "_".join(g[0].lower() for g in gates[:3])  # Limit to 3
                        base_name += f"_{suffix}"

            if original_block_module and original_block_module != "__main__":
                # Extract just the last part of the module name (e.g., 'test_linearity_patterns')
                module_parts = original_block_module.split(".")
                module_name = module_parts[-1] if module_parts else ""
                if module_name and module_name.startswith("test_"):
                    # For test modules, include the module name
                    func_name = self._generate_function_name(
                        f"{module_name}_{base_name}",
                        struct_prefix,
                    )
                else:
                    func_name = self._generate_function_name(base_name, struct_prefix)
            else:
                func_name = self._generate_function_name(base_name, struct_prefix)
            self.block_registry[block_signature] = func_name

            # Add to pending functions if not already discovered
            if func_name not in self.discovered_functions:
                self.pending_functions.append((block, func_name, block_signature))
                self.discovered_functions.add(func_name)
        else:
            func_name = self.block_registry[block_signature]

        # Generate function call
        stmt = self._generate_function_call(func_name, block)
        if stmt:
            self.current_block.statements.append(stmt)
        return None  # Already added to current block

    def _get_block_signature(self, block) -> tuple:
        """Get a unique signature for a block type."""
        block_type = type(block)
        block_name = block_type.__name__
        original_block_name = getattr(block, "block_name", block_name)
        original_block_module = getattr(block, "block_module", block_type.__module__)

        # For Parallel blocks, include content hash to differentiate blocks
        # with different operations
        if original_block_name == "Parallel" and hasattr(block, "ops"):
            content_hash = self._get_block_content_hash(block)
            return (original_block_name, original_block_module, content_hash)

        # For now, use block name and module as signature
        # Could be enhanced to include parameter info
        return (original_block_name, original_block_module)

    def _generate_function_name(
        self,
        block_name: str,
        struct_prefix: str | None = None,
    ) -> str:
        """Generate a unique function name for a block.

        Args:
            block_name: The original block name (e.g., 'H', 'PrepRUS')
            struct_prefix: Optional struct prefix (e.g., 'c' for c_struct)
        """
        # Convert CamelCase to snake_case, handling acronyms better
        import re

        # First, handle transitions from lowercase to uppercase
        snake_case = re.sub("([a-z0-9])([A-Z])", r"\1_\2", block_name)

        # Then handle multiple consecutive capitals (acronyms)
        snake_case = re.sub("([A-Z]+)([A-Z][a-z])", r"\1_\2", snake_case)

        # Convert to lowercase
        snake_case = snake_case.lower()

        # Add struct prefix if provided
        base_name = f"{struct_prefix}_{snake_case}" if struct_prefix else snake_case

        # Ensure uniqueness
        func_name = base_name
        counter = 1
        while func_name in self.generated_functions:
            func_name = f"{base_name}_{counter}"
            counter += 1

        return func_name

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

    def _generate_function_call(self, func_name: str, block) -> Statement:
        """Generate a function call for a block."""
        # Analyze block dependencies to determine arguments
        deps = self._analyze_block_dependencies(block)

        # Determine which variables need to be passed as arguments
        args = []
        quantum_args = []  # Track quantum args for return value assignment

        # Check if we should pass structs instead of individual arrays
        struct_args = set()  # Structs we've already added
        vars_in_structs = set()  # Variables that are part of structs

        # First pass: identify which variables are part of structs
        for prefix, info in self.struct_info.items():
            for var in info["var_names"].values():
                if var in deps["quantum"] or var in deps["classical"]:
                    vars_in_structs.add(var)
                    if prefix not in struct_args:
                        # Add the struct as an argument
                        args.append(VariableRef(prefix))
                        struct_args.add(prefix)
                        # Track this for return value handling
                        if var in deps["quantum"]:
                            quantum_args.append(prefix)

        # Black Box Pattern: Pass complete global arrays to maintain SLR semantics
        for var in sorted(deps["quantum"] & deps["reads"]):
            # Check if this is an ancilla that was excluded from structs
            is_excluded_ancilla = (
                hasattr(self, "ancilla_qubits") and var in self.ancilla_qubits
            )

            # Skip if this variable is part of a struct UNLESS it's an excluded ancilla
            if var in vars_in_structs and not is_excluded_ancilla:
                continue

            # Check if this variable needs remapping (we're inside a function)
            actual_var = var
            if hasattr(self, "var_remapping") and var in self.var_remapping:
                actual_var = self.var_remapping[var]

            # Black Box Pattern: Always reconstruct global arrays before function calls
            if hasattr(self, "unpacked_vars") and actual_var in self.unpacked_vars:
                # Reconstruct the global array from unpacked elements
                element_names = self.unpacked_vars[actual_var]
                array_construction = self._create_array_construction(element_names)

                # Reconstruct directly into the original array name to maintain SLR semantics
                reconstruction_stmt = Assignment(
                    target=VariableRef(actual_var),
                    value=array_construction,
                )
                self.current_block.statements.append(reconstruction_stmt)

                # Clear the unpacking info since we've reconstructed the array
                del self.unpacked_vars[actual_var]
                args.append(VariableRef(actual_var))
            else:
                # Array is already in the correct global form
                args.append(VariableRef(actual_var))
            quantum_args.append(actual_var)

        # Pass classical variables that are read or written (arrays are passed by reference)
        for var in sorted(deps["classical"] & (deps["reads"] | deps["writes"])):
            # Skip if this variable is part of a struct
            if var in vars_in_structs:
                continue

            # Check if this variable needs remapping
            actual_var = var
            if hasattr(self, "var_remapping") and var in self.var_remapping:
                actual_var = self.var_remapping[var]
            args.append(VariableRef(actual_var))

        # Create function call
        call = FunctionCall(
            func_name=func_name,
            args=args,
        )

        # Check if this function consumes its parameters
        function_consumes = self._function_consumes_parameters(func_name, block)

        # Track consumed arrays in main function
        if function_consumes and hasattr(self, "consumed_arrays"):
            for arg in quantum_args:
                self.consumed_arrays.add(arg)

        # Use natural SLR semantics: arrays are global resources modified in-place
        # Functions that use unpacking still return arrays at boundaries to maintain this illusion
        quantum_args = [
            arg for arg in quantum_args if isinstance(arg, str)
        ]  # Filter for array names

        # Check if we're returning structs
        any(arg in self.struct_info for arg in quantum_args)

        # Check if the function returns something based on our function definitions
        function_returns_something = self._function_returns_something(func_name)

        if quantum_args and (not function_consumes or function_returns_something):
            # Black Box Pattern: Function returns modified global arrays/structs
            # Assign directly back to original names to maintain SLR semantics
            # ALSO handle @owned functions that return reconstructed structs
            statements = []

            if len(quantum_args) == 1:
                # Single return - assign directly back to original name
                name = quantum_args[0]
                assignment = Assignment(target=VariableRef(name), value=call)
                statements.append(assignment)

                # If this is a struct that was unpacked, re-unpack it after the call
                if name in self.struct_info and hasattr(self, "var_remapping"):
                    struct_info = self.struct_info[name]
                    # Check if any of the struct's fields are in var_remapping
                    # (indicating unpacking)
                    needs_re_unpack = any(
                        var in self.var_remapping
                        for var in struct_info["var_names"].values()
                    )

                    if needs_re_unpack:
                        # IMPORTANT: We cannot re-unpack from the struct because it may have been
                        # consumed by the function call. Instead, we need to
                        # update our var_remapping
                        # to indicate that the unpacked variables are no longer valid.
                        # The code should use the struct fields directly after function calls.

                        # Comment explaining why we can't re-unpack
                        statements.append(
                            Comment(
                                "Note: Cannot use unpacked variables after calling "
                                "function with @owned struct",
                            ),
                        )

                        # Update var_remapping to indicate these variables should not be used
                        # by mapping them back to struct field access
                        for var_name in struct_info["var_names"].values():
                            if var_name in self.var_remapping:
                                # This will cause future references to use struct.field notation
                                del self.var_remapping[var_name]

                # If caller needs unpacking, unpack the returned array
                elif name in self.plan.unpack_at_start and name not in self.struct_info:
                    # Get the array info to determine size
                    if name in self.plan.arrays_to_unpack:
                        info = self.plan.arrays_to_unpack[name]
                        self._add_array_unpacking(name, info.size)

            else:
                # Multiple arrays - tuple assignment to original names
                targets = list(quantum_args)

                class TupleAssignment(Statement):
                    def __init__(self, targets, value):
                        self.targets = targets
                        self.value = value

                    def analyze(self, context):
                        self.value.analyze(context)

                    def render(self, context):
                        target_str = ", ".join(self.targets)
                        value_str = self.value.render(context)[0]
                        return [f"{target_str} = {value_str}"]

                assignment = TupleAssignment(targets=targets, value=call)
                statements.append(assignment)

                # Handle struct field invalidation after function call
                for array_name in quantum_args:
                    if array_name in self.struct_info and hasattr(
                        self,
                        "var_remapping",
                    ):
                        struct_info = self.struct_info[array_name]
                        # Check if any of the struct's fields are in var_remapping
                        needs_update = any(
                            var in self.var_remapping
                            for var in struct_info["var_names"].values()
                        )

                        if needs_update:
                            # Cannot re-unpack - invalidate the unpacked variables
                            statements.append(
                                Comment(
                                    "Note: Cannot use unpacked variables after calling "
                                    "function with @owned struct",
                                ),
                            )

                            # Update var_remapping to indicate these variables should not be used
                            for var_name in struct_info["var_names"].values():
                                if var_name in self.var_remapping:
                                    del self.var_remapping[var_name]

                # Unpack any arrays that need it after the function call
                for array_name in quantum_args:
                    if (
                        array_name in self.plan.unpack_at_start
                        and array_name not in self.struct_info
                        and array_name in self.plan.arrays_to_unpack
                    ):
                        info = self.plan.arrays_to_unpack[array_name]
                        self._add_array_unpacking(array_name, info.size)

            # Return block with all statements
            if len(statements) == 1:
                return statements[0]
            return Block(statements=statements)

        # Either no quantum arrays OR function consumes its parameters
        # In both cases, just call the function without assignment
        class ExpressionStatement(Statement):
            def __init__(self, expr):
                self.expr = expr

            def analyze(self, context):
                self.expr.analyze(context)

            def render(self, context):
                return self.expr.render(context)

        return ExpressionStatement(call)

    def _function_returns_something(self, func_name: str) -> bool:
        """Check if a function returns a value (not None)."""
        # Functions that work with structs and return modified structs
        # Check if this function name indicates it works with structs
        if self.struct_info:
            for info in self.struct_info.values():
                struct_name = info.get("struct_name", "")
                # Extract the base name from the struct name (e.g., "steane" from "steane_struct")
                if "_struct" in struct_name:
                    base_name = struct_name.replace("_struct", "").lower()
                else:
                    base_name = struct_name.lower()

                if func_name.startswith(f"{base_name}_"):
                    # Struct functions typically return the modified struct
                    # Exception: functions ending in 'discard' or 'decompose'
                    # don't return the struct
                    return not (func_name.endswith(("_discard", "_decompose")))

        # For other functions, assume they return something if they have quantum args
        # This is a conservative approach
        return False

    def _analyze_block_dependencies(self, block) -> dict[str, Any]:
        """Analyze what variables a block depends on."""
        dependencies = {
            "reads": set(),  # Variables read
            "writes": set(),  # Variables written
            "quantum": set(),  # Quantum variables used
            "classical": set(),  # Classical variables used
        }

        # Analyze operations in the block
        if hasattr(block, "ops"):
            for op in block.ops:
                self._analyze_op_dependencies(op, dependencies, depth=0)

        return dependencies

    def _analyze_op_dependencies(
        self,
        op,
        deps: dict[str, set],
        depth: int = 0,
    ) -> None:
        """Analyze dependencies of a single operation."""
        op_type = type(op).__name__

        # Handle quantum gates
        if hasattr(op, "qargs"):
            for qarg in op.qargs:
                # Handle tuple arguments (e.g., CX gates with (control, target) pairs)
                if isinstance(qarg, tuple):
                    for sub_qarg in qarg:
                        if hasattr(sub_qarg, "reg") and hasattr(sub_qarg.reg, "sym"):
                            var_name = sub_qarg.reg.sym
                            deps["reads"].add(var_name)
                            deps["quantum"].add(var_name)
                        elif hasattr(sub_qarg, "sym"):
                            var_name = sub_qarg.sym
                            deps["reads"].add(var_name)
                            deps["quantum"].add(var_name)
                elif hasattr(qarg, "reg") and hasattr(qarg.reg, "sym"):
                    var_name = qarg.reg.sym
                    deps["reads"].add(var_name)
                    deps["quantum"].add(var_name)
                elif hasattr(qarg, "sym"):
                    # Direct QReg reference
                    var_name = qarg.sym
                    deps["reads"].add(var_name)
                    deps["quantum"].add(var_name)

        # Handle measurements
        if op_type == "Measure":
            if hasattr(op, "qargs"):
                for qarg in op.qargs:
                    if hasattr(qarg, "reg") and hasattr(qarg.reg, "sym"):
                        var_name = qarg.reg.sym
                        deps["reads"].add(var_name)
                        deps["quantum"].add(var_name)
            if hasattr(op, "cout") and op.cout:
                for cout in op.cout:
                    if hasattr(cout, "reg") and hasattr(cout.reg, "sym"):
                        var_name = cout.reg.sym
                        deps["writes"].add(var_name)
                        deps["classical"].add(var_name)

        # Handle SET operations
        if op_type == "SET":
            if hasattr(op, "left") and hasattr(op.left, "reg"):
                var_name = op.left.reg.sym
                deps["writes"].add(var_name)
                deps["classical"].add(var_name)
            if hasattr(op, "right"):
                self._analyze_expression_deps(op.right, deps)

        # Handle control flow
        if op_type in ["If", "While", "For", "Repeat"]:
            # Analyze condition
            if hasattr(op, "condition"):
                self._analyze_expression_deps(op.condition, deps)
            # Analyze body operations
            if hasattr(op, "ops"):
                for sub_op in op.ops:
                    self._analyze_op_dependencies(sub_op, deps, depth + 1)

        # Handle nested blocks (but not too deep to avoid infinite recursion)
        elif hasattr(op, "ops") and hasattr(op, "vars") and depth < 2:
            # This is a block call - analyze it recursively but not too deep
            for sub_op in op.ops:
                self._analyze_op_dependencies(sub_op, deps, depth + 1)

    def _analyze_expression_deps(self, expr, deps: dict[str, set]) -> None:
        """Analyze dependencies in an expression."""
        expr_type = type(expr).__name__

        if expr_type == "Bit":
            if hasattr(expr, "reg") and hasattr(expr.reg, "sym"):
                var_name = expr.reg.sym
                deps["reads"].add(var_name)
                deps["classical"].add(var_name)
        elif expr_type == "Qubit":
            if hasattr(expr, "reg") and hasattr(expr.reg, "sym"):
                var_name = expr.reg.sym
                deps["reads"].add(var_name)
                deps["quantum"].add(var_name)
        elif hasattr(expr, "left") and hasattr(expr, "right"):
            self._analyze_expression_deps(expr.left, deps)
            self._analyze_expression_deps(expr.right, deps)
        elif hasattr(expr, "value"):
            self._analyze_expression_deps(expr.value, deps)

    def _add_final_handling(self, block) -> None:
        """Handle struct decomposition, results, and cleanup in the correct order."""
        # First, decompose any structs that need cleanup
        struct_decompositions = {}  # prefix -> list of decomposed variable names

        for prefix, info in self.struct_info.items():
            # Check if this struct has unconsumed quantum fields
            has_unconsumed_quantum = False
            for suffix, var_type, size in info["fields"]:
                if var_type == "qubit":
                    var_name = info["var_names"][suffix]
                    if var_name not in self.consumed_arrays:
                        has_unconsumed_quantum = True
                        break

            if has_unconsumed_quantum:
                # Decompose the struct
                qec_code_name = info.get("qec_code_name", prefix)
                func_name = (
                    f"{qec_code_name}_decompose"
                    if qec_code_name
                    else f"{prefix}_decompose"
                )

                # Generate variable names for decomposed fields
                decomposed_vars = []
                for suffix, _, _ in sorted(info["fields"]):
                    decomposed_vars.append(f"{prefix}_{suffix}_final")

                # Create the decomposition call
                targets = decomposed_vars
                call = FunctionCall(
                    func_name=func_name,
                    args=[VariableRef(prefix)],
                )

                # Create assignment
                target_tuple = TupleExpression(
                    elements=[VariableRef(name) for name in targets],
                )
                stmt = Assignment(target=target_tuple, value=call)

                self.current_block.statements.append(
                    Comment(f"Decompose struct {prefix} for cleanup"),
                )
                self.current_block.statements.append(stmt)

                # Store decomposition info
                struct_decompositions[prefix] = list(
                    zip(
                        [f[0] for f in sorted(info["fields"])],  # suffixes
                        decomposed_vars,
                        [f[1] for f in sorted(info["fields"])],  # types
                        [f[2] for f in sorted(info["fields"])],  # sizes
                    ),
                )

        # Now add results, using decomposed variables where necessary
        self._add_results_with_decomposition(block, struct_decompositions)

        # Track what arrays have been cleaned up to avoid double-discard
        cleaned_up_arrays = set()

        # Finally, clean up quantum arrays
        self._add_cleanup_with_decomposition(
            block,
            struct_decompositions,
            cleaned_up_arrays,
        )

        # Also run the regular cleanup for non-struct arrays
        self._add_cleanup(block, cleaned_up_arrays)

    def _add_results_with_decomposition(self, block, struct_decompositions) -> None:
        """Add result calls, using decomposed variables where necessary."""
        if hasattr(block, "vars"):
            for var in block.vars:
                if type(var).__name__ == "CReg":
                    var_name = var.sym

                    # Check for renaming
                    actual_name = var_name
                    if var_name in self.plan.renamed_variables:
                        actual_name = self.plan.renamed_variables[var_name]

                    # Check if this variable is part of a decomposed struct
                    value_ref = None
                    for prefix, info in self.struct_info.items():
                        if var_name in info["var_names"].values():
                            # Find the field name for this variable
                            for suffix, mapped_var in info["var_names"].items():
                                if mapped_var == var_name:
                                    # Check if struct was decomposed
                                    if prefix in struct_decompositions:
                                        # Find the decomposed variable
                                        for (
                                            field_suffix,
                                            decomposed_var,
                                            _,
                                            _,
                                        ) in struct_decompositions[prefix]:
                                            if field_suffix == suffix:
                                                value_ref = VariableRef(decomposed_var)
                                                break
                                    else:
                                        # Struct not decomposed, use field access
                                        value_ref = FieldAccess(
                                            obj=VariableRef(prefix),
                                            field=suffix,
                                        )
                                    break
                            break

                    if value_ref is None:
                        # Not in a struct, use direct variable reference
                        value_ref = VariableRef(actual_name)

                    # Add result call
                    call = FunctionCall(
                        func_name="result",
                        args=[
                            Literal(var.sym),  # Original name as label
                            value_ref,  # Actual variable or decomposed field
                        ],
                    )

                    # Create a wrapper that renders just the function call
                    class ExpressionStatement(Statement):
                        def __init__(self, expr):
                            self.expr = expr

                        def analyze(self, context):
                            self.expr.analyze(context)

                        def render(self, context):
                            return self.expr.render(context)

                    self.current_block.statements.append(ExpressionStatement(call))

    def _add_cleanup_with_decomposition(
        self,
        block,
        struct_decompositions,
        cleaned_up_arrays,
    ) -> None:
        _ = block  # Currently not used
        """Add cleanup for quantum arrays, using decomposed variables."""
        # First handle decomposed struct fields
        for prefix, fields in struct_decompositions.items():
            self.current_block.statements.append(
                Comment(f"Discard quantum fields from {prefix}"),
            )
            for suffix, decomposed_var, var_type, size in fields:
                if var_type == "qubit" and decomposed_var not in cleaned_up_arrays:
                    stmt = FunctionCall(
                        func_name="quantum.discard_array",
                        args=[VariableRef(decomposed_var)],
                    )
                    cleaned_up_arrays.add(decomposed_var)
                    # Also track the original variable name to prevent double cleanup
                    if prefix in self.struct_info:
                        info = self.struct_info[prefix]
                        if suffix in info["var_names"]:
                            original_var = info["var_names"][suffix]
                            cleaned_up_arrays.add(original_var)

                    # Create expression statement wrapper
                    class ExpressionStatement(Statement):
                        def __init__(self, expr):
                            self.expr = expr

                        def analyze(self, context):
                            self.expr.analyze(context)

                        def render(self, context):
                            return self.expr.render(context)

                    self.current_block.statements.append(ExpressionStatement(stmt))

        # Note: Non-struct arrays are handled in _add_cleanup, not here

    def _add_cleanup(self, block, cleaned_up_arrays=None) -> None:
        """Add cleanup for unconsumed qubits."""
        if cleaned_up_arrays is None:
            cleaned_up_arrays = set()
        # Track consumed qubits during operation conversion
        consumed = {}  # qreg_name -> set of indices

        # Analyze operations to find consumed qubits
        if hasattr(block, "ops"):
            for op in block.ops:
                self._track_consumed_qubits(op, consumed)

        # First, check if we have structs that need cleanup
        struct_cleanup_done = set()
        for prefix, info in self.struct_info.items():
            # Check if any quantum arrays in this struct need cleanup
            needs_cleanup = False
            for suffix, var_type, size in info["fields"]:
                if var_type == "qubit":
                    var_name = info["var_names"][suffix]
                    if var_name not in self.consumed_arrays:
                        needs_cleanup = True
                        break

            if needs_cleanup and prefix not in struct_cleanup_done:
                # We're at the end of main, after results.
                # We can't access struct fields directly after consuming the struct,
                # so we'll just leave quantum arrays in structs for now.
                # The HUGR compiler will need to handle this pattern.

                # Add a comment noting this limitation
                self.current_block.statements.append(
                    Comment(
                        f"Note: struct {prefix} contains unconsumed quantum arrays",
                    ),
                )

                struct_cleanup_done.add(prefix)
                # Mark arrays as handled
                for suffix, var_type, size in info["fields"]:
                    if var_type == "qubit":
                        var_name = info["var_names"][suffix]
                        self.consumed_arrays.add(var_name)

        # Check each quantum register not in structs
        if hasattr(block, "vars"):
            for var in block.vars:
                if type(var).__name__ == "QReg":
                    var_name = var.sym

                    # Skip if this array is part of a struct
                    in_struct = False
                    for prefix, info in self.struct_info.items():
                        if var_name in info["var_names"].values():
                            in_struct = True
                            break

                    if in_struct:
                        continue
                    # Check for renaming
                    if var_name in self.plan.renamed_variables:
                        var_name = self.plan.renamed_variables[var_name]

                    consumed_indices = consumed.get(var.sym, set())

                    # Check if this array was consumed by an @owned function or measurement
                    was_consumed_by_function = (
                        hasattr(self, "consumed_arrays")
                        and var.sym in self.consumed_arrays
                    )
                    was_consumed_by_measurement = (
                        hasattr(self, "consumed_resources")
                        and var.sym in self.consumed_resources
                    )
                    was_dynamically_allocated = (
                        hasattr(self, "dynamic_allocations")
                        and var.sym in self.dynamic_allocations
                    )

                    # Handle partially consumed arrays
                    if len(consumed_indices) > 0 and len(consumed_indices) < var.size:
                        # Array was partially consumed - need to discard entire array
                        if var_name not in cleaned_up_arrays:
                            self.current_block.statements.append(
                                Comment(f"Discard {var.sym}"),
                            )
                            stmt = FunctionCall(
                                func_name="quantum.discard_array",
                                args=[VariableRef(var_name)],
                            )

                            # Create expression statement wrapper
                            class ExpressionStatement(Statement):
                                def __init__(self, expr):
                                    self.expr = expr

                                def analyze(self, context):
                                    self.expr.analyze(context)

                                def render(self, context):
                                    return self.expr.render(context)

                            self.current_block.statements.append(
                                ExpressionStatement(stmt),
                            )
                            cleaned_up_arrays.add(var_name)
                    # Only discard arrays that weren't consumed by @owned functions or measurements
                    elif (
                        not was_consumed_by_function and not was_consumed_by_measurement
                    ):
                        if was_dynamically_allocated:
                            # For dynamically allocated arrays, discard individual
                            # qubits that weren't measured
                            self.current_block.statements.append(
                                Comment(f"Discard dynamically allocated {var.sym}"),
                            )

                            # Check which individual qubits were allocated and not consumed
                            if hasattr(self, "allocated_ancillas"):
                                # Discard each allocated ancilla
                                for i in range(var.size):
                                    ancilla_var = f"{var.sym}_{i}"
                                    if ancilla_var in self.allocated_ancillas:
                                        discard_stmt = FunctionCall(
                                            func_name="quantum.discard",
                                            args=[VariableRef(ancilla_var)],
                                        )

                                        # Create expression statement wrapper
                                        class ExpressionStatement(Statement):
                                            def __init__(self, expr):
                                                self.expr = expr

                                            def analyze(self, context):
                                                self.expr.analyze(context)

                                            def render(self, context):
                                                return self.expr.render(context)

                                        self.current_block.statements.append(
                                            ExpressionStatement(discard_stmt),
                                        )
                        else:
                            # Regular pre-allocated array
                            if var_name not in cleaned_up_arrays:
                                self.current_block.statements.append(
                                    Comment(f"Discard {var.sym}"),
                                )

                                # Use quantum.discard_array() for the whole array
                                array_ref = VariableRef(var_name)
                                stmt = FunctionCall(
                                    func_name="quantum.discard_array",
                                    args=[array_ref],
                                )

                                # Create expression statement wrapper
                                class ExpressionStatement(Statement):
                                    def __init__(self, expr):
                                        self.expr = expr

                                    def analyze(self, context):
                                        self.expr.analyze(context)

                                    def render(self, context):
                                        return self.expr.render(context)

                                self.current_block.statements.append(
                                    ExpressionStatement(stmt),
                                )
                                cleaned_up_arrays.add(var_name)

    def _check_has_element_operations(self, block, var_name: str) -> bool:
        """Check if a block has element-wise operations on a variable.

        This is used to determine if we should use @owned for array parameters.
        Element-wise operations (like reset on individual elements) don't work
        with @owned arrays in Guppy.
        """
        if not hasattr(block, "ops"):
            return False

        for op in block.ops:
            op_type = type(op).__name__

            # Check for Prep operations on the whole array
            if op_type == "Prep" and hasattr(op, "qargs"):
                for qarg in op.qargs:
                    if hasattr(qarg, "sym") and qarg.sym == var_name:
                        # Prep on the whole array - this needs element access
                        return True

            # Check for operations on individual elements
            if hasattr(op, "qargs"):
                for qarg in op.qargs:
                    if (
                        hasattr(qarg, "reg")
                        and hasattr(qarg.reg, "sym")
                        and qarg.reg.sym == var_name
                        and hasattr(qarg, "index")
                        and op_type in ["Prep", "Measure"]
                    ):
                        return True

            # Recursively check nested blocks
            if hasattr(op, "ops") and self._check_has_element_operations(op, var_name):
                return True

        return False

    def _track_consumed_qubits(self, op, consumed: dict[str, set[int]]) -> None:
        """Track which qubits are consumed by an operation."""
        op_type = type(op).__name__

        if op_type == "Measure" and hasattr(op, "qargs") and op.qargs:
            for qarg in op.qargs:
                # Handle full array measurement
                if hasattr(qarg, "sym") and hasattr(qarg, "size"):
                    qreg_name = qarg.sym
                    if qreg_name not in consumed:
                        consumed[qreg_name] = set()
                    # Mark all qubits as consumed
                    indices = set(range(qarg.size))
                    for i in indices:
                        consumed[qreg_name].add(i)
                    # Track in scope manager
                    self.scope_manager.track_resource_usage(
                        qreg_name,
                        indices,
                        consumed=True,
                    )
                # Handle individual qubit measurement
                elif hasattr(qarg, "reg") and hasattr(qarg.reg, "sym"):
                    qreg_name = qarg.reg.sym
                    if qreg_name not in consumed:
                        consumed[qreg_name] = set()

                    if hasattr(qarg, "index"):
                        consumed[qreg_name].add(qarg.index)
                        # Track in scope manager
                        self.scope_manager.track_resource_usage(
                            qreg_name,
                            {qarg.index},
                            consumed=True,
                        )

        # Recurse into nested blocks
        if hasattr(op, "ops"):
            for nested_op in op.ops:
                self._track_consumed_qubits(nested_op, consumed)

        # Check else blocks
        if (
            op_type == "If"
            and hasattr(op, "else_block")
            and op.else_block
            and hasattr(op.else_block, "ops")
        ):
            for nested_op in op.else_block.ops:
                self._track_consumed_qubits(nested_op, consumed)

    def _array_needs_full_allocation(self, array_name: str, block) -> bool:
        """Check if an array needs full allocation due to full array operations."""
        if not hasattr(block, "ops"):
            return False

        for op in block.ops:
            if self._operation_uses_full_array(op, array_name):
                return True

            # Check nested operations
            if hasattr(op, "ops"):
                for nested_op in op.ops:
                    if self._operation_uses_full_array(nested_op, array_name):
                        return True

            # Check else blocks
            if (
                hasattr(op, "else_block")
                and op.else_block
                and hasattr(op.else_block, "ops")
            ):
                for nested_op in op.else_block.ops:
                    if self._operation_uses_full_array(nested_op, array_name):
                        return True

        return False

    def _operation_uses_full_array(self, op, array_name: str) -> bool:
        """Check if an operation uses a full array (e.g., Measure(q) > c)."""
        if hasattr(op, "qargs") and len(op.qargs) == 1:
            qarg = op.qargs[0]
            # Check for full array reference (has sym and size but no index)
            if (
                hasattr(qarg, "sym")
                and qarg.sym == array_name
                and hasattr(qarg, "size")
                and qarg.size > 1
                and not hasattr(qarg, "index")
            ):
                return True
        return False

    def _add_results(self, block) -> None:
        """Add result() calls for classical registers."""
        if hasattr(block, "vars"):
            for var in block.vars:
                if type(var).__name__ == "CReg":
                    var_name = var.sym

                    # Check for renaming
                    actual_name = var_name
                    if var_name in self.plan.renamed_variables:
                        actual_name = self.plan.renamed_variables[var_name]

                    # Check if this variable is part of a struct
                    value_ref = None
                    for prefix, info in self.struct_info.items():
                        if var_name in info["var_names"].values():
                            # Find the field name for this variable
                            for suffix, mapped_var in info["var_names"].items():
                                if mapped_var == var_name:
                                    # Access through struct field
                                    value_ref = FieldAccess(
                                        obj=VariableRef(prefix),
                                        field=suffix,
                                    )
                                    break
                            break

                    if value_ref is None:
                        # Not in a struct, use direct variable reference
                        value_ref = VariableRef(actual_name)

                    # Add result call
                    call = FunctionCall(
                        func_name="result",
                        args=[
                            Literal(var.sym),  # Original name as label
                            value_ref,  # Actual variable or struct field
                        ],
                    )

                    # Create a wrapper that renders just the function call
                    class ExpressionStatement(Statement):
                        def __init__(self, expr):
                            self.expr = expr

                        def analyze(self, context):
                            self.expr.analyze(context)

                        def render(self, context):
                            return self.expr.render(context)

                    self.current_block.statements.append(ExpressionStatement(call))

    def _detect_struct_patterns(self, block: SLRBlock) -> None:
        """Detect variables that should be grouped into structs.

        Looking for patterns where multiple variables share a common prefix
        (e.g., x_d, x_a, x_c all belong to quantum code 'x').
        """
        # First, try to determine the quantum code class from variable metadata
        qec_code_name = None
        qec_instance_mapping = {}  # Maps instance name -> class name

        # Check if block.vars has source class information
        if hasattr(block, "vars") and hasattr(block.vars, "var_source_classes"):
            # Get the source class from the metadata
            for var_name, source_class in block.vars.var_source_classes.items():
                # Extract the prefix from the variable name
                if "_" in var_name:
                    prefix = var_name.split("_")[0]
                    if prefix not in qec_instance_mapping:
                        qec_instance_mapping[prefix] = source_class.lower()
                        if not qec_code_name:
                            qec_code_name = source_class.lower()

        # If no QEC class found in vars, fall back to searching operations
        if not qec_code_name:
            # Helper function to recursively search for QEC code
            def find_qec_code_in_block(op, depth=0, max_depth=5):
                if depth > max_depth:
                    return None

                results = []

                # Check if this op has QEC module info
                if hasattr(op, "__class__") and hasattr(op.__class__, "__module__"):
                    module = op.__class__.__module__
                    # Extract QEC code name from module path like
                    # 'pecos.qeclib.steane.preps.pauli_states'
                    if "pecos.qeclib." in module:
                        parts = module.split(".")
                        if len(parts) > 2 and "qeclib" in parts:
                            qec_idx = parts.index("qeclib")
                            if qec_idx + 1 < len(parts):
                                candidate = parts[qec_idx + 1]
                                # Skip generic names like 'qubit'
                                if candidate not in ["qubit", "bit", "ops", "gates"]:
                                    results.append(candidate)

                # Check nested operations
                if hasattr(op, "ops"):
                    for nested_op in op.ops:
                        result = find_qec_code_in_block(nested_op, depth + 1, max_depth)
                        if result:
                            results.append(result)

                # Return the first non-generic result
                for r in results:
                    if r not in ["qubit", "bit", "ops", "gates"]:
                        return r

                return results[0] if results else None

            # Try to find the QEC code class from the operations
            if hasattr(block, "ops"):
                for op in block.ops:
                    qec_code_name = find_qec_code_in_block(op)
                    if qec_code_name:
                        break

        # Collect all variables
        all_vars = {}
        if hasattr(block, "vars"):
            for var in block.vars:
                if hasattr(var, "sym"):
                    var_name = var.sym
                    all_vars[var_name] = var

        # Also check context variables
        for var_name, var_info in self.context.variables.items():
            if var_name not in all_vars:
                all_vars[var_name] = var_info

        # Group by prefix
        prefix_groups = {}
        for var_name, var in all_vars.items():
            if "_" in var_name:
                prefix = var_name.split("_")[0]
                suffix = "_".join(var_name.split("_")[1:])

                if prefix not in prefix_groups:
                    prefix_groups[prefix] = []

                # Determine type and size
                size = var.size if hasattr(var, "size") else 1

                # Determine if quantum or classical
                is_quantum = True
                if hasattr(var, "is_quantum"):
                    is_quantum = var.is_quantum
                elif type(var).__name__ == "CReg":
                    is_quantum = False
                elif hasattr(var, "resource_type"):
                    is_quantum = var.resource_type == ResourceState.QUANTUM

                var_type = "qubit" if is_quantum else "bool"

                # Check if this is an ancilla qubit that should be kept separate
                is_ancilla = False
                if var_type == "qubit" and hasattr(self, "qubit_usage_stats"):
                    stats = self.qubit_usage_stats.get(var_name)
                    if stats:
                        role = stats.classify_role()
                        if role == QubitRole.ANCILLA:
                            is_ancilla = True
                            # Store this for later use
                            if not hasattr(self, "ancilla_qubits"):
                                self.ancilla_qubits = set()
                            self.ancilla_qubits.add(var_name)

                if not is_ancilla:
                    prefix_groups[prefix].append((suffix, var_type, size, var_name))

        # Create struct info for groups with multiple related variables
        for prefix, vars_list in prefix_groups.items():
            if len(vars_list) >= 2:
                # Check if this looks like a quantum code pattern
                has_quantum = any(var[1] == "qubit" for var in vars_list)
                if has_quantum:
                    # Use QEC code name for struct if available, otherwise use prefix
                    struct_base_name = qec_code_name if qec_code_name else prefix

                    self.struct_info[prefix] = {
                        "fields": [(v[0], v[1], v[2]) for v in vars_list],
                        "struct_name": f"{struct_base_name}_struct",
                        "var_names": {
                            v[0]: v[3] for v in vars_list
                        },  # suffix -> full var name
                        "qec_code_name": qec_code_name,  # Store for function naming
                        "ancilla_vars": getattr(
                            self,
                            "ancilla_qubits",
                            set(),
                        ),  # Track which vars were excluded
                    }

    def _generate_struct_definitions(self) -> list[str]:
        """Generate Guppy struct definitions."""
        lines = []

        for prefix, info in sorted(self.struct_info.items()):
            struct_name = info["struct_name"]

            # Generate struct
            lines.append("@guppy.struct")
            lines.append("@no_type_check")
            lines.append(f"class {struct_name}:")

            # Add fields sorted by suffix
            for suffix, var_type, size in sorted(info["fields"]):
                field_type = f"array[{var_type}, {size}]" if size > 1 else var_type
                lines.append(f"    {suffix}: {field_type}")

            lines.append("")  # Empty line after struct

        return lines

    def _generate_struct_decompose_function(
        self,
        prefix: str,
        info: dict,
    ) -> Function | None:
        """Generate a decompose function for a struct."""
        struct_name = info["struct_name"]
        qec_code_name = info.get("qec_code_name", prefix)
        func_name = (
            f"{qec_code_name}_decompose" if qec_code_name else f"{prefix}_decompose"
        )

        # Build return type - tuple of all fields
        return_types = []
        field_names = []
        for suffix, var_type, size in sorted(info["fields"]):
            field_names.append(suffix)
            return_types.append(
                f"array[{var_type}, {size}]" if size > 1 else var_type,
            )

        return_type = f"tuple[{', '.join(return_types)}]"

        # Create function body
        body = Block()

        # Return all fields as a tuple
        field_refs = [
            FieldAccess(obj=VariableRef(prefix), field=suffix) for suffix in field_names
        ]

        return_stmt = ReturnStatement(value=TupleExpression(elements=field_refs))
        body.statements.append(return_stmt)

        return Function(
            name=func_name,
            params=[(prefix, f"{struct_name} @owned")],
            return_type=return_type,
            body=body,
            decorators=["guppy", "no_type_check"],
        )

    def _generate_struct_discard_function(
        self,
        prefix: str,
        info: dict,
    ) -> Function | None:
        """Generate a discard function for a struct."""
        # Check if struct has quantum fields
        has_quantum = any(field[1] == "qubit" for field in info["fields"])
        if not has_quantum:
            return None

        struct_name = info["struct_name"]
        qec_code_name = info.get("qec_code_name", prefix)
        func_name = f"{qec_code_name}_discard" if qec_code_name else f"{prefix}_discard"

        # Create function body
        body = Block()

        # Add discard calls for each quantum field
        for suffix, var_type, size in sorted(info["fields"]):
            if var_type == "qubit":
                field_access = FieldAccess(obj=VariableRef(prefix), field=suffix)
                stmt = FunctionCall(
                    func_name="quantum.discard_array",
                    args=[field_access],
                )

                # Create expression statement wrapper
                class ExpressionStatement(Statement):
                    def __init__(self, expr):
                        self.expr = expr

                    def analyze(self, context):
                        self.expr.analyze(context)

                    def render(self, context):
                        return self.expr.render(context)

                body.statements.append(ExpressionStatement(stmt))

        return Function(
            name=func_name,
            params=[(prefix, f"{struct_name} @owned")],
            return_type="None",
            body=body,
            decorators=["guppy", "no_type_check"],
        )

    def _add_struct_initialization(
        self,
        prefix: str,
        info: dict,
        block: SLRBlock,
    ) -> None:
        """Add struct initialization to current block."""
        struct_name = info["struct_name"]

        # Create the struct instance
        # For now, initialize fields individually then create struct
        # TODO: Could be optimized to initialize struct directly

        # First, declare the individual arrays
        for suffix, var_type, size in info["fields"]:
            var_name = info["var_names"][suffix]
            # Find the original variable
            for var in block.vars:
                if hasattr(var, "sym") and var.sym == var_name:
                    self._add_variable_declaration(var)
                    break

        # Then create struct instance
        field_refs = []
        for suffix, _, _ in sorted(info["fields"]):
            var_name = info["var_names"][suffix]
            field_refs.append(VariableRef(var_name))

        # Create struct construction expression
        struct_expr = self._create_struct_construction(
            struct_name,
            [f[0] for f in sorted(info["fields"])],
            field_refs,
        )

        # Add assignment: prefix = struct_name(field1=var1, field2=var2, ...)
        stmt = Assignment(
            target=VariableRef(prefix),
            value=struct_expr,
        )
        self.current_block.statements.append(stmt)

        # Update context to track struct variable
        self.context.add_variable(
            VariableInfo(
                name=prefix,
                original_name=prefix,
                var_type=struct_name,
                is_struct=True,
                struct_info=info,
            ),
        )

        # Mark the individual arrays as part of the struct so operations use struct fields
        for suffix, var_type, size in info["fields"]:
            var_name = info["var_names"][suffix]
            var_info = self.context.lookup_variable(var_name)
            if var_info:
                var_info.is_struct_field = True
                var_info.struct_name = prefix
                var_info.field_name = suffix
