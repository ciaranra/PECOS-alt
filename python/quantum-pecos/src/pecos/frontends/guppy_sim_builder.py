"""Builder pattern for Guppy simulation matching qasm_sim API.

This module provides a builder pattern interface for running Guppy quantum programs,
similar to how qasm_sim works but keeping everything in memory for performance.
"""

__all__ = [
    'guppy_sim',
    'GuppySimulation',
    'GuppySimulationBuilder',
    'GuppySimulationConfig',
]

from collections.abc import Callable
from dataclasses import dataclass
from typing import Any, TypeVar, Optional, Dict, List
import time
import os
import tempfile
import atexit

from pecos.compilation_pipeline import compile_guppy_to_hugr, compile_hugr_to_llvm

T = TypeVar("T")

try:
    from guppylang import guppy
    GUPPY_AVAILABLE = True
except ImportError:
    GUPPY_AVAILABLE = False
    guppy = None

# Try to import Rust bindings
try:
    from pecos_rslib import (
        execute_llvm as rust_execute_llvm,
        reset_llvm_runtime,
        llvm_engine,
        LlvmProgram,
        state_vector,
        sparse_stabilizer,
        DepolarizingNoiseModelBuilder,
        BiasedDepolarizingNoiseModelBuilder,
        GeneralNoiseModelBuilder,
    )
    RUST_EXECUTION_AVAILABLE = True
except ImportError:
    RUST_EXECUTION_AVAILABLE = False
    llvm_engine = None


@dataclass
class GuppySimulationConfig:
    """Configuration for Guppy simulation."""
    seed: Optional[int] = None
    workers: Optional[int] = None
    noise_model: Optional[Any] = None
    engine: Optional[str] = None
    verbose: bool = False
    debug: bool = False
    optimize: bool = True
    binary_string_format: bool = False  # Match qasm_sim option
    keep_intermediate_files: bool = False  # Keep compilation artifacts
    max_qubits: Optional[int] = None  # Maximum number of qubits to simulate (REQUIRED)


