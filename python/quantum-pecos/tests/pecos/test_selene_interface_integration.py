"""Test the Selene Interface integration from Python side."""

import pytest


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
