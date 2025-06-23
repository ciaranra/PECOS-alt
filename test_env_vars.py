"""Check environment variables in pytest"""

import os

def test_env_vars():
    """Check what environment variables pytest sets"""
    print("Environment variables containing 'PYTEST' or 'TEST':")
    for key, value in sorted(os.environ.items()):
        if 'PYTEST' in key or 'TEST' in key:
            print(f"  {key}={value}")
    
    # Check specific variables
    pytest_test = os.environ.get('PYTEST_CURRENT_TEST')
    python_test = os.environ.get('PYTHON_TEST_MODE')
    
    print(f"\nPYTEST_CURRENT_TEST: {pytest_test}")
    print(f"PYTHON_TEST_MODE: {python_test}")
    
    # This will help us see if the detection logic works
    is_python_test = pytest_test is not None or python_test is not None
    print(f"Is Python test: {is_python_test}")