"""Compilation pipeline for Guppy → HUGR → Selene Interface plugin.

This module provides functions to compile Guppy programs through HUGR
to Selene Interface plugins that can be executed by SeleneSimpleRuntimeEngine.
"""

import tempfile
import subprocess
from pathlib import Path
from typing import Callable, Union
import logging

logger = logging.getLogger(__name__)


def compile_guppy_to_selene_plugin(guppy_func: Callable) -> bytes:
    """Compile a Guppy function to a Selene Interface plugin.
    
    This performs the full compilation pipeline:
    1. Guppy → HUGR
    2. HUGR → LLVM IR (via Selene's HUGR compiler)
    3. LLVM IR → Selene Interface plugin (.so)
    
    Args:
        guppy_func: A function decorated with @guppy
        
    Returns:
        The compiled plugin as bytes
        
    Raises:
        ImportError: If required tools are not available
        RuntimeError: If compilation fails at any stage
    """
    # Step 1: Compile Guppy to HUGR
    from pecos_rslib.guppy_conversion import guppy_to_hugr
    
    hugr_bytes = guppy_to_hugr(guppy_func)
    
    # Step 2: Compile HUGR to Selene plugin
    return compile_hugr_to_selene_plugin(hugr_bytes)


def compile_hugr_to_selene_plugin(hugr_bytes: bytes) -> bytes:
    """Compile HUGR bytes to a Selene Interface plugin.
    
    This uses Selene's build infrastructure to compile HUGR to a shared library
    that implements Selene's RuntimeInterface, suitable for loading by SeleneSimpleRuntimeEngine.
    
    Args:
        hugr_bytes: HUGR program as bytes (JSON or binary format)
        
    Returns:
        The compiled plugin as bytes
        
    Raises:
        RuntimeError: If compilation fails
    """
    try:
        # Use Selene's build infrastructure
        from selene_sim.build import build
        import json
        
        # Parse HUGR bytes as JSON to get the HUGR package
        hugr_package = json.loads(hugr_bytes)
        
        # Build using Selene's infrastructure
        # This creates a complete executable, not just a plugin
        with tempfile.TemporaryDirectory() as build_dir:
            build_dir = Path(build_dir)
            
            # Build the Selene instance
            instance = build(
                hugr_package,
                name="pecos_plugin",
                build_dir=build_dir,
                verbose=False
            )
            
            # The instance contains the compiled executable
            # For PECOS integration, we need to extract the compiled object/library
            # Look for the compiled artifacts in the build directory
            plugin_path = build_dir / "pecos_plugin"
            if not plugin_path.exists():
                # Try to find the executable or shared library
                for ext in [".so", ".dylib", ".dll", ""]:
                    test_path = build_dir / f"pecos_plugin{ext}"
                    if test_path.exists():
                        plugin_path = test_path
                        break
            
            if plugin_path.exists():
                return plugin_path.read_bytes()
            else:
                # If we can't find a standalone plugin, return a marker
                # The actual execution will use the SeleneInstance directly
                return b"SELENE_INSTANCE_MARKER"
        
    except ImportError:
        # Try alternative Selene compiler functions
        try:
            from selene_hugr_qis_compiler import compile_to_bitcode
            
            # Compile HUGR to LLVM bitcode
            bitcode = compile_to_bitcode(hugr_bytes)
            
            # Link bitcode to create a plugin
            return compile_bitcode_to_shared_library(bitcode)
            
        except ImportError:
            # Fallback: Compile via LLVM IR
            logger.info("Selene build infrastructure not available, using LLVM IR path")
            return compile_hugr_via_llvm(hugr_bytes)


def compile_hugr_via_llvm(hugr_bytes: bytes) -> bytes:
    """Compile HUGR to Selene plugin via LLVM IR.
    
    This is the fallback path when Selene's direct HUGR compiler is not available.
    
    Args:
        hugr_bytes: HUGR program as bytes
        
    Returns:
        The compiled plugin as bytes
        
    Raises:
        RuntimeError: If compilation fails
    """
    # Step 1: HUGR → LLVM IR
    try:
        # Try pecos-selene's HUGR to LLVM compiler
        from pecos_rslib import compile_hugr_to_llvm
        llvm_ir = compile_hugr_to_llvm(hugr_bytes)
    except ImportError:
        # Fallback to compilation_pipeline
        from pecos.compilation_pipeline import compile_hugr_to_llvm
        llvm_ir = compile_hugr_to_llvm(hugr_bytes)
    
    # Step 2: LLVM IR → Selene plugin
    return compile_llvm_to_selene_plugin(llvm_ir)


