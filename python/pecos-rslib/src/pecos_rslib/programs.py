"""Program types for PECOS quantum simulation.

This module provides the Rust program types for the unified simulation API.
"""

from typing import TYPE_CHECKING

# Import the Rust program types
from pecos_rslib._pecos_rslib import (
    QasmProgram,
    LlvmProgram,
    HugrProgram,
    PhirJsonProgram,
)

# TODO: Import WasmProgram and WatProgram once exposed from Rust
# For now, provide Python stubs
class WasmProgram:
    """A WebAssembly program wrapper."""
    
    def __init__(self, wasm_bytes: bytes):
        """Initialize with WASM bytes."""
        self.wasm = wasm_bytes
    
    @classmethod
    def from_bytes(cls, wasm_bytes: bytes) -> "WasmProgram":
        """Create a WASM program from bytes."""
        return cls(wasm_bytes)
    
    def bytes(self) -> bytes:
        """Get the WASM bytes."""
        return self.wasm


class WatProgram:
    """A WebAssembly Text program wrapper."""
    
    def __init__(self, source: str):
        """Initialize with WAT source code."""
        self.source = source
    
    @classmethod
    def from_string(cls, source: str) -> "WatProgram":
        """Create a WAT program from a string."""
        return cls(source)
    
    def __str__(self) -> str:
        return self.source


__all__ = [
    "QasmProgram",
    "LlvmProgram", 
    "HugrProgram",
    "PhirJsonProgram",
    "WasmProgram",
    "WatProgram",
]