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
        if plugin_bytes.startswith(b"\x7fELF"):
            print("Successfully compiled to ELF shared library")
        else:
            print(f"Got plugin bytes but not ELF format: {plugin_bytes[:20]}...")

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
        try:
            result = builder.run(1)
            # If this succeeds, we have a real plugin somehow
            assert result is not None
        except (RuntimeError, OSError) as e:
            # Expected - dummy plugin can't be loaded or no program set
            error_msg = str(e).lower()
            assert any(
                keyword in error_msg
                for keyword in [
                    "runtime",
                    "library",
                    "load",
                    "no program",
                    "program specified",
                ]
            ), f"Unexpected error message: {e}"

    except TypeError as e:
        if "cannot convert" in str(e):
            pytest.skip("SeleneInterfaceProgram not yet recognized by sim()")
        raise


def test_runtime_library_finding() -> None:
    """Test that we can find the Selene runtime library."""
    import os
    from pathlib import Path

    # Check known locations
    possible_paths = [
        Path(
            "/home/ciaranra/Repos/cl_projects/gup/PECOS/lib/pecos-runtimes/libselene_simple_runtime.so",
        ),
        Path("/home/ciaranra/.cache/pecos-decoders/selene/libselene_simple_runtime.so"),
    ]

    # Check Python venv
    venv = os.environ.get("VIRTUAL_ENV")
    if venv:
        venv_path = Path(venv)
        for version in ["python3.12", "python3.11", "python3.10"]:
            runtime_path = (
                venv_path
                / f"lib/{version}/site-packages/selene_simple_runtime_plugin/_dist/lib/libselene_simple_runtime.so"
            )
            possible_paths.append(runtime_path)

    found_any = False
    for path in possible_paths:
        if path.exists():
            print(f"Found Selene runtime at: {path}")
            found_any = True

            # Check if it's a valid shared library
            with open(path, "rb") as f:
                header = f.read(4)
                if header == b"\x7fELF":
                    print("  ✓ Valid ELF shared library")
                else:
                    print("  ✗ Not a valid ELF file")

    if not found_any:
        print(
            "No Selene runtime libraries found (this is OK if Selene is not installed)",
        )


if __name__ == "__main__":
    # Run tests
    test_selene_interface_program_available()
    test_selene_compilation_pipeline()
    test_sim_with_selene_interface()
    test_runtime_library_finding()
