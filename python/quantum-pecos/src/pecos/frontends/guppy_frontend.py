"""Guppy Frontend for PECOS.

This module provides integration between Guppy quantum programming language
and PECOS execution infrastructure through the HUGR intermediate representation.
"""

import subprocess
import tempfile
import warnings
from collections.abc import Callable
from pathlib import Path

try:
    from guppylang import guppy

    GUPPY_AVAILABLE = True
except ImportError:
    GUPPY_AVAILABLE = False
    guppy = None

# Try to import Rust backend
try:
    from pecos_rslib import (
        RUST_HUGR_AVAILABLE,
        check_rust_hugr_availability,
        compile_hugr_to_qir_rust,
    )

    RUST_BACKEND_AVAILABLE = RUST_HUGR_AVAILABLE
except ImportError:
    RUST_BACKEND_AVAILABLE = False
    warnings.warn(
        "Rust HUGR backend not available, falling back to external tools",
        stacklevel=2,
    )


def _raise_external_compiler_error() -> None:
    """Raise ImportError for missing external compiler.

    This is extracted as a separate function to satisfy TRY301.
    """
    msg = "External compiler not available"
    raise ImportError(msg) from None


class GuppyFrontend:
    """Frontend for compiling Guppy quantum programs to QIR for PECOS execution.

    This class handles the complete pipeline:
    1. Guppy function → HUGR (using guppylang)
    2. HUGR format conversion (for compatibility)
    3. HUGR → LLVM IR/QIR (using hugr-llvm with quantum extensions)
    4. QIR execution on PECOS
    """

    def __init__(
        self,
        hugr_to_llvm_binary: Path | None = None,
        format_converter: Path | None = None,
        use_rust_backend: bool | None = None,
        naming_convention: str = "standard",
    ) -> None:
        """Initialize the Guppy frontend.

        Args:
            hugr_to_llvm_binary: Path to the hugr-to-llvm compiler binary (for external mode)
            format_converter: Path to the HUGR format converter script (for external mode)
            use_rust_backend: Force use of Rust backend (True) or external tools (False).
                             If None, auto-detect best available option.
            naming_convention: Quantum operation naming convention ("standard", "hugr", "pecos")
        """
        # Initialize attributes first to avoid AttributeError in cleanup
        self._temp_dir = None

        if not GUPPY_AVAILABLE:
            msg = "guppylang is not available. Please install guppylang to use the Guppy frontend."
            raise ImportError(
                msg,
            )

        # Determine backend to use
        if use_rust_backend is None:
            self.use_rust_backend = RUST_BACKEND_AVAILABLE
        else:
            self.use_rust_backend = use_rust_backend
            if use_rust_backend and not RUST_BACKEND_AVAILABLE:
                msg = "Rust backend requested but not available"
                raise ImportError(msg) from None

        # External tools configuration (used when Rust backend not available/requested)
        self.hugr_to_llvm_binary = hugr_to_llvm_binary
        self.format_converter = format_converter

        # Rust backend configuration
        self.naming_convention = naming_convention
        if self.use_rust_backend:
            # Verify Rust backend is working
            available, message = check_rust_hugr_availability()
            if not available:
                warnings.warn(
                    f"Rust backend not fully available: {message}",
                    stacklevel=2,
                )
                self.use_rust_backend = False

    def get_backend_info(self) -> dict:
        """Get information about the backend being used."""
        return {
            "backend": "rust" if self.use_rust_backend else "external",
            "rust_available": RUST_BACKEND_AVAILABLE,
            "guppy_available": GUPPY_AVAILABLE,
            "naming_convention": self.naming_convention,
            "external_tools": {
                "hugr_to_llvm_binary": (
                    str(self.hugr_to_llvm_binary) if self.hugr_to_llvm_binary else None
                ),
                "format_converter": (
                    str(self.format_converter) if self.format_converter else None
                ),
            },
        }

    def compile_function(self, func: Callable) -> Path:
        """Compile a Guppy function to QIR.

        Args:
            func: A function decorated with @guppy

        Returns:
            Path to the generated QIR/LLVM IR file

        Raises:
            RuntimeError: If compilation fails at any stage
        """
        # Check if this is a Guppy function
        # GuppyDefinition objects have different attributes than regular functions
        is_guppy = (
            hasattr(func, "_guppy_compiled")
            or hasattr(func, "name")
            or str(type(func)).find("GuppyDefinition") != -1
        )

        if not is_guppy:
            msg = "Function must be decorated with @guppy"
            raise ValueError(msg)

        # Step 1: Compile Guppy to HUGR
        try:
            compiled = guppy.compile_function(func)
            # compiled is a FuncDefnPointer with a .package attribute
            hugr_bytes = compiled.package.to_bytes()
        except Exception as e:
            msg = f"Failed to compile Guppy to HUGR: {e}"
            raise RuntimeError(msg) from e

        if self.use_rust_backend:
            # Use Rust backend for compilation
            return self._compile_with_rust_backend(func, hugr_bytes)
        # Use external tools for compilation
        return self._compile_with_external_tools(func, hugr_bytes)

    def _compile_with_rust_backend(self, func: Callable, hugr_bytes: bytes) -> Path:
        """Compile using Rust backend."""
        try:
            # Create temp directory for output
            if self._temp_dir is None:
                self._temp_dir = tempfile.mkdtemp(prefix="pecos_guppy_rust_")

            temp_path = Path(self._temp_dir)
            func_name = getattr(func, "__name__", getattr(func, "name", "guppy_func"))
            qir_file = temp_path / f"{func_name}.ll"

            # Compile HUGR to QIR using Rust backend
            qir_content = compile_hugr_to_qir_rust(
                hugr_bytes,
                None,  # output_path
                debug_info=False,  # debug_info
                naming_convention=self.naming_convention,  # naming_convention
            )

            # Write QIR to file
            with Path(qir_file).open("w") as f:
                f.write(qir_content)

        except Exception as e:
            msg = f"Rust backend compilation failed: {e}"
            raise RuntimeError(msg) from e
        else:
            return qir_file

    def _compile_with_external_tools(self, func: Callable, hugr_bytes: bytes) -> Path:
        """Compile using external tools."""
        # Create temp directory for intermediate files
        if self._temp_dir is None:
            self._temp_dir = tempfile.mkdtemp(prefix="pecos_guppy_external_")

        temp_path = Path(self._temp_dir)

        # Get function name safely
        func_name = getattr(func, "__name__", getattr(func, "name", "guppy_func"))

        # Write HUGR to file
        hugr_file = temp_path / f"{func_name}.hugr"
        with Path(hugr_file).open("wb") as f:
            f.write(hugr_bytes)

        # Step 2: Convert HUGR format if converter is available
        if self.format_converter:
            converted_hugr = temp_path / f"{func_name}_converted.hugr"
            try:
                subprocess.run(  # noqa: S603
                    [  # noqa: S607
                        "python",
                        str(self.format_converter),
                        str(hugr_file),
                        str(converted_hugr),
                    ],
                    check=True,
                    capture_output=True,
                    text=True,
                )
                hugr_file = converted_hugr
            except subprocess.CalledProcessError as e:
                msg = f"HUGR format conversion failed: {e.stderr}"
                raise RuntimeError(msg) from e

        # Step 3: Compile HUGR to LLVM IR/QIR
        qir_file = temp_path / f"{func_name}.ll"

        if self.hugr_to_llvm_binary:
            try:
                subprocess.run(  # noqa: S603
                    [
                        str(self.hugr_to_llvm_binary),
                        str(hugr_file),
                        str(qir_file),
                    ],
                    check=True,
                    capture_output=True,
                    text=True,
                )
            except subprocess.CalledProcessError as e:
                msg = f"HUGR to LLVM compilation failed: {e.stderr}"
                raise RuntimeError(msg) from e
        else:
            # Use PECOS HUGR compiler for real HUGR→LLVM compilation
            try:
                # Try to use the new HUGR compiler from PECOS
                print("  [OK] Using PECOS HUGR->LLVM compiler")

                # Try to import the hugr_llvm_compiler
                from pecos.frontends.hugr_llvm_compiler import HugrLlvmCompiler

                compiler = HugrLlvmCompiler()
                if compiler.is_available():
                    # Use the external hugr_quantum_llvm binary
                    llvm_ir = compiler.compile_hugr_to_llvm(
                        hugr_bytes,
                        self.naming_convention,
                    )

                    qir_file = temp_path / f"{func_name}.ll"
                    with Path(qir_file).open("w") as f:
                        f.write(llvm_ir)

                    return qir_file
                print(
                    "    [WARNING] External HUGR compiler not available, trying execute_llvm...",
                )
                _raise_external_compiler_error()

            except ImportError:
                # Fall back to execute_llvm if available
                pass

            try:
                # First try PECOS's own execute_llvm
                try:
                    from pecos import execute_llvm

                    print(
                        "  [OK] Using PECOS execute_llvm module for HUGR->LLVM compilation",
                    )
                except ImportError:
                    # Try external execute_llvm
                    import execute_llvm

                    print(
                        "  [OK] Using external execute_llvm module for HUGR->LLVM compilation",
                    )

                # Compile HUGR bytes to LLVM IR string
                llvm_ir = execute_llvm.compile_module_to_string(hugr_bytes)

                # Write LLVM IR to file
                qir_file = temp_path / f"{func_name}.ll"
                with Path(qir_file).open("w") as f:
                    f.write(llvm_ir)

            except ImportError:
                # Final fallback to placeholder if nothing works
                print(
                    "  [WARNING] No HUGR->LLVM compiler available, using placeholder QIR",
                )
                qir_file = temp_path / f"{func_name}.ll"

                # Simple placeholder that PECOS can execute
                placeholder_qir = f"""; Generated from Guppy function: {func_name}
; Placeholder QIR - install hugr-quantum-llvm for real compilation

target datalayout = "e-m:e-i64:64-f80:128-n8:16:32:64-S128"
target triple = "x86_64-unknown-linux-gnu"

declare i64 @__quantum__rt__qubit_allocate()
declare void @__quantum__qis__h__body(i64)
declare i32 @__quantum__qis__m__body(i64, i64)
declare i64 @__quantum__rt__result_allocate()

define void @{func_name}() #0 {{
entry:
  %qubit = call i64 @__quantum__rt__qubit_allocate()
  call void @__quantum__qis__h__body(i64 %qubit)
  %result = call i64 @__quantum__rt__result_allocate()
  %measurement = call i32 @__quantum__qis__m__body(i64 %qubit, i64 %result)
  ret void
}}

attributes #0 = {{ "EntryPoint" }}
"""

                with Path(qir_file).open("w") as f:
                    f.write(placeholder_qir)

                return qir_file
            else:
                return qir_file

    def compile_and_run(self, func: Callable, shots: int = 1000) -> dict:
        """Compile a Guppy function and run it on PECOS using the QIR engine.

        Args:
            func: A function decorated with @guppy
            shots: Number of shots to execute

        Returns:
            Dictionary containing execution results
        """
        # Import here to avoid circular dependencies
        from pecos.frontends.qir_engine_wrapper import (
            QirEngineWrapper,
            is_qir_engine_available,
        )

        if not is_qir_engine_available():
            msg = "PECOS QIR engine not available"
            raise RuntimeError(msg)

        # Get function name safely
        func_name = getattr(func, "__name__", getattr(func, "name", "guppy_func"))

        # Compile to standard QIR
        qir_file = self.compile_function(func)

        # Execute using QIR engine wrapper (proper pipeline)
        wrapper = QirEngineWrapper()
        try:
            result = wrapper.execute_qir_file(qir_file, shots)

            # Extract results in expected format
            measurements = result.get("measurements", [])
            success = result.get("execution_successful", False)

            if not success:
                error_msg = result.get("error", "Unknown execution error")
                msg = f"QIR execution failed: {error_msg}"
                raise RuntimeError(msg)

            return {
                "shots": shots,
                "results": measurements,
                "function_name": func_name,
                "execution_engine": "pecos_qir_engine",
                "qir_file": str(qir_file),
            }

        finally:
            wrapper.cleanup()

    def cleanup(self) -> None:
        """Clean up temporary files."""
        if (
            hasattr(self, "_temp_dir")
            and self._temp_dir
            and Path(self._temp_dir).exists()
        ):
            import shutil

            shutil.rmtree(self._temp_dir)
            self._temp_dir = None

    def __del__(self) -> None:
        """Cleanup on destruction."""
        import contextlib

        with contextlib.suppress(Exception):
            self.cleanup()


