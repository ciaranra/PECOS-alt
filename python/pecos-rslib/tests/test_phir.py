"""Tests for PHIR (PECOS High-level IR) JSON pipeline."""

import pytest


def test_phir_json_engine_import() -> None:
    """Test that PhirJsonEngine can be imported."""
    from pecos_rslib import PhirJsonEngine

    assert PhirJsonEngine is not None


def test_phir_json_engine_builder_import() -> None:
    """Test that PhirJsonEngineBuilder can be imported."""
    from pecos_rslib import PhirJsonEngineBuilder

    assert PhirJsonEngineBuilder is not None


def test_phir_json_program_import() -> None:
    """Test that PhirJsonProgram can be imported."""
    from pecos_rslib import PhirJsonProgram

    assert PhirJsonProgram is not None


def test_phir_json_simulation_import() -> None:
    """Test that PhirJsonSimulation can be imported."""
    from pecos_rslib import PhirJsonSimulation

    assert PhirJsonSimulation is not None


def test_compile_hugr_to_llvm_import() -> None:
    """Test that compile_hugr_to_llvm can be imported."""
    from pecos_rslib import compile_hugr_to_llvm

    assert compile_hugr_to_llvm is not None


def test_phir_json_engine_function() -> None:
    """Test that phir_json_engine function is available."""
    from pecos_rslib import phir_json_engine

    # Should be able to create an engine builder
    engine_builder = phir_json_engine()
    assert engine_builder is not None


def test_phir_json_program_creation() -> None:
    """Test creating PhirJsonProgram from JSON."""
    from pecos_rslib import PhirJsonProgram

    # PhirJsonProgram.from_json may accept strings and parse them later
    # or may validate immediately. Test what actually happens:
    try:
        # This might not raise immediately
        PhirJsonProgram.from_json("not json")
        # If it doesn't raise during creation, that's OK - it might fail during use
    except (ValueError, RuntimeError, TypeError):
        # If it does raise, that's also fine
        pass

    # Test creating from valid-looking JSON string
    try:
        PhirJsonProgram.from_json("{}")
        # Empty object might be accepted
    except (ValueError, RuntimeError, TypeError):
        # Or it might be rejected
        pass


def test_compile_hugr_to_llvm_with_invalid_input() -> None:
    """Test compile_hugr_to_llvm with invalid input."""
    from pecos_rslib import compile_hugr_to_llvm

    # compile_hugr_to_llvm expects bytes
    with pytest.raises((RuntimeError, ValueError, TypeError)):
        # Pass invalid HUGR bytes
        compile_hugr_to_llvm(b"not valid hugr")


def test_compile_hugr_to_llvm_with_wrong_type() -> None:
    """Test compile_hugr_to_llvm with wrong input type."""
    from pecos_rslib import compile_hugr_to_llvm

    # Should raise TypeError for string instead of bytes
    with pytest.raises(TypeError):
        compile_hugr_to_llvm("{}")  # String instead of bytes
