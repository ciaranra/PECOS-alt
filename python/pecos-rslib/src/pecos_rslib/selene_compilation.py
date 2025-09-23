"""Compilation pipeline for Guppy → HUGR → Selene Interface plugin.

This module provides functions to compile Guppy programs through HUGR
to Selene Interface plugins that can be executed by SeleneSimpleRuntimeEngine.
"""

import logging
import shutil
import subprocess
import tempfile
from collections.abc import Callable
from pathlib import Path

logger = logging.getLogger(__name__)


def _run_trusted_build_tool(
    tool_name: str, args: list[str], **kwargs
) -> subprocess.CompletedProcess:
    """Run a trusted build tool with validated path.

    This function explicitly validates that the tool exists in PATH before execution,
    making it clear that the subprocess call is safe and intentional.

    Args:
        tool_name: Name of the build tool (llc, gcc, etc.)
        args: Complete argument list including tool path as first element
        **kwargs: Additional arguments to subprocess.run

    Returns:
        CompletedProcess result

    Raises:
        FileNotFoundError: If tool is not found in PATH
        subprocess.CalledProcessError: If tool execution fails
    """
    # Validate that the tool exists and is in PATH
    tool_path = shutil.which(tool_name)
    if not tool_path:
        raise FileNotFoundError(f"{tool_name} not found in PATH")

    # Ensure first argument matches the validated tool path
    if not args or Path(args[0]).name != tool_name:
        raise ValueError(
            f"Tool path mismatch: expected {tool_name}, got {args[0] if args else 'empty'}"
        )

    # Execute with explicit security settings
    kwargs.setdefault("shell", False)
    kwargs.setdefault("capture_output", True)

    return subprocess.run(args, **kwargs)  # noqa: S603


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
    # For now, skip the selene_sim.build approach which requires a Package object
    # that we can't properly construct. Instead, use the LLVM compilation path.
    # This is a temporary workaround until we can properly create Package objects
    # from HUGR JSON that selene_sim.build can accept.
    return compile_hugr_via_llvm(hugr_bytes)


