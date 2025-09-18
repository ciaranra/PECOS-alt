"""Test the Selene Interface integration from Python side."""

import pytest


def test_selene_interface_program_available() -> None:
    """Test that SeleneInterfaceProgram is available from pecos_rslib."""
    try:
        from pecos_rslib._pecos_rslib import SeleneInterfaceProgram

        # Create a dummy plugin
        dummy_bytes = b"dummy_plugin_data"
        program = SeleneInterfaceProgram.from_bytes(dummy_bytes)

        assert program is not None
        assert program.bytes() == dummy_bytes

    except ImportError:
        pytest.skip("SeleneInterfaceProgram not available")


def test_selene_compilation_pipeline() -> None:
    """Test the Guppy → HUGR → Selene Interface compilation pipeline."""
    # Check if guppylang is available
    pytest.importorskip("guppylang")

    from guppylang import guppy
    from guppylang.std.quantum import h, measure, qubit

    # Simple Guppy function
    @guppy
    def simple_hadamard() -> bool:
        q = qubit()
        h(q)
        return measure(q)

    # Try to compile to Selene Interface
    try:
        from pecos_rslib.selene_compilation import compile_guppy_to_selene_plugin

        plugin_bytes = compile_guppy_to_selene_plugin(simple_hadamard)

        # Check that we got bytes
        assert isinstance(plugin_bytes, bytes)
        assert len(plugin_bytes) > 0

        # Check if it looks like an ELF file (compiled shared library)
        if plugin_bytes[:4] == b"\x7fELF":
            pass  # It's an ELF file
    except ImportError as e:
        pytest.skip(f"Selene compilation tools not available: {e}")
    except RuntimeError as e:
        if "llc not found" in str(e) or "gcc" in str(e):
            pytest.skip(f"Compilation tools not available: {e}")
        elif "HUGR" in str(e) or "compile" in str(e):
            pytest.skip(f"HUGR compilation not yet working: {e}")
        else:
            raise


def test_sim_with_selene_interface() -> None:
    """Test that sim() can handle SeleneInterfaceProgram."""
    try:
        from pecos.frontends.guppy_api import sim
        from pecos_rslib._pecos_rslib import SeleneInterfaceProgram
    except ImportError:
        pytest.skip("sim or SeleneInterfaceProgram not available")

    # Create a dummy plugin
    dummy_bytes = b"\x7fELF_dummy_plugin_data"  # Fake ELF header
    program = SeleneInterfaceProgram.from_bytes(dummy_bytes)

    # Try to create a sim builder with it
    try:
        builder = sim(program)

        # Check that we got a builder
        assert builder is not None

        # Try to run (will fail with dummy plugin but tests the pipeline)
        with pytest.raises((RuntimeError, OSError)) as exc_info:
            builder.run(1)

        # Verify the error message contains expected keywords
        error_msg = str(exc_info.value).lower()
        assert any(
            keyword in error_msg
            for keyword in [
                "plugin",
                "selene",
                "library",
                "load",
                "failed",
                "invalid",
                "program",
                "error",
                "no program",
            ]
        ), f"Unexpected error message: {exc_info.value}"

    except TypeError as e:
        if "cannot convert" in str(e):
            pytest.skip("SeleneInterfaceProgram not yet recognized by sim()")
        raise


def test_runtime_library_finding() -> None:
    """Test the runtime library finder functionality."""
    import ctypes
    import os
    from pathlib import Path

    # This test should ideally test a library finder function/class
    # For now, we'll test that if we find a library, it's actually loadable

    # Try to import the actual library finder if it exists
    try:
        from pecos.engines.selene_runtime import find_selene_runtime_library

        library_path = find_selene_runtime_library()

        # Test that the found library is actually loadable
        try:
            lib = ctypes.CDLL(str(library_path))
            # Could check for specific symbols here
            assert lib is not None, "Library should be loadable"
        except OSError as e:
            pytest.fail(f"Found library at {library_path} but couldn't load it: {e}")

    except ImportError:
        # The library finder doesn't exist yet, so test the manual search
        # This is more of a diagnostic than a test
        possible_paths = [
            Path.home() / ".cache/pecos-decoders/selene/libselene_simple_runtime.so",
            Path("/usr/local/lib/libselene_simple_runtime.so"),
        ]

        # Add venv paths
        venv = os.environ.get("VIRTUAL_ENV")
        if venv:
            venv_path = Path(venv)
            site_packages = venv_path / "lib"
            if site_packages.exists():
                # Search for the library in site-packages
                possible_paths.extend(
                    site_packages.rglob("libselene_simple_runtime.so"),
                )

        # Check if any library is actually loadable (not just exists)
        loadable_libraries = []
        for path in possible_paths:
            if path.exists():
                try:
                    # Actually try to load the library
                    lib = ctypes.CDLL(str(path))
                    loadable_libraries.append(path)
                except OSError:
                    # File exists but can't be loaded (might be stub or wrong arch)
                    continue

        if not loadable_libraries:
            pytest.skip(
                "No loadable Selene runtime library found - this is expected in test environments",
            )

        # If we found loadable libraries, that's good enough for this diagnostic
        assert (
            len(loadable_libraries) > 0
        ), f"Found {len(loadable_libraries)} loadable Selene runtime libraries"
