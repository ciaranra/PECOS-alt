"""Simple run_guppy() API for PECOS.

This module provides a simple, qasm_sim-like interface for running Guppy quantum programs.
"""

import sys
from collections.abc import Callable
from typing import Any, TypeVar

from pecos.frontends.guppy_frontend import GuppyFrontend

T = TypeVar("T")

try:
    from guppylang import guppy

    GUPPY_AVAILABLE = True
except ImportError:
    GUPPY_AVAILABLE = False
    guppy = None

# TODO: Remove backend stuff... there is only one backend... Rust
def run_guppy(
    guppy_function: Callable[..., T],
    shots: int = 1,
    backend: str | None = None,
    *,
    verbose: bool = False,
    seed: int | None = None,
    **kwargs: Any,  # noqa: ANN401
) -> dict[str, Any]:
    """Run a Guppy quantum function on PECOS - simple API similar to run_qasm().

    Args:
        guppy_function: A function decorated with @guppy
        shots: Number of shots to execute (default: 1000)
        backend: Backend to use ("rust", "external", or None for auto-detect)
        verbose: Enable verbose output
        seed: Random seed for reproducible results (default: None for random)
        **kwargs: Additional arguments passed to GuppyFrontend

    Returns:
        Dictionary containing:
        - 'results': List of measurement results
        - 'shots': Number of shots executed
        - 'function_name': Name of the executed function
        - 'backend_used': Which backend was used for compilation
        - 'compilation_time': Time taken for compilation (if available)
        - 'execution_time': Time taken for execution (if available)

    Raises:
        ImportError: If guppylang is not available
        ValueError: If the function is not decorated with @guppy
        RuntimeError: If compilation or execution fails

    Example:
        ```python
        from guppylang import guppy
        from guppylang.std.quantum import qubit, h, measure
        from pecos import run_guppy


        @guppy
        def bell_state() -> tuple[bool, bool]:
            q0, q1 = qubit(), qubit()
            h(q0)
            cx(q0, q1)
            return measure(q0), measure(q1)


        results = run_guppy(bell_state, shots=1000)
        print(
            f"Correlation rate: {sum(1 for r in results['results'] if r[0] == r[1]) / 1000}"
        )
        ```
    """
    import time

    if not GUPPY_AVAILABLE:
        msg = (
            "guppylang is not available. Install with: pip install quantum-pecos[guppy]"
        )
        raise ImportError(
            msg,
        )

    # Check if this is a Guppy function
    # GuppyDefinition objects have different attributes than regular functions
    function_name = getattr(
        guppy_function,
        "__name__",
        getattr(guppy_function, "name", str(guppy_function)),
    )

    # Accept both compiled Guppy functions and GuppyDefinition objects
    is_guppy = (
        hasattr(guppy_function, "_guppy_compiled")
        or hasattr(guppy_function, "name")
        or str(type(guppy_function)).find("GuppyDefinition") != -1
    )

    if not is_guppy:
        msg = f"Function {function_name} must be decorated with @guppy"
        raise ValueError(msg)

    if verbose:
        print(f"Running Guppy function: {function_name}")
        print(f"Shots: {shots}")
        print(f"Backend preference: {backend}")

    # Determine backend selection
    use_rust_backend = None
    if backend == "rust":
        use_rust_backend = True
    elif backend == "external":
        use_rust_backend = False
    # else: None for auto-detect

    # Create frontend
    try:
        frontend = GuppyFrontend(
            use_rust_backend=use_rust_backend,
            **kwargs,
        )
    except Exception as e:
        msg = f"Failed to create Guppy frontend: {e}"
        raise RuntimeError(msg) from e

    # Get backend info
    backend_info = frontend.get_backend_info()
    backend_used = backend_info["backend"]

    if verbose:
        print(f"Using backend: {backend_used}")
        if backend_used == "rust":
            print("[OK] High-performance Rust backend")
        else:
            print("[WARNING] Using external tools (slower)")

    # Compile function
    start_time = time.time()
    try:
        qir_file = frontend.compile_function(guppy_function)
        compilation_time = time.time() - start_time

        if verbose:
            print(f"[PASS] Compilation completed in {compilation_time:.4f}s")
            print(f"QIR file: {qir_file}")

    except Exception as e:
        msg = f"Compilation failed: {e}"
        raise RuntimeError(msg) from e

    # Execute using QIR Engine PyO3 bindings
    execution_start = time.time()
    
    from pecos_rslib import execute_qir, reset_qir_runtime
    import os
    
    # IMPORTANT: Reset QIR runtime state before execution to prevent
    # global state accumulation that causes aborts in Python test suites
    try:
        reset_qir_runtime()
    except Exception as e:
        # Log the error but don't fail - reset errors may indicate deeper issues
        # but shouldn't prevent execution entirely
        if verbose:
            print(f"[WARNING] QIR runtime reset failed: {e}")
        import logging
        logging.getLogger(__name__).warning(f"QIR runtime reset failed: {e}")
    
    # Check if we're running in pytest
    in_pytest = "pytest" in sys.modules
    if in_pytest and verbose:
        print("[INFO] Running in pytest environment - using defensive execution")
    
    if verbose:
        print("[OK] Using PECOS QIR PyO3 bindings for execution")
    
    # Both Rust HUGR and PMIR backends generate HUGR-style LLVM-IR
    # Only HUGR convention is supported after removing QIR convention support
    actual_convention = "hugr"
    
    # Execute the QIR file with the PyO3 bindings
    qir_result = execute_qir(
        str(qir_file),
        shots,
        seed,
        None,  # noise_probability
        None,  # workers
        llvm_convention=actual_convention  # Use HUGR convention
    )
    
    # Extract results from the returned dictionary
    if qir_result.get("execution_successful", False):
        results = qir_result.get("results", [])
        execution_time = time.time() - execution_start
        
        if verbose:
            print(f"[PASS] QIR execution completed in {execution_time:.4f}s")
            print(f"Got {len(results)} results from QIR engine")
        
        # Return the results
        return {
            "results": results,
            "shots": shots,
            "function_name": function_name,
            "backend_used": backend_used,
            "compilation_time": compilation_time,
            "execution_time": execution_time,
            "qir_file": str(qir_file),
            "backend_info": backend_info,
        }
    else:
        error_details = qir_result.get("error", "Unknown error")
        msg = f"QIR execution failed: {error_details}"
        raise RuntimeError(msg)