class GuppySimulation:
    """A built Guppy simulation ready to run multiple times.
    
    This class holds the compiled LLVM IR in memory and manages temporary
    files efficiently for multiple runs.
    """
    
    # Class variable to track all temporary files for cleanup
    _temp_files: List[str] = []
    
    def __init__(self, 
                 guppy_func: Callable,
                 config: GuppySimulationConfig,
                 hugr_bytes: bytes,
                 llvm_ir: str):
        """Initialize a built simulation.
        
        Args:
            guppy_func: The original Guppy function
            config: Simulation configuration
            hugr_bytes: Compiled HUGR bytes
            llvm_ir: Compiled LLVM IR string
        """
        self.guppy_func = guppy_func
        self._config = config
        self.hugr_bytes = hugr_bytes
        self.llvm_ir = llvm_ir
        # Get a short function name for file naming
        if hasattr(guppy_func, "__name__"):
            self.function_name = guppy_func.__name__
        elif hasattr(guppy_func, "name"):
            self.function_name = guppy_func.name
        else:
            # Use a hash of the function for long GuppyDefinition strings
            import hashlib
            self.function_name = f"guppy_{hashlib.md5(str(guppy_func).encode()).hexdigest()[:8]}"
        
        # Track intermediate files directory if keeping files
        self.temp_dir = None
        
        # Create a persistent temp file for this simulation
        self._temp_file = None
        self._create_temp_file()
        
        # Track execution statistics
        self.total_shots = 0
        self.total_runs = 0
    
    def _convert_noise_model(self, noise_model):
        """Convert old-style noise model classes to new builder pattern.
        
        Args:
            noise_model: Old-style noise model object or new builder
            
        Returns:
            Noise model builder or None
        """
        # Import here to avoid circular dependencies
        try:
            from pecos_rslib.qasm_sim import (
                DepolarizingNoise, 
                DepolarizingCustomNoise,
                BiasedDepolarizingNoise,
                PassThroughNoise
            )
        except ImportError:
            # If can't import, assume it's already a builder
            return noise_model
            
        # Check if it's already a builder
        if hasattr(noise_model, 'inner'):
            return noise_model
            
        # Convert based on type
        type_name = type(noise_model).__name__
        
        if isinstance(noise_model, DepolarizingNoise):
            # Uniform depolarizing noise
            return DepolarizingNoiseModelBuilder().with_uniform_probability(noise_model.p)
        elif isinstance(noise_model, DepolarizingCustomNoise):
            # Custom depolarizing noise
            builder = DepolarizingNoiseModelBuilder()
            builder = builder.with_prep_probability(noise_model.p_prep)
            builder = builder.with_meas_probability(noise_model.p_meas)
            builder = builder.with_p1_probability(noise_model.p1)
            builder = builder.with_p2_probability(noise_model.p2)
            return builder
        elif isinstance(noise_model, BiasedDepolarizingNoise):
            # For biased depolarizing, use uniform probability
            # (BiasedDepolarizingNoiseModelBuilder might need different parameters)
            return BiasedDepolarizingNoiseModelBuilder().with_uniform_probability(noise_model.p)
        elif isinstance(noise_model, PassThroughNoise):
            # No noise - return None
            return None
        else:
            # Unknown type, assume it's already a builder or compatible
            return noise_model
    
    def _create_temp_file(self):
        """Create a temporary file for LLVM IR that persists across runs."""
        if self._config.keep_intermediate_files:
            # Create a persistent directory for intermediate files
            if not self.temp_dir:
                self.temp_dir = tempfile.mkdtemp(prefix=f'guppy_{self.function_name}_')
                if self._config.verbose:
                    print(f"Created intermediate files directory: {self.temp_dir}")
            
            # Create file in the persistent directory
            path = os.path.join(self.temp_dir, f'{self.function_name}.ll')
            with open(path, 'w') as f:
                f.write(self.llvm_ir)
            
            # Also save HUGR for debugging
            hugr_path = os.path.join(self.temp_dir, f'{self.function_name}.hugr')
            with open(hugr_path, 'wb') as f:
                f.write(self.hugr_bytes)
            
            self._temp_file = path
            # Don't add to cleanup list if keeping files
        else:
            # Create temp file that will be cleaned up
            fd, path = tempfile.mkstemp(suffix='.ll', prefix=f'guppy_{self.function_name}_')
            
            # Write LLVM IR to file
            with os.fdopen(fd, 'w') as f:
                f.write(self.llvm_ir)
            
            self._temp_file = path
            GuppySimulation._temp_files.append(path)
        
        if self._config.verbose:
            print(f"Created {'persistent' if self._config.keep_intermediate_files else 'temporary'} file: {self._temp_file}")
    
    @classmethod
    def cleanup_all_temp_files(cls):
        """Clean up all temporary files created by simulations."""
        for path in cls._temp_files:
            try:
                if os.path.exists(path):
                    os.unlink(path)
            except Exception:
                pass  # Ignore cleanup errors
        cls._temp_files.clear()
    
    def run(self, shots: int) -> Dict[str, Any]:
        """Run the simulation with the given number of shots.
        
        Args:
            shots: Number of measurement shots
            
        Returns:
            Dictionary with results in columnar format, matching qasm_sim
        """
        if self._config.verbose:
            print(f"Running {self.function_name} for {shots} shots")
        
        start_time = time.time()
        
        # Reset LLVM runtime if available
        if RUST_EXECUTION_AVAILABLE:
            try:
                reset_llvm_runtime()
            except Exception as e:
                if self._config.verbose:
                    print(f"[WARNING] Runtime reset failed: {e}")
        
        # Execute using the new unified API
        if RUST_EXECUTION_AVAILABLE and llvm_engine is not None:
            
            # Build using the new unified API
            # Read the LLVM IR from the temp file
            with open(self._temp_file, 'r') as f:
                llvm_ir = f.read()
            
            builder = (
                llvm_engine()
                .program(LlvmProgram.from_string(llvm_ir))
                .to_sim()
            )
            
            # Configure max_qubits if specified
            if self._config.max_qubits is not None:
                builder = builder.qubits(self._config.max_qubits)
            
            # Configure seed
            if self._config.seed is not None:
                builder = builder.seed(self._config.seed)
            
            # Configure workers
            if self._config.workers is not None:
                builder = builder.workers(self._config.workers)
            
            # Configure noise model
            if self._config.noise_model is not None:
                # Convert old-style noise classes to new builder pattern
                noise_builder = self._convert_noise_model(self._config.noise_model)
                if noise_builder is not None:
                    builder = builder.noise(noise_builder)
            
            # Configure quantum engine
            if self._config.engine is not None:
                if self._config.engine.lower() == "statevector":
                    builder = builder.quantum(state_vector())
                elif self._config.engine.lower() == "sparsestabilizer":
                    builder = builder.quantum(sparse_stabilizer())
            else:
                # Default to state vector engine to support all gates
                builder = builder.quantum(state_vector())
            
            # Run simulation
            results = builder.run(shots)
            
            # The new API returns ShotVec, convert to dict for compatibility
            if hasattr(results, 'to_dict'):
                raw_results = results.to_dict()
            else:
                # Fallback for older format
                raw_results = results
        else:
            # Fallback to basic execute_llvm
            if RUST_EXECUTION_AVAILABLE:
                result = rust_execute_llvm(
                    self._temp_file,
                    shots,
                    self._config.seed,
                    None,  # basic execute doesn't support noise
                    self._config.workers,
                )
                
                if result.get("execution_successful", False):
                    raw_results = result.get("results", [])
                else:
                    raise RuntimeError(f"Execution failed: {result.get('error', 'Unknown error')}")
            else:
                raise RuntimeError("Rust execution backend not available")
        
        execution_time = time.time() - start_time
        
        # If using llvm_engine, results are already in columnar format
        if RUST_EXECUTION_AVAILABLE and llvm_engine is not None and isinstance(raw_results, dict):
            # llvm_engine returns results with register names like "c", "c1" etc.
            # We need to check if this is a multi-value return that should be combined
            columnar_results = self._process_llvm_engine_results(raw_results)
        else:
            # Convert to columnar format like qasm_sim
            columnar_results = self._format_results_columnar(raw_results)
        
        # Update statistics
        self.total_shots += shots
        self.total_runs += 1
        
        # Return in same format as qasm_sim
        result_dict = columnar_results.copy()
        
        # Always add metadata for consistency with qasm_sim
        result_dict["_metadata"] = {
            "shots": shots,
            "execution_time": execution_time,
            "function_name": self.function_name,
            "total_runs": self.total_runs,
            "total_shots": self.total_shots,
        }
        
        return result_dict
    
    def _format_results_columnar(self, raw_results: List[Any]) -> Dict[str, List[Any]]:
        """Format results in columnar format like qasm_sim.
        
        For a Bell state returning tuple[bool, bool], qasm_sim returns:
        {"c": [0, 3, 0, 3, ...]} where 0 = |00⟩ and 3 = |11⟩
        
        We'll use "result" as the default register name for Guppy returns.
        """
        if not raw_results:
            return {"result": []}
        
        # Handle different result types
        if isinstance(raw_results[0], (list, tuple)):
            # Multiple return values - convert tuple of bools to integer
            if self._config.binary_string_format:
                # Return as binary strings like "00", "11"
                values = []
                for res in raw_results:
                    binary_str = ''.join('1' if b else '0' for b in res)
                    values.append(binary_str)
            else:
                # Return as integers (default)
                values = []
                for res in raw_results:
                    # Convert bool tuple to integer representation
                    val = 0
                    for i, b in enumerate(res):
                        if b:
                            val |= (1 << i)
                    values.append(val)
            return {"result": values}
        else:
            # Single return value - keep as is
            if self._config.binary_string_format and isinstance(raw_results[0], bool):
                # Convert bools to "0" or "1"
                return {"result": ['1' if r else '0' for r in raw_results]}
            else:
                return {"result": raw_results}
    
    def _process_llvm_engine_results(self, raw_results: Dict[str, List[Any]]) -> Dict[str, List[Any]]:
        """Process results from llvm_engine.
        
        Simply return the results as-is since we're not maintaining backward compatibility.
        The new format uses "result" as the key.
        """
        return raw_results
    
    def __del__(self):
        """Clean up temporary file when simulation is deleted."""
        # Don't clean up if we're keeping intermediate files
        if hasattr(self, '_config') and self._config.keep_intermediate_files:
            return
            
        if hasattr(self, '_temp_file') and self._temp_file:
            try:
                if os.path.exists(self._temp_file):
                    os.unlink(self._temp_file)
                # Remove from class list
                if self._temp_file in GuppySimulation._temp_files:
                    GuppySimulation._temp_files.remove(self._temp_file)
            except Exception:
                pass  # Ignore cleanup errors


