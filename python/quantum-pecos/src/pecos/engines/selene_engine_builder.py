"""Builder for Selene engine that compiles Guppy to shared libraries.

This module handles:
1. Detecting Guppy programs
2. Compiling to HUGR via guppylang
3. Using Selene to build shared libraries
4. Passing library path to SeleneLibraryEngine
"""

import importlib.util
import logging
import platform
import shutil
import subprocess
import tempfile
from pathlib import Path
from typing import TYPE_CHECKING

from pecos.protocols import GuppyCallable

if TYPE_CHECKING:
    from pecos.programs import HugrProgram


logger = logging.getLogger(__name__)

# Check for required dependencies
GUPPY_AVAILABLE = importlib.util.find_spec("guppylang") is not None
if not GUPPY_AVAILABLE:
    logger.warning("guppylang not available - Guppy support disabled")

SELENE_AVAILABLE = importlib.util.find_spec("selene_sim") is not None
if SELENE_AVAILABLE:
    try:
        import selene_sim
        from selene_sim import build
    except ImportError:
        SELENE_AVAILABLE = False
        logger.warning("selene_sim import failed - Selene compilation disabled")
else:
    logger.warning("selene_sim not available - Selene compilation disabled")

try:
    from pecos.compilation_pipeline import compile_guppy_to_hugr

    COMPILATION_AVAILABLE = True
except ImportError:
    COMPILATION_AVAILABLE = False
    logger.warning("Compilation pipeline not available")

# Import the Rust engine wrapper
try:
    from pecos_rslib import SeleneLibraryEngine as RustSeleneEngine
except ImportError:
    # Provide a stub if not available
    class RustSeleneEngine:
        """Stub class for when SeleneLibraryEngine is not available."""

        def __init__(self, *_args: object, **_kwargs: object) -> None:
            """Raise ImportError as the real engine is not available."""
            msg = "SeleneLibraryEngine not available in pecos_rslib"
            raise ImportError(msg)


