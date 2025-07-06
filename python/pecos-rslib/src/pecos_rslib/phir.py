"""PHIR (PECOS High-level IR) compilation pipeline.

This module provides an alternative compilation path from HUGR to LLVM IR
via MLIR infrastructure, using the PHIR intermediate representation.
"""

from typing import Optional

# Import PHIR functions from the Rust bindings
from pecos_rslib._pecos_rslib import (
    hugr_to_phir_mlir,
    compile_hugr_via_phir,
    compile_and_execute_via_phir,
    PhirLlvmEngine,
)


class PhirCompiler:
    """High-level interface for the PHIR compilation pipeline.
    
    This class provides convenient methods for working with the PHIR
    (PECOS High-level IR) compilation pipeline.
    """
    
    def __init__(
        self,
        debug_output: bool = False,
        optimization_level: int = 2,
        target_triple: Optional[str] = None,
    ):
        """Initialize the PHIR compiler.
        
        Args:
            debug_output: Whether to output debug information
            optimization_level: LLVM optimization level (0-3)
            target_triple: Target triple for LLVM (e.g., "x86_64-unknown-linux-gnu")
        """
        self.debug_output = debug_output
        self.optimization_level = optimization_level
        self.target_triple = target_triple
    
    
    def get_phir(self, hugr_json: str) -> str:
        """Convert HUGR JSON to PHIR (MLIR text format).
        
        Args:
            hugr_json: HUGR circuit in JSON format
            
        Returns:
            PHIR representation as MLIR text
        """
        return hugr_to_phir_mlir(
            hugr_json,
            self.debug_output,
            self.optimization_level,
        )
    
    def compile(self, hugr_json: str) -> str:
        """Compile HUGR to LLVM IR via the PHIR pipeline.
        
        Args:
            hugr_json: HUGR circuit in JSON format
            
        Returns:
            LLVM IR as a string
        """
        return compile_hugr_via_phir(
            hugr_json,
            self.debug_output,
            self.optimization_level,
            self.target_triple,
        )
    
    def execute(self, hugr_json: str, shots: int = 1, seed: int | None = None) -> list:
        """Compile and execute HUGR via the PHIR pipeline.
        
        Args:
            hugr_json: HUGR circuit in JSON format
            shots: Number of shots to execute
            seed: Random seed for reproducible results (optional)
            
        Returns:
            List of execution results
        """
        return compile_and_execute_via_phir(
            hugr_json,
            shots,
            seed,
            self.debug_output,
            self.optimization_level,
        )
    
    def create_engine(self, hugr_json: str) -> 'PhirLlvmEngine':
        """Create a PHIR LLVM engine from HUGR (in-memory pipeline).
        
        Args:
            hugr_json: HUGR circuit in JSON format
            
        Returns:
            PhirLlvmEngine instance ready for execution
        """
        llvm_ir = self.compile(hugr_json)
        return PhirLlvmEngine(llvm_ir)
    
    def execute_inmemory(self, hugr_json: str, shots: int = 1) -> dict:
        """Execute HUGR via complete in-memory PHIR pipeline.
        
        This method keeps all intermediate representations (PHIR, LLVM IR) 
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
        engine = PhirLlvmEngine(llvm_ir)
        engine.set_shots(shots)
        results = engine.run()
        
        return {
            'status': results.get('status', 'unknown'),
            'shots': shots,
            'llvm_ir_size': len(llvm_ir),
            'results': results.get('raw_output', ''),
            'compilation_pipeline': 'HUGR → PHIR → LLVM IR (in-memory)'
        }


__all__ = [
    "hugr_to_phir_mlir",
    "compile_hugr_via_phir",
    "compile_and_execute_via_phir",
    "PhirCompiler",
    "PhirLlvmEngine",
]