# Register cleanup function
atexit.register(GuppySimulation.cleanup_all_temp_files)


class GuppySimulationBuilder:
    """Builder for creating Guppy simulations with fluent interface.
    
    Matches the qasm_sim builder pattern API.
    """
    
    def __init__(self, guppy_func: Callable, max_qubits: int):
        """Initialize builder with a Guppy function and max qubits.
        
        Args:
            guppy_func: A function decorated with @guppy
            max_qubits: Maximum number of qubits to simulate
            
        Raises:
            ValueError: If function is not a Guppy function or max_qubits invalid
        """
        if not GUPPY_AVAILABLE:
            raise ImportError("guppylang is not available. Install with: pip install quantum-pecos[guppy]")
        
        # Validate it's a Guppy function
        is_guppy = (
            hasattr(guppy_func, "_guppy_compiled") or
            hasattr(guppy_func, "name") or
            str(type(guppy_func)).find("GuppyDefinition") != -1
        )
        
        if not is_guppy:
            func_name = getattr(guppy_func, "__name__", str(guppy_func))
            raise ValueError(f"Function {func_name} must be decorated with @guppy")
        
        # Validate max_qubits
        if not isinstance(max_qubits, int) or max_qubits < 1:
            raise ValueError("max_qubits must be a positive integer")
        
        self.guppy_func = guppy_func
        self._config = GuppySimulationConfig()
        self._config.max_qubits = max_qubits  # Set max_qubits immediately
        self._built = False
        self._simulation: Optional[GuppySimulation] = None
    
    def seed(self, seed: int) -> "GuppySimulationBuilder":
        """Set random seed for reproducible results."""
        self._config.seed = seed
        return self
    
    def workers(self, num_workers: int) -> "GuppySimulationBuilder":
        """Set number of worker threads."""
        self._config.workers = num_workers
        return self
    
    def noise(self, noise_model: Any) -> "GuppySimulationBuilder":
        """Set noise model for simulation."""
        self._config.noise_model = noise_model
        return self
    
    def engine(self, engine: str) -> "GuppySimulationBuilder":
        """Set quantum simulation engine (StateVector or SparseStabilizer)."""
        self._config.engine = engine
        return self
    
    def verbose(self, enable: bool = True) -> "GuppySimulationBuilder":
        """Enable verbose output."""
        self._config.verbose = enable
        return self
    
    def debug(self, enable: bool = True) -> "GuppySimulationBuilder":
        """Enable debug information."""
        self._config.debug = enable
        return self
    
    def optimize(self, enable: bool = True) -> "GuppySimulationBuilder":
        """Enable LLVM optimizations."""
        self._config.optimize = enable
        return self
    
    def binary_string_format(self, enable: bool = True) -> "GuppySimulationBuilder":
        """Return results as binary strings instead of integers."""
        self._config.binary_string_format = enable
        return self
    
    def keep_intermediate_files(self, enable: bool = True) -> "GuppySimulationBuilder":
        """Keep intermediate compilation files (HUGR and LLVM IR) for debugging."""
        self._config.keep_intermediate_files = enable
        return self
    
    
    def config(self, config_dict: Dict[str, Any]) -> "GuppySimulationBuilder":
        """Apply configuration from dictionary."""
        for key, value in config_dict.items():
            if hasattr(self._config, key):
                setattr(self._config, key, value)
        return self
    
    def build(self) -> GuppySimulation:
        """Build the simulation, compiling once for multiple runs.
        
        Returns:
            GuppySimulation instance ready to run
            
        Raises:
            ValueError: If max_qubits has not been set
        """
        if self._built and self._simulation:
            return self._simulation
        
        # max_qubits is now guaranteed to be set in constructor
        
        if self._config.verbose:
            func_name = getattr(self.guppy_func, "__name__", "guppy_function")
            print(f"Building simulation for {func_name}")
        
        # Step 1: Compile Guppy to HUGR (must be done in Python)
        start_time = time.time()
        hugr_bytes = compile_guppy_to_hugr(self.guppy_func)
        hugr_time = time.time() - start_time
        
        if self._config.verbose:
            print(f"  Guppy → HUGR: {hugr_time:.4f}s ({len(hugr_bytes)} bytes)")
        
        # Step 2: Compile HUGR to LLVM (uses Rust via PyO3)
        start_time = time.time()
        llvm_ir = compile_hugr_to_llvm(
            hugr_bytes
        )
        llvm_time = time.time() - start_time
        
        if self._config.verbose:
            print(f"  HUGR → LLVM: {llvm_time:.4f}s ({len(llvm_ir)} bytes)")
            print(f"  Total compilation: {hugr_time + llvm_time:.4f}s")
        
        # Create the simulation object
        self._simulation = GuppySimulation(
            self.guppy_func,
            self._config,
            hugr_bytes,
            llvm_ir
        )
        self._built = True
        
        return self._simulation
    
    def run(self, shots: int) -> Dict[str, Any]:
        """Build and run simulation in one call.
        
        Args:
            shots: Number of measurement shots
            
        Returns:
            Results in columnar format like qasm_sim
        """
        sim = self.build()
        return sim.run(shots)


