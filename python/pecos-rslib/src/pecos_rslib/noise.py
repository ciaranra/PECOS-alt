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
    >>> results = engines.qasm()\\
    >>>     .program(program)\\
    >>>     .to_sim()\\
    >>>     .noise(depolarizing_noise)\\
    >>>     .run(1000)
"""

# Import from the unified sim module
from pecos_rslib.sim import (
    GeneralNoiseModelBuilder,
    DepolarizingNoiseModelBuilder,
    BiasedDepolarizingNoiseModelBuilder,
)

# Import from engine builders module (once noise free functions are exposed)
# from pecos_rslib._pecos_rslib import (
#     general_noise,
#     depolarizing_noise,
#     biased_depolarizing_noise,
# )

# For now, create factory functions until free functions are exposed from Rust
def general():
    """Create a general noise model builder.
    
    Returns:
        GeneralNoiseModelBuilder: A new general noise model builder
    """
    return GeneralNoiseModelBuilder()

def depolarizing():
    """Create a depolarizing noise model builder.
    
    Returns:
        DepolarizingNoiseModelBuilder: A new depolarizing noise model builder
    """
    return DepolarizingNoiseModelBuilder()

def biased_depolarizing():
    """Create a biased depolarizing noise model builder.
    
    Returns:
        BiasedDepolarizingNoiseModelBuilder: A new biased depolarizing noise model builder
    """
    return BiasedDepolarizingNoiseModelBuilder()

__all__ = [
    # Free functions
    "general",
    "depolarizing",
    "biased_depolarizing",
    # Builder classes
    "GeneralNoiseModelBuilder",
    "DepolarizingNoiseModelBuilder",
    "BiasedDepolarizingNoiseModelBuilder",
]