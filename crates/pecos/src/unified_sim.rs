//! Unified simulation API with automatic engine selection
//!
//! Convenience wrapper around the lower-level `sim_builder`
//! from pecos-engines, adding automatic engine selection based on program type.

use pecos_core::errors::PecosError;
use pecos_engines::{ClassicalControlEngineBuilder, MonteCarloEngine, SimBuilder, sim_builder};
use pecos_programs::Program;
use pecos_qasm::qasm_engine;
#[cfg(feature = "qis")]
use pecos_qis::{IntoQisInterface, qis_engine};

/// Set up a QIS engine with Selene runtime and Helios interface for the given program.
#[cfg(feature = "qis")]
fn build_qis_engine<P: IntoQisInterface + 'static>(
    program: P,
) -> Result<pecos_qis::QisEngineBuilder, PecosError> {
    let selene_runtime = crate::selene_simple_runtime()
        .map_err(|e| PecosError::Generic(format!("Failed to load Selene runtime: {e}")))?;
    let helios_builder = crate::helios_interface_builder();
    qis_engine()
        .runtime(selene_runtime)
        .interface(helios_builder)
        .try_program(program)
        .map_err(|e| PecosError::Generic(format!("Failed to load program: {e}")))
}

/// Which simulation stack executes the program.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SimStack {
    /// The engine/`EngineSystem` stack in `pecos-engines` (current default).
    #[default]
    Engines,
    /// The data-oriented `pecos-neo` stack (experimental).
    ///
    /// Requires building pecos with the `neo` cargo feature. Currently
    /// routes QASM and HUGR programs with the default quantum backend;
    /// explicit `.classical()`, `.noise()`, and `.quantum()` configuration
    /// is not yet translated and is rejected with an error at `run()`.
    Neo,
}

/// Extension trait for `SimBuilder` to add program-based methods
pub trait SimBuilderExt {
    /// Set the program and automatically select an appropriate engine
    ///
    /// This method inspects the program type and selects:
    /// - QASM programs → QASM engine
    /// - QIS programs → QIS control engine (Selene Helios interface)
    /// - HUGR programs → QIS control engine (Selene Helios interface)
    /// - WASM/WAT programs → Error (not yet supported)
    /// - PHIR JSON programs → Error (not yet supported)
    ///
    /// The engine can be overridden by calling `.classical()` after this method.
    fn program<P: Into<Program>>(self, program: P) -> ProgrammedSimBuilder;
}

impl SimBuilderExt for SimBuilder {
    fn program<P: Into<Program>>(self, program: P) -> ProgrammedSimBuilder {
        ProgrammedSimBuilder {
            base_builder: self,
            program: program.into(),
            override_classical: false,
            stack: SimStack::default(),
            routed: RoutedConfig::default(),
        }
    }
}

/// Config recorded at the facade for routing to the neo stack.
///
/// The engines `SimBuilder` keeps its own copy via the delegating setters;
/// this records what the neo translation needs (values it can map, flags
/// for config it cannot yet map and must reject).
#[derive(Default)]
struct RoutedConfig {
    seed: Option<u64>,
    workers: Option<usize>,
    auto_workers: bool,
    qubits: Option<usize>,
    noise_set: bool,
    quantum_set: bool,
}

/// A simulation builder that has a program set and can auto-select engines
pub struct ProgrammedSimBuilder {
    base_builder: SimBuilder,
    program: Program,
    override_classical: bool,
    stack: SimStack,
    routed: RoutedConfig,
}

