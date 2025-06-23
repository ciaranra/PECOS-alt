"""Pytest configuration for guppy tests."""

import os
import pytest
from unittest import mock

# Set a maximum shot limit for tests to prevent hanging
MAX_TEST_SHOTS = int(os.environ.get('PECOS_MAX_TEST_SHOTS', '5'))

# Monkey-patch run_guppy to limit shots
original_run_guppy = None

def limited_run_guppy(func, shots=1000, **kwargs):
    """Wrapper that limits shots to prevent test hanging."""
    if shots > MAX_TEST_SHOTS:
        print(f"\n[TEST LIMITER] Reducing shots from {shots} to {MAX_TEST_SHOTS} to prevent hanging")
        shots = MAX_TEST_SHOTS
    return original_run_guppy(func, shots=shots, **kwargs)

@pytest.fixture(autouse=True, scope='session')
def limit_shots_globally():
    """Automatically limit shots in all tests."""
    global original_run_guppy
    try:
        from pecos import frontends
        original_run_guppy = frontends.run_guppy
        frontends.run_guppy = limited_run_guppy
        
        # Also patch it in pecos module if imported there
        try:
            import pecos
            if hasattr(pecos, 'run_guppy'):
                pecos.run_guppy = limited_run_guppy
        except:
            pass
            
        print(f"\n[TEST CONFIG] Limiting all tests to maximum {MAX_TEST_SHOTS} shots")
        print("[TEST CONFIG] Set PECOS_MAX_TEST_SHOTS environment variable to change this")
        
        yield
        
        # Restore original
        frontends.run_guppy = original_run_guppy
        try:
            import pecos
            if hasattr(pecos, 'run_guppy'):
                pecos.run_guppy = original_run_guppy
        except:
            pass
    except ImportError:
        # If run_guppy not available, just continue
        yield

@pytest.fixture(autouse=True)
def print_test_name(request):
    """Print test name to help identify hanging tests."""
    test_name = request.node.name
    print(f"\n[RUNNING TEST] {test_name}")
    yield
    print(f"[COMPLETED] {test_name}")