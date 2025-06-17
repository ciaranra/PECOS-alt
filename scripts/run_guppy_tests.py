#!/usr/bin/env python3
"""
Guppy Test Runner - Utility to run Guppy-related tests

This script runs various Guppy-related tests and checks. It's a test runner,
not a test itself.

Usage:
    python scripts/run_guppy_tests.py           # Run all tests
    python scripts/run_guppy_tests.py --rust    # Run only Rust tests
    python scripts/run_guppy_tests.py --python  # Run only Python tests
    python scripts/run_guppy_tests.py --quick   # Quick infrastructure check
"""

import argparse
import subprocess
import sys
from pathlib import Path

PECOS_ROOT = Path(__file__).parent.parent


def run_command(cmd: list, description: str, verbose: bool = False) -> bool:
    """Run a command and return success status"""
    if verbose:
        print(f"  Running: {' '.join(cmd)}")
    
    result = subprocess.run(cmd, capture_output=True, text=True, cwd=PECOS_ROOT)
    
    if result.returncode == 0:
        print(f"✅ {description}")
        return True
    else:
        print(f"❌ {description} - FAILED")
        if verbose:
            if result.stderr.strip():
                print(f"   Error: {result.stderr.strip()}")
            if result.stdout.strip():
                print(f"   Output: {result.stdout.strip()}")
        return False


def run_rust_tests(verbose: bool = False) -> bool:
    """Run Rust HUGR tests"""
    print("🦀 Running Rust HUGR Tests...")
    
    success = True
    success &= run_command(
        ["cargo", "check", "-p", "pecos-qir", "--features", "hugr-support"],
        "HUGR support compilation", verbose
    )
    
    success &= run_command(
        ["cargo", "test", "-p", "pecos-qir", "hugr", "--features", "hugr-support", "--", "--quiet"],
        "HUGR unit tests", verbose
    )
    
    return success


def run_python_tests(verbose: bool = False) -> bool:
    """Run Python Guppy tests"""
    print("\n🐍 Running Python Guppy Tests...")
    
    # First check if pytest is available
    check_result = subprocess.run(
        ["python", "-c", "import pytest"], 
        capture_output=True, cwd=PECOS_ROOT
    )
    
    if check_result.returncode != 0:
        print("❌ pytest not available - install with: uv pip install pytest")
        return False
    
    success = True
    
    # Run pytest on the guppy test directory
    success &= run_command(
        ["python", "-m", "pytest", "python/tests/guppy/test_infrastructure.py", "-v" if verbose else "-q", "--tb=short"],
        "Python Guppy tests", verbose
    )
    
    return success


def run_infrastructure_check(verbose: bool = False) -> bool:
    """Run a quick infrastructure check"""
    print("\n🔍 Quick Infrastructure Check...")
    
    # Just run one simple test to check if everything is set up
    test_file = PECOS_ROOT / "python/tests/guppy/test_guppy_simple.py"
    if test_file.exists():
        return run_command(
            ["python", str(test_file)],
            "Infrastructure check", verbose
        )
    else:
        print("❌ Test file not found: python/tests/guppy/test_guppy_simple.py")
        return False


def main():
    parser = argparse.ArgumentParser(
        description="Run Guppy-related tests",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog="""
Examples:
    python scripts/run_guppy_tests.py                    # Run all tests
    python scripts/run_guppy_tests.py --rust --verbose   # Run Rust tests with details
    python scripts/run_guppy_tests.py --quick            # Quick check only
        """
    )
    
    parser.add_argument("--rust", action="store_true", help="Run only Rust tests")
    parser.add_argument("--python", action="store_true", help="Run only Python tests")
    parser.add_argument("--quick", action="store_true", help="Quick infrastructure check only")
    parser.add_argument("--verbose", "-v", action="store_true", help="Verbose output")
    
    args = parser.parse_args()
    
    print("🚀 Guppy Test Runner")
    print("=" * 50)
    
    success = True
    
    if args.quick:
        success = run_infrastructure_check(args.verbose)
    elif args.rust:
        success = run_rust_tests(args.verbose)
    elif args.python:
        success = run_python_tests(args.verbose)
    else:
        # Run all tests
        success &= run_rust_tests(args.verbose)
        success &= run_python_tests(args.verbose)
    
    print("\n" + "=" * 50)
    if success:
        print("✅ All tests passed!")
    else:
        print("❌ Some tests failed")
        print("\n💡 Troubleshooting:")
        print("   - Ensure dependencies are installed: uv pip install -e python/quantum-pecos[guppy]")
        print("   - Check HUGR support: cargo build -p pecos-qir --features hugr-support")
        print("   - For Guppy issues, consider pinning guppylang==0.19.1 in pyproject.toml")
    
    return 0 if success else 1


if __name__ == "__main__":
    sys.exit(main())