class SeleneEngineBuilder:
    """Builder for creating Selene engines from Guppy programs."""

    def __init__(self, num_qubits: int) -> None:
        """Initialize the builder.

        Args:
            num_qubits: Number of qubits for the quantum system
        """
        self.num_qubits = num_qubits
        self.guppy_program = None
        self.hugr_bytes = None
        self.library_path = None
        self.build_dir = None
        self.verbose = False

    def with_guppy_program(self, program: GuppyCallable) -> "SeleneEngineBuilder":
        """Set the Guppy program to compile.

        Args:
            program: A Guppy-decorated function or module

        Returns:
            Self for chaining
        """
        if not GUPPY_AVAILABLE:
            msg = "guppylang is required for Guppy programs"
            raise ImportError(msg)

        self.guppy_program = program
        return self

    def with_hugr_program(self, program: "HugrProgram") -> "SeleneEngineBuilder":
        """Set HUGR program directly.

        Args:
            program: A HugrProgram object with HUGR bytes

        Returns:
            Self for chaining
        """
        # Extract HUGR bytes from the program
        if hasattr(program, "inner"):
            # It's a PyHugrProgram from Rust
            # We need to extract the bytes somehow
            # For now, assume it has a method to get bytes
            self.hugr_bytes = bytes(program.inner)  # This might need adjustment
        elif isinstance(program, bytes):
            self.hugr_bytes = program
        else:
            msg = f"Unsupported HUGR program type: {type(program)}"
            raise ValueError(msg)

        return self

    def with_verbose(self, *, verbose: bool = True) -> "SeleneEngineBuilder":
        """Enable verbose output during compilation.

        Args:
            verbose: Whether to show detailed compilation output

        Returns:
            Self for chaining
        """
        self.verbose = verbose
        return self

    def _compile_to_hugr(self) -> dict | bytes:
        """Compile Guppy program to HUGR.

        Returns:
            HUGR Package or bytes

        Raises:
            RuntimeError: If compilation fails
        """
        if self.guppy_program is None:
            msg = "No Guppy program provided"
            raise ValueError(msg)

        logger.info("Compiling Guppy program to HUGR...")

        try:
            # Following Selene's approach: Entry points must have no parameters.
            # Guppy's compile() method will raise an error if the function has parameters,
            # matching Selene's behavior. We just need to pass through that error with
            # a clear message.

            # Try to compile - this will fail for parametric functions
            if hasattr(self.guppy_program, "compile"):
                package = self.guppy_program.compile()
                logger.info("Compiled to HUGR Package: %s", package)
                return package

            # Fallback to bytes if needed
            if COMPILATION_AVAILABLE:
                hugr_bytes = compile_guppy_to_hugr(self.guppy_program)
                logger.info("Compiled to HUGR: %s bytes", len(hugr_bytes))
                return hugr_bytes

            msg = "Could not compile Guppy program"
            raise RuntimeError(msg)
        except Exception as e:
            # Check if this is a parametric function error from Guppy
            error_str = str(e)
            error_type = type(e).__name__
            if "EntrypointArgsError" in error_str or "EntrypointArgsError" in error_type:
                # Re-raise with a message matching Selene's approach
                # Extract the number of args if possible
                import re
                args_match = re.search(r"args=\[([^\]]*)\]", error_str)
                if args_match:
                    args = args_match.group(1).replace("'", "").split(", ")
                    num_args = len([a for a in args if a])  # Count non-empty args
                    msg = (
                        f"Entry point function must have no input parameters (found {num_args}). "
                        "Following Selene's approach, parametric functions should be called from a "
                        "parameter-less main() function."
                    )
                else:
                    msg = (
                        "Entry point function must have no input parameters. "
                        "Following Selene's approach, parametric functions should be called from a "
                        "parameter-less main() function."
                    )
                raise ValueError(msg) from e
            # For other errors, preserve the original
            msg = f"Failed to compile Guppy to HUGR: {e}"
            raise RuntimeError(msg) from e

    def _build_shared_library(self, hugr_input: dict | bytes) -> Path:
        """Use Selene to build a shared library from HUGR.

        Args:
            hugr_input: HUGR Package or bytes

        Returns:
            Path to the built shared library

        Raises:
            RuntimeError: If build fails
        """
        if not SELENE_AVAILABLE:
            msg = "selene_sim is required for building"
            raise ImportError(msg)

        # Create build directory
        if self.build_dir is None:
            self.build_dir = Path(tempfile.mkdtemp(prefix="selene_build_"))

        logger.info("Building Selene library in %s", self.build_dir)

        try:
            # Build with Selene - it accepts Package objects directly
            if (
                hasattr(hugr_input, "__class__")
                and hugr_input.__class__.__name__ == "Package"
            ):
                # It's a Package object, use directly
                instance = build(
                    src=hugr_input,
                    name="quantum_program",
                    build_dir=self.build_dir,
                    interface=selene_sim.HeliosInterface(),  # QIS interface
                    verbose=self.verbose,
                )
            else:
                # It's bytes, save to file first
                hugr_file = self.build_dir / "program.hugr"
                hugr_file.write_bytes(hugr_input)

                instance = build(
                    src=str(hugr_file),
                    name="quantum_program",
                    build_dir=self.build_dir,
                    interface=selene_sim.HeliosInterface(),  # QIS interface
                    verbose=self.verbose,
                )

            # Find the built library
            lib_patterns = ["*.so", "*.dylib", "*.dll"]
            for pattern in lib_patterns:
                libs = list(self.build_dir.glob(pattern))
                if libs:
                    library_path = libs[0]
                    logger.info("Built shared library: %s", library_path)
                    return library_path

            # If Selene created an executable instead, we need to modify it
            # to be loadable as a library
            executable = instance.executable
            if executable.exists():
                logger.warning(
                    "Selene built executable instead of library, attempting conversion...",
                )
                return self._convert_executable_to_library(executable)

            msg = "No library found after build"
            raise RuntimeError(msg)

        except (OSError, RuntimeError, ValueError) as e:
            msg = f"Failed to build Selene library: {e}"
            raise RuntimeError(msg) from e

    def _convert_executable_to_library(self, executable: Path) -> Path:
        """Convert a Selene executable to a shared library.

        This is a fallback if Selene doesn't directly support library output.

        Args:
            executable: Path to the Selene executable

        Returns:
            Path to the converted library
        """
        # On Unix systems, executables and shared libraries are similar
        # We may just need to rename and ensure proper linking
        library_path = executable.with_suffix(".so")

        if executable != library_path:
            shutil.copy2(executable, library_path)

        # Make sure it's marked as a shared library
        # This might require platform-specific handling
        if platform.system() == "Linux":
            # On Linux, ensure it has the right ELF type
            # This is a simplified approach - might need more work

            try:
                # Check if it's already a shared object
                result = subprocess.run(
                    ["file", str(library_path)],
                    check=False,
                    capture_output=True,
                    text=True,
                )
                if "shared object" not in result.stdout:
                    logger.warning("File is not a shared object, may not load properly")
            except (OSError, subprocess.SubprocessError):
                pass

        return library_path

    def build(self) -> RustSeleneEngine:
        """Build the Selene engine.

        Returns:
            A configured SeleneLibraryEngine ready for use

        Raises:
            RuntimeError: If build fails
        """
        # Compile Guppy to HUGR
        hugr_data = None
        if self.guppy_program is not None:
            hugr_data = self._compile_to_hugr()
        elif self.hugr_bytes is not None:
            hugr_data = self.hugr_bytes

        # Build shared library from HUGR
        if hugr_data is not None and self.library_path is None:
            self.library_path = self._build_shared_library(hugr_data)

        if not self.library_path:
            msg = "No library path - compilation failed"
            raise RuntimeError(msg)

        # Create the Rust engine
        logger.info(
            "Creating SeleneLibraryEngine with library at %s",
            self.library_path,
        )

        return RustSeleneEngine(
            library_path=str(self.library_path),
            num_qubits=self.num_qubits,
        )

    def cleanup(self) -> None:
        """Clean up temporary build files."""
        if self.build_dir and self.build_dir.exists():
            try:
                shutil.rmtree(self.build_dir)
                logger.info("Cleaned up build directory: %s", self.build_dir)
            except (OSError, PermissionError) as e:
                logger.warning("Failed to clean up build directory: %s", e)


def selene_engine_from_guppy(
    program: GuppyCallable,
    num_qubits: int,
) -> RustSeleneEngine:
    """Convenience function to create a Selene engine from a Guppy program.

    Args:
        program: A Guppy-decorated function
        num_qubits: Number of qubits

    Returns:
        A configured SeleneLibraryEngine

    Example:
        >>> @guppy
        ... def bell_state() -> tuple[bool, bool]:
        ...     q0, q1 = qubit(), qubit()
        ...     h(q0)
        ...     cx(q0, q1)
        ...     return measure(q0), measure(q1)
        ...
        >>>
        >>> engine = selene_engine_from_guppy(bell_state, 2)
    """
    builder = SeleneEngineBuilder(num_qubits)
    return builder.with_guppy_program(program).build()
