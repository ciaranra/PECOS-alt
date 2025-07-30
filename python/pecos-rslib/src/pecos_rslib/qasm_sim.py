"""Python interface for QASM simulation with enhanced API.

This module provides a clean Python interface for running quantum circuit simulations
using OpenQASM 2.0. It supports various noise models, quantum engines, and parallel execution.

For detailed usage examples, see the PECOS documentation:
https://github.com/CQCL/PECOS/blob/master/docs/user-guide/qasm-simulation.md
"""

from dataclasses import dataclass
from typing import List, Dict, Optional, Any, Tuple
# Old bindings are no longer available - provide compatibility layer
try:
    from pecos_rslib._pecos_rslib import GeneralNoiseModelBuilder
except ImportError:
    GeneralNoiseModelBuilder = None

# Compatibility enums for backward compatibility
class NoiseModel:
    """Enum-like class for noise models."""
    def __init__(self, model_type: str):
        # Normalize the input
        normalized = {
            "passthrough": "PassThrough",
            "depolarizing": "Depolarizing",
            "depolarizingcustom": "DepolarizingCustom",
            "biaseddepolarizing": "BiasedDepolarizing",
            "general": "General"
        }
        key = model_type.lower().replace("_", "").replace("-", "")
        if key in normalized:
            self.value = normalized[key]
        elif model_type in ["PassThrough", "Depolarizing", "DepolarizingCustom", "BiasedDepolarizing", "General"]:
            self.value = model_type
        else:
            raise ValueError(f"Unknown noise model type: {model_type}")
    
    def __str__(self):
        return self.value
    
    def __repr__(self):
        return f"NoiseModel('{self.value}')"

class QuantumEngine:
    """Enum-like class for quantum engines."""
    def __init__(self, engine_type: str):
        # Normalize the input
        normalized = {
            "statevector": "StateVector",
            "state_vector": "StateVector",
            "sv": "StateVector",
            "sparsestabilizer": "SparseStabilizer",
            "sparse_stabilizer": "SparseStabilizer",
            "stab": "SparseStabilizer",
            "stabilizer": "SparseStabilizer"
        }
        key = engine_type.lower().replace("-", "_")
        if key in normalized:
            self.value = normalized[key]
        elif engine_type in ["StateVector", "SparseStabilizer"]:
            self.value = engine_type
        else:
            raise ValueError(f"Unknown quantum engine type: {engine_type}")
    
    def __str__(self):
        return self.value
    
    def __repr__(self):
        return f"QuantumEngine('{self.value}')"
    
    # Add these as class attributes for compatibility
    StateVector = "StateVector"
    SparseStabilizer = "SparseStabilizer"

# Stubs for backward compatibility
QasmSimulation = None 
QasmSimulationBuilder = None
_run_qasm = None
_qasm_sim = None
_get_noise_models = None
_get_quantum_engines = None

__all__ = [
    "NoiseModel",
    "QuantumEngine",
    "QasmSimulation",
    "QasmSimulationBuilder",
    "get_noise_models",
    "get_quantum_engines",
    # Noise model dataclasses
    "PassThroughNoise",
    "DepolarizingNoise",
    "DepolarizingCustomNoise",
    "BiasedDepolarizingNoise",
    "GeneralNoise",
    # Builder classes
    "GeneralNoiseModelBuilder",  # Rust-native builder
    # Main interface
    "run_qasm",
    "qasm_sim",
]


# Noise model dataclasses


@dataclass
class PassThroughNoise:
    """No noise - ideal quantum simulation."""

    @classmethod
    def from_config(cls, config: Dict[str, Any]) -> "PassThroughNoise":
        """Create PassThroughNoise from configuration dictionary."""
        return cls()


@dataclass
class DepolarizingNoise:
    """Standard depolarizing noise with uniform probability.

    Args:
        p: Uniform error probability for all operations
    """

    p: float = 0.001

    @classmethod
    def from_config(cls, config: Dict[str, Any]) -> "DepolarizingNoise":
        """Create DepolarizingNoise from configuration dictionary."""
        return cls(p=config.get("p", 0.001))


