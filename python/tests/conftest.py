"""Pytest configuration for PECOS tests."""

import sys
import warnings
from typing import Any


def pytest_configure(config: Any) -> None:  # noqa: ANN401
    """Configure pytest with Python version-specific handling."""
    if sys.version_info >= (3, 13):
        # Suppress guppylang deprecation warning on Python 3.13+
        warnings.filterwarnings(
            "ignore",
            message="DesugaredGenerator.__init__ got an unexpected keyword argument",
            category=DeprecationWarning,
            module="guppylang.cfg.builder",
        )

        # Add a warning to the test session
        config.warn(
            "W1",
            "Python 3.13+ detected: Suppressing guppylang DesugaredGenerator deprecation warnings. "
            "This is a known compatibility issue with guppylang 0.19.1 and Python 3.13+. "
            "Consider using Python 3.12 for full compatibility.",
        )
