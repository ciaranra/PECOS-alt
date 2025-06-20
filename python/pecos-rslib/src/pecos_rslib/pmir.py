"""PMIR (PECOS Middle-level IR) compilation pipeline.

This module provides an alternative compilation path from HUGR to LLVM IR
via MLIR infrastructure, using the PMIR intermediate representation.
"""

from typing import Optional

# Import PMIR functions from the Rust bindings
from pecos_rslib._pecos_rslib import (
    hugr_to_past_ron,
    hugr_to_pmir_mlir,
    past_ron_to_pmir_mlir,
    past_ron_to_llvm_ir,
    compile_hugr_via_pmir,
    compile_and_execute_via_pmir,
    PMIRQirEngine,
)


class PMIRCompiler:
    """High-level interface for the PMIR compilation pipeline.
    
    This class provides convenient methods for working with the PMIR
    (PECOS Middle-level IR) compilation pipeline.
    """
    
    def __init__(
        self,
        debug_output: bool = False,
        optimization_level: int = 2,
        target_triple: Optional[str] = None,
    ):
        """Initialize the PMIR compiler.
        
        Args:
            debug_output: Whether to output debug information
            optimization_level: LLVM optimization level (0-3)
            target_triple: Target triple for LLVM (e.g., "x86_64-unknown-linux-gnu")
        """
        self.debug_output = debug_output
        self.optimization_level = optimization_level
        self.target_triple = target_triple
    
    def get_past(self, hugr_json: str) -> str:
        """Convert HUGR JSON to PAST (PECOS AST) in RON format.
        
        Args:
            hugr_json: HUGR circuit in JSON format
            
        Returns:
            PAST representation in RON format
        """
        return hugr_to_past_ron(hugr_json)
    
    def get_pmir(self, hugr_json: str) -> str:
        """Convert HUGR JSON to PMIR (MLIR text format).
        
        Args:
            hugr_json: HUGR circuit in JSON format
            
        Returns:
            PMIR representation as MLIR text
        """
        return hugr_to_pmir_mlir(
            hugr_json,
            self.debug_output,
            self.optimization_level,
        )
    
    def compile(self, hugr_json: str) -> str:
        """Compile HUGR to LLVM IR via the PMIR pipeline.
        
        Args:
            hugr_json: HUGR circuit in JSON format
            
        Returns:
            LLVM IR as a string
        """
        return compile_hugr_via_pmir(
            hugr_json,
            self.debug_output,
            self.optimization_level,
            self.target_triple,
        )
    
    def execute(self, hugr_json: str, shots: int = 1) -> list:
        """Compile and execute HUGR via the PMIR pipeline.
        
        Args:
            hugr_json: HUGR circuit in JSON format
            shots: Number of shots to execute
            
        Returns:
            List of execution results
        """
        return compile_and_execute_via_pmir(
            hugr_json,
            shots,
            self.debug_output,
            self.optimization_level,
        )
    
    def create_engine(self, hugr_json: str) -> 'PMIRQirEngine':
        """Create a PMIR QIR engine from HUGR (in-memory pipeline).
        
        Args:
            hugr_json: HUGR circuit in JSON format
            
        Returns:
            PMIRQirEngine instance ready for execution
        """
        llvm_ir = self.compile(hugr_json)
        return PMIRQirEngine(llvm_ir)
    
    def execute_inmemory(self, hugr_json: str, shots: int = 1) -> dict:
        """Execute HUGR via complete in-memory PMIR pipeline.
        
        This method keeps all intermediate representations (PAST, PMIR, LLVM IR) 
        in memory and only creates a temporary file for the final execution step.
        
        Args:
            hugr_json: HUGR circuit in JSON format
            shots: Number of shots to execute
            
        Returns:
            Dictionary with execution results and metadata
        """
        # Compile HUGR to LLVM IR (all in memory)
        llvm_ir = self.compile(hugr_json)
        
        # Create and execute engine (minimal temp file usage)
        engine = PMIRQirEngine(llvm_ir)
        engine.set_shots(shots)
        results = engine.run()
        
        return {
            'status': results.get('status', 'unknown'),
            'shots': shots,
            'llvm_ir_size': len(llvm_ir),
            'results': results.get('raw_output', ''),
            'compilation_pipeline': 'HUGR → PAST → PMIR → LLVM IR (in-memory)'
        }


__all__ = [
    "hugr_to_past_ron",
    "hugr_to_pmir_mlir",
    "past_ron_to_pmir_mlir",
    "past_ron_to_llvm_ir",
    "compile_hugr_via_pmir",
    "compile_and_execute_via_pmir",
    "PMIRCompiler",
    "PMIRQirEngine",
]