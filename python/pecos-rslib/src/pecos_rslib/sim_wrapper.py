"""Python wrapper for sim() that handles Guppy programs.

This module provides a Python-side sim() function that acts as a thin wrapper:
1. Detects Guppy programs and compiles them to QIS format
2. Passes all programs to the Rust sim() which has robust fallback handling

This follows the user's guidance: "the python side should be mostly a thin wrapper
of the Rust code... except for things where we don't have a Rust equivalent like
Guppy->HUGR and HUGR->QIS"
"""

import logging
from typing import TYPE_CHECKING, Protocol, Union

if TYPE_CHECKING:
    from pecos_rslib.programs import HugrProgram, QisProgram, QasmProgram

logger = logging.getLogger(__name__)


class GuppyFunction(Protocol):
    """Protocol for Guppy-decorated functions."""

    def compile(self) -> dict: ...


ProgramType = Union[
    GuppyFunction, "QasmProgram", "QisProgram", "HugrProgram", bytes, str
]


def sim(program: ProgramType) -> object:
    """Thin Python wrapper for sim() that handles Guppy programs.

    This wrapper:
    1. Detects Guppy functions and compiles them to QIS format
    2. Passes all programs to the Rust sim() which has robust fallback handling

    Args:
        program: The program to simulate (Guppy function, QasmProgram, etc.)

    Returns:
        SimBuilder instance
    """
    from . import _pecos_rslib

    # Check if this is a Guppy function
    def is_guppy_function(obj: object) -> bool:
        """Check if an object is a Guppy-decorated function."""
        return (
            hasattr(obj, "_guppy_compiled")
            or hasattr(obj, "compile")
            or str(type(obj)).find("GuppyFunctionDefinition") != -1
        )

    # Check if this is a HugrProgram that needs compilation
    if type(program).__name__ == "HugrProgram":
        logger.info("Detected HugrProgram, attempting to compile to QIS format")

        try:
            # Get HUGR bytes from the HugrProgram
            if hasattr(program, "to_bytes"):
                hugr_bytes = program.to_bytes()
            elif hasattr(program, "hugr_bytes"):
                hugr_bytes = program.hugr_bytes
            else:
                # Try to get the raw bytes
                hugr_bytes = bytes(program)

            # Compile HUGR to QIS using Selene's hugr-qis compiler
            from selene_hugr_qis_compiler import compile_to_llvm_ir
            qis_ir = compile_to_llvm_ir(hugr_bytes)
            logger.info("Compiled HUGR to QIS LLVM IR successfully")

            # Create QIS program
            qis_program = _pecos_rslib.QisProgram.from_string(qis_ir)
            logger.info("Created QisProgram from HUGR, passing to Rust sim()")

            # Debug support
            import os
            if os.getenv("DEBUG_QIS_CRASH"):
                with open("/tmp/qis_before_sim.ll", "w") as f:
                    f.write(qis_ir)
                logger.info("Saved QIS IR to /tmp/qis_before_sim.ll for debugging")

            program = qis_program
        except Exception as e:
            # If HUGR compilation fails, pass the HugrProgram to Rust
            # This allows Rust to provide appropriate error messages
            logger.warning(f"HUGR compilation failed: {e}. Passing HugrProgram to Rust for error handling.")
            # Keep program as HugrProgram - Rust will handle it

    elif is_guppy_function(program):
        logger.info("Detected Guppy function, compiling to QIS format")

        # Compile Guppy → HUGR → QIS
        hugr_package = program.compile()
        logger.info("Compiled Guppy function to HUGR package")

        # Convert HUGR to bytes for QIS compilation
        if hasattr(hugr_package, "to_bytes"):
            hugr_bytes = hugr_package.to_bytes()
        else:
            hugr_str = hugr_package.to_str()
            hugr_bytes = hugr_str.encode("utf-8")

        # Compile HUGR to QIS using Selene's hugr-qis compiler
        from selene_hugr_qis_compiler import compile_to_llvm_ir
        qis_ir = compile_to_llvm_ir(hugr_bytes)
        logger.info("Compiled HUGR to QIS LLVM IR successfully")

        # Create QIS program - Rust sim() handles all fallback logic
        qis_program = _pecos_rslib.QisProgram.from_string(qis_ir)
        logger.info("Created QisProgram, passing to Rust sim() with fallback handling")

        # Debug support
        import os
        if os.getenv("DEBUG_QIS_CRASH"):
            with open("/tmp/qis_before_sim.ll", "w") as f:
                f.write(qis_ir)
            logger.info("Saved QIS IR to /tmp/qis_before_sim.ll for debugging")

        program = qis_program

    # Pass to Rust sim() which handles all fallback logic
    logger.info("Using Rust sim() for program type: %s", type(program))
    result = _pecos_rslib.sim(program)

    # Force comprehensive cleanup after each simulation to prevent state pollution between tests
    try:
        _pecos_rslib.clear_jit_cache()
    except Exception:
        pass

    # Force garbage collection to clean up any lingering engine resources
    import gc
    gc.collect()

    return result