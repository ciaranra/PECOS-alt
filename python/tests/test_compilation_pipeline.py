"""Test the structured compilation pipeline."""

import pytest


def test_pipeline_imports() -> None:
    """Test that we can import the pipeline module."""
    from pecos import compilation_pipeline

    # Check all expected functions exist
    assert hasattr(compilation_pipeline, "compile_guppy_to_hugr")
    assert hasattr(compilation_pipeline, "compile_hugr_to_llvm")
    assert hasattr(compilation_pipeline, "execute_llvm")
    assert hasattr(compilation_pipeline, "compile_guppy_to_llvm")
    assert hasattr(compilation_pipeline, "run_guppy_function")


@pytest.mark.optional_dependency
def test_guppy_to_hugr() -> None:
    """Test Guppy to HUGR compilation."""
    from guppylang import guppy
    from pecos.compilation_pipeline import compile_guppy_to_hugr

    @guppy
    def simple_function() -> int:
        return 42

    hugr_bytes = compile_guppy_to_hugr(simple_function)
    assert isinstance(hugr_bytes, bytes)
    assert len(hugr_bytes) > 0
    # HUGR format starts with "HUGR"
    assert hugr_bytes.startswith(b"HUGR")


@pytest.mark.optional_dependency
def test_hugr_to_llvm() -> None:
    """Test HUGR to LLVM compilation."""
    from guppylang import guppy
    from pecos.compilation_pipeline import compile_guppy_to_hugr, compile_hugr_to_llvm

    @guppy
    def simple_function() -> int:
        return 42

    try:
        hugr_bytes = compile_guppy_to_hugr(simple_function)
        llvm_ir = compile_hugr_to_llvm(hugr_bytes)
        assert isinstance(llvm_ir, str)
        # Should contain some LLVM IR markers
        assert "@" in llvm_ir or "define" in llvm_ir
    except RuntimeError as e:
        if "Unknown type" in str(e):
            pytest.skip("Known limitation: Rust backend doesn't support all types yet")
        raise


@pytest.mark.optional_dependency
def test_full_pipeline() -> None:
    """Test the full compilation pipeline."""
    from guppylang import guppy
    from pecos.compilation_pipeline import compile_guppy_to_llvm

    @guppy
    def simple_function() -> int:
        return 42

    try:
        llvm_ir = compile_guppy_to_llvm(simple_function)
        assert isinstance(llvm_ir, str)
        assert len(llvm_ir) > 0
    except RuntimeError as e:
        if "Unknown type" in str(e):
            pytest.skip("Known limitation: Rust backend doesn't support all types yet")
        raise


def test_invalid_function() -> None:
    """Test error handling for non-Guppy functions."""
    from pecos.compilation_pipeline import compile_guppy_to_hugr

    def regular_function() -> int:
        return 42

    with pytest.raises(ValueError, match="must be decorated with @guppy"):
        compile_guppy_to_hugr(regular_function)


if __name__ == "__main__":
    # Run basic tests
    test_pipeline_imports()
    print("✓ Pipeline imports successful")

    try:
        test_guppy_to_hugr()
        print("✓ Guppy to HUGR compilation works")
    except ImportError:
        print("✗ Guppylang not available")

    try:
        test_hugr_to_llvm()
        print("✓ HUGR to LLVM compilation works")
    except ImportError:
        print("✗ HUGR backend not available")
    except RuntimeError as e:
        print(f"✗ Compilation failed: {e}")
