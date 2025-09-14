"""LLVM simulation compatibility layer.

This module provides backward compatibility for the old llvm_sim API.
For new code, use the unified API with selene_engine() instead:

    from pecos_rslib import selene_engine
    from pecos_rslib.programs import LlvmProgram

    results = selene_engine().program(LlvmProgram.from_string(llvm_ir)).to_sim().run(shots)

Or for Guppy programs:

    from pecos_rslib import selene_engine

    results = selene_engine().program(guppy_func).to_sim().run(shots)
"""

from pecos_rslib import selene_engine
from pecos_rslib.noise import (
    BiasedDepolarizingNoise,
    DepolarizingNoise,
    GeneralNoise,
    PassThroughNoise,
)
from pecos_rslib.programs import LlvmProgram


def llvm_sim(
    llvm_ir: str,
    shots: int,
    noise_model: object | None = None,
    seed: int | None = None,
    workers: int | None = None,
) -> dict[str, list[int]]:
    """Run an LLVM IR quantum program simulation.

    NOTE: This function is provided for backward compatibility.
    Consider using the new unified API instead:

        from pecos_rslib import selene_engine
        from pecos_rslib.programs import LlvmProgram

        results = selene_engine().program(LlvmProgram.from_string(llvm_ir)).to_sim().noise(noise_model).seed(42).run(shots)

    Args:
        llvm_ir: LLVM IR string
        shots: Number of simulation shots
        noise_model: Optional noise model builder
        seed: Optional random seed
        workers: Optional number of worker threads

    Returns:
        Dictionary mapping register names to measurement results
    """
    # Use the new unified API with selene_engine
    sim_builder = selene_engine().program(LlvmProgram.from_string(llvm_ir)).to_sim()

    if noise_model is not None:
        sim_builder = sim_builder.noise(noise_model)

    if seed is not None:
        sim_builder = sim_builder.seed(seed)

    if workers is not None:
        sim_builder = sim_builder.workers(workers)

    shot_vec = sim_builder.run(shots)

    # Convert ShotVec to dict format for backward compatibility
    shot_map = shot_vec.try_as_shot_map()
    if shot_map is None:
        raise ValueError("Failed to convert results to shot map")

    # Get all register names and convert to dict
    result = {}
    for reg in shot_map.get_registers():
        values = shot_map.try_bits_as_u64(reg)
        if values is not None:
            result[reg] = values

    return result


# Re-export for compatibility
__all__ = [
    "BiasedDepolarizingNoise",
    "DepolarizingNoise",
    "GeneralNoise",
    "PassThroughNoise",
    "llvm_sim",
]
