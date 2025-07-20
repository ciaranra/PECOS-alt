"""Guppy to HUGR conversion utilities.

This module provides functions for converting Guppy quantum programs to HUGR format,
which can be used with Selene and other HUGR-compatible engines.
"""

from typing import Callable


def guppy_to_hugr(guppy_func: Callable) -> bytes:
    """Convert a Guppy function to HUGR bytes.
    
    This function compiles a Guppy quantum program to HUGR format, which can then
    be executed by HUGR-compatible engines like Selene.
    
    Args:
        guppy_func: A function decorated with @guppy
        
    Returns:
        HUGR program as bytes
        
    Raises:
        ImportError: If guppylang is not available
        ValueError: If the function is not a Guppy function
        RuntimeError: If compilation fails
        
    Examples:
        >>> from guppylang import guppy
        >>> from guppylang.std.quantum import qubit, h, measure
        >>> 
        >>> @guppy
        ... def bell_state() -> tuple[bool, bool]:
        ...     q0, q1 = qubit(), qubit()
        ...     h(q0)
        ...     cx(q0, q1)
        ...     return measure(q0), measure(q1)
        ...
        >>> # Pre-compile Guppy to HUGR
        >>> hugr_bytes = guppy_to_hugr(bell_state)
        >>> 
        >>> # Use with Selene engine
        >>> from pecos_rslib import selene_engine
        >>> engine = selene_engine().program(hugr_bytes).qubits(2).build()
    """
    try:
        # Import the compilation function from pecos
        from pecos.compilation_pipeline import compile_guppy_to_hugr
    except ImportError as e:
        raise ImportError(
            "Guppy compilation tools not available. "
            "Install with: pip install quantum-pecos[guppy]"
        ) from e
    
    # Delegate to the actual compilation function
    return compile_guppy_to_hugr(guppy_func)


__all__ = ["guppy_to_hugr"]