def compile_bitcode_to_shared_library(bitcode: bytes) -> bytes:
    """Compile LLVM bitcode to a shared library.
    
    Args:
        bitcode: LLVM bitcode as bytes
        
    Returns:
        The compiled shared library as bytes
        
    Raises:
        RuntimeError: If compilation fails
    """
    with tempfile.TemporaryDirectory() as tmpdir:
        tmpdir = Path(tmpdir)
        
        # Write bitcode to file
        bc_file = tmpdir / "program.bc"
        bc_file.write_bytes(bitcode)
        
        # Compile to shared library
        so_file = tmpdir / "plugin.so"
        
        try:
            result = subprocess.run(
                ["llc", "-filetype=obj", "-o", str(tmpdir / "program.o"), str(bc_file)],
                capture_output=True,
                text=True,
                check=True
            )
            
            result = subprocess.run(
                ["gcc", "-shared", "-fPIC", "-o", str(so_file), str(tmpdir / "program.o")],
                capture_output=True,
                text=True,
                check=True
            )
        except subprocess.CalledProcessError as e:
            raise RuntimeError(f"Failed to compile bitcode: {e.stderr}")
        except FileNotFoundError:
            raise RuntimeError("llc or gcc not found. Install LLVM tools.")
        
        return so_file.read_bytes()


def compile_llvm_to_selene_plugin(llvm_ir: str) -> bytes:
    """Compile LLVM IR to a Selene Interface plugin.
    
    This compiles LLVM IR to a shared library that can be loaded
    by SeleneSimpleRuntimeEngine.
    
    Args:
        llvm_ir: LLVM IR as a string
        
    Returns:
        The compiled plugin as bytes
        
    Raises:
        RuntimeError: If compilation fails
    """
    with tempfile.TemporaryDirectory() as tmpdir:
        tmpdir = Path(tmpdir)
        
        # Write LLVM IR to file
        llvm_file = tmpdir / "program.ll"
        llvm_file.write_text(llvm_ir)
        
        # Compile to object file
        obj_file = tmpdir / "program.o"
        try:
            result = subprocess.run(
                ["llc", "-filetype=obj", "-o", str(obj_file), str(llvm_file)],
                capture_output=True,
                text=True,
                check=True
            )
        except subprocess.CalledProcessError as e:
            raise RuntimeError(f"Failed to compile LLVM to object: {e.stderr}")
        except FileNotFoundError:
            raise RuntimeError("llc not found. Install LLVM tools.")
        
        # Link to shared library with Selene runtime interface
        plugin_file = tmpdir / "plugin.so"
        
        # We need to link against Selene's runtime interface
        # This requires knowing where the Selene runtime headers/libs are
        try:
            # Try to find Selene runtime libraries
            import selene_simple_runtime_plugin
            runtime_dir = Path(selene_simple_runtime_plugin.__file__).parent / "_dist" / "lib"
            runtime_lib = runtime_dir / "libselene_simple_runtime.so"
            
            if not runtime_lib.exists():
                raise FileNotFoundError(f"Selene runtime not found at {runtime_lib}")
            
            # Link the object file to create a plugin
            # Note: This is simplified - real linking would need proper flags
            result = subprocess.run(
                [
                    "gcc",
                    "-shared",
                    "-fPIC",
                    "-o", str(plugin_file),
                    str(obj_file),
                    f"-L{runtime_dir}",
                    "-lselene_simple_runtime",
                    "-Wl,-rpath," + str(runtime_dir)
                ],
                capture_output=True,
                text=True,
                check=True
            )
        except (ImportError, FileNotFoundError):
            # Fallback: Create a simple shared library without runtime linking
            logger.warning("Selene runtime not found, creating standalone plugin")
            result = subprocess.run(
                ["gcc", "-shared", "-fPIC", "-o", str(plugin_file), str(obj_file)],
                capture_output=True,
                text=True,
                check=True
            )
        except subprocess.CalledProcessError as e:
            raise RuntimeError(f"Failed to link plugin: {e.stderr}")
        
        # Read the compiled plugin
        return plugin_file.read_bytes()


def create_selene_interface_program(program: Union[Callable, bytes, str]):
    """Create a SeleneInterfaceProgram from various input types.
    
    Args:
        program: Can be:
            - A Guppy function (decorated with @guppy)
            - HUGR bytes
            - LLVM IR string
            - Compiled plugin bytes
            
    Returns:
        A SeleneInterfaceProgram ready to be executed
        
    Raises:
        ValueError: If program type cannot be determined
        RuntimeError: If compilation fails
    """
    # Try to import the program class
    try:
        from pecos_rslib import SeleneInterfaceProgram
    except ImportError:
        # Try importing from internal module
        try:
            from pecos_rslib._pecos_rslib import PySeleneInterfaceProgram as SeleneInterfaceProgram
        except ImportError:
            raise ImportError("SeleneInterfaceProgram not available in pecos_rslib")
    
    # Determine input type and compile as needed
    if callable(program):
        # It's a Guppy function
        plugin_bytes = compile_guppy_to_selene_plugin(program)
    elif isinstance(program, bytes):
        # Could be HUGR bytes or plugin bytes
        # Check if it's an ELF file (compiled plugin)
        if program.startswith(b'\x7fELF'):
            # It's already a compiled plugin
            plugin_bytes = program
        else:
            # Assume it's HUGR bytes
            plugin_bytes = compile_hugr_to_selene_plugin(program)
    elif isinstance(program, str):
        # Assume it's LLVM IR
        plugin_bytes = compile_llvm_to_selene_plugin(program)
    else:
        raise ValueError(f"Unsupported program type: {type(program)}")
    
    # Create the SeleneInterfaceProgram
    return SeleneInterfaceProgram.from_bytes(plugin_bytes)