# Copyright 2024 The PECOS Developers
#
# Licensed under the Apache License, Version 2.0 (the "License"); you may not use this file except in compliance with
# the License.You may obtain a copy of the License at
#
#     https://www.apache.org/licenses/LICENSE-2.0
#
# Unless required by applicable law or agreed to in writing, software distributed under the License is distributed on an
# "AS IS" BASIS, WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied. See the License for the
# specific language governing permissions and limitations under the License.

"""Guppy code generator for SLR programs."""

from __future__ import annotations

from typing import TYPE_CHECKING

from pecos.slr.gen_codes.generator import Generator

if TYPE_CHECKING:
    from pecos.slr import Block


class GuppyGenerator(Generator):
    """Generator that converts SLR programs to Guppy code."""

    def __init__(self, *, module_name: str = "generated_module"):
        """Initialize the Guppy generator.
        
        Args:
            module_name: Name of the generated module.
        """
        self.output = []
        self.indent_level = 0
        self.module_name = module_name
        self.current_scope = None
        self.quantum_ops_used = set()
        self.var_types = {}  # Track variable types
        
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
        # Add imports at the beginning
        imports = []
        imports.append("from __future__ import annotations")
        imports.append("")
        imports.append("from guppylang.decorator import guppy")
        imports.append("from guppylang.std import quantum")
        imports.append("from guppylang.std.builtins import array, owned")
        
        # Add any additional imports needed
        if self.quantum_ops_used:
            imports.append("")
        
        return "\n".join(imports + ["", ""] + self.output)
    
    def generate_block(self, block: Block) -> None:
        """Generate Guppy code for a block."""
        self._handle_block(block)
        
    def _handle_block(self, block: Block) -> None:
        """Handle a block of operations."""
        previous_scope = self.enter_block(block)
        
        block_name = type(block).__name__
        
        # Check if this block has a custom handler
        handler_method = f"_handle_{block_name.lower()}_block"
        if hasattr(self, handler_method):
            getattr(self, handler_method)(block)
        else:
            # Default handling for unknown blocks
            self._handle_generic_block(block)
                
        self.exit_block(previous_scope)
        
    def _handle_main_block(self, block) -> None:
        """Handle Main block - generates a function."""
        self.write("@guppy")
        self.write(f"def {self.module_name}() -> None:")
        self.indent()
        
        # Generate variable declarations
        for var in block.vars:
            self._generate_var_declaration(var)
            
        # Generate operations
        if block.ops:
            for op in block.ops:
                self.generate_op(op)
        else:
            # Empty function body needs pass
            self.write("pass")
            
        self.dedent()
        
    def _handle_if_block(self, block) -> None:
        """Handle If block - generates conditional."""
        cond = self._generate_condition(block.cond)
        self.write(f"if {cond}:")
        self.indent()
        
        if not block.ops:
            self.write("pass")
        else:
            for op in block.ops:
                self.generate_op(op)
                
        self.dedent()
        
    def _handle_repeat_block(self, block) -> None:
        """Handle Repeat block - generates for loop."""
        # Repeat blocks store their count in cond
        limit = block.cond if hasattr(block, 'cond') else 1
        self.write(f"for _ in range({limit}):")
        self.indent()
        
        if not block.ops:
            self.write("pass")
        else:
            for op in block.ops:
                self.generate_op(op)
                
        self.dedent()
        
            
    def _handle_generic_block(self, block) -> None:
        """Handle generic/unknown blocks by processing their operations."""
        block_name = type(block).__name__
        
        # Add a comment to indicate the block type
        if block_name not in ["Block", "Main"]:
            self.write(f"# {block_name} block")
            
        # Process all operations in the block
        if hasattr(block, 'ops'):
            for op in block.ops:
                self.generate_op(op)
        else:
            self.write(f"# TODO: Handle {block_name} block - no specific handler implemented")
        
    def enter_block(self, block) -> None:
        """Enter a new block scope."""
        previous_scope = self.current_scope
        self.current_scope = block
        return previous_scope
        
    def exit_block(self, previous_scope) -> None:
        """Exit the current block scope."""
        self.current_scope = previous_scope
        
    def _generate_var_declaration(self, var) -> None:
        """Generate variable declarations."""
        var_type = type(var).__name__
        
        if var_type == "QReg":
            self.var_types[var.sym] = "quantum"
            self.write(f"{var.sym} = array(quantum.qubit() for _ in range({var.size}))")
        elif var_type == "CReg":
            self.var_types[var.sym] = "classical"
            self.write(f"{var.sym} = array(False for _ in range({var.size}))")
        else:
            # For any other variable types, check if they have standard attributes
            if hasattr(var, 'vars'):
                # This is a complex type with sub-variables (like Steane)
                # Generate declarations for all sub-variables
                for sub_var in var.vars:
                    self._generate_var_declaration(sub_var)
            else:
                # Unknown variable type
                var_name = var.sym if hasattr(var, 'sym') else str(var)
                self.write(f"# TODO: Initialize {var_type} instance '{var_name}'")
                self.write(f"# Unknown variable type: {var_type}")
            
    def _generate_condition(self, cond) -> str:
        """Generate a condition expression."""
        op_name = type(cond).__name__
        
        # First check if this is a bitwise operation that should be handled as an expression
        if op_name in ["AND", "OR", "XOR", "NOT"]:
            # These are bitwise operations when used in conditions
            return self._generate_bitwise_expr(cond, None)
        
        # Handle direct bit references (e.g., If(c[0]))
        if op_name == "Bit":
            return self._generate_expr(cond)
        
        if op_name == "EQUIV":
            left = self._generate_expr(cond.left)
            right = self._generate_expr(cond.right)
            return f"{left} == {right}"
        elif op_name == "NEQUIV":
            left = self._generate_expr(cond.left)
            right = self._generate_expr(cond.right)
            return f"{left} != {right}"
        elif op_name == "LT":
            left = self._generate_expr(cond.left)
            right = self._generate_expr(cond.right)
            return f"{left} < {right}"
        elif op_name == "GT":
            left = self._generate_expr(cond.left)
            right = self._generate_expr(cond.right)
            return f"{left} > {right}"
        elif op_name == "LE":
            left = self._generate_expr(cond.left)
            right = self._generate_expr(cond.right)
            return f"{left} <= {right}"
        elif op_name == "GE":
            left = self._generate_expr(cond.left)
            right = self._generate_expr(cond.right)
            return f"{left} >= {right}"
        else:
            return f"__TODO_CONDITION_{op_name}__"  # Placeholder that will cause syntax error if used
            
    def _generate_expr(self, expr) -> str:
        """Generate an expression."""
        if hasattr(expr, 'value'):
            return str(expr.value)
        elif hasattr(expr, 'reg') and hasattr(expr, 'index'):
            # Handle bit/qubit references like c[0]
            return f"{expr.reg.sym}[{expr.index}]"
        elif hasattr(expr, 'sym'):
            return expr.sym
        elif isinstance(expr, int) or isinstance(expr, float) or isinstance(expr, bool):
            return str(expr)
        else:
            return str(expr)
            
    def generate_op(self, op) -> None:
        """Generate code for an operation."""
        try:
            op_name = type(op).__name__
            
            # Handle blocks first
            if hasattr(op, 'ops'):
                self._handle_block(op)
            # Handle measurements
            elif op_name == "Measure":
                self._generate_measurement(op)
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
                self.write(f"# WARNING: Unhandled operation type: {op_name}")
                self.write(f"# Module: {op.__module__ if hasattr(op, '__module__') else 'unknown'}")
                self.write(f"# Attributes: {[attr for attr in dir(op) if not attr.startswith('_')][:5]}...")  # Show first 5 attributes
        except Exception as e:
            # Catch any unexpected errors and generate a comment instead of crashing
            self.write(f"# ERROR: Failed to generate operation {type(op).__name__}")
            self.write(f"# Exception: {type(e).__name__}: {str(e)}")
            
    def _generate_quantum_gate(self, gate) -> None:
        """Generate quantum gate operations."""
        gate_name = type(gate).__name__
        
        # Map gate names to Guppy quantum operations
        gate_map = {
            "H": "quantum.h",
            "X": "quantum.x", 
            "Y": "quantum.y",
            "Z": "quantum.z",
            "S": "quantum.s",
            "SZ": "quantum.s",  # SZ is the S gate
            "Sdg": "quantum.sdg",
            "SZdg": "quantum.sdg",  # SZdg is the Sdg gate
            "T": "quantum.t",
            "Tdg": "quantum.tdg",
            "CX": "quantum.cx",
            "CY": "quantum.cy",
            "CZ": "quantum.cz",
        }
        
        if gate_name in gate_map:
            self.quantum_ops_used.add(gate_name)
            guppy_gate = gate_map[gate_name]
            
            if gate_name in ["CX", "CY", "CZ"]:
                # Two-qubit gates - check for multiple tuple pairs pattern
                # e.g., CX((q[0], q[1]), (q[2], q[3]), (q[4], q[5]))
                if gate.qargs and all(isinstance(arg, tuple) and len(arg) == 2 for arg in gate.qargs):
                    # Multiple (control, target) pairs passed as separate arguments
                    for ctrl, tgt in gate.qargs:
                        ctrl_ref = self._get_qubit_ref(ctrl)
                        tgt_ref = self._get_qubit_ref(tgt)
                        self.write(f"{guppy_gate}({ctrl_ref}, {tgt_ref})")
                elif len(gate.qargs) == 2:
                    # Standard two-qubit gate with control and target
                    ctrl = self._get_qubit_ref(gate.qargs[0])
                    tgt = self._get_qubit_ref(gate.qargs[1])
                    self.write(f"{guppy_gate}({ctrl}, {tgt})")
                else:
                    self.write(f"# ERROR: Two-qubit gate {gate_name} requires exactly 2 qubits, got {len(gate.qargs)}")
                    self.write(f"# Gate arguments: {gate.qargs}")
            else:
                # Single-qubit gates
                if gate.qargs:
                    # Check if this is a full register operation
                    if len(gate.qargs) == 1 and hasattr(gate.qargs[0], 'size') and gate.qargs[0].size > 1:
                        # Apply gate to all qubits in register
                        reg = gate.qargs[0]
                        self.write(f"for i in range({reg.size}):")
                        self.indent()
                        self.write(f"{guppy_gate}({reg.sym}[i])")
                        self.dedent()
                    else:
                        # Single qubit operation(s)
                        for q in gate.qargs:
                            qubit = self._get_qubit_ref(q)
                            self.write(f"{guppy_gate}({qubit})")
                else:
                    self.write(f"# ERROR: Single-qubit gate {gate_name} called with no qubit arguments")
        else:
            self.write(f"# WARNING: Unknown quantum gate: {gate_name}")
            self.write(f"# Add mapping for this gate in gate_map dictionary")
            
    def _get_qubit_ref(self, qubit) -> str:
        """Get the string reference for a qubit."""
        if hasattr(qubit, 'reg') and hasattr(qubit, 'index'):
            return f"{qubit.reg.sym}[{qubit.index}]"
        elif hasattr(qubit, 'sym'):
            # For full registers
            return qubit.sym
        else:
            # Fallback - convert to string but try to clean it up
            s = str(qubit)
            # Try to extract just the bit reference from strings like "<Bit 1 of c>"
            import re
            match = re.match(r'<Bit (\d+) of (\w+)>', s)
            if match:
                return f"{match.group(2)}[{match.group(1)}]"
            return s
            
    def _generate_measurement(self, meas) -> None:
        """Generate measurement operations."""
        # Check if it's a single qubit or array measurement
        if hasattr(meas, 'cout') and meas.cout:
            # Measurement with explicit output bits
            # Check if it's a full register measurement
            if (len(meas.qargs) == 1 and hasattr(meas.qargs[0], 'size') and 
                len(meas.cout) == 1 and hasattr(meas.cout[0], 'size')):
                # Full register to full register measurement
                qreg = meas.qargs[0]
                creg = meas.cout[0]
                self.write(f"{creg.sym} = quantum.measure_array({qreg.sym})")
            elif (len(meas.qargs) > 1 and len(meas.cout) == 1 and 
                  hasattr(meas.cout[0], 'size') and meas.cout[0].size == len(meas.qargs)):
                # Multiple qubits to single register
                creg = meas.cout[0]
                qubit_refs = [self._get_qubit_ref(q) for q in meas.qargs]
                self.write(f"# Measure {len(meas.qargs)} qubits to {creg.sym}")
                for i, q in enumerate(meas.qargs):
                    qubit_ref = self._get_qubit_ref(q)
                    self.write(f"{creg.sym}[{i}] = quantum.measure({qubit_ref})")
            else:
                # Individual measurements
                # Check if cout contains a single list for multiple qubits
                if len(meas.cout) == 1 and isinstance(meas.cout[0], list) and len(meas.cout[0]) == len(meas.qargs):
                    # Multiple qubits to list of bits: Measure(q0, q1) > [c0, c1]
                    for q, c in zip(meas.qargs, meas.cout[0]):
                        qubit_ref = self._get_qubit_ref(q)
                        bit_ref = self._get_qubit_ref(c)
                        self.write(f"{bit_ref} = quantum.measure({qubit_ref})")
                else:
                    # Standard case: pair each qubit with each output
                    for i, (q, c) in enumerate(zip(meas.qargs, meas.cout)):
                        qubit_ref = self._get_qubit_ref(q)
                        # Check if c is a list (multiple bits)
                        if isinstance(c, list):
                            # Generate list of bit references
                            bit_refs = [self._get_qubit_ref(bit) for bit in c]
                            bit_ref_str = "[" + ", ".join(bit_refs) + "]"
                            self.write(f"{bit_ref_str} = quantum.measure({qubit_ref})")
                        else:
                            bit_ref = self._get_qubit_ref(c)
                            self.write(f"{bit_ref} = quantum.measure({qubit_ref})")
        elif hasattr(meas, 'qargs'):
            # Array measurement without explicit output
            if len(meas.qargs) == 1 and hasattr(meas.qargs[0], 'size'):
                # Full register measurement
                reg = meas.qargs[0]
                self.write(f"# Measure all qubits in {reg.sym}")
                self.write(f"meas_{reg.sym} = quantum.measure_array({reg.sym})")
            else:
                # Individual qubit measurements
                for q in meas.qargs:
                    qubit_ref = self._get_qubit_ref(q)
                    self.write(f"quantum.measure({qubit_ref})")
        else:
            self.write("# ERROR: Measurement operation has unexpected structure")
            self.write(f"# Measurement object type: {type(meas)}")
        
    def _generate_assignment(self, assign) -> None:
        """Generate classical assignment operations."""
        lhs = self._generate_expr(assign.left)
        rhs = self._generate_bitwise_expr(assign.right, None)
        self.write(f"{lhs} = {rhs}")
        
    def _generate_bitwise_op(self, op) -> None:
        """Generate bitwise operations."""
        op_name = type(op).__name__
        
        # For standalone bitwise operations (not in assignments),
        # we need to generate them as statements that might have side effects
        # This is rare but can happen in generated code
        if op_name in ["XOR", "AND", "OR", "NOT"]:
            expr = self._generate_bitwise_expr(op, None)
            self.write(f"# Standalone bitwise operation: {expr}")
            self.write(f"_ = {expr}  # Result discarded")
        else:
            self.write(f"# WARNING: Unknown bitwise operation: {op_name}")
            
    def _generate_comment(self, op) -> None:
        """Generate comments."""
        if hasattr(op, 'text'):
            self.write(f"# {op.text}")
        else:
            self.write("# Comment")
            
    def _generate_barrier(self, op) -> None:
        """Generate barrier operations."""
        # Barriers don't have a direct equivalent in Guppy
        # They're used for circuit optimization hints
        self.write("# Barrier")
        
    def _generate_prep(self, op) -> None:
        """Generate state preparation operations."""
        if hasattr(op, 'qargs') and op.qargs:
            # Prep resets qubits to |0> state
            # Generate reset operations for each qubit
            for q in op.qargs:
                qubit_ref = self._get_qubit_ref(q)
                self.write(f"quantum.reset({qubit_ref})")
                self.quantum_ops_used.add("reset")
        else:
            self.write("# ERROR: Prep operation has no qubit arguments")
            
    def _generate_permute(self, op) -> None:
        """Generate permute operations."""
        if hasattr(op, 'elems_i') and hasattr(op, 'elems_f'):
            # Get the initial and final elements
            elems_i = op.elems_i
            elems_f = op.elems_f
            
            # Handle register-level permutation
            if hasattr(elems_i, 'sym') and hasattr(elems_f, 'sym'):
                # Whole register swap - need to swap each element
                if hasattr(elems_i, 'size') and hasattr(elems_f, 'size'):
                    if elems_i.size == elems_f.size:
                        # Generate a loop to swap all elements
                        self.write(f"# Permute registers {elems_i.sym} <-> {elems_f.sym}")
                        self.write(f"for i in range({elems_i.size}):")
                        self.indent()
                        self.write(f"{elems_i.sym}[i], {elems_f.sym}[i] = {elems_f.sym}[i], {elems_i.sym}[i]")
                        self.dedent()
                    else:
                        self.write(f"# ERROR: Cannot permute registers of different sizes ({elems_i.sym}: {elems_i.size}, {elems_f.sym}: {elems_f.size})")
                else:
                    # Simple variable swap
                    self.write(f"{elems_i.sym}, {elems_f.sym} = {elems_f.sym}, {elems_i.sym}")
            
            # Handle single element permutation (e.g., Permute(q[0], q[1]))
            elif hasattr(elems_i, 'reg') and hasattr(elems_i, 'index') and \
                 hasattr(elems_f, 'reg') and hasattr(elems_f, 'index'):
                # Single qubit/bit swap
                ref_i = self._get_qubit_ref(elems_i)
                ref_f = self._get_qubit_ref(elems_f)
                self.write(f"# Permute single elements")
                self.write(f"{ref_i}, {ref_f} = {ref_f}, {ref_i}")
            
            # Handle element-level permutation
            elif isinstance(elems_i, list) and isinstance(elems_f, list):
                if len(elems_i) == len(elems_f):
                    # Generate the references for both sides
                    left_refs = [self._get_qubit_ref(elem) for elem in elems_i]
                    right_refs = [self._get_qubit_ref(elem) for elem in elems_f]
                    
                    # Generate tuple unpacking assignment
                    left_side = ", ".join(left_refs)
                    right_side = ", ".join(right_refs)
                    
                    self.write(f"# Permute elements")
                    self.write(f"{left_side} = {right_side}")
                else:
                    self.write(f"# ERROR: Permute lists must have same length (got {len(elems_i)} and {len(elems_f)})")
            else:
                self.write(f"# WARNING: Permute operation with unexpected structure")
                self.write(f"# elems_i type: {type(elems_i)}, elems_f type: {type(elems_f)}")
        else:
            self.write("# ERROR: Permute operation missing required attributes (elems_i, elems_f)")
            
    def _generate_bitwise_expr(self, expr, parent_op=None) -> str:
        """Generate bitwise expressions for use in assignments.
        
        Args:
            expr: The expression to generate
            parent_op: The parent operation type (for precedence handling)
        """
        if not hasattr(expr, '__class__'):
            return self._generate_expr(expr)
            
        op_name = type(expr).__name__
        
        # Python operator precedence (highest to lowest):
        # NOT > AND > XOR > OR
        precedence = {
            "NOT": 4,
            "AND": 3,
            "XOR": 2,
            "OR": 1,
        }
        
        if op_name == "XOR":
            left = self._generate_bitwise_expr(expr.left, "XOR")
            right = self._generate_bitwise_expr(expr.right, "XOR")
            result = f"{left} ^ {right}"
        elif op_name == "AND":
            left = self._generate_bitwise_expr(expr.left, "AND")
            right = self._generate_bitwise_expr(expr.right, "AND")
            result = f"{left} & {right}"
        elif op_name == "OR":
            left = self._generate_bitwise_expr(expr.left, "OR")
            right = self._generate_bitwise_expr(expr.right, "OR")
            result = f"{left} | {right}"
        elif op_name == "NOT":
            value = self._generate_bitwise_expr(expr.value, "NOT")
            # NOT binds tightly, only needs parens if the inner expr is complex
            if hasattr(expr.value, '__class__') and type(expr.value).__name__ in precedence:
                result = f"not ({value})"
            else:
                result = f"not {value}"
        else:
            # Not a bitwise operation, handle normally
            return self._generate_expr(expr)
            
        # Add parentheses if needed based on precedence
        if parent_op and op_name in precedence and parent_op in precedence:
            if precedence[op_name] < precedence[parent_op]:
                result = f"({result})"
                
        return result