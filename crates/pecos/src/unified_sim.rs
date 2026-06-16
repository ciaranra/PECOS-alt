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
    /// routes QASM programs with the default quantum backend (HUGR runs but
    /// does not yet match the engines result contract, so it is rejected).
    /// The translated noise surface is the depolarizing family
    /// (`PassThroughNoise`, `DepolarizingNoise`, `BiasedDepolarizingNoise`,
    /// and their builders) and the `GeneralNoiseModel` simple-probability
    /// subset, including angle-dependent two-qubit scaling. Other noise
    /// configurations, explicit `.classical()`, and explicit `.quantum()`
    /// are not yet translated and are rejected with an error at `run()`.
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
    /// The noise config as passed, for translation to the neo stack.
    /// Type-erased because `.noise()` is generic; the neo route downcasts
    /// against the known engines noise types.
    noise: Option<Box<dyn std::any::Any + Send>>,
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
        let neo_noise = match &self.routed.noise {
            None => None,
            Some(noise) => map_noise_to_neo(noise.as_ref())?,
        };
        if self.routed.quantum_set {
            return Err(PecosError::Input(
                "Explicit quantum backends are not yet routed to the neo stack (it uses the \
                 default sparse stabilizer); remove .quantum() or use .stack(SimStack::Engines)."
                    .to_string(),
            ));
        }
        match &self.program {
            Program::Qasm(_) => {}
            Program::Hugr(_) => {
                // HUGR runs on neo via pecos_hugr::hugr_engine, whose result
                // contract differs from the engines/QASM path: for HUGR programs
                // without explicit result() captures it emits per-qubit `q0`/`q1`
                // values and a `measurements` array instead of the program's
                // named classical register (e.g. "c"). Returning that silently
                // would break consumers that index `shot.data["c"]`, so reject
                // until the neo HUGR result contract is reconciled (the physics
                // is already correct; only the register naming diverges).
                return Err(PecosError::Input(
                    "HUGR programs are not yet routed to the neo stack: the neo HUGR \
                     engine emits a different result contract (per-qubit q0/q1 plus a \
                     `measurements` array) than the engines and QASM paths (the \
                     program's named classical register, e.g. \"c\") for programs \
                     without explicit result() captures, so neo HUGR results are not \
                     yet drop-in compatible. Use .stack(SimStack::Engines) for HUGR."
                        .to_string(),
                ));
            }
            _ => {
                return Err(PecosError::Input(
                    "Only QASM programs are routed to the neo stack so far; \
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
        if let Some(noise) = neo_noise {
            builder = builder.noise(noise);
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
}

/// Translate an engines noise config into the neo stack's noise model.
///
/// Gate and prep conventions are identical on both stacks (uniform X/Y/Z
/// at p1, uniform 15 two-qubit Paulis at p2, X after prep for `p_prep`)
/// and probabilities map one-to-one. Measurement noise differs BY MODEL
/// on the engines side and the mapping preserves each model's physics:
///
/// - The depolarizing family injects a physical X into the state before
///   each measurement (the error persists and propagates — a qubit
///   measured twice without a reset flips at `2p(1-p)` the second time),
///   mapped to neo's `MeasurementStateFlipChannel` via
///   `with_p_meas_state_flip`.
/// - `GeneralNoiseModel` flips only the classical record (the
///   post-measurement state is untouched), mapped to neo's
///   record-flipping `MeasurementChannel` via `with_p_meas`.
///
/// `GeneralNoiseModel` beyond the simple probability subset is NOT
/// mapped: its full configuration (leakage, idle, crosstalk, emission
/// models) is not readable from the built model; configure `sim_neo()`
/// directly with neo's `GeneralNoiseModelBuilder` for those.
///
/// Returns `Ok(None)` for pass-through (no noise).
#[cfg(feature = "neo")]
fn map_noise_to_neo(
    noise: &(dyn std::any::Any + Send),
) -> Result<Option<pecos_neo::noise::GeneralNoiseModelBuilder>, PecosError> {
    use pecos_engines::noise::{
        BiasedDepolarizingNoiseModelBuilder, DepolarizingNoiseModelBuilder,
        PassThroughNoiseModelBuilder,
    };
    use pecos_engines::{BiasedDepolarizingNoise, DepolarizingNoise, PassThroughNoise};
    use pecos_neo::noise::{AngleScaling, GeneralNoiseModelBuilder};

    let uniform = |p_prep: f64, p_meas: f64, p1: f64, p2: f64| {
        GeneralNoiseModelBuilder::new()
            .with_p_prep(p_prep)
            .with_p_meas_state_flip(p_meas)
            .with_p1(p1)
            .with_p2(p2)
    };

    // The biased-depolarizing family applies its measurement bias to the
    // RECORDED outcome AFTER readout (`apply_bias_to_measurement`), never to
    // the state -- the opposite of the plain depolarizing family, which injects
    // a physical X BEFORE measurement. So its measurement maps to neo's
    // record-flipping channel (`with_p_meas`), which also carries the
    // asymmetric `p_meas_0` (0->1) / `p_meas_1` (1->0) bias one-to-one. Gate
    // and prep noise are ordinary uniform depolarizing.
    let biased = |p_prep: f64, p_meas_0: f64, p_meas_1: f64, p1: f64, p2: f64| {
        GeneralNoiseModelBuilder::new()
            .with_p_prep(p_prep)
            .with_p_meas(p_meas_0, p_meas_1)
            .with_p1(p1)
            .with_p2(p2)
    };

    if noise.downcast_ref::<PassThroughNoise>().is_some()
        || noise
            .downcast_ref::<PassThroughNoiseModelBuilder>()
            .is_some()
    {
        return Ok(None);
    }
    if let Some(depolarizing) = noise.downcast_ref::<DepolarizingNoise>() {
        let p = depolarizing.p;
        return Ok(Some(uniform(p, p, p, p)));
    }
    if let Some(builder) = noise.downcast_ref::<DepolarizingNoiseModelBuilder>() {
        // Resolve the configured probabilities via the built model; this
        // enforces the same all-probabilities-set requirement the engines
        // path would.
        let (p_prep, p_meas, p1, p2) = builder.clone().build().probabilities();
        return Ok(Some(uniform(p_prep, p_meas, p1, p2)));
    }
    if let Some(biased_noise) = noise.downcast_ref::<BiasedDepolarizingNoise>() {
        // `BiasedDepolarizingNoise { p }` builds `new_uniform(p)`: every rate is
        // `p`, with symmetric measurement bias.
        let p = biased_noise.p;
        return Ok(Some(biased(p, p, p, p, p)));
    }
    if let Some(builder) = noise.downcast_ref::<BiasedDepolarizingNoiseModelBuilder>() {
        let (p_prep, p_meas_0, p_meas_1, p1, p2) = builder.clone().build().probabilities();
        return Ok(Some(biased(p_prep, p_meas_0, p_meas_1, p1, p2)));
    }
    if let Some(builder) = noise.downcast_ref::<pecos_engines::noise::GeneralNoiseModelBuilder>() {
        // The stored p1/p2 are already in standard depolarizing convention
        // (the with_average_* setters convert on the way in), so they map
        // one-to-one onto neo's builder. Angle-dependent two-qubit scaling, if
        // present, is translated below; everything else outside the simple
        // Pauli subset is still rejected.
        let Some((p_prep, p_meas_0, p_meas_1, p1, p2, angle)) = builder.pauli_with_angle_scaling()
        else {
            return Err(PecosError::Input(
                "This GeneralNoiseModel configuration uses features beyond the simple \
                 probability subset (leakage, emission, seepage, idle, crosstalk, scales, \
                 or noiseless gates), which are not yet mapped to the neo stack. Use \
                 .stack(SimStack::Engines) or configure sim_neo() directly with a neo \
                 noise model."
                    .to_string(),
            ));
        };
        let mut neo_builder = GeneralNoiseModelBuilder::new()
            .with_p_prep(p_prep)
            .with_p_meas(p_meas_0, p_meas_1)
            .with_p1(p1)
            .with_p2(p2);
        if let Some((a, b, c, d, power)) = angle {
            // Engines' angle-dependent two-qubit error rate is
            //   p2 * (a*|theta/pi|^power + b)  for theta < 0
            //   p2 * (c*|theta/pi|^power + d)  for theta > 0
            // (GeneralNoiseModel::p2_angle_error_rate). neo's AngleScaling
            // evaluates offset + linear*|theta/pi| + scale*|theta/pi|^power per
            // sign, so the engines coefficients map to scale (the power term)
            // and the engines offsets map to offset, with the linear terms
            // zero. This reproduces engines exactly, including the zero-angle
            // (b+d)/2 average. (NOT AngleScaling::from_general_params, which is
            // a different symmetric offset/linear/scale parameterization.)
            //
            // Both stacks read the gate angle as the SIGNED principal value
            // (-pi, pi] -- neo via `to_radians_signed`, engines likewise after
            // its noise call site was aligned with its gate unitaries -- so the
            // sign-dependent coefficients agree cross-stack at every angle
            // (locked by `gnm_angle_scaling_negative_matches`).
            neo_builder = neo_builder
                .with_p2_angle_scaling(AngleScaling::asymmetric(b, 0.0, a, d, 0.0, c, power));
        }
        return Ok(Some(neo_builder));
    }

    Err(PecosError::Input(
        "This noise type is not yet mapped to the neo stack (mapped so far: PassThroughNoise, \
         DepolarizingNoise, DepolarizingNoiseModelBuilder, BiasedDepolarizingNoise, \
         BiasedDepolarizingNoiseModelBuilder, GeneralNoiseModelBuilder's simple \
         probability subset). Remove .noise(), use .stack(SimStack::Engines), or configure \
         sim_neo() directly with a neo noise model."
            .to_string(),
    ))
}

impl ProgrammedSimBuilder {
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
        N: pecos_engines::noise::IntoNoiseModel + Clone + Send + 'static,
    {
        self.routed.noise = Some(Box::new(noise_builder.clone()));
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