@dataclass
class DepolarizingCustomNoise:
    """Depolarizing noise with custom probabilities for different operations.

    Args:
        p_prep: State preparation error probability
        p_meas: Measurement error probability
        p1: Single-qubit gate error probability
        p2: Two-qubit gate error probability
    """

    p_prep: float = 0.001
    p_meas: float = 0.001
    p1: float = 0.001
    p2: float = 0.002

    @classmethod
    def from_config(cls, config: Dict[str, Any]) -> "DepolarizingCustomNoise":
        """Create DepolarizingCustomNoise from configuration dictionary."""
        return cls(
            p_prep=config.get("p_prep", 0.001),
            p_meas=config.get("p_meas", 0.001),
            p1=config.get("p1", 0.001),
            p2=config.get("p2", 0.002),
        )


@dataclass
class BiasedDepolarizingNoise:
    """Biased depolarizing noise model.

    Args:
        p: Uniform probability for all operations
    """

    p: float = 0.001

    @classmethod
    def from_config(cls, config: Dict[str, Any]) -> "BiasedDepolarizingNoise":
        """Create BiasedDepolarizingNoise from configuration dictionary."""
        return cls(p=config.get("p", 0.001))


@dataclass
class GeneralNoise:
    """General noise model with full parameter configuration.

    This noise model supports detailed configuration of various error types including:
    - Idle/memory errors with coherent and incoherent noise
    - State preparation errors with leakage and crosstalk
    - Single-qubit gate errors with emission and Pauli models
    - Two-qubit gate errors with angle-dependent noise
    - Measurement errors with asymmetric bit-flip probabilities

    All parameters are optional. If not specified, default values from the
    GeneralNoiseModel will be used.
    """

    # Global parameters
    noiseless_gates: Optional[List[str]] = None
    seed: Optional[int] = None
    scale: Optional[float] = None
    leakage_scale: Optional[float] = None
    emission_scale: Optional[float] = None

    # Idle noise parameters
    p_idle_coherent: Optional[bool] = None
    p_idle_linear_rate: Optional[float] = None
    p_idle_linear_model: Optional[Dict[str, float]] = None
    p_idle_quadratic_rate: Optional[float] = None
    p_idle_coherent_to_incoherent_factor: Optional[float] = None
    idle_scale: Optional[float] = None

    # Preparation noise parameters
    p_prep: Optional[float] = None
    p_prep_leak_ratio: Optional[float] = None
    p_prep_crosstalk: Optional[float] = None
    prep_scale: Optional[float] = None
    p_prep_crosstalk_scale: Optional[float] = None

    # Single-qubit gate noise parameters
    p1: Optional[float] = None
    p1_emission_ratio: Optional[float] = None
    p1_emission_model: Optional[Dict[str, float]] = None
    p1_seepage_prob: Optional[float] = None
    p1_pauli_model: Optional[Dict[str, float]] = None
    p1_scale: Optional[float] = None

    # Two-qubit gate noise parameters
    p2: Optional[float] = None
    p2_angle_params: Optional[Tuple[float, float, float, float]] = None
    p2_angle_power: Optional[float] = None
    p2_emission_ratio: Optional[float] = None
    p2_emission_model: Optional[Dict[str, float]] = None
    p2_seepage_prob: Optional[float] = None
    p2_pauli_model: Optional[Dict[str, float]] = None
    p2_idle: Optional[float] = None
    p2_scale: Optional[float] = None

    # Measurement noise parameters
    p_meas_0: Optional[float] = None
    p_meas_1: Optional[float] = None
    p_meas_crosstalk: Optional[float] = None
    meas_scale: Optional[float] = None
    p_meas_crosstalk_scale: Optional[float] = None

    @classmethod
    def from_config(cls, config: Dict[str, Any]) -> "GeneralNoise":
        """Create GeneralNoise from configuration dictionary."""
        # Filter out non-GeneralNoise fields
        filtered_config = {k: v for k, v in config.items() if k != "type"}
        return cls(**filtered_config)


