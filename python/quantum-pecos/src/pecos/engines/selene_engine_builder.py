"""Builder for Selene engine that compiles Guppy to shared libraries.

This module handles:
1. Detecting Guppy programs
2. Compiling to HUGR via guppylang
3. Using Selene to build shared libraries
4. Passing library path to SeleneLibraryEngine
"""

import logging
import shutil
import tempfile
from pathlib import Path
from typing import Any

logger = logging.getLogger(__name__)

# Check for required dependencies
try:
    from guppylang import GuppyModule

    GUPPY_AVAILABLE = True
except ImportError:
    GUPPY_AVAILABLE = False
    logger.warning("guppylang not available - Guppy support disabled")

try:
    import selene_sim
    from selene_sim import SeleneInstance, build
    from selene_sim.backends import IdealErrorModel, SimpleRuntime

    SELENE_AVAILABLE = True
except ImportError:
    SELENE_AVAILABLE = False
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
        def __init__(self, *args, **kwargs) -> None:
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

    def with_guppy_program(self, program: Any) -> "SeleneEngineBuilder":
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

    def with_hugr_program(self, program: Any) -> "SeleneEngineBuilder":
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

    def with_verbose(self, verbose: bool = True) -> "SeleneEngineBuilder":
        """Enable verbose output during compilation.

        Args:
            verbose: Whether to show detailed compilation output

        Returns:
            Self for chaining
        """
        self.verbose = verbose
        return self

    def _compile_to_hugr(self) -> bytes:
        """Compile Guppy program to HUGR.

        Returns:
            HUGR bytes

        Raises:
            RuntimeError: If compilation fails
        """
        if self.guppy_program is None:
            msg = "No Guppy program provided"
            raise ValueError(msg)

        if not COMPILATION_AVAILABLE:
            msg = "Compilation pipeline not available"
            raise ImportError(msg)

        logger.info("Compiling Guppy program to HUGR...")

        try:
            hugr_bytes = compile_guppy_to_hugr(self.guppy_program)
            logger.info(f"Compiled to HUGR: {len(hugr_bytes)} bytes")
            return hugr_bytes
        except Exception as e:
            msg = f"Failed to compile Guppy to HUGR: {e}"
            raise RuntimeError(msg)

    def _build_shared_library(self, hugr_bytes: bytes) -> Path:
        """Use Selene to build a shared library from HUGR.

        Args:
            hugr_bytes: HUGR program bytes

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

        logger.info(f"Building Selene library in {self.build_dir}")

        # Save HUGR to file
        hugr_file = self.build_dir / "program.hugr"
        hugr_file.write_bytes(hugr_bytes)

        # Create HUGR artifact for Selene
        from selene_core.build_utils.builtins.hugr import HUGRPackageKind
        from selene_core.build_utils.types import Artifact

        hugr_artifact = Artifact(
            kind=HUGRPackageKind,
            resource=hugr_bytes,
            metadata={},
        )

        try:
            # Build with Selene to create an executable
            instance = build(
                src=hugr_artifact,
                name="quantum_program",
                build_dir=self.build_dir,
                interface=selene_sim.interfaces.HeliosInterface(),  # QIS interface
                verbose=self.verbose,
            )

            # Find the built library
            lib_patterns = ["*.so", "*.dylib", "*.dll"]
            for pattern in lib_patterns:
                libs = list(self.build_dir.glob(pattern))
                if libs:
                    library_path = libs[0]
                    logger.info(f"Built shared library: {library_path}")
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

        except Exception as e:
            msg = f"Failed to build Selene library: {e}"
            raise RuntimeError(msg)

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
        import platform

        if platform.system() == "Linux":
            # On Linux, ensure it has the right ELF type
            # This is a simplified approach - might need more work
            import subprocess

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
            except:
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
        if self.guppy_program is not None and not self.hugr_bytes:
            self.hugr_bytes = self._compile_to_hugr()

        # Build shared library from HUGR
        if self.hugr_bytes is not None and self.library_path is None:
            self.library_path = self._build_shared_library(self.hugr_bytes)

        if not self.library_path:
            msg = "No library path - compilation failed"
            raise RuntimeError(msg)

        # Create the Rust engine
        logger.info(f"Creating SeleneLibraryEngine with library at {self.library_path}")

        return RustSeleneEngine(
            library_path=str(self.library_path),
            num_qubits=self.num_qubits,
        )

    def cleanup(self) -> None:
        """Clean up temporary build files."""
        if self.build_dir and self.build_dir.exists():
            try:
                shutil.rmtree(self.build_dir)
                logger.info(f"Cleaned up build directory: {self.build_dir}")
            except Exception as e:
                logger.warning(f"Failed to clean up build directory: {e}")


def selene_engine_from_guppy(program: Any, num_qubits: int) -> RustSeleneEngine:
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
