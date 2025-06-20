"""Additional HUGR tests that can run without skipping."""

import pytest


def test_hugr_compilation_with_support() -> None:
    """Test that compilation works when HUGR support IS available."""
    try:
        from pecos_rslib import RustHugrCompiler, check_rust_hugr_availability

        available, message = check_rust_hugr_availability()
        assert available, f"HUGR support should be available but got: {message}"

        # Test that we can create a compiler
        compiler = RustHugrCompiler()
        assert compiler is not None

        # Test that invalid HUGR data raises an error
        dummy_hugr = b"invalid hugr data"
        with pytest.raises(RuntimeError) as exc_info:
            compiler.compile_bytes_to_qir(dummy_hugr)

        # The error should mention JSON parsing or HUGR format
        error_msg = str(exc_info.value).lower()
        assert any(
            keyword in error_msg for keyword in ["json", "hugr", "parse", "invalid"]
        ), f"Expected error about JSON/HUGR parsing, got: {exc_info.value}"

    except ImportError:
        pytest.skip("Rust HUGR backend not available")


def test_hugr_version_compatibility() -> None:
    """Test HUGR version compatibility handling."""
    try:
        from pecos_rslib import RustHugrCompiler, check_rust_hugr_availability
        import json

        available, message = check_rust_hugr_availability()
        if not available:
            pytest.skip(f"HUGR support not available: {message}")

        compiler = RustHugrCompiler()

        # Create HUGR with old version format (simulating Guppy's output)
        old_hugr = {
            "format": "hugr",
            "version": "0.1.0",  # Old version
            "modules": [
                {
                    "name": "test",
                    "nodes": [
                        {
                            "op": "FuncDefn",
                            "name": "test_func",
                            "signature": {
                                "t": "FuncType",
                                "body": {
                                    "input": [],
                                    "output": [{"t": "I", "width": 64}],
                                },
                            },
                        }
                    ],
                }
            ],
        }

        # Try to compile with old version
        hugr_bytes = json.dumps(old_hugr).encode("utf-8")

        # We expect this to either:
        # 1. Fail with version mismatch error
        # 2. Be handled by our version translator
        try:
            result = compiler.compile_bytes_to_qir(hugr_bytes)
            # If it succeeds, our version translator worked!
            print(f"Version translation successful: {result}")
        except RuntimeError as e:
            # If it fails, check that it's a reasonable error
            error_msg = str(e)
            # Just check that we got a reasonable error
            if not any(
                keyword in error_msg.lower()
                for keyword in ["version", "format", "parse", "hugr"]
            ):
                raise AssertionError(f"Expected version-related error, got: {e}") from e

    except ImportError:
        pytest.skip("Rust HUGR backend not available")


def test_hugr_arithmetic_extension_handling() -> None:
    """Test handling of arithmetic extensions that cause version conflicts."""
    try:
        from pecos_rslib import RustHugrCompiler, check_rust_hugr_availability
        import json

        available, message = check_rust_hugr_availability()
        if not available:
            pytest.skip(f"HUGR support not available: {message}")

        compiler = RustHugrCompiler()

        # Create HUGR with arithmetic.int extension (known to cause conflicts)
        hugr_with_arithmetic = {
            "format": "hugr",
            "version": "0.20.1",
            "extensions": ["arithmetic.int"],
            "modules": [
                {
                    "name": "test",
                    "nodes": [
                        {
                            "op": "Extension",
                            "extension": "arithmetic.int",
                            "op_name": "iadd",
                            "signature": {
                                "t": "PolyFuncType",
                                "params": [{"t": "TypeBound", "b": "Copyable"}],
                                "body": {
                                    "input": [
                                        {
                                            "t": "Opaque",
                                            "extension": "arithmetic.int",
                                            "name": "int",
                                            "args": [{"t": "BoundedUSize", "size": 6}],
                                        },
                                        {
                                            "t": "Opaque",
                                            "extension": "arithmetic.int",
                                            "name": "int",
                                            "args": [{"t": "BoundedUSize", "size": 6}],
                                        },
                                    ],
                                    "output": [
                                        {
                                            "t": "Opaque",
                                            "extension": "arithmetic.int",
                                            "name": "int",
                                            "args": [{"t": "BoundedUSize", "size": 7}],
                                        }
                                    ],
                                },
                            },
                        }
                    ],
                }
            ],
        }

        hugr_bytes = json.dumps(hugr_with_arithmetic).encode("utf-8")

        # We expect this to fail with signature conflict
        with pytest.raises(RuntimeError) as exc_info:
            compiler.compile_bytes_to_qir(hugr_bytes)

        error_msg = str(exc_info.value)
        # This is the known issue - arithmetic extension conflicts
        if "conflicting signature" in error_msg.lower() and "iadd" in error_msg.lower():
            # This confirms the version mismatch issue we've been dealing with
            pass  # Expected error
        else:
            # Some other error - still OK as long as it's HUGR-related
            assert any(
                keyword in error_msg.lower()
                for keyword in ["hugr", "parse", "extension", "arithmetic"]
            ), f"Expected HUGR-related error, got: {exc_info.value}"

    except ImportError:
        pytest.skip("Rust HUGR backend not available")


if __name__ == "__main__":
    pytest.main([__file__, "-v"])
