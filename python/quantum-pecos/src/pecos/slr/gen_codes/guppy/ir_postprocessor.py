"""Post-processor for IR nodes to fix array access after unpacking.

This module provides a post-processing pass that runs after IR building
but before rendering to replace ArrayAccess nodes with VariableRef nodes
for arrays that have been unpacked.
"""

from __future__ import annotations

from typing import Optional, Dict, List, Any
from dataclasses import dataclass, field

from .ir import (
    IRNode, Module, Function, Block, Statement, Expression,
    ArrayAccess, VariableRef, FieldAccess, Assignment, FunctionCall,
    MethodCall, BinaryOp, UnaryOp, IfStatement, WhileStatement, ForStatement,
    Measurement, ReturnStatement, TupleExpression, ArrayUnpack, Comment,
    ScopeContext, VariableInfo
)


class IRPostProcessor:
    """Post-processes IR to fix array accesses after unpacking decisions."""
    
    def __init__(self):
        # Track unpacked arrays globally: array_name -> list of unpacked variable names
        self.unpacked_arrays: Dict[str, List[str]] = {}
        # Track current scope for variable lookups
        self.current_scope: Optional[ScopeContext] = None
        
    def process_module(self, module: Module, context: ScopeContext) -> None:
        """Process a module and all its functions."""
        self.current_scope = context
        
        # First, analyze the module to populate unpacking information
        module.analyze(context)
        
        # Then traverse and fix array accesses
        for func in module.functions:
            self._process_function(func, context)
            
    def _process_function(self, func: Function, parent_context: ScopeContext) -> None:
        """Process a function."""
        # Create function scope
        func_context = ScopeContext(parent=parent_context)
        
        # Add parameters to scope
        for param_name, param_type in func.params:
            var_info = VariableInfo(
                name=param_name,
                original_name=param_name,
                var_type=param_type
            )
            func_context.add_variable(var_info)
            
        # Process function body
        self._process_block(func.body, func_context)
        
    def _process_block(self, block: Block, context: ScopeContext) -> None:
        """Process a block of statements."""
        old_scope = self.current_scope
        self.current_scope = context
        
        # First pass: collect unpacking information
        for stmt in block.statements:
            if isinstance(stmt, ArrayUnpack):
                # Record unpacking info
                self.unpacked_arrays[stmt.source] = stmt.targets
                # Also update the context
                var = context.lookup_variable(stmt.source)
                if var:
                    var.is_unpacked = True
                    var.unpacked_names = stmt.targets
                else:
                    # Create variable info if it doesn't exist
                    var_info = VariableInfo(
                        name=stmt.source,
                        original_name=stmt.source,
                        var_type="quantum",
                        is_array=True,
                        is_unpacked=True,
                        unpacked_names=stmt.targets
                    )
                    context.add_variable(var_info)
        
        # Second pass: process all statements
        for i, stmt in enumerate(block.statements):
            block.statements[i] = self._process_node(stmt, context)
            
        self.current_scope = old_scope
        
    def _process_node(self, node: IRNode, context: ScopeContext) -> IRNode:
        """Process any IR node, replacing ArrayAccess as needed."""
        if node is None:
            return None
            
        # Handle ArrayAccess specially
        if isinstance(node, ArrayAccess):
            return self._process_array_access(node, context)
            
        # Handle compound nodes that contain other nodes
        elif isinstance(node, Assignment):
            node.target = self._process_node(node.target, context)
            node.value = self._process_node(node.value, context)
            
        elif isinstance(node, FunctionCall):
            node.args = [self._process_node(arg, context) for arg in node.args]
            
        elif isinstance(node, MethodCall):
            node.obj = self._process_node(node.obj, context)
            node.args = [self._process_node(arg, context) for arg in node.args]
            
        elif isinstance(node, BinaryOp):
            node.left = self._process_node(node.left, context)
            node.right = self._process_node(node.right, context)
            
        elif isinstance(node, UnaryOp):
            node.operand = self._process_node(node.operand, context)
            
        elif isinstance(node, Measurement):
            node.qubit = self._process_node(node.qubit, context)
            if node.target:
                node.target = self._process_node(node.target, context)
                
        elif isinstance(node, ReturnStatement):
            if node.value:
                node.value = self._process_node(node.value, context)
                
        elif isinstance(node, TupleExpression):
            node.elements = [self._process_node(elem, context) for elem in node.elements]
            
        elif isinstance(node, IfStatement):
            node.condition = self._process_node(node.condition, context)
            self._process_block(node.then_block, ScopeContext(parent=context))
            if node.else_block:
                self._process_block(node.else_block, ScopeContext(parent=context))
                
        elif isinstance(node, WhileStatement):
            node.condition = self._process_node(node.condition, context)
            self._process_block(node.body, ScopeContext(parent=context))
            
        elif isinstance(node, ForStatement):
            node.iterable = self._process_node(node.iterable, context)
            self._process_block(node.body, ScopeContext(parent=context))
            
        elif isinstance(node, Block):
            self._process_block(node, context)
            
        elif isinstance(node, FieldAccess):
            node.obj = self._process_node(node.obj, context)
            
        # Return the node (possibly modified)
        return node
        
    def _process_array_access(self, node: ArrayAccess, context: ScopeContext) -> IRNode:
        """Process an ArrayAccess node, possibly replacing it with VariableRef."""
        # Check if this is accessing an unpacked array
        array_name = None
        
        # Extract array name from different forms
        if node.array_name:
            # Old API: direct array name
            array_name = node.array_name
        elif isinstance(node.array, VariableRef):
            # New API: array is a VariableRef
            array_name = node.array.name
        elif isinstance(node.array, FieldAccess):
            # Complex case: struct.field[index]
            # Process the field access but don't replace the array access
            node.array = self._process_node(node.array, context)
            return node
            
        # Debug: Print what we're processing
        # print(f"DEBUG: Processing ArrayAccess - array_name={array_name}, index={node.index}")
        # print(f"DEBUG: unpacked_arrays={self.unpacked_arrays}")
            
        # If we have an array name and a constant index, check for unpacking
        if array_name and isinstance(node.index, int):
            # Look up variable info
            var = context.lookup_variable(array_name)
            if var and var.is_unpacked and node.index < len(var.unpacked_names):
                # Replace with VariableRef to the unpacked variable
                # print(f"DEBUG: Replacing {array_name}[{node.index}] with {var.unpacked_names[node.index]}")
                return VariableRef(var.unpacked_names[node.index])
                
            # Also check our local tracking
            if array_name in self.unpacked_arrays:
                unpacked_names = self.unpacked_arrays[array_name]
                if node.index < len(unpacked_names):
                    # print(f"DEBUG: Replacing {array_name}[{node.index}] with {unpacked_names[node.index]}")
                    return VariableRef(unpacked_names[node.index])
                    
        # Process array if it's an IRNode
        if node.array and isinstance(node.array, IRNode):
            node.array = self._process_node(node.array, context)
            
        # Process index if it's an IRNode
        if isinstance(node.index, IRNode):
            node.index = self._process_node(node.index, context)
            
        return node