def compile_hugr_via_llvm(hugr_bytes: bytes, compiler: str = "selene") -> bytes:
    """Compile HUGR to Selene plugin via LLVM IR.

    Args:
        hugr_bytes: HUGR program as bytes
        compiler: Which HUGR compiler to use ("selene" or "rust")

    Returns:
        The compiled plugin as bytes

    Raises:
        RuntimeError: If compilation fails
        ValueError: If invalid compiler specified
    """
    # Step 1: HUGR → LLVM IR
    if compiler == "selene":
        from pecos_rslib import compile_hugr_to_llvm_selene
        llvm_ir = compile_hugr_to_llvm_selene(hugr_bytes)
    elif compiler == "rust":
        from pecos_rslib import compile_hugr_to_llvm_rust
        llvm_ir = compile_hugr_to_llvm_rust(hugr_bytes)
    else:
        raise ValueError(f"Invalid compiler '{compiler}'. Choose 'selene' or 'rust'.")

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
    with tempfile.TemporaryDirectory() as tmpdir_str:
        tmpdir = Path(tmpdir_str)

        # Write bitcode to file
        bc_file = tmpdir / "program.bc"
        bc_file.write_bytes(bitcode)

        # Compile to shared library
        so_file = tmpdir / "plugin.so"

        try:
            llc_path = shutil.which("llc")
            if not llc_path:
                raise FileNotFoundError("llc not found in PATH")

            _run_trusted_build_tool(
                "llc",
                [
                    llc_path,
                    "-filetype=obj",
                    "-o",
                    str(tmpdir / "program.o"),
                    str(bc_file),
                ],
                text=True,
                check=True,
            )

            gcc_path = shutil.which("gcc")
            if not gcc_path:
                raise FileNotFoundError("gcc not found in PATH")

            _run_trusted_build_tool(
                "gcc",
                [
                    gcc_path,
                    "-shared",
                    "-fPIC",
                    "-o",
                    str(so_file),
                    str(tmpdir / "program.o"),
                ],
                text=True,
                check=True,
            )
        except subprocess.CalledProcessError as e:
            raise RuntimeError(f"Failed to compile bitcode: {e.stderr}") from e
        except FileNotFoundError as e:
            raise RuntimeError("llc or gcc not found. Install LLVM tools.") from e

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
    with tempfile.TemporaryDirectory() as tmpdir_str:
        tmpdir = Path(tmpdir_str)

        # Write LLVM IR to file
        llvm_file = tmpdir / "program.ll"
        llvm_file.write_text(llvm_ir)

        # Compile to object file
        obj_file = tmpdir / "program.o"

        try:
            llc_path = shutil.which("llc")
            if not llc_path:
                raise FileNotFoundError("llc not found in PATH")

            _run_trusted_build_tool(
                "llc",
                [llc_path, "-filetype=obj", "-o", str(obj_file), str(llvm_file)],
                text=True,
                check=True,
            )
        except subprocess.CalledProcessError as e:
            raise RuntimeError(f"Failed to compile LLVM to object: {e.stderr}") from e
        except FileNotFoundError as e:
            raise RuntimeError("llc not found. Install LLVM tools.") from e

        # Link to shared library with Selene runtime interface
        plugin_file = tmpdir / "plugin.so"

        # We need to link against Selene's runtime interface
        # This requires knowing where the Selene runtime headers/libs are
        try:
            # Try to find Selene runtime libraries
            import selene_simple_runtime_plugin

            runtime_dir = (
                Path(selene_simple_runtime_plugin.__file__).parent / "_dist" / "lib"
            )
            runtime_lib = runtime_dir / "libselene_simple_runtime.so"

            if not runtime_lib.exists():
                raise FileNotFoundError(f"Selene runtime not found at {runtime_lib}")

            # Link the object file to create a plugin
            # Note: This is simplified - real linking would need proper flags
            gcc_path = shutil.which("gcc")
            if not gcc_path:
                raise FileNotFoundError("gcc not found in PATH")

            _run_trusted_build_tool(
                "gcc",
                [
                    gcc_path,
                    "-shared",
                    "-fPIC",
                    "-o",
                    str(plugin_file),
                    str(obj_file),
                    f"-L{runtime_dir}",
                    "-lselene_simple_runtime",
                    "-Wl,-rpath," + str(runtime_dir),
                ],
                text=True,
                check=True,
            )
        except (ImportError, FileNotFoundError):
            # Fallback: Create a simple shared library without runtime linking
            logger.warning("Selene runtime not found, creating standalone plugin")
            gcc_path = shutil.which("gcc")
            if not gcc_path:
                raise FileNotFoundError("gcc not found in PATH") from None

            _run_trusted_build_tool(
                "gcc",
                [gcc_path, "-shared", "-fPIC", "-o", str(plugin_file), str(obj_file)],
                text=True,
                check=True,
            )
        except subprocess.CalledProcessError as e:
            raise RuntimeError(f"Failed to link plugin: {e.stderr}") from e

        # Read the compiled plugin
        return plugin_file.read_bytes()


def create_selene_interface_program(program: Callable | bytes | str):
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
            from pecos_rslib._pecos_rslib import (
                PySeleneInterfaceProgram as SeleneInterfaceProgram,
            )
        except ImportError as e:
            raise ImportError(
                "SeleneInterfaceProgram not available in pecos_rslib",
            ) from e

    # Determine input type and compile as needed
    if callable(program):
        # It's a Guppy function
        plugin_bytes = compile_guppy_to_selene_plugin(program)
    elif isinstance(program, bytes):
        # Could be HUGR bytes or plugin bytes
        # Check if it's an ELF file (compiled plugin)
        if program.startswith(b"\x7fELF"):
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
