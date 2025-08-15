"""Handler for expressions and conditions."""

from __future__ import annotations

from typing import TYPE_CHECKING, Optional

if TYPE_CHECKING:
    from .generator import GuppyGenerator


class ExpressionHandler:
    """Handles conversion of SLR expressions to Guppy code."""
    
    def __init__(self, generator: GuppyGenerator):
        self.generator = generator
        
    def generate_condition(self, cond) -> str:
        """Generate a condition expression."""
        op_name = type(cond).__name__
        
        # First check if this is a bitwise operation that should be handled as an expression
        if op_name in ["AND", "OR", "XOR", "NOT"]:
            # These are bitwise operations when used in conditions
            return self.generate_bitwise_expr(cond, None)
        
        # Handle direct bit references (e.g., If(c[0]))
        if op_name == "Bit":
            return self.generate_expr(cond)
        
        if op_name == "EQUIV":
            left = self.generate_expr(cond.left)
            right = self.generate_expr(cond.right)
            return f"{left} == {right}"
        elif op_name == "NEQUIV":
            left = self.generate_expr(cond.left)
            right = self.generate_expr(cond.right)
            return f"{left} != {right}"
        elif op_name == "LT":
            left = self.generate_expr(cond.left)
            right = self.generate_expr(cond.right)
            return f"{left} < {right}"
        elif op_name == "GT":
            left = self.generate_expr(cond.left)
            right = self.generate_expr(cond.right)
            return f"{left} > {right}"
        elif op_name == "LE":
            left = self.generate_expr(cond.left)
            right = self.generate_expr(cond.right)
            return f"{left} <= {right}"
        elif op_name == "GE":
            left = self.generate_expr(cond.left)
            right = self.generate_expr(cond.right)
            return f"{left} >= {right}"
        else:
            return f"__TODO_CONDITION_{op_name}__"  # Placeholder that will cause syntax error if used
            
    def generate_expr(self, expr) -> str:
        """Generate an expression."""
        if hasattr(expr, 'value'):
            # Convert integer comparisons with booleans to proper boolean values
            if expr.value == 1:
                return "True"
            elif expr.value == 0:
                return "False"
            else:
                return str(expr.value)
        elif hasattr(expr, 'reg') and hasattr(expr, 'index'):
            # Handle bit/qubit references like c[0]
            reg_name = expr.reg.sym
            index = expr.index
            
            # Check if this variable was renamed to avoid conflicts
            if hasattr(self.generator, 'renamed_vars') and reg_name in self.generator.renamed_vars:
                reg_name = self.generator.renamed_vars[reg_name]
            
            # Check if this register has been unpacked
            if reg_name in self.generator.unpacked_arrays:
                unpacked_info = self.generator.unpacked_arrays[reg_name]
                if isinstance(unpacked_info, list) and index < len(unpacked_info):
                    # Use the unpacked variable name
                    return unpacked_info[index]
                elif isinstance(unpacked_info, dict) and index in unpacked_info:
                    # Individual element tracking (e.g., for measurements)
                    return unpacked_info[index]
                elif isinstance(unpacked_info, str) and unpacked_info.startswith("__measure_array"):
                    # This was handled by measure_array, use standard indexing
                    return f"{reg_name}[{index}]"
            
            # Default: use standard array indexing
            return f"{reg_name}[{index}]"
        elif hasattr(expr, 'sym'):
            # Check if this variable was renamed to avoid conflicts
            var_name = expr.sym
            if hasattr(self.generator, 'renamed_vars') and var_name in self.generator.renamed_vars:
                var_name = self.generator.renamed_vars[var_name]
            return var_name
        elif isinstance(expr, bool):
            return "True" if expr else "False"
        elif isinstance(expr, int):
            # Convert 0/1 to False/True when used in boolean context
            if expr == 1:
                return "True"
            elif expr == 0:
                return "False"
            else:
                return str(expr)
        elif isinstance(expr, float):
            return str(expr)
        else:
            return str(expr)
            
    def generate_bitwise_expr(self, expr, parent_op: Optional[str] = None) -> str:
        """Generate bitwise expressions for use in assignments.
        
        Args:
            expr: The expression to generate
            parent_op: The parent operation type (for precedence handling)
        """
        if not hasattr(expr, '__class__'):
            return self.generate_expr(expr)
            
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
            left = self.generate_bitwise_expr(expr.left, "XOR")
            right = self.generate_bitwise_expr(expr.right, "XOR")
            result = f"{left} ^ {right}"
        elif op_name == "AND":
            left = self.generate_bitwise_expr(expr.left, "AND")
            right = self.generate_bitwise_expr(expr.right, "AND")
            result = f"{left} & {right}"
        elif op_name == "OR":
            left = self.generate_bitwise_expr(expr.left, "OR")
            right = self.generate_bitwise_expr(expr.right, "OR")
            result = f"{left} | {right}"
        elif op_name == "NOT":
            value = self.generate_bitwise_expr(expr.value, "NOT")
            # NOT binds tightly, only needs parens if the inner expr is complex
            if hasattr(expr.value, '__class__') and type(expr.value).__name__ in precedence:
                result = f"not ({value})"
            else:
                result = f"not {value}"
        else:
            # Not a bitwise operation, handle normally
            return self.generate_expr(expr)
            
        # Add parentheses if needed based on precedence
        if parent_op and op_name in precedence and parent_op in precedence:
            if precedence[op_name] < precedence[parent_op]:
                result = f"({result})"
                
        return result