def run_qasm(
    qasm: str,
    shots: int,
    noise_model: Optional[Any] = None,
    engine: Optional[QuantumEngine] = None,
    workers: Optional[int] = None,
    seed: Optional[int] = None,
) -> Dict[str, List[int]]:
    """Run a QASM simulation with specified parameters.
    
    NOTE: This function is provided for backward compatibility.
    Consider using the new unified API instead:
    
        from pecos_rslib import qasm_engine
        from pecos_rslib.programs import QasmProgram
        
        results = qasm_engine().program(QasmProgram.from_string(qasm)).to_sim().seed(42).noise(noise_model).run(shots)

    Args:
        qasm: QASM code as a string
        shots: Number of measurement shots to perform
        noise_model: Noise model instance (e.g., DepolarizingNoise(p=0.01)) or None for no noise
        engine: Quantum simulation engine (QuantumEngine.StateVector or QuantumEngine.SparseStabilizer)
        workers: Number of worker threads (None for default of 1)
        seed: Random seed for reproducibility (None for non-deterministic)

    Returns:
        Dict mapping register names to lists of measurement values (as integers).
        For example: {"c": [0, 3, 0, 3, ...]} for a Bell state measurement.

    Example:
        >>> from pecos_rslib.qasm_sim import run_qasm, DepolarizingNoise, QuantumEngine
        >>> qasm = '''
        ... OPENQASM 2.0;
        ... include "qelib1.inc";
        ... qreg q[2];
        ... creg c[2];
        ... h q[0];
        ... cx q[0], q[1];
        ... measure q -> c;
        ... '''
        >>> results = run_qasm(qasm, shots=1000, noise_model=DepolarizingNoise(p=0.01))
        >>> # Results are in columnar format
        >>> print(f"Got {len(results['c'])} measurements")
        >>> # Count occurrences of each measurement outcome
        >>> from collections import Counter
        >>> counts = Counter(results["c"])
        >>> print(counts)  # Should show roughly equal counts of 0 (00) and 3 (11)
    """
    # Use the new unified API with qasm_engine
    from pecos_rslib import qasm_engine
    from pecos_rslib.programs import QasmProgram
    
    sim_builder = qasm_engine().program(QasmProgram.from_string(qasm)).to_sim()
    
    if seed is not None:
        sim_builder = sim_builder.seed(seed)
    if workers is not None:
        sim_builder = sim_builder.workers(workers)
    elif workers is None:
        # Default to 1 worker as per docstring
        sim_builder = sim_builder.workers(1)
    if noise_model is not None:
        # Convert noise model if needed
        if isinstance(noise_model, NoiseModel):
            # Convert NoiseModel enum to appropriate builder
            if noise_model.value == "PassThrough":
                pass  # No noise
            elif noise_model.value == "Depolarizing":
                from pecos_rslib import depolarizing_noise
                sim_builder = sim_builder.noise(depolarizing_noise().with_uniform_probability(0.001))
            elif noise_model.value == "DepolarizingCustom":
                from pecos_rslib import depolarizing_noise
                sim_builder = sim_builder.noise(depolarizing_noise()
                    .with_prep_probability(0.001)
                    .with_meas_probability(0.001)
                    .with_p1_probability(0.001)
                    .with_p2_probability(0.002))
            elif noise_model.value == "BiasedDepolarizing":
                from pecos_rslib import biased_depolarizing_noise
                sim_builder = sim_builder.noise(biased_depolarizing_noise().with_uniform_probability(0.001))
            elif noise_model.value == "General":
                from pecos_rslib import general_noise
                sim_builder = sim_builder.noise(general_noise())
        elif isinstance(noise_model, DepolarizingNoise):
            from pecos_rslib import depolarizing_noise
            sim_builder = sim_builder.noise(depolarizing_noise().with_uniform_probability(noise_model.p))
        elif isinstance(noise_model, DepolarizingCustomNoise):
            from pecos_rslib import depolarizing_noise
            sim_builder = sim_builder.noise(depolarizing_noise()
                .with_prep_probability(noise_model.p_prep)
                .with_meas_probability(noise_model.p_meas)
                .with_p1_probability(noise_model.p1)
                .with_p2_probability(noise_model.p2))
        elif isinstance(noise_model, BiasedDepolarizingNoise):
            from pecos_rslib import biased_depolarizing_noise
            sim_builder = sim_builder.noise(biased_depolarizing_noise().with_uniform_probability(noise_model.p))
        elif isinstance(noise_model, GeneralNoise):
            from pecos_rslib import general_noise
            builder = general_noise()
            # Set parameters that are not None
            if noise_model.seed is not None:
                builder = builder.with_seed(noise_model.seed)
            if noise_model.p_prep is not None:
                builder = builder.with_prep_probability(noise_model.p_prep)
            if noise_model.p_meas_0 is not None and noise_model.p_meas_1 is not None:
                builder = builder.with_meas_0_probability(noise_model.p_meas_0).with_meas_1_probability(noise_model.p_meas_1)
            if noise_model.p1 is not None:
                builder = builder.with_p1_probability(noise_model.p1)
            if noise_model.p2 is not None:
                builder = builder.with_p2_probability(noise_model.p2)
            sim_builder = sim_builder.noise(builder)
        else:
            # Assume it's already a builder
            sim_builder = sim_builder.noise(noise_model)
    if engine is not None:
        # Convert engine if needed
        if isinstance(engine, QuantumEngine):
            if engine.value == "StateVector":
                from pecos_rslib import state_vector
                sim_builder = sim_builder.quantum(state_vector())
            elif engine.value == "SparseStabilizer":
                from pecos_rslib import sparse_stabilizer
                sim_builder = sim_builder.quantum(sparse_stabilizer())
        elif isinstance(engine, str):
            if engine == "StateVector":
                from pecos_rslib import state_vector
                sim_builder = sim_builder.quantum(state_vector())
            elif engine == "SparseStabilizer":
                from pecos_rslib import sparse_stabilizer
                sim_builder = sim_builder.quantum(sparse_stabilizer())
    
    # Run and convert to dict
    results = sim_builder.run(shots)
    # If it's a ShotVec, convert to dict
    if hasattr(results, 'to_dict'):
        return results.to_dict()
    return results


