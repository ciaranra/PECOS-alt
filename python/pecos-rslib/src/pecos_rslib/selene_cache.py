"""Caching mechanism for compiled Selene executables.

This module provides a cache for Selene executables to avoid recompilation
of the same programs, significantly improving test performance.
"""

import hashlib
import json
import logging
import shutil
from pathlib import Path
from typing import Optional, Tuple

logger = logging.getLogger(__name__)

# Global cache directory
CACHE_DIR = Path.home() / ".cache" / "pecos" / "selene" / "executables"


def get_cache_key(hugr_bytes: bytes, num_qubits: int) -> str:
    """Generate a cache key for a HUGR program and qubit count.

    Args:
        hugr_bytes: The HUGR program as bytes
        num_qubits: Number of qubits for the simulation

    Returns:
        A hex string hash that uniquely identifies this program+qubits combo
    """
    hasher = hashlib.sha256()
    hasher.update(hugr_bytes)
    hasher.update(str(num_qubits).encode())
    return hasher.hexdigest()[:16]  # Use first 16 chars for reasonable uniqueness


def get_cached_executable(
    hugr_bytes: bytes, num_qubits: int
) -> Optional[Tuple[Path, Path]]:
    """Check if we have a cached executable for this program.

    Args:
        hugr_bytes: The HUGR program as bytes
        num_qubits: Number of qubits for the simulation

    Returns:
        Tuple of (executable_path, artifacts_path) if cached, None otherwise
    """
    cache_key = get_cache_key(hugr_bytes, num_qubits)
    cache_path = CACHE_DIR / cache_key

    exec_path = cache_path / "artifacts" / "program.selene.x"
    artifacts_path = cache_path / "artifacts"

    if exec_path.exists():
        # Verify the cache metadata matches
        metadata_file = cache_path / "metadata.json"
        if metadata_file.exists():
            try:
                with open(metadata_file) as f:
                    metadata = json.load(f)
                if metadata.get("num_qubits") == num_qubits:
                    logger.debug(f"Cache hit for key {cache_key}")
                    return (exec_path, artifacts_path)
            except (json.JSONDecodeError, KeyError):
                logger.warning(f"Invalid cache metadata for {cache_key}, rebuilding")

    logger.debug(f"Cache miss for key {cache_key}")
    return None


def cache_executable(
    hugr_bytes: bytes,
    num_qubits: int,
    source_exec_path: Path,
    source_artifacts_path: Path,
) -> Tuple[Path, Path]:
    """Store a compiled executable in the cache.

    Args:
        hugr_bytes: The HUGR program as bytes
        num_qubits: Number of qubits for the simulation
        source_exec_path: Path to the compiled executable
        source_artifacts_path: Path to the artifacts directory

    Returns:
        Tuple of (cached_exec_path, cached_artifacts_path)
    """
    cache_key = get_cache_key(hugr_bytes, num_qubits)
    cache_path = CACHE_DIR / cache_key

    # Create cache directory
    cache_path.mkdir(parents=True, exist_ok=True)

    # Copy executable
    cached_exec = cache_path / "executable"
    shutil.copy2(source_exec_path, cached_exec)

    # Copy artifacts directory
    cached_artifacts = cache_path / "artifacts"
    if cached_artifacts.exists():
        shutil.rmtree(cached_artifacts)
    shutil.copytree(source_artifacts_path, cached_artifacts)

    # Write metadata
    metadata = {
        "num_qubits": num_qubits,
        "cache_key": cache_key,
    }
    with open(cache_path / "metadata.json", "w") as f:
        json.dump(metadata, f)

    logger.debug(f"Cached executable with key {cache_key}")

    return (cached_exec, cached_artifacts)


def clear_cache() -> None:
    """Clear the entire Selene executable cache."""
    if CACHE_DIR.exists():
        shutil.rmtree(CACHE_DIR)
        logger.info(f"Cleared Selene cache at {CACHE_DIR}")


def get_cache_size() -> int:
    """Get the total size of the cache in bytes."""
    if not CACHE_DIR.exists():
        return 0

    total_size = 0
    for item in CACHE_DIR.rglob("*"):
        if item.is_file():
            total_size += item.stat().st_size
    return total_size


def prune_cache(max_entries: int = 50) -> None:
    """Remove old cache entries if there are too many.

    Args:
        max_entries: Maximum number of cached executables to keep
    """
    if not CACHE_DIR.exists():
        return

    entries = list(CACHE_DIR.iterdir())
    if len(entries) <= max_entries:
        return

    # Sort by modification time and remove oldest
    entries.sort(key=lambda p: p.stat().st_mtime)
    for entry in entries[:-max_entries]:
        shutil.rmtree(entry)
        logger.debug(f"Pruned cache entry: {entry.name}")

    logger.info(f"Pruned cache to {max_entries} entries")
