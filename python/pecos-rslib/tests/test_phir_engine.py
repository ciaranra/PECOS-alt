# PECOS/python/pecos-rslib/tests/test_phir_engine.py
"""Tests for PHIR engine integration with Rust-based simulators.

This module contains test cases for verifying the integration between PHIR (PECOS High-level Intermediate
Representation) and the Rust-based quantum simulators, ensuring proper execution of quantum programs and
correct simulation results.
"""
import json

import pytest
from pecos_rslib._pecos_rslib import PHIREngine


# Helper function to create a PHIREngine instance with a simple test program
def create_test_bell_program() -> str:
    """Create a simple PHIR program for testing register mapping.

    This function returns a PHIR JSON program that creates a Bell state,
    measures two qubits, and maps the results to both 'm' and 'output' registers.
    """
    return json.dumps(
        {
            "format": "PHIR/JSON",
            "version": "0.1.0",
            "metadata": {"description": "Bell state with register mapping"},
            "ops": [
                {
                    "data": "qvar_define",
                    "data_type": "qubits",
                    "variable": "q",
                    "size": 2,
                },
                {"data": "cvar_define", "data_type": "i64", "variable": "m", "size": 2},
                {
                    "data": "cvar_define",
                    "data_type": "i64",
                    "variable": "output",
                    "size": 2,
                },
                {"qop": "H", "args": [["q", 0]]},
                {"qop": "CX", "args": [["q", 0], ["q", 1]]},
                {"qop": "Measure", "args": [["q", 0]], "returns": [["m", 0]]},
                {"qop": "Measure", "args": [["q", 1]], "returns": [["m", 1]]},
            ],
        }
    )


def test_phir_minimal() -> None:
    """Test with a minimal PHIR program to verify basic functionality."""
    phir_json = json.dumps(
        {
            "format": "PHIR/JSON",
            "version": "0.1.0",
            "metadata": {"generated_by": "PECOS version 0.6.0.dev8"},
            "ops": [
                {
                    "data": "qvar_define",
                    "data_type": "qubits",
                    "variable": "q",
                    "size": 2,
                },
                {
                    "data": "cvar_define",
                    "data_type": "i64",
                    "variable": "m",
                    "size": 2,
                },
                {
                    "qop": "Measure",
                    "args": [["q", 0]],
                    "returns": [["m", 0]],
                },
            ],
        },
    )

    # Create engine
    engine = PHIREngine(phir_json)

    # Get commands
    commands = engine.process_program()
    assert len(commands) == 1, "Expected 1 quantum command"

    # Verify it's a measure gate
    cmd = commands[0]
    assert cmd["gate_type"] == "Measure", "Expected Measure gate"
    assert cmd["params"]["result_id"] == 0, "Expected result_id to be 0"
    assert cmd["qubits"][0] == 0, "Expected measurement on qubit 0"

    # Handle measurement
    engine.handle_measurement(1)  # Send a measurement result of 1

    # Get results
    results = engine.get_results()

    # Extract the measurement key
    assert len(results) > 0, "Expected at least one measurement result"
    measurement_key = next(iter(results.keys()))

    # Verify the result
    assert results[measurement_key] == 0, f"Expected {measurement_key} to have value 0"


def test_phir_invalid_json() -> None:
    """Test PHIR engine handling of invalid JSON input."""
    invalid_json = '{"format": "PHIR/JSON", "invalid": }'
    with pytest.raises(
        json.decoder.JSONDecodeError,
        match=r"Expecting value: line 1 column 36 \(char 35\)",
    ):
        PHIREngine(invalid_json)


def test_phir_empty_program() -> None:
    """Test PHIR engine processing of empty program."""
    phir = json.dumps(
        {
            "format": "PHIR/JSON",
            "version": "0.1.0",
            "metadata": {"generated_by": "Test"},
            "ops": [],
        },
    )

    engine = PHIREngine(phir)
    commands = engine.process_program()
    assert len(commands) == 0, "Expected empty command list"


def test_phir_full_circuit() -> None:
    """Test PHIR engine processing of complete quantum circuit."""
    phir = json.dumps(
        {
            "format": "PHIR/JSON",
            "version": "0.1.0",
            "metadata": {"generated_by": "PECOS version 0.6.0.dev8"},
            "ops": [
                {
                    "data": "qvar_define",
                    "data_type": "qubits",
                    "variable": "q",
                    "size": 2,
                },
                {"data": "cvar_define", "data_type": "i64", "variable": "c", "size": 2},
                {"qop": "Measure", "args": [["q", 0]], "returns": [["c", 0]]},
                {"qop": "Measure", "args": [["q", 1]], "returns": [["c", 1]]},
            ],
        },
    )

    # Create engine
    engine = PHIREngine(phir)

    # Process the program and get commands
    commands = engine.process_program()
    print(f"Got {len(commands)} commands")

    # Handle example measurements
    engine.handle_measurement(1)

    # Get final results
    results = engine.get_results()
    print(f"Got results: {results}")

    assert len(results) > 0, "Expected measurement results"


def test_phir_full() -> None:
    """Test with a full PHIR program."""
    phir = {
        "format": "PHIR/JSON",
        "version": "0.1.0",
        "metadata": {"generated_by": "PECOS version 0.6.0.dev8"},
        "ops": [
            {
                "data": "qvar_define",
                "data_type": "qubits",
                "variable": "q",
                "size": 2,
            },
            {
                "data": "cvar_define",
                "data_type": "i64",
                "variable": "m",
                "size": 2,
            },
            {
                "qop": "Measure",
                "args": [["q", 0]],
                "returns": [["m", 0]],
            },
        ],
    }

    phir_json = json.dumps(phir)
    engine = PHIREngine(phir_json)
    results = engine.results_dict
    assert isinstance(results, dict)


def test_register_mapping_simulation() -> None:
    """Test the register mapping behavior that requires the Result instruction.

    The Result instruction is part of the PHIR specification but has different
    support levels across implementations:

    1. Rust PHIREngine - Requires Result instruction to be present but may not
       actually implement the mapping logic yet
    2. Python PHIRClassicalInterpreter - Doesn't support Result instruction
       even with validation disabled (no implementation of the cop "=" logic)

    Since neither implementation fully supports Result instruction functionality,
    we skip this test but document what it would verify once support is added.
    """
    pytest.skip(
        "Result instruction not fully implemented in either Rust or Python PHIR interpreters. "
        "Rust requires it but doesn't implement mapping; Python doesn't support it at all."
    )

    # Document what the test would verify once Result instruction is supported:
    # 1. Measurements populate the "m" register with measurement outcomes
    # 2. Result instructions (cop "=") copy values from "m" to "output" register
    # 3. Both registers contain the same values after execution
    # 4. For a Bell state, the values would be correlated (00 or 11)