impl ProgrammedSimBuilder {
    /// Auto-select the classical engine based on program type, returning a configured `SimBuilder`.
    fn configure_engine(self) -> Result<SimBuilder, PecosError> {
        if self.override_classical {
            return Ok(self.base_builder);
        }

        match self.program {
            Program::Qasm(qasm) => Ok(self.base_builder.classical(qasm_engine().program(qasm))),
            Program::Qis(qis) => {
                #[cfg(feature = "qis")]
                {
                    let engine_builder = build_qis_engine(qis)?;
                    Ok(self.base_builder.classical(engine_builder))
                }
                #[cfg(not(feature = "qis"))]
                {
                    let _ = qis;
                    Err(PecosError::Generic(
                        "QIS programs require Selene and LLVM support. Please rebuild with --features selene,llvm".to_string()
                    ))
                }
            }
            Program::Hugr(hugr) => {
                #[cfg(feature = "qis")]
                {
                    let engine_builder = build_qis_engine(hugr)?;
                    Ok(self.base_builder.classical(engine_builder))
                }
                #[cfg(not(feature = "qis"))]
                {
                    let _ = hugr;
                    Err(PecosError::Generic(
                        "HUGR programs require Selene and LLVM support. Please rebuild with --features selene,llvm".to_string()
                    ))
                }
            }
            Program::Wasm(_) => Err(PecosError::Input(
                "WASM programs are not yet supported in unified simulation".to_string(),
            )),
            Program::Wat(_) => Err(PecosError::Input(
                "WAT programs are not yet supported in unified simulation".to_string(),
            )),
            Program::PhirJson(_) => Err(PecosError::Input(
                "PHIR JSON programs are not yet supported in unified simulation".to_string(),
            )),
            Program::SeleneInterface(_) => Err(PecosError::Input(
                "SeleneInterface programs are not yet supported in unified simulation".to_string(),
            )),
        }
    }

    /// Select which simulation stack executes the program.
    ///
    /// Defaults to [`SimStack::Engines`]. [`SimStack::Neo`] is experimental
    /// and requires the `neo` cargo feature; see [`SimStack`] for the
    /// configuration it can route so far. The result type and contract are
    /// identical on both stacks.
    #[must_use]
    pub fn stack(mut self, stack: SimStack) -> Self {
        self.stack = stack;
        self
    }

    /// Build the simulation with automatic engine selection
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The program type is not yet supported (WASM, WAT, PHIR JSON, `SeleneInterface`)
    /// - Engine building fails
    /// - The neo stack is selected (it has no `MonteCarloEngine`; use
    ///   [`run()`](Self::run) directly)
    pub fn build(self) -> Result<MonteCarloEngine, PecosError> {
        if self.stack == SimStack::Neo {
            return Err(PecosError::Input(
                "The neo stack does not expose a MonteCarloEngine; call .run(shots) directly."
                    .to_string(),
            ));
        }
        self.configure_engine()?.build()
    }

    /// Build and run the simulation with automatic engine selection
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The program type is not yet supported (WASM, WAT, PHIR JSON, `SeleneInterface`)
    /// - Engine building or running fails
    /// - The neo stack is selected with configuration it cannot route yet
    pub fn run(self, shots: usize) -> Result<pecos_engines::shot_results::ShotVec, PecosError> {
        match self.stack {
            SimStack::Engines => self.configure_engine()?.run(shots),
            SimStack::Neo => self.run_neo(shots),
        }
    }

    /// Run the program on the pecos-neo stack.
    #[cfg(feature = "neo")]
    fn run_neo(self, shots: usize) -> Result<pecos_engines::shot_results::ShotVec, PecosError> {
        use pecos_neo::tool::{monte_carlo, sim_neo};

        if self.override_classical {
            return Err(PecosError::Input(
                "Explicit .classical() engine builders are not yet routed to the neo stack; \
                 remove .classical() or use .stack(SimStack::Engines)."
                    .to_string(),
            ));
        }
        if self.routed.noise_set {
            return Err(PecosError::Input(
                "Noise models are not yet routed to the neo stack (the engines-to-neo noise \
                 mapping is pending); remove .noise() or use .stack(SimStack::Engines)."
                    .to_string(),
            ));
        }
        if self.routed.quantum_set {
            return Err(PecosError::Input(
                "Explicit quantum backends are not yet routed to the neo stack (it uses the \
                 default sparse stabilizer); remove .quantum() or use .stack(SimStack::Engines)."
                    .to_string(),
            ));
        }
        match &self.program {
            Program::Qasm(_) | Program::Hugr(_) => {}
            _ => {
                return Err(PecosError::Input(
                    "Only QASM and HUGR programs are routed to the neo stack so far; \
                     use .stack(SimStack::Engines) for other program types."
                        .to_string(),
                ));
            }
        }

        let mut sampler = monte_carlo(shots);
        if let Some(workers) = self.routed.workers {
            sampler = sampler.workers(workers);
        }
        if self.routed.auto_workers {
            sampler = sampler.auto_workers();
        }

        let mut builder = sim_neo(self.program).auto().sampling(sampler);
        if let Some(seed) = self.routed.seed {
            builder = builder.seed(seed);
        }
        if let Some(qubits) = self.routed.qubits {
            builder = builder.qubits(qubits);
        }

        let results = builder.run();
        results.shots.ok_or_else(|| {
            PecosError::Generic(
                "The neo stack produced no register results for a classical-engine program; \
                 this is a bug in the neo routing."
                    .to_string(),
            )
        })
    }

