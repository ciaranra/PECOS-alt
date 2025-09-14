"""Temporary directory cleanup for PECOS.

This module provides cleanup functionality for temporary directories
created during PECOS operations.
"""

import atexit
import logging
import shutil
import tempfile
from pathlib import Path
from typing import Set

logger = logging.getLogger(__name__)

# Track temporary directories created by PECOS
_temp_dirs: Set[Path] = set()


def register_temp_dir(path: Path) -> None:
    """Register a temporary directory for cleanup at exit.

    Args:
        path: Path to the temporary directory
    """
    _temp_dirs.add(path)
    logger.debug(f"Registered temp dir for cleanup: {path}")


def cleanup_temp_dir(path: Path) -> None:
    """Clean up a specific temporary directory.

    Args:
        path: Path to the temporary directory to clean up
    """
    if path.exists() and path.is_dir():
        try:
            shutil.rmtree(path)
            logger.debug(f"Cleaned up temp dir: {path}")
        except (OSError, PermissionError) as e:
            logger.warning(f"Failed to clean up temp dir {path}: {e}")

    # Remove from tracking
    _temp_dirs.discard(path)


def cleanup_all_temp_dirs() -> None:
    """Clean up all registered temporary directories."""
    for path in list(_temp_dirs):
        cleanup_temp_dir(path)


def cleanup_old_pecos_temp_dirs() -> None:
    """Clean up old PECOS temporary directories in the system temp directory.

    This removes directories matching the pattern pecos_* that are
    older than the current session.
    """
    import tempfile
    import time

    tmp_dir = Path(tempfile.gettempdir())
    if not tmp_dir.exists():
        return

    # Current time for age check (clean up dirs older than 1 hour)
    current_time = time.time()
    max_age_seconds = 3600  # 1 hour

    patterns = [
        "pecos_guppy_external_*",
        "pecos_selene_*",
        "pecos_hugr_*",
        "tmp*",  # Some temp dirs created by tempfile
    ]

    cleaned_count = 0
    for pattern_prefix in [p.replace("*", "") for p in patterns]:
        for entry in tmp_dir.iterdir():
            if not entry.is_dir():
                continue

            if not entry.name.startswith(pattern_prefix):
                continue

            # Check age
            try:
                stat = entry.stat()
                age = current_time - stat.st_mtime
                if age > max_age_seconds:
                    shutil.rmtree(entry)
                    cleaned_count += 1
                    logger.debug(f"Cleaned up old temp dir: {entry}")
            except (OSError, PermissionError) as e:
                logger.debug(f"Could not clean up {entry}: {e}")

    if cleaned_count > 0:
        logger.info(f"Cleaned up {cleaned_count} old temporary directories")


# Register cleanup function to run at exit
atexit.register(cleanup_all_temp_dirs)


# Context manager for temporary directories with automatic cleanup
class TempDirectory:
    """Context manager for temporary directories with automatic cleanup."""

    def __init__(self, prefix: str = "pecos_", suffix: str = None, dir: Path = None):
        """Initialize the temporary directory context manager.

        Args:
            prefix: Prefix for the directory name
            suffix: Suffix for the directory name
            dir: Parent directory for the temp directory
        """
        self.prefix = prefix
        self.suffix = suffix
        self.dir = dir
        self.path = None

    def __enter__(self) -> Path:
        """Create and enter the temporary directory."""
        self.path = Path(
            tempfile.mkdtemp(prefix=self.prefix, suffix=self.suffix, dir=self.dir)
        )
        register_temp_dir(self.path)
        return self.path

    def __exit__(self, exc_type, exc_val, exc_tb):
        """Clean up the temporary directory on exit."""
        if self.path:
            cleanup_temp_dir(self.path)


# Clean up old temp dirs on module import
try:
    cleanup_old_pecos_temp_dirs()
except Exception as e:
    logger.debug(f"Error cleaning up old temp dirs: {e}")
