"""Test helpers for Guppy tests."""

def needs_state_vector_desc(func):
    """Decorator to indicate test needs state vector engine for non-Clifford gates."""
    func._needs_state_vector = True
    return func