def qasm_sim(qasm: str) -> QasmSimulationBuilder:
    """Create a QASM simulation builder for flexible configuration.

    This provides a builder pattern for QASM simulations, allowing you to
    build once and run multiple times with different shot counts.

    Args:
        qasm: QASM code as a string

    Returns:
        QasmSimulationBuilder that can be configured and run

    Example:
        >>> from pecos_rslib.qasm_sim import qasm_sim, DepolarizingNoise, QuantumEngine
        >>> qasm = '''
        ... OPENQASM 2.0;
        ... include "qelib1.inc";
        ... qreg q[2];
        ... creg c[2];
        ... h q[0];
        ... cx q[0], q[1];
        ... measure q -> c;
        ... '''
        >>> # Build once, run multiple times
        >>> sim = qasm_sim(qasm).seed(42).noise(DepolarizingNoise(p=0.01)).build()
        >>>
        >>> results_100 = sim.run(100)
        >>> results_1000 = sim.run(1000)
        >>>
        >>> # Or run directly without building
        >>> results = (
        ...     qasm_sim(qasm).noise(DepolarizingNoise(p=0.01)).workers(4).run(1000)
        ... )
        >>>
        >>> # Use Rust-native builder with fluent chaining
        >>> from pecos_rslib.qasm_sim import GeneralNoiseModelBuilder
        >>> builder = (
        ...     GeneralNoiseModelBuilder()
        ...     .with_seed(42)
        ...     .with_p1_probability(0.001)
        ...     .with_p2_probability(0.01)
        ... )
        >>>
        >>> # Direct configuration with method chaining (like Rust API)
        >>> sim = (
        ...     qasm_sim(qasm)
        ...     .seed(42)
        ...     .auto_workers()
        ...     .noise(builder)
        ...     .quantum_engine(QuantumEngine.StateVector)
        ...     .with_binary_string_format()
        ...     .build()
        ... )
        >>> results = sim.run(1000)
        >>>
        >>> # Using WebAssembly functions (requires wasm feature)
        >>> qasm_with_wasm = '''
        ... OPENQASM 2.0;
        ... creg a[10];
        ... creg b[10];
        ... creg result[10];
        ... a = 5;
        ... b = 3;
        ... result = add(a, b);  // Call WASM function
        ... '''
        >>> # Run with WASM module
        >>> results = qasm_sim(qasm_with_wasm).wasm("add.wasm").run(100)
    """
    # The old bindings are gone, provide a compatibility shim
    # This function should return a builder-like object that supports the old API
    from pecos_rslib import qasm_engine
    from pecos_rslib.programs import QasmProgram
    
    class QasmSimulationBuilderCompat:
        """Compatibility wrapper for old QasmSimulationBuilder API."""
        def __init__(self, qasm: str):
            self._qasm = qasm
            self._engine_builder = None
            self._sim_builder = None
            self._binary_format = False
            self._wasm_path = None
            self._seed = None
            self._workers = None
            self._noise_model = None
            self._quantum_engine = None
            
        def _get_sim_builder(self):
            """Get or create the sim builder."""
            if self._sim_builder is None:
                # Create engine builder if needed
                if self._engine_builder is None:
                    self._engine_builder = qasm_engine().program(QasmProgram.from_string(self._qasm))
                
                # WASM support - apply to engine builder before creating sim builder
                if self._wasm_path:
                    if hasattr(self._engine_builder, 'wasm'):
                        self._engine_builder = self._engine_builder.wasm(self._wasm_path)
                    else:
                        import warnings
                        warnings.warn(
                            "WASM support is not available. Make sure PECOS was compiled with the 'wasm' feature."
                        )
                
                # Convert to sim builder using new API
                self._sim_builder = self._engine_builder.to_sim()
                
                # Apply stored settings
                if self._seed is not None:
                    self._sim_builder = self._sim_builder.seed(self._seed)
                if self._workers is not None:
                    self._sim_builder = self._sim_builder.workers(self._workers)
                if self._noise_model is not None:
                    self._sim_builder = self._apply_noise_model(self._sim_builder, self._noise_model)
                if self._quantum_engine is not None:
                    self._sim_builder = self._apply_quantum_engine(self._sim_builder, self._quantum_engine)
            return self._sim_builder
            
        def seed(self, seed: int):
            self._seed = seed
            return self
            
        def noise(self, noise_model):
            self._noise_model = noise_model
            return self
        
        def _apply_noise_model(self, sim_builder, noise_model):
            if noise_model is None:
                return sim_builder
                
            # Convert old noise model types to new builders if needed
            if isinstance(noise_model, DepolarizingNoise):
                from pecos_rslib import depolarizing_noise
                builder = depolarizing_noise().with_uniform_probability(noise_model.p)
                return sim_builder.noise(builder)
            elif isinstance(noise_model, DepolarizingCustomNoise):
                from pecos_rslib import depolarizing_noise
                builder = (depolarizing_noise()
                          .with_prep_probability(noise_model.p_prep)
                          .with_meas_probability(noise_model.p_meas)
                          .with_p1_probability(noise_model.p1)
                          .with_p2_probability(noise_model.p2))
                return sim_builder.noise(builder)
            elif isinstance(noise_model, BiasedDepolarizingNoise):
                from pecos_rslib import biased_depolarizing_noise
                builder = biased_depolarizing_noise().with_uniform_probability(noise_model.p)
                return sim_builder.noise(builder)
            elif isinstance(noise_model, GeneralNoise):
                # For GeneralNoise, create a GeneralNoiseModelBuilder and set parameters
                from pecos_rslib import general_noise
                builder = general_noise()
                # Set all the parameters that are not None
                if noise_model.seed is not None:
                    builder = builder.with_seed(noise_model.seed)
                if noise_model.p_prep is not None:
                    builder = builder.with_prep_probability(noise_model.p_prep)
                if noise_model.p_meas_0 is not None and noise_model.p_meas_1 is not None:
                    builder = builder.with_meas_0_probability(noise_model.p_meas_0).with_meas_1_probability(noise_model.p_meas_1)
                if noise_model.p1 is not None:
                    builder = builder.with_p1_probability(noise_model.p1)
                if noise_model.p2 is not None:
                    builder = builder.with_p2_probability(noise_model.p2)
                return sim_builder.noise(builder)
            else:
                # Assume it's already a builder
                return sim_builder.noise(noise_model)
            
        def workers(self, workers: int):
            self._workers = workers
            return self
            
        def auto_workers(self):
            # The new API doesn't have auto_workers, just use default
            return self
            
        def quantum_engine(self, engine):
            self._quantum_engine = engine
            return self
            
        def _apply_quantum_engine(self, sim_builder, engine):
            if engine is None:
                return sim_builder
                
            if engine == "StateVector" or (isinstance(engine, str) and engine == "StateVector"):
                from pecos_rslib import state_vector
                return sim_builder.quantum(state_vector())
            elif engine == "SparseStabilizer" or (isinstance(engine, str) and engine == "SparseStabilizer"):
                from pecos_rslib import sparse_stabilizer
                return sim_builder.quantum(sparse_stabilizer())
            elif hasattr(engine, 'value'):
                # Handle QuantumEngine enum-like objects
                if engine.value == "StateVector":
                    from pecos_rslib import state_vector
                    return sim_builder.quantum(state_vector())
                elif engine.value == "SparseStabilizer":
                    from pecos_rslib import sparse_stabilizer
                    return sim_builder.quantum(sparse_stabilizer())
            return sim_builder
            
        def with_binary_string_format(self):
            # Track that we want binary format
            self._binary_format = True
            return self
            
        def wasm(self, wasm_path: str):
            # Store the WASM path to be applied to the engine builder
            self._wasm_path = wasm_path
            if self._sim_builder is not None:
                import warnings
                warnings.warn("WASM path set after sim builder was created. WASM will not be available.")
            return self
            
        def build(self):
            # Build the simulation and return a wrapper
            self._get_sim_builder()
            sim = self._sim_builder.build()
            return QasmSimulationCompat(sim, self._binary_format)
            
        def run(self, shots: int):
            self._get_sim_builder()
            results = self._sim_builder.run(shots)
            # Convert ShotVec to dict for backward compatibility
            if hasattr(results, 'to_dict'):
                if self._binary_format and hasattr(results, 'to_binary_dict'):
                    return results.to_binary_dict()
                else:
                    return results.to_dict()
            return results
    
    class QasmSimulationCompat:
        """Compatibility wrapper for built simulation object."""
        def __init__(self, sim, binary_format=False):
            self._sim = sim
            self._binary_format = binary_format
            
        def run(self, shots: int):
            results = self._sim.run(shots)
            # Convert ShotVec to dict for backward compatibility
            if hasattr(results, 'to_dict'):
                if self._binary_format and hasattr(results, 'to_binary_dict'):
                    return results.to_binary_dict()
                else:
                    return results.to_dict()
            return results
    
    # Define helper wrapper for old API
    class OldApiWrapper:
        """Wrapper to make old API behave like new SimBuilder."""
        def __init__(self, old_builder):
            self._old_builder = old_builder
            
        def seed(self, seed: int):
            self._old_builder = self._old_builder.seed(seed)
            return self
            
        def workers(self, workers: int):
            self._old_builder = self._old_builder.workers(workers)
            return self
            
        def noise(self, noise_model):
            self._old_builder = self._old_builder.noise(noise_model)
            return self
            
        def quantum(self, engine):
            self._old_builder = self._old_builder.quantum_engine(engine)
            return self
            
        def build(self):
            return self._old_builder.build()
            
        def run(self, shots: int):
            return self._old_builder.run(shots)
    
    return QasmSimulationBuilderCompat(qasm)


def get_noise_models() -> List[str]:
    """Get a list of available noise model names.

    Returns:
        List of string names of available noise models, such as
        'PassThrough', 'Depolarizing', 'DepolarizingCustom', etc.

    Example:
        >>> from pecos_rslib.qasm_sim import get_noise_models
        >>> noise_models = get_noise_models()
        >>> print(noise_models)
        ['PassThrough', 'Depolarizing', 'DepolarizingCustom', ...]
    """
    # Return hardcoded list since the old bindings are gone
    return ['PassThrough', 'Depolarizing', 'DepolarizingCustom', 'BiasedDepolarizing', 'General']


def get_quantum_engines() -> List[str]:
    """Get a list of available quantum engine names.

    Returns:
        List of string names of available quantum engines, such as
        'StateVector', 'SparseStabilizer', etc.

    Example:
        >>> from pecos_rslib.qasm_sim import get_quantum_engines
        >>> engines = get_quantum_engines()
        >>> print(engines)
        ['StateVector', 'SparseStabilizer']
    """
    # Return hardcoded list since the old bindings are gone
    return ['StateVector', 'SparseStabilizer']
