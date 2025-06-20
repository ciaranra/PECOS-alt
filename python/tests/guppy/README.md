# Guppy Integration Tests

This directory contains tests for the Guppy → HUGR → QIR → PECOS pipeline.

## Test Organization

### Unit Tests
- `test_guppy_minimal.py` - Minimal Guppy compilation test
- `test_hugr_compilation.py` - HUGR compilation and QIR generation tests

### Integration Tests
- `test_working_guppy_pipeline.py` - Complete pipeline test with working components
- `test_guppy_qir_pipeline.py` - Guppy to QIR pipeline test
- `test_run_guppy_demo.py` - Demo of run_guppy API

### Example/Development Tests
- `test_simple_guppy.py` - Simple Guppy examples
- `test_guppy_execute_llvm.py` - LLVM execution tests
- `test_guppy_simple_pipeline.py` - Simplified pipeline test
- `test_minimal_working.py` - Minimal working example

## Running Tests

### Run all Guppy tests
```bash
cd /path/to/PECOS
uv run python -m pytest python/tests/guppy/
```

### Run specific test
```bash
uv run python python/tests/guppy/test_minimal_working.py
```

### Run with pytest options
```bash
uv run python -m pytest python/tests/guppy/ -v  # verbose
uv run python -m pytest python/tests/guppy/ -k "minimal"  # pattern matching
```

## Test Requirements

- Python >= 3.10
- guppylang == 0.19.1 (recommended to pin version)
- PECOS with HUGR support compiled
- pecos-rslib installed with HUGR support

## Notes

- These tests were developed during Guppy integration
- Some tests may fail due to guppylang API changes
- The infrastructure tests should always pass
- Quantum function compilation depends on guppylang version
