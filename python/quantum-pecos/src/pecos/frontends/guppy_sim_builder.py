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
    )
    from pecos_rslib.llvm_sim import (
        llvm_sim,
        DepolarizingNoise,
        BiasedDepolarizingNoise,
        DepolarizingCustomNoise,
        PassThroughNoise,
    )
    RUST_EXECUTION_AVAILABLE = True
except ImportError:
    RUST_EXECUTION_AVAILABLE = False
    llvm_sim = None


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
        
        # Execute using llvm_sim with full feature support
        if RUST_EXECUTION_AVAILABLE and llvm_sim is not None:
            # Build llvm_sim with our configuration
            builder = llvm_sim(self._temp_file)
            
            # Configure seed
            if self._config.seed is not None:
                builder = builder.seed(self._config.seed)
            
            # Configure workers
            if self._config.workers is not None:
                builder = builder.workers(self._config.workers)
            
            # Configure noise model
            if self._config.noise_model is not None:
                builder = builder.noise(self._config.noise_model)
            
            # Configure engine
            if self._config.engine is not None:
                if self._config.engine.lower() == "statevector":
                    builder = builder.with_state_vector_engine()
                elif self._config.engine.lower() == "sparsestabilizer":
                    builder = builder.with_sparse_stabilizer_engine()
            
            # Configure verbosity and debug
            if self._config.verbose:
                builder = builder.verbose(True)
            if self._config.debug:
                builder = builder.debug(True)
            
            # Run simulation
            results = builder.run(shots)
            
            # llvm_sim returns columnar format, we need to convert back to raw for our formatter
            # For now, just store the results dict and handle in formatting
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
        
        # If using llvm_sim, results are already in columnar format
        if RUST_EXECUTION_AVAILABLE and llvm_sim is not None and isinstance(raw_results, dict):
            # llvm_sim returns results with register names like "c", "c1" etc.
            # We need to check if this is a multi-value return that should be combined
            columnar_results = self._process_llvm_sim_results(raw_results)
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
        
        We'll use "_result" as the default register name for Guppy returns.
        """
        if not raw_results:
            return {"_result": []}
        
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
            return {"_result": values}
        else:
            # Single return value - keep as is
            if self._config.binary_string_format and isinstance(raw_results[0], bool):
                # Convert bools to "0" or "1"
                return {"_result": ['1' if r else '0' for r in raw_results]}
            else:
                return {"_result": raw_results}
    
    def _process_llvm_sim_results(self, raw_results: Dict[str, List[Any]]) -> Dict[str, List[Any]]:
        """Process results from llvm_sim, which may have multiple _result_N registers.
        
        For Guppy functions returning tuples, llvm_sim creates separate registers 
        (_result_0, _result_1, etc.) for each measurement. We need to combine these
        into a single "_result" register with encoded values for compatibility.
        """
        # Check if we have multiple _result_N registers
        result_registers = [(k, v) for k, v in raw_results.items() if k.startswith("_result_")]
        
        if len(result_registers) == 0:
            # No _result_N registers, return as-is
            return raw_results
        elif len(result_registers) == 1:
            # Single result register - rename to "_result" for compatibility
            key, values = result_registers[0]
            if key == "_result_0":
                # Rename _result_0 to _result
                result = raw_results.copy()
                result["_result"] = result.pop("_result_0")
                return result
            return raw_results
        else:
            # Multiple result registers - need to combine into tuple representation
            # Sort by register number to ensure consistent ordering
            result_registers.sort(key=lambda x: int(x[0].split("_")[-1]))
            
            # Get the number of shots from the first register
            num_shots = len(result_registers[0][1])
            
            # Combine the results shot by shot
            combined_results = []
            for shot_idx in range(num_shots):
                if self._config.binary_string_format:
                    # Create binary string representation
                    bits = []
                    for reg_name, reg_values in result_registers:
                        bit = reg_values[shot_idx]
                        bits.append('1' if bit else '0')
                    combined_results.append(''.join(bits))
                else:
                    # Create integer representation (bit encoding)
                    value = 0
                    for i, (reg_name, reg_values) in enumerate(result_registers):
                        bit = reg_values[shot_idx]
                        if bit:
                            value |= (1 << i)
                    combined_results.append(value)
            
            # Return with single "_result" register
            result = {"_result": combined_results}
            
            # Copy over any metadata
            for k, v in raw_results.items():
                if not k.startswith("_result_"):
                    result[k] = v
            
            return result
    
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
    
    def __init__(self, guppy_func: Callable):
        """Initialize builder with a Guppy function.
        
        Args:
            guppy_func: A function decorated with @guppy
            
        Raises:
            ValueError: If function is not a Guppy function
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
        
        self.guppy_func = guppy_func
        self._config = GuppySimulationConfig()
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
        """
        if self._built and self._simulation:
            return self._simulation
        
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


def guppy_sim(guppy_func: Callable) -> GuppySimulationBuilder:
    """Create a Guppy simulation builder for flexible configuration.
    
    This provides a builder pattern for Guppy simulations matching qasm_sim,
    allowing you to build once and run multiple times with different shot counts.
    
    Args:
        guppy_func: A function decorated with @guppy
        
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
        >>> sim = guppy_sim(bell_state).seed(42).build()
        >>> results_100 = sim.run(100)
        >>> results_1000 = sim.run(1000)
        >>>
        >>> # Or run directly without building
        >>> results = guppy_sim(bell_state).seed(42).run(1000)
        >>>
        >>> # With binary string format (like qasm_sim)
        >>> results = guppy_sim(bell_state).binary_string_format().run(100)
        >>> # Results: {"_result": ["00", "11", "00", ...]}
    """
    return GuppySimulationBuilder(guppy_func)