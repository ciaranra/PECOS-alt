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

def run_guppy(
    guppy_function: Callable[..., T],
    shots: int = 1,
    *,
    verbose: bool = False,
    seed: int | None = None,
    max_qubits: int | None = None,
    **kwargs: Any,  # noqa: ANN401
) -> dict[str, Any]:
    """Run a Guppy quantum function on PECOS - simple API similar to run_qasm().
    
    NOTE: This function is provided for backward compatibility.
    Consider using the new unified API instead:
    
        from pecos_rslib import selene_engine
        
        results = selene_engine().program(guppy_func).qubits(n).to_sim().seed(42).run(shots)

    Args:
        guppy_function: A function decorated with @guppy
        shots: Number of shots to execute (default: 1)
        verbose: Enable verbose output
        seed: Random seed for reproducible results (default: None for random)
        max_qubits: Maximum number of qubits to allocate (default: None for automatic)
        **kwargs: Additional arguments passed to GuppyFrontend

    Returns:
        Dictionary containing:
        - 'results': List of measurement results
        - 'shots': Number of shots executed
        - 'function_name': Name of the executed function
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
        or str(type(guppy_function)).find("GuppyFunctionDefinition") != -1
    )

    if not is_guppy:
        msg = f"Function {function_name} must be decorated with @guppy"
        raise ValueError(msg)

    if verbose:
        print(f"Running Guppy function: {function_name}")
        print(f"Shots: {shots}")

    # Create frontend (try Rust backend first, but fall back to Selene if HUGR version mismatch)
    try:
        frontend = GuppyFrontend(
            use_rust_backend=True,
            **kwargs,
        )
    except Exception as e:
        msg = f"Failed to create Guppy frontend: {e}"
        raise RuntimeError(msg) from e

    if verbose:
        print("[OK] Using high-performance Rust backend")

    # Compile function
    start_time = time.time()
    used_selene = False
    try:
        qir_file = frontend.compile_function(guppy_function)
        compilation_time = time.time() - start_time

        if verbose:
            print(f"[PASS] Compilation completed in {compilation_time:.4f}s")
            print(f"QIR file: {qir_file}")

    except Exception as e:
        # Check if it's a HUGR version incompatibility error
        if "HUGR version incompatibility" in str(e):
            if verbose:
                print("[WARNING] HUGR version incompatibility detected, switching to Selene backend")
            
            # Try with Selene backend (HUGR 0.13 compatible)
            try:
                from pecos.frontends.guppy_selene_compiler import GuppySeleneCompiler
                compiler = GuppySeleneCompiler()
                output_dir = compiler.compile_function(guppy_function)
                compilation_time = time.time() - start_time
                used_selene = True
                
                # For Selene, we use the output directory which contains HUGR and LLVM files
                qir_file = output_dir
                
                if verbose:
                    print(f"[PASS] Compilation completed with Selene backend in {compilation_time:.4f}s")
                    print(f"Output directory: {output_dir}")
            except Exception as selene_error:
                # If Selene also fails, report the original error
                msg = f"Compilation failed: {e}"
                raise RuntimeError(msg) from e
        else:
            msg = f"Compilation failed: {e}"
            raise RuntimeError(msg) from e

    # Execute using appropriate backend
    execution_start = time.time()
    
    if used_selene:
        # For Selene backend, we have HUGR and LLVM files - execute them properly
        if verbose:
            print("[INFO] Executing through Selene runtime with PECOS simulation infrastructure")
        
        # Use the Selene engine to execute the plugin
        from pecos_rslib import selene_engine, state_vector
        
        # Determine number of qubits by analyzing the actual LLVM IR
        def count_qubits_in_llvm_ir(llvm_content):
            """Count qubit allocations in LLVM IR content."""
            try:
                # Count calls to __quantum__rt__qubit_allocate()
                alloc_count = llvm_content.count("call i64 @__quantum__rt__qubit_allocate()")
                return max(1, alloc_count)  # At least 1 qubit
            except:
                return 1  # Default fallback
        
        try:
            # For Selene backend, we have HUGR and LLVM files in the output directory
            if used_selene:
                # qir_file is actually the output directory for Selene
                output_dir = qir_file
                llvm_file = output_dir / f"{function_name}.ll"
                hugr_file = output_dir / f"{function_name}.hugr"
                
                # If files don't exist with expected name, look for any .ll and .hugr files
                if not llvm_file.exists():
                    llvm_files = list(output_dir.glob("*.ll"))
                    if llvm_files:
                        llvm_file = llvm_files[0]
                        if verbose:
                            print(f"[INFO] Using LLVM file: {llvm_file}")
                
                if not hugr_file.exists():
                    hugr_files = list(output_dir.glob("*.hugr"))
                    if hugr_files:
                        hugr_file = hugr_files[0]
                        if verbose:
                            print(f"[INFO] Using HUGR file: {hugr_file}")
                
                if llvm_file.exists():
                    # Read LLVM IR and count qubits
                    with open(llvm_file, 'r') as f:
                        llvm_content = f.read()
                    n_qubits = count_qubits_in_llvm_ir(llvm_content)
                    if verbose:
                        print(f"[INFO] Detected {n_qubits} qubits from LLVM IR analysis")
                    
                    # Use LLVM IR file for direct execution
                    engine_builder = selene_engine().llvm_file(str(llvm_file)).qubits(n_qubits)
                elif hugr_file.exists():
                    # For HUGR files, use a reasonable default since we can't easily analyze them
                    n_qubits = 2  # Default for most quantum circuits
                    if verbose:
                        print(f"[INFO] Using default {n_qubits} qubits for HUGR file")
                    
                    # Use HUGR file (will be compiled to LLVM)
                    engine_builder = selene_engine().hugr_file(str(hugr_file)).qubits(n_qubits)
                else:
                    raise RuntimeError(f"No LLVM or HUGR file found in {output_dir}")
            else:
                # For regular path, analyze the QIR file if possible
                try:
                    with open(qir_file, 'r') as f:
                        llvm_content = f.read()
                    n_qubits = count_qubits_in_llvm_ir(llvm_content)
                    if verbose:
                        print(f"[INFO] Detected {n_qubits} qubits from QIR analysis")
                except:
                    n_qubits = 2  # Default fallback
                    if verbose:
                        print(f"[INFO] Could not analyze QIR file, using default {n_qubits} qubits")
                
                # For regular path, use the QIR file as LLVM
                engine_builder = selene_engine().llvm_file(str(qir_file)).qubits(n_qubits)
        
            # Ensure we don't exceed max_qubits
            if max_qubits is not None and n_qubits > max_qubits:
                if verbose:
                    print(f"[WARNING] Detected {n_qubits} qubits but max_qubits is {max_qubits}, using {max_qubits}")
                n_qubits = max_qubits
                engine_builder = engine_builder.qubits(n_qubits)
        
            # Create quantum state backend and configure simulation
            sim_builder = engine_builder.to_sim()
            
            # Check if we need state vector simulator for non-Clifford gates
            needs_state_vector = False
            if 'llvm_file' in locals() and llvm_file.exists():
                try:
                    with open(llvm_file, 'r') as f:
                        llvm_content = f.read()
                    # Check for non-Clifford gates
                    non_clifford_gates = ["__quantum__qis__rx__body", "__quantum__qis__ry__body", 
                                          "__quantum__qis__rz__body", "__quantum__qis__t__body",
                                          "__quantum__qis__tdg__body", "__quantum__qis__crz__body"]
                    for gate in non_clifford_gates:
                        if gate in llvm_content:
                            needs_state_vector = True
                            if verbose:
                                print(f"[INFO] Detected non-Clifford gate {gate}, using state vector simulator")
                            break
                except:
                    pass
            elif 'llvm_content' in locals():
                # Check the llvm_content that may have been read earlier
                non_clifford_gates = ["__quantum__qis__rx__body", "__quantum__qis__ry__body", 
                                      "__quantum__qis__rz__body", "__quantum__qis__t__body",
                                      "__quantum__qis__tdg__body", "__quantum__qis__crz__body"]
                for gate in non_clifford_gates:
                    if gate in llvm_content:
                        needs_state_vector = True
                        if verbose:
                            print(f"[INFO] Detected non-Clifford gate {gate}, using state vector simulator")
                        break
            
            # Use state vector simulator if needed
            if needs_state_vector:
                sim_builder = sim_builder.quantum(state_vector())
            
            # Add seed if specified
            if seed is not None:
                sim_builder = sim_builder.seed(seed)
            
            # Run simulation
            sim_results = sim_builder.run(shots)
            
            # Convert results to expected format
            # sim_results is a ShotVec object, not a list
            results = []
            
            # Try different ways to access the results
            try:
                if verbose:
                    print(f"[DEBUG] ShotVec type: {type(sim_results)}")
                    print(f"[DEBUG] ShotVec attributes: {dir(sim_results)}")
                
                # Try to convert the ShotVec to a usable format
                converted = False
                
                # Method 1: Check if it has a to_dict method
                if hasattr(sim_results, 'to_dict') and callable(getattr(sim_results, 'to_dict')):
                    try:
                        dict_results = sim_results.to_dict()
                        if verbose:
                            print(f"[DEBUG] to_dict() result: {dict_results}")
                        # Extract the results from dict format
                        if isinstance(dict_results, dict):
                            # Check for single result key
                            if 'result' in dict_results:
                                results = dict_results['result']
                                converted = True
                            else:
                                # Handle multiple result keys (result_0, result_1, etc.)
                                result_keys = [k for k in dict_results.keys() if k.startswith('result_')]
                                if result_keys:
                                    if len(result_keys) == 1:
                                        # Single result - return as list of values
                                        results = dict_results[result_keys[0]]
                                        converted = True
                                        if verbose:
                                            print(f"[DEBUG] Using single result key: {result_keys[0]}")
                                    else:
                                        # Multiple results - need to decide how to handle them
                                        # For single-qubit functions that return bool, take the first result
                                        # For multi-qubit functions, combine into tuples
                                        
                                        # Try to determine expected return type
                                        function_returns_single_bool = False
                                        try:
                                            import inspect
                                            if hasattr(guppy_function, 'wrapped') and hasattr(guppy_function.wrapped, 'python_func'):
                                                sig = inspect.signature(guppy_function.wrapped.python_func)
                                                return_type = sig.return_annotation
                                                function_returns_single_bool = (return_type == bool)
                                                if verbose:
                                                    print(f"[DEBUG] Function return type: {return_type}, single bool: {function_returns_single_bool}")
                                        except Exception as e:
                                            if verbose:
                                                print(f"[DEBUG] Could not determine return type: {e}")
                                        
                                        if function_returns_single_bool:
                                            # Take the first result for single bool functions  
                                            results = dict_results[sorted(result_keys)[0]]
                                            converted = True
                                            if verbose:
                                                print(f"[DEBUG] Using first result key for single bool function: {sorted(result_keys)[0]}")
                                        else:
                                            # Combine into tuples for multi-value functions
                                            sorted_keys = sorted(result_keys)
                                            shot_count = len(dict_results[sorted_keys[0]])
                                            results = []
                                            for i in range(shot_count):
                                                shot_result = tuple(dict_results[key][i] for key in sorted_keys)
                                                results.append(shot_result)
                                            converted = True
                                            if verbose:
                                                print(f"[DEBUG] Combined {len(sorted_keys)} result keys into tuples")
                                else:
                                    if verbose:
                                        print(f"[DEBUG] No result keys found in dict: {list(dict_results.keys())}")
                            if verbose and converted:
                                print(f"[DEBUG] Extracted {len(results)} results from dict")
                    except Exception as e:
                        if verbose:
                            print(f"[DEBUG] to_dict() failed: {e}")
                
                # Method 2: Try iteration
                if not converted:
                    try:
                        results = list(sim_results)
                        converted = True
                        if verbose:
                            print(f"[DEBUG] Direct list conversion worked, got {len(results)} results")
                    except Exception as e:
                        if verbose:
                            print(f"[DEBUG] Direct list conversion failed: {e}")
                
                # Method 3: Try indexing if it has __len__
                if not converted and hasattr(sim_results, '__len__'):
                    try:
                        shot_count = len(sim_results)
                        if verbose:
                            print(f"[DEBUG] ShotVec has length: {shot_count}")
                        results = []
                        for i in range(shot_count):
                            outcome = sim_results[i]
                            results.append(outcome)
                        converted = True
                        if verbose:
                            print(f"[DEBUG] Index-based conversion worked, got {len(results)} results")
                    except Exception as e:
                        if verbose:
                            print(f"[DEBUG] Index-based conversion failed: {e}")
                
                # Method 4: If all else fails, use the ShotVec as-is
                if not converted:
                    if verbose:
                        print(f"[WARNING] Could not convert ShotVec, returning raw object")
                    results = sim_results
                    
            except Exception as e:
                if verbose:
                    print(f"[WARNING] Failed to convert ShotVec results: {e}")
                # Return the raw results
                results = sim_results
            
            execution_time = time.time() - execution_start
            
            if verbose:
                print(f"[PASS] Selene execution completed in {execution_time:.4f}s")
                print(f"Got {len(results)} results from Selene engine")
            
            return {
                "results": results,
                "shots": shots,
                "function_name": function_name,
                "compilation_time": compilation_time,
                "execution_time": execution_time,
                "qir_file": str(qir_file),  # Actually an output directory for Selene
            }
        
        except Exception as e:
            if verbose:
                print(f"[WARNING] Selene execution failed: {e}")
                print("[INFO] This may be due to missing plugin support or incomplete implementation")
            
            # If Selene execution fails due to missing implementation, fallback to regular path
            if "No backend for selene execution" in str(e) or "not implemented" in str(e):
                if verbose:
                    print("[INFO] Selene execution not yet implemented, falling back to regular QIR path")
                # Fall through to regular QIR execution
                used_selene = False
            else:
                # For other errors, raise them
                msg = f"Selene execution failed: {e}"
                raise RuntimeError(msg) from e
    
    # Original QIR execution path
    from pecos_rslib import execute_llvm, reset_llvm_runtime
    import os
    
    # IMPORTANT: Reset LLVM runtime state before execution to prevent
    # global state accumulation that causes aborts in Python test suites
    try:
        reset_llvm_runtime()
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
        print("[OK] Using PECOS LLVM PyO3 bindings for execution")
    
    # Execute the QIR file with the PyO3 bindings
    qir_result = execute_llvm(
        str(qir_file),
        shots,
        seed,
        None,  # noise_probability
        None,  # workers
        max_qubits,  # max_qubits
    )
    
    # Extract results from the returned dictionary
    if qir_result.get("execution_successful", False):
        results = qir_result.get("results", [])
        execution_time = time.time() - execution_start
        
        if verbose:
            print(f"[PASS] QIR execution completed in {execution_time:.4f}s")
            print(f"Got {len(results)} results from QIR engine")
        
        # Post-process results to match the function's return type
        # When all measurements are recorded, results come back as tuples
        # but the function signature might specify a single value
        try:
            import inspect
            
            # Get the function's return type
            if hasattr(guppy_function, 'wrapped') and hasattr(guppy_function.wrapped, 'python_func'):
                sig = inspect.signature(guppy_function.wrapped.python_func)
                return_type = sig.return_annotation
                
                # If function returns bool but we got tuples, extract the last element
                # (which corresponds to the return value)
                if return_type == bool and results and isinstance(results[0], tuple):
                    if verbose:
                        print(f"Post-processing: Extracting bool from {len(results[0])}-tuples")
                    results = [r[-1] for r in results]
                # If function returns a specific tuple size, ensure we match it
                elif hasattr(return_type, '__origin__') and return_type.__origin__ == tuple:
                    expected_size = len(return_type.__args__)
                    if results and isinstance(results[0], tuple) and len(results[0]) != expected_size:
                        if verbose:
                            print(f"Post-processing: Adjusting tuple size from {len(results[0])} to {expected_size}")
                        # Take the last N elements to match expected size
                        results = [r[-expected_size:] if len(r) >= expected_size else r for r in results]
        except Exception as e:
            # If we can't determine the return type, just return the raw results
            if verbose:
                print(f"[WARNING] Could not process return type: {e}")
        
        # Return the results
        return {
            "results": results,
            "shots": shots,
            "function_name": function_name,
            "compilation_time": compilation_time,
            "execution_time": execution_time,
            "qir_file": str(qir_file),
        }
    else:
        error_details = qir_result.get("error", "Unknown error")
        msg = f"QIR execution failed: {error_details}"
        raise RuntimeError(msg)


def run_guppy_batch(
    guppy_functions: list[Callable[..., T]],
    shots: int = 1000,
    *,
    verbose: bool = False,
    **kwargs: Any,  # noqa: ANN401
) -> dict[str, dict[str, Any]]:
    """Run multiple Guppy functions in batch.

    Args:
        guppy_functions: List of functions decorated with @guppy
        shots: Number of shots per function
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
        Dictionary showing Guppy and Rust backend availability
    """
    backends = {
        "guppy_available": GUPPY_AVAILABLE,
    }

    # Check Rust backend (the only backend)
    try:
        from pecos_rslib import check_rust_hugr_availability

        rust_available, rust_message = check_rust_hugr_availability()
        backends["rust_backend"] = rust_available
        backends["rust_message"] = rust_message
    except ImportError:
        backends["rust_backend"] = False
        backends["rust_message"] = "Rust backend not installed"

    return backends



# Import the builder pattern implementation
from pecos.frontends.guppy_sim_builder import guppy_sim

# Convenience aliases for consistency with existing PECOS APIs
run_guppy_circuit = run_guppy  # Alternative name
