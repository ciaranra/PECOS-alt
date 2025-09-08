"""Selene engine builder with Guppy support.

This module extends the Rust selene_engine implementation with Python-side
support for Guppy programs.
"""

from typing import Union, Callable
from pathlib import Path

# Import the Rust bindings
from pecos_rslib._pecos_rslib import (
    selene_engine as _rust_selene_engine,
    LlvmProgram as _RustLlvmProgram,
    HugrProgram as _RustHugrProgram,
)

# Import Guppy conversion utility
from .guppy_conversion import guppy_to_hugr


class SeleneEngineBuilder:
    """Python wrapper for Selene engine builder with Guppy support."""

    def __init__(self, rust_builder=None):
        """Initialize with an optional Rust builder instance."""
        self._rust_builder = rust_builder if rust_builder else _rust_selene_engine()
        self._pending_program = None
        self._is_guppy = False

    def program(
        self,
        program: Union[
            str, Path, Callable, bytes, "_RustLlvmProgram", "_RustHugrProgram"
        ],
    ) -> "SeleneEngineBuilder":
        """Set the program to execute.

        Args:
            program: Can be:
                - LlvmProgram instance
                - HugrProgram instance
                - Guppy function (will be converted to HUGR)
                - Path to compiled plugin (.so file)
                - Raw LLVM IR string (deprecated)
                - Raw HUGR bytes (deprecated)

        Returns:
            Self for method chaining
        """
        if isinstance(program, (_RustLlvmProgram, _RustHugrProgram)):
            # Already a program object, pass to Rust
            self._rust_builder = self._rust_builder.program(program)
        elif isinstance(program, Path):
            # Path to plugin file
            self._rust_builder = self._rust_builder.plugin(str(program))
        elif callable(program):
            # Guppy function - store for conversion at build/run time
            self._pending_program = program
            self._is_guppy = True
        elif isinstance(program, str):
            # Could be a path or LLVM IR
            if (
                program.endswith(".so")
                or program.endswith(".dll")
                or program.endswith(".dylib")
            ):
                # Treat as plugin path
                self._rust_builder = self._rust_builder.plugin(program)
            else:
                # Legacy: raw LLVM IR string
                self._rust_builder = self._rust_builder.program(
                    _RustLlvmProgram.from_string(program)
                )
        elif isinstance(program, bytes):
            # Legacy: raw HUGR bytes
            self._rust_builder = self._rust_builder.program(
                _RustHugrProgram.from_bytes(program)
            )
        else:
            raise TypeError(
                f"Program must be LlvmProgram, HugrProgram, Guppy function, "
                f"plugin Path, LLVM IR string, or HUGR bytes, got {type(program)}"
            )
        return self

    def plugin(self, path: Union[str, Path]) -> "SeleneEngineBuilder":
        """Set a plugin file path directly.

        Args:
            path: Path to compiled plugin (.so file)

        Returns:
            Self for method chaining
        """
        self._rust_builder = self._rust_builder.plugin(str(path))
        return self

    def llvm_file(self, path: Union[str, Path]) -> "SeleneEngineBuilder":
        """Load LLVM IR from a file.

        Args:
            path: Path to LLVM IR file (.ll)

        Returns:
            Self for method chaining
        """
        path = Path(path)
        if not path.exists():
            raise FileNotFoundError(f"LLVM file not found: {path}")

        # Read LLVM IR from file
        llvm_ir = path.read_text()
        # Create LlvmProgram from string
        self._rust_builder = self._rust_builder.program(
            _RustLlvmProgram.from_string(llvm_ir)
        )
        return self

    def hugr_file(self, path: Union[str, Path]) -> "SeleneEngineBuilder":
        """Load HUGR from a file.

        Args:
            path: Path to HUGR file (.hugr)

        Returns:
            Self for method chaining
        """
        path = Path(path)
        if not path.exists():
            raise FileNotFoundError(f"HUGR file not found: {path}")

        # Read HUGR bytes from file
        hugr_bytes = path.read_bytes()
        # Create HugrProgram from bytes
        self._rust_builder = self._rust_builder.program(
            _RustHugrProgram.from_bytes(hugr_bytes)
        )
        return self

    def qubits(self, n: int) -> "SeleneEngineBuilder":
        """Set the number of qubits.

        Args:
            n: Number of qubits to allocate

        Returns:
            Self for method chaining
        """
        self._rust_builder = self._rust_builder.qubits(n)
        return self

    def to_sim(self):
        """Convert to a simulation builder.

        This handles Guppy conversion if needed.
        """
        # If we have a pending Guppy program, convert it now
        if self._pending_program and self._is_guppy:
            # Convert Guppy to HUGR bytes
            hugr_bytes = guppy_to_hugr(self._pending_program)

            # Try to use selene_hugr_qis_compiler if available
            try:
                from selene_hugr_qis_compiler import compile_to_llvm_ir

                llvm_ir = compile_to_llvm_ir(hugr_bytes)
                self._rust_builder = self._rust_builder.program(
                    _RustLlvmProgram.from_string(llvm_ir)
                )
            except ImportError:
                # Fall back to using HUGR directly if Selene supports it
                self._rust_builder = self._rust_builder.program(
                    _RustHugrProgram.from_bytes(hugr_bytes)
                )

        # Return the simulation builder from Rust
        return self._rust_builder.to_sim()


def selene_engine() -> SeleneEngineBuilder:
    """Create a Selene engine builder.

    Returns:
        SeleneEngineBuilder: Builder for configuring Selene simulations

    Examples:
        >>> # With LLVM program
        >>> from pecos_rslib.programs import LlvmProgram
        >>> results = (
        ...     selene_engine()
        ...     .program(LlvmProgram.from_string(llvm_ir))
        ...     .to_sim()
        ...     .run(1000)
        ... )

        >>> # With HUGR program
        >>> from pecos_rslib.programs import HugrProgram
        >>> results = (
        ...     selene_engine()
        ...     .program(HugrProgram.from_bytes(hugr_bytes))
        ...     .to_sim()
        ...     .run(1000)
        ... )

        >>> # With Guppy function (Python-side conversion)
        >>> @guppy
        ... def bell_state():
        ...     q0, q1 = qubit(), qubit()
        ...     h(q0)
        ...     cx(q0, q1)
        ...     return measure(q0), measure(q1)
        ...
        >>> results = selene_engine().program(bell_state).to_sim().run(1000)
    """
    return SeleneEngineBuilder()


# Export the main function and classes
__all__ = [
    "selene_engine",
    "SeleneEngineBuilder",
    "guppy_to_hugr",  # Re-export for convenience
]
