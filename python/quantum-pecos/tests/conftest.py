# Copyright 2025 The PECOS Developers
#
# Licensed under the Apache License, Version 2.0 (the "License"); you may not use this file except in compliance with
# the License.You may obtain a copy of the License at
#
#     https://www.apache.org/licenses/LICENSE-2.0
#
# Unless required by applicable law or agreed to in writing, software distributed under the License is distributed on an
# "AS IS" BASIS, WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied. See the License for the
# specific language governing permissions and limitations under the License.

"""Test configuration and shared fixtures."""

# Check if llvmlite is available
import importlib.util

# Configure matplotlib to use non-interactive backend for tests
# This must be done before importing matplotlib.pyplot to avoid GUI backend issues on Windows
import matplotlib as mpl
import pytest

mpl.use("Agg")

HAS_LLVMLITE = importlib.util.find_spec("llvmlite") is not None

# Decorator to skip tests that require llvmlite
skipif_no_llvmlite = pytest.mark.skipif(
    not HAS_LLVMLITE,
    reason="llvmlite is not installed (not available for Python >= 3.13)",
)


# Make skipif_no_llvmlite available to all test modules
def pytest_configure(config: pytest.Config) -> None:
    """Make custom markers available globally."""
    # Register the marker
    config.addinivalue_line(
        "markers",
        "skipif_no_llvmlite: skip test if llvmlite is not available",
    )

    # Make skipif_no_llvmlite available in the pytest namespace
    pytest.skipif_no_llvmlite = skipif_no_llvmlite
