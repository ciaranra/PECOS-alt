"""Handler for SLR operations - converts operations to Guppy code."""

from __future__ import annotations

from typing import TYPE_CHECKING

if TYPE_CHECKING:
    from .generator import GuppyGenerator


class OperationHandler:
    """Handles conversion of SLR operations to Guppy code."""
    
    def __init__(self, generator: GuppyGenerator):
        self.generator = generator
        self.individual_measurements = {}  # Track individual measurement results
        
    def generate_op(self, op, position: int = -1) -> None:
        """Generate code for an operation."""
        try:
            op_name = type(op).__name__
            # print(f"DEBUG operation_handler: Processing op type={op_name}")
            
            # Handle blocks first (check if it's a Block subclass)
            if hasattr(op, 'ops') and hasattr(op, 'vars'):
                # print(f"DEBUG operation_handler: Detected as block, passing to block_handler")"
                self.generator.block_handler.handle_block(op)
            # Handle measurements
            elif op_name == "Measure":
                self._generate_measurement(op, position)
            # Handle misc operations first (before checking module)
            elif op_name == "Comment":
                self._generate_comment(op)
            elif op_name == "Barrier":
                self._generate_barrier(op)
            elif op_name == "Prep":
                self._generate_prep(op)
            elif op_name == "Permute":
                self._generate_permute(op)
            # Handle quantum gates
            elif hasattr(op, '__module__') and 'qubit' in op.__module__:
                self._generate_quantum_gate(op)
            # Handle classical operations
            elif op_name == "SET":
                self._generate_assignment(op)
            # Handle bitwise operations
            elif op_name in ["XOR", "AND", "OR", "NOT"]:
                self._generate_bitwise_op(op)
            else:
                self.generator.write(f"# WARNING: Unhandled operation type: {op_name}")
        except Exception as e:
            self.generator.write(f"# ERROR generating {type(op).__name__}: {str(e)}")
            import traceback
            self.generator.write(f"# {traceback.format_exc()}")
            
    def _generate_comment(self, op) -> None:
        """Generate comments."""
        if hasattr(op, 'txt'):
            # Split the comment text into lines
            lines = op.txt.split("\n")
            
            # Add space prefix if requested
            if hasattr(op, 'space') and op.space:
                lines = [f" {line}" if line.strip() != "" else line for line in lines]
            
            # Format as Python comments
            for line in lines:
                if line.strip():  # Only add comment prefix to non-empty lines
                    self.generator.write(f"# {line}")
                else:
                    self.generator.write("")  # Empty line
        else:
            # Fallback if no txt attribute
            self.generator.write("# Comment")
            
    def _generate_quantum_gate(self, gate) -> None:
        """Generate quantum gate operations."""
        gate_name = type(gate).__name__
        
        # Map SLR gate names to Guppy quantum operations
        gate_map = {
            "H": "quantum.h",
            "X": "quantum.x",
            "Y": "quantum.y", 
            "Z": "quantum.z",
            "S": "quantum.s",
            "SZ": "quantum.s",  # SZ is the S gate
            "SZdg": "quantum.sdg",  # SZdg is the Sdg gate
            "T": "quantum.t",
            "Tdg": "quantum.tdg",
            "CX": "quantum.cx",
            "CY": "quantum.cy",
            "CZ": "quantum.cz",
        }
        
        if gate_name in gate_map:
            self.generator.quantum_ops_used.add(gate_name)
            guppy_gate = gate_map[gate_name]
            
            if gate_name in ["CX", "CY", "CZ"]:
                # Two-qubit gates - check for multiple tuple pairs pattern
                if gate.qargs and all(isinstance(arg, tuple) and len(arg) == 2 for arg in gate.qargs):
                    # Multiple (control, target) pairs passed as separate arguments
                    for ctrl, tgt in gate.qargs:
                        ctrl_ref = self._get_qubit_ref(ctrl)
                        tgt_ref = self._get_qubit_ref(tgt)
                        self.generator.write(f"{guppy_gate}({ctrl_ref}, {tgt_ref})")
                elif len(gate.qargs) == 2:
                    # Standard two-qubit gate with control and target
                    ctrl = self._get_qubit_ref(gate.qargs[0])
                    tgt = self._get_qubit_ref(gate.qargs[1])
                    self.generator.write(f"{guppy_gate}({ctrl}, {tgt})")
                else:
                    self.generator.write(f"# ERROR: Two-qubit gate {gate_name} requires exactly 2 qubits")
            else:
                # Single-qubit gates
                if gate.qargs:
                    # Check if this is a full register operation
                    if len(gate.qargs) == 1 and hasattr(gate.qargs[0], 'size') and gate.qargs[0].size > 1:
                        # Apply gate to all qubits in register
                        reg = gate.qargs[0]
                        self.generator.write(f"for i in range({reg.size}):")
                        self.generator.indent()
                        self.generator.write(f"{guppy_gate}({reg.sym}[i])")
                        self.generator.dedent()
                    else:
                        # Single qubit operation(s)
                        for q in gate.qargs:
                            qubit = self._get_qubit_ref(q)
                            self.generator.write(f"{guppy_gate}({qubit})")
                else:
                    self.generator.write(f"# ERROR: Single-qubit gate {gate_name} called with no qubit arguments")
        else:
            self.generator.write(f"# WARNING: Unknown quantum gate: {gate_name}")
            self.generator.write(f"# Add mapping for this gate in gate_map dictionary")
            
    def _get_qubit_ref(self, qubit) -> str:
        """Get the reference string for a qubit."""
        # Check if this qubit has been unpacked (works in any function)
        if hasattr(qubit, 'reg') and hasattr(qubit.reg, 'sym') and hasattr(qubit, 'index'):
            qreg_name = qubit.reg.sym
            index = qubit.index
            
            # Check if this variable was renamed to avoid conflicts
            if hasattr(self.generator, 'renamed_vars') and qreg_name in self.generator.renamed_vars:
                qreg_name = self.generator.renamed_vars[qreg_name]
            
            # Check if this register has been unpacked
            if qreg_name in self.generator.unpacked_arrays:
                # Use the unpacked variable name
                unpacked_names = self.generator.unpacked_arrays[qreg_name]
                if isinstance(unpacked_names, list) and index < len(unpacked_names):
                    return unpacked_names[index]
                    
        # Default behavior - generate standard reference
        if hasattr(qubit, 'reg') and hasattr(qubit, 'index'):
            reg_name = qubit.reg.sym
            # Check if renamed
            if hasattr(self.generator, 'renamed_vars') and reg_name in self.generator.renamed_vars:
                reg_name = self.generator.renamed_vars[reg_name]
            return f"{reg_name}[{qubit.index}]"
        elif hasattr(qubit, 'sym'):
            var_name = qubit.sym
            # Check if renamed
            if hasattr(self.generator, 'renamed_vars') and var_name in self.generator.renamed_vars:
                var_name = self.generator.renamed_vars[var_name]
            return var_name
        else:
            # Try to extract from string representation
            s = str(qubit)
            import re
            match = re.match(r'<(?:Qubit|Bit) (\d+) of (\w+)>', s)
            if match:
                return f"{match.group(2)}[{match.group(1)}]"
            return s
            
    def _check_and_unpack_arrays(self, meas, position: int) -> None:
        """Check if we need to unpack quantum arrays before measurement."""
        # We need to unpack arrays in all contexts when measuring individual elements
        current_scope_type = type(self.generator.current_scope).__name__ if self.generator.current_scope else None
            
        # Extract quantum registers involved in this measurement
        qregs_in_measurement = set()
        cregs_in_measurement = set()
        
        if hasattr(meas, 'qargs') and meas.qargs:
            for qarg in meas.qargs:
                if hasattr(qarg, 'reg') and hasattr(qarg.reg, 'sym'):
                    qregs_in_measurement.add(qarg.reg.sym)
                    
        if hasattr(meas, 'cout') and meas.cout:
            for cout in meas.cout:
                if hasattr(cout, 'reg') and hasattr(cout.reg, 'sym'):
                    cregs_in_measurement.add(cout.reg.sym)
        
        # Check each qreg to see if it needs unpacking
        for qreg_name in qregs_in_measurement:
            if qreg_name in self.generator.measurement_info:
                info = self.generator.measurement_info[qreg_name]
                
                # If this is the first measurement and all qubits will be measured together
                if (position == info.first_measurement_pos and 
                    info.all_measured_together and
                    qreg_name not in self.generator.unpacked_arrays):
                    
                    # Check if we can use measure_array by looking at the CReg
                    # We need to ensure there's a matching CReg for the full array measurement
                    can_use_measure_array = False
                    for creg_name in cregs_in_measurement:
                        if creg_name in self.generator.variable_context:
                            creg = self.generator.variable_context[creg_name]
                            if hasattr(creg, 'size') and creg.size == info.qreg_size:
                                can_use_measure_array = True
                                # Mark this qreg as "virtually unpacked" to prevent actual unpacking
                                self.generator.unpacked_arrays[qreg_name] = f"__measure_array_{qreg_name}"
                                break
                    
                    if can_use_measure_array:
                        continue  # Skip unpacking for this register
                
                # If this is the first measurement and we need to unpack
                if (position == info.first_measurement_pos and 
                    not info.all_measured_together and
                    qreg_name not in self.generator.unpacked_arrays):
                    
                    # Generate unpacking code
                    unpacked_names = self.generator.measurement_analyzer.get_unpacked_var_names(
                        qreg_name, info.qreg_size
                    )
                    
                    # Write the unpacking statement
                    self.generator.write("")
                    self.generator.write(f"# Unpack {qreg_name} for measurement")
                    if len(unpacked_names) == 1:
                        # Single element array needs special syntax
                        self.generator.write(f"{unpacked_names[0]}, = {qreg_name}")
                    else:
                        unpacked_str = ", ".join(unpacked_names)
                        self.generator.write(f"{unpacked_str} = {qreg_name}")
                    
                    # Store the unpacked names
                    self.generator.unpacked_arrays[qreg_name] = unpacked_names
            
    def _should_use_measure_array(self, meas, position: int) -> tuple[bool, str, str]:
        """Check if we should use measure_array for this measurement.
        
        Returns:
            (should_use, qreg_name, temp_var_name) - True if measure_array should be used
        """
        # Check if this is an individual qubit measurement that's part of a full array pattern
        if hasattr(meas, 'qargs') and len(meas.qargs) == 1 and hasattr(meas.qargs[0], 'reg'):
            qarg = meas.qargs[0]
            if hasattr(qarg.reg, 'sym'):
                qreg_name = qarg.reg.sym
                
                # Check if this register has all measurements together
                if (qreg_name in self.generator.measurement_info and
                    self.generator.measurement_info[qreg_name].all_measured_together and
                    position == self.generator.measurement_info[qreg_name].first_measurement_pos):
                    
                    # We'll use a temporary array for the measurement results
                    temp_var_name = f"_temp_measure_{qreg_name}"
                    return True, qreg_name, temp_var_name
        
        return False, "", ""
    
    def _generate_measurement(self, meas, position: int = -1) -> None:
        """Generate measurement operations with array unpacking support."""
        # Track consumed qubits globally for ALL measurements
        if hasattr(meas, 'qargs'):
            for qarg in meas.qargs:
                if hasattr(qarg, 'reg') and hasattr(qarg.reg, 'sym'):
                    qreg_name = qarg.reg.sym
                    if qreg_name not in self.generator.consumed_qubits:
                        self.generator.consumed_qubits[qreg_name] = set()
                    
                    if hasattr(qarg, 'index'):
                        # Single qubit measurement
                        self.generator.consumed_qubits[qreg_name].add(qarg.index)
                    elif hasattr(qarg, 'size'):
                        # Full register measurement
                        for i in range(qarg.size):
                            self.generator.consumed_qubits[qreg_name].add(i)
        
        # Check if we should use measure_array for individual measurements
        should_use_array, qreg_name, temp_var_name = self._should_use_measure_array(meas, position)
        if should_use_array:
            # Get the QReg size
            qreg = self.generator.variable_context.get(qreg_name)
            qreg_size = qreg.size if qreg and hasattr(qreg, 'size') else 0
            
            # Generate measure_array to temporary variable
            self.generator.write(f"{temp_var_name} = quantum.measure_array({qreg_name})")
            
            # Mark this register as handled with measurement destinations
            self.generator.unpacked_arrays[qreg_name] = {
                'type': 'measure_array_temp',
                'temp_var': temp_var_name,
                'destinations': {}  # Will be filled as we process individual measurements
            }
            
            # Process this first measurement
            self._handle_measure_array_distribution(meas, qreg_name)
            return
            
        # Check if this measurement is part of an already handled measure_array
        if hasattr(meas, 'qargs') and len(meas.qargs) == 1 and hasattr(meas.qargs[0], 'reg'):
            qarg = meas.qargs[0]
            if hasattr(qarg.reg, 'sym') and hasattr(qarg, 'index'):
                qreg_name = qarg.reg.sym
                if qreg_name in self.generator.unpacked_arrays:
                    unpacked_value = self.generator.unpacked_arrays[qreg_name]
                    if isinstance(unpacked_value, dict) and unpacked_value.get('type') == 'measure_array_temp':
                        # Handle distribution from temporary array
                        self._handle_measure_array_distribution(meas, qreg_name)
                        return
                    elif isinstance(unpacked_value, str) and unpacked_value.startswith("__measure_array_handled_"):
                        # Skip this measurement as it's already handled by measure_array
                        return
        
        # Check if we need to unpack arrays first
        self._check_and_unpack_arrays(meas, position)
        
        # Check if it's a single qubit or array measurement
        if hasattr(meas, 'cout') and meas.cout:
            # First, check if this is measuring an entire QReg
            if (len(meas.qargs) == 1 and hasattr(meas.qargs[0], 'size') and 
                len(meas.cout) == 1 and hasattr(meas.cout[0], 'size') and
                meas.qargs[0].size == meas.cout[0].size):
                
                qreg = meas.qargs[0]
                creg = meas.cout[0]
                
                # Check if all qubits are being measured together
                if (qreg.sym in self.generator.measurement_info and 
                    self.generator.measurement_info[qreg.sym].all_measured_together):
                    # Use measure_array for efficiency
                    # Check for renamed variables
                    qreg_name = qreg.sym
                    creg_name = creg.sym
                    if hasattr(self.generator, 'renamed_vars'):
                        if qreg_name in self.generator.renamed_vars:
                            qreg_name = self.generator.renamed_vars[qreg_name]
                        if creg_name in self.generator.renamed_vars:
                            creg_name = self.generator.renamed_vars[creg_name]
                    self.generator.write(f"{creg_name} = quantum.measure_array({qreg_name})")
                    
                    # Mark entire array as consumed
                    if qreg.sym not in self.generator.consumed_qubits:
                        self.generator.consumed_qubits[qreg.sym] = set()
                    for i in range(qreg.size):
                        self.generator.consumed_qubits[qreg.sym].add(i)
                    
                    return
            
            # Handle other measurement patterns
            if (len(meas.qargs) == 1 and hasattr(meas.qargs[0], 'size') and 
                len(meas.cout) == 1 and hasattr(meas.cout[0], 'size')):
                # Full register to full register measurement (but not all together)
                qreg = meas.qargs[0]
                creg = meas.cout[0]
                # Fall through to individual measurements
            elif (len(meas.qargs) > 1 and len(meas.cout) == 1 and 
                  hasattr(meas.cout[0], 'size') and meas.cout[0].size == len(meas.qargs)):
                # Multiple qubits to single register
                creg = meas.cout[0]
                qubit_refs = [self._get_qubit_ref(q) for q in meas.qargs]
                self.generator.write(f"# Measure {len(meas.qargs)} qubits to {creg.sym}")
                for i, q in enumerate(meas.qargs):
                    qubit_ref = self._get_qubit_ref(q)
                    self.generator.write(f"{creg.sym}[{i}] = quantum.measure({qubit_ref})")
                return
            
            # Individual measurements
            # Check if cout contains a single list for multiple qubits
            if len(meas.cout) == 1 and isinstance(meas.cout[0], list) and len(meas.cout[0]) == len(meas.qargs):
                # Multiple qubits to list of bits: Measure(q0, q1) > [c0, c1]
                for q, c in zip(meas.qargs, meas.cout[0]):
                    qubit_ref = self._get_qubit_ref(q)
                    bit_ref = self._get_qubit_ref(c)
                    self._generate_individual_measurement(q, c, qubit_ref, bit_ref)
            else:
                # Standard one-to-one measurement
                # Check if this is a single full-register measurement
                if (len(meas.qargs) == 1 and len(meas.cout) == 1 and
                    hasattr(meas.qargs[0], 'sym') and hasattr(meas.cout[0], 'sym')):
                    # Full register measurement - use measure_array for HUGR compatibility
                    qreg = meas.qargs[0]
                    creg = meas.cout[0]
                    # Check for renamed variables
                    qreg_name = qreg.sym
                    creg_name = creg.sym
                    if hasattr(self.generator, 'renamed_vars'):
                        if qreg_name in self.generator.renamed_vars:
                            qreg_name = self.generator.renamed_vars[qreg_name]
                        if creg_name in self.generator.renamed_vars:
                            creg_name = self.generator.renamed_vars[creg_name]
                    self.generator.write(f"{creg_name} = quantum.measure_array({qreg_name})")
                    
                    # Mark entire array as consumed
                    if hasattr(qreg, 'sym') and hasattr(qreg, 'size'):
                        if qreg.sym not in self.generator.consumed_qubits:
                            self.generator.consumed_qubits[qreg.sym] = set()
                        for i in range(qreg.size):
                            self.generator.consumed_qubits[qreg.sym].add(i)
                else:
                    # Individual qubit measurements
                    for q, c in zip(meas.qargs, meas.cout):
                        qubit_ref = self._get_qubit_ref(q)
                        bit_ref = self._get_qubit_ref(c)
                        self._generate_individual_measurement(q, c, qubit_ref, bit_ref)
        else:
            # No explicit output bits - just measure and discard results
            for q in meas.qargs:
                qubit_ref = self._get_qubit_ref(q)
                self.generator.write(f"quantum.measure({qubit_ref})")
                
    def _generate_barrier(self, op) -> None:
        """Generate barrier operations."""
        self.generator.write("# Barrier")
        
    def _generate_prep(self, op) -> None:
        """Generate qubit preparation (reset) operations."""
        if hasattr(op, 'qargs') and op.qargs:
            # Check if this is a full register prep
            if len(op.qargs) == 1 and hasattr(op.qargs[0], 'size') and op.qargs[0].size > 1:
                # Full register reset
                reg = op.qargs[0]
                self.generator.write(f"quantum.reset({reg.sym})")
            else:
                # Individual qubit resets
                for q in op.qargs:
                    qubit_ref = self._get_qubit_ref(q)
                    self.generator.write(f"quantum.reset({qubit_ref})")
        
    def _generate_permute(self, op) -> None:
        """Generate permutation operations."""
        if len(op.qargs) == 2:
            # Permute is essentially a swap in Guppy
            qreg1 = op.qargs[0]
            qreg2 = op.qargs[1]
            
            if hasattr(qreg1, 'sym') and hasattr(qreg2, 'sym'):
                # Swap two registers
                # In Guppy, we might need to use a temporary
                self.generator.write(f"# Permute {qreg1.sym} and {qreg2.sym}")
                self.generator.write(f"# TODO: Implement register swap")
            else:
                self.generator.write(f"# WARNING: Permute with non-register arguments")
    
    def _generate_assignment(self, op) -> None:
        """Generate classical assignment operations."""
        if hasattr(op, 'left') and hasattr(op, 'right'):
            left = self.generator.expression_handler.generate_expr(op.left)
            right = self.generator.expression_handler.generate_expr(op.right)
            self.generator.write(f"{left} = {right}")
            
    def _generate_bitwise_op(self, op) -> None:
        """Generate bitwise operations."""
        op_name = type(op).__name__
        
        if op_name == "NOT":
            # Unary NOT operation
            if hasattr(op, 'arg'):
                arg = self.generator.expression_handler.generate_expr(op.arg)
                result = self.generator.expression_handler.generate_expr(op.result)
                self.generator.write(f"{result} = not {arg}")
        else:
            # Binary operations (XOR, AND, OR)
            if hasattr(op, 'left') and hasattr(op, 'right') and hasattr(op, 'result'):
                left = self.generator.expression_handler.generate_expr(op.left)
                right = self.generator.expression_handler.generate_expr(op.right)
                result = self.generator.expression_handler.generate_expr(op.result)
                
                if op_name == "XOR":
                    self.generator.write(f"{result} = {left} != {right}")  # Boolean XOR
                elif op_name == "AND":
                    self.generator.write(f"{result} = {left} and {right}")
                elif op_name == "OR":
                    self.generator.write(f"{result} = {left} or {right}")
                    
    def _handle_measure_array_distribution(self, meas, qreg_name: str) -> None:
        """Handle distributing measurement results from a temporary array."""
        info = self.generator.unpacked_arrays[qreg_name]
        temp_var = info['temp_var']
        
        # Extract the qubit index and destination
        if hasattr(meas, 'qargs') and len(meas.qargs) == 1:
            qarg = meas.qargs[0]
            if hasattr(qarg, 'index'):
                index = qarg.index
                
                # Get the destination
                if hasattr(meas, 'cout') and len(meas.cout) == 1:
                    cout = meas.cout[0]
                    bit_ref = self._get_qubit_ref(cout)
                    
                    # Generate the assignment from temporary array
                    self.generator.write(f"{bit_ref} = {temp_var}[{index}]")
                    
                    # Track this destination
                    info['destinations'][index] = bit_ref
                    
    def _generate_individual_measurement(self, q, c, qubit_ref: str, bit_ref: str) -> None:
        """Generate individual measurement and track if we need to pack results."""
        # Only track individual measurements for packing in main function
        in_main = self.generator.current_scope and type(self.generator.current_scope).__name__ == "Main"
        
        # Check if this is measuring an unpacked qubit IN MAIN FUNCTION
        if in_main and hasattr(q, 'reg') and hasattr(q.reg, 'sym'):
            qreg_name = q.reg.sym
            if qreg_name in self.generator.unpacked_arrays:
                # This is an unpacked measurement
                if hasattr(c, 'reg') and hasattr(c.reg, 'sym') and hasattr(c, 'index'):
                    creg_name = c.reg.sym
                    index = c.index
                    
                    # Generate a unique variable name for this measurement
                    var_name = f"{creg_name}_{index}"
                    
                    # Track this individual measurement
                    if creg_name not in self.individual_measurements:
                        self.individual_measurements[creg_name] = {}
                    self.individual_measurements[creg_name][index] = var_name
                    
                    # NOTE: We track in individual_measurements for packing later
                    # but don't track in unpacked_arrays because that would require
                    # handling all references before they're created
                    
                    # Generate the measurement to the individual variable
                    self.generator.write(f"{var_name} = quantum.measure({qubit_ref})")
                    return
                    
        # Default: generate standard measurement
        self.generator.write(f"{bit_ref} = quantum.measure({qubit_ref})")