    /// Stub when pecos is built without the `neo` feature.
    #[cfg(not(feature = "neo"))]
    fn run_neo(self, _shots: usize) -> Result<pecos_engines::shot_results::ShotVec, PecosError> {
        Err(PecosError::Input(
            "pecos was built without the 'neo' cargo feature; rebuild with features = [\"neo\"] \
             to route sim() to the neo stack."
                .to_string(),
        ))
    }

    /// Override the classical engine selection
    ///
    /// This allows you to specify a different engine than the auto-selected one.
    #[must_use]
    pub fn classical<B: ClassicalControlEngineBuilder + Send + 'static>(
        mut self,
        engine_builder: B,
    ) -> Self
    where
        B::Engine: 'static,
    {
        self.base_builder = self.base_builder.classical(engine_builder);
        self.override_classical = true;
        self
    }

    /// Set the random seed (delegates to base builder)
    #[must_use]
    pub fn seed(mut self, seed: u64) -> Self {
        self.routed.seed = Some(seed);
        self.base_builder = self.base_builder.seed(seed);
        self
    }

    /// Set the number of worker threads (delegates to base builder)
    #[must_use]
    pub fn workers(mut self, workers: usize) -> Self {
        self.routed.workers = Some(workers);
        self.base_builder = self.base_builder.workers(workers);
        self
    }

    /// Use automatic worker count (delegates to base builder)
    #[must_use]
    pub fn auto_workers(mut self) -> Self {
        self.routed.auto_workers = true;
        self.base_builder = self.base_builder.auto_workers();
        self
    }

    /// Enable verbose output (delegates to base builder)
    #[must_use]
    pub fn verbose(mut self, verbose: bool) -> Self {
        self.base_builder = self.base_builder.verbose(verbose);
        self
    }

    /// Set the noise model (delegates to base builder)
    #[must_use]
    pub fn noise<N>(mut self, noise_builder: N) -> Self
    where
        N: pecos_engines::noise::IntoNoiseModel + Send + 'static,
    {
        self.routed.noise_set = true;
        self.base_builder = self.base_builder.noise(noise_builder);
        self
    }

    /// Set the quantum engine (delegates to base builder)
    #[must_use]
    pub fn quantum<Q>(mut self, quantum_builder: Q) -> Self
    where
        Q: pecos_engines::quantum_engine_builder::IntoQuantumEngineBuilder + 'static,
        Q::Builder: Send + 'static,
    {
        self.routed.quantum_set = true;
        self.base_builder = self.base_builder.quantum(quantum_builder);
        self
    }

    /// Set the number of qubits (delegates to base builder)
    #[must_use]
    pub fn qubits(mut self, num_qubits: usize) -> Self {
        self.routed.qubits = Some(num_qubits);
        self.base_builder = self.base_builder.qubits(num_qubits);
        self
    }
}

/// Create a simulation builder with a program and automatic engine selection
///
/// Primary API for quantum simulations in PECOS.
/// Automatically selects the appropriate classical engine based on the program type.
///
/// # Automatic Engine Selection
///
/// - QASM programs → QASM engine
/// - QIS programs → QIS control engine (Selene Helios interface)
/// - HUGR programs → QIS control engine (Selene Helios interface)
/// - Other formats → Error (not yet supported)
///
/// # Examples
///
/// ```rust,no_run
/// use pecos::sim;
/// use pecos_programs::Qasm;
/// use pecos_engines::{sparse_stab, DepolarizingNoise};
///
/// // Automatic engine selection based on program type
/// let qasm_prog = Qasm::from_string("OPENQASM 2.0; qreg q[1]; h q[0];");
/// let results = sim(qasm_prog)
///     .quantum(sparse_stab())
///     .noise(DepolarizingNoise { p: 0.01 })
///     .seed(42)
///     .run(100)?;
/// # Ok::<(), pecos_core::errors::PecosError>(())
/// ```
pub fn sim<P: Into<Program>>(program: P) -> ProgrammedSimBuilder {
    ProgrammedSimBuilder {
        base_builder: sim_builder(),
        program: program.into(),
        override_classical: false,
        stack: SimStack::default(),
        routed: RoutedConfig::default(),
    }
}