def run_guppy_batch(
    guppy_functions: list[Callable[..., T]],
    shots: int = 1000,
    backend: str | None = None,
    *,
    verbose: bool = False,
    **kwargs: Any,  # noqa: ANN401
) -> dict[str, dict[str, Any]]:
    """Run multiple Guppy functions in batch.

    Args:
        guppy_functions: List of functions decorated with @guppy
        shots: Number of shots per function
        backend: Backend to use for all functions
        verbose: Enable verbose output
        **kwargs: Additional arguments passed to run_guppy

    Returns:
        Dictionary mapping function names to their results

    Example:
        ```python
        results = run_guppy_batch([bell_state, random_bit], shots=1000)
        for func_name, result in results.items():
            print(f"{func_name}: {len(result['results'])} results")
        ```
    """
    results = {}

    if verbose:
        print(f"Running {len(guppy_functions)} Guppy functions in batch")

    for i, func in enumerate(guppy_functions):
        func_name = getattr(func, "__name__", getattr(func, "name", str(func)))
        if verbose:
            print(f"\n[{i+1}/{len(guppy_functions)}] Running {func_name}")

        try:
            result = run_guppy(
                func,
                shots=shots,
                backend=backend,
                verbose=verbose,
                **kwargs,
            )
            func_name = getattr(func, "__name__", getattr(func, "name", str(func)))
            results[func_name] = result

        except Exception as e:  # noqa: BLE001
            func_name = getattr(func, "__name__", getattr(func, "name", str(func)))
            if verbose:
                print(f"[ERROR] Failed to run {func_name}: {e}")
            results[func_name] = {
                "error": str(e),
                "function_name": getattr(
                    func,
                    "__name__",
                    getattr(func, "name", str(func)),
                ),
                "shots": 0,
                "results": [],
            }

    return results


def get_guppy_backends() -> dict[str, bool]:
    """Get available backends for Guppy execution.

    Returns:
        Dictionary showing which backends are available
    """
    backends = {
        "guppy_available": GUPPY_AVAILABLE,
    }

    # Check Rust backend
    try:
        from pecos_rslib import check_rust_hugr_availability

        rust_available, rust_message = check_rust_hugr_availability()
        backends["rust_backend"] = rust_available
        backends["rust_message"] = rust_message
    except ImportError:
        backends["rust_backend"] = False
        backends["rust_message"] = "Rust backend not installed"

    # Check external tools (this would require more sophisticated detection)
    backends["external_tools"] = True  # Assume available if binaries are provided

    return backends



# Convenience aliases for consistency with existing PECOS APIs
guppy_sim = run_guppy  # Alias similar to qasm_sim
run_guppy_circuit = run_guppy  # Alternative name