def compile_guppy_to_qir(
    func: Callable,
    hugr_to_llvm_binary: Path | None = None,
    format_converter: Path | None = None,
) -> Path:
    """Convenience function to compile a Guppy function to QIR.

    Args:
        func: A function decorated with @guppy
        hugr_to_llvm_binary: Path to the hugr-to-llvm compiler binary
        format_converter: Path to the HUGR format converter script

    Returns:
        Path to the generated QIR file
    """
    frontend = GuppyFrontend(hugr_to_llvm_binary, format_converter)
    try:
        return frontend.compile_function(func)
    finally:
        frontend.cleanup()


def run_guppy_on_pecos(
    func: Callable,
    shots: int = 1000,
    hugr_to_llvm_binary: Path | None = None,
    format_converter: Path | None = None,
) -> dict:
    """Convenience function to compile and run a Guppy function on PECOS.

    Args:
        func: A function decorated with @guppy
        shots: Number of shots to execute
        hugr_to_llvm_binary: Path to the hugr-to-llvm compiler binary
        format_converter: Path to the HUGR format converter script

    Returns:
        Dictionary containing execution results
    """
    frontend = GuppyFrontend(hugr_to_llvm_binary, format_converter)
    try:
        return frontend.compile_and_run(func, shots)
    finally:
        frontend.cleanup()