# Helper functions to create noise models
def depolarizing_noise(p1=0.0, p2=0.0, pn=0.0):
    """Create a depolarizing noise model."""
    builder = DepolarizingNoiseModelBuilder()
    if p1 > 0:
        builder = builder.with_p1_probability(p1)
    if p2 > 0:
        builder = builder.with_p2_probability(p2)
    if pn > 0:
        builder = builder.with_pn_probability(pn)
    return builder


def biased_depolarizing_noise(px=0.0, py=0.0, pz=0.0):
    """Create a biased depolarizing noise model."""
    return BiasedDepolarizingNoiseModelBuilder(px, py, pz)


def guppy_sim(guppy_func: Callable, max_qubits: int) -> GuppySimulationBuilder:
    """Create a Guppy simulation builder for flexible configuration.
    
    This provides a builder pattern for Guppy simulations matching qasm_sim,
    allowing you to build once and run multiple times with different shot counts.
    
    Args:
        guppy_func: A function decorated with @guppy
        max_qubits: Maximum number of qubits to simulate
        
    Returns:
        GuppySimulationBuilder that can be configured and run
        
    Example:
        >>> from guppylang import guppy
        >>> from guppylang.std.quantum import qubit, h, cx, measure
        >>> 
        >>> @guppy
        ... def bell_state() -> tuple[bool, bool]:
        ...     q0, q1 = qubit(), qubit()
        ...     h(q0)
        ...     cx(q0, q1)
        ...     return measure(q0), measure(q1)
        ...
        >>> # Build once, run multiple times
        >>> sim = guppy_sim(bell_state, max_qubits=10).seed(42).build()
        >>> results_100 = sim.run(100)
        >>> results_1000 = sim.run(1000)
        >>>
        >>> # Or run directly without building
        >>> results = guppy_sim(bell_state, max_qubits=10).seed(42).run(1000)
        >>>
        >>> # With binary string format (like qasm_sim)
        >>> results = guppy_sim(bell_state, max_qubits=10).binary_string_format().run(100)
        >>> # Results: {"_result": ["00", "11", "00", ...]}
    """
    return GuppySimulationBuilder(guppy_func, max_qubits)