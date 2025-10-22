"""Noise model builders for the unified simulation API.

This module provides a namespace for all noise model builders, making them easily
discoverable through IDE autocomplete and documentation.

Examples:
    >>> from pecos_rslib import noise
    >>>
    >>> # Available noise models via namespace
    >>> general = noise.general()
    >>> depolarizing = noise.depolarizing()
    >>> biased_depolarizing = noise.biased_depolarizing()
    >>>
    >>> # Configure noise models
    >>> depolarizing_noise = noise.depolarizing().with_p1_probability(0.01)
    >>>
    >>> # Direct class instantiation also available
    >>> general = noise.GeneralNoiseModelBuilder()
    >>> depolarizing = noise.DepolarizingNoiseModelBuilder()
    >>> biased = noise.BiasedDepolarizingNoiseModelBuilder()
    >>>
    >>> # Use in simulation
    >>> from pecos_rslib import engines
    >>> results = (
    ...     engines.qasm().program(program).to_sim().noise(depolarizing_noise).run(1000)
    ... )
"""

from dataclasses import dataclass

# Import from the unified sim module
from pecos_rslib.sim import (
    BiasedDepolarizingNoiseModelBuilder,
    DepolarizingNoiseModelBuilder,
    GeneralNoiseModelBuilder,
)

# Import from engine builders module (once noise free functions are exposed)
# from pecos_rslib._pecos_rslib import (
#     general_noise,
#     depolarizing_noise,
#     biased_depolarizing_noise,
# )


# For now, create factory functions until free functions are exposed from Rust
def general() -> GeneralNoiseModelBuilder:
    """Create a general noise model builder.

    Returns:
        GeneralNoiseModelBuilder: A new general noise model builder
    """
    return GeneralNoiseModelBuilder()


def depolarizing() -> DepolarizingNoiseModelBuilder:
    """Create a depolarizing noise model builder.

    Returns:
        DepolarizingNoiseModelBuilder: A new depolarizing noise model builder
    """
    return DepolarizingNoiseModelBuilder()


def biased_depolarizing() -> BiasedDepolarizingNoiseModelBuilder:
    """Create a biased depolarizing noise model builder.

    Returns:
        BiasedDepolarizingNoiseModelBuilder: A new biased depolarizing noise model builder
    """
    return BiasedDepolarizingNoiseModelBuilder()


# Simple noise model dataclasses for backward compatibility
# These are being replaced by the builder pattern but kept for existing code


@dataclass
class PassThroughNoise:
    """No noise - ideal quantum simulation."""


@dataclass
class DepolarizingNoise:
    """Standard depolarizing noise with uniform probability.

    Args:
        p: Uniform error probability for all operations
    """

    p: float = 0.001


@dataclass
class BiasedDepolarizingNoise:
    """Biased depolarizing noise model.

    Args:
        p: Uniform probability for all operations
    """

    p: float = 0.001


@dataclass
class GeneralNoise:
    """General noise model with full parameter configuration."""

    # Global parameters
    seed: int | None = None
    scale: float | None = None
    # Gate error probabilities
    p1: float | None = None
    p2: float | None = None
    p_meas: float | None = None
    p_prep: float | None = None


__all__ = [
    # Free functions
    "general",
    "depolarizing",
    "biased_depolarizing",
    # Builder classes
    "GeneralNoiseModelBuilder",
    "DepolarizingNoiseModelBuilder",
    "BiasedDepolarizingNoiseModelBuilder",
    # Legacy dataclasses for compatibility
    "PassThroughNoise",
    "DepolarizingNoise",
    "BiasedDepolarizingNoise",
    "GeneralNoise",
]
