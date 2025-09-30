//! QIS Classical Control Engine implementation
//!
//! This module provides the QisCCEngine that orchestrates between
//! QisInterface and QisRuntime while implementing ClassicalControlEngine
//! for integration with PECOS's EngineSystem.

use log::{debug, trace};
use pecos_core::errors::PecosError;
use pecos_engines::{
    byte_message::{ByteMessage, ByteMessageBuilder},
    engine_system::{ClassicalEngine, ControlEngine, EngineStage},
    shot_results::Shot as PecosShot,
    Engine,
};
use pecos_qis_interface::{QisInterface, QuantumOp};
use pecos_qis_runtime_trait::QisRuntime;
use std::any::Any;
use std::collections::HashMap;

/// QIS Control Engine
///
/// Orchestrates between QisInterface (linked program) and QisRuntime (interpreter),
/// converting quantum operations to ByteMessages for PECOS integration.
pub struct QisControlEngine {
    /// The QIS runtime (interpreter)
    runtime: Box<dyn QisRuntime>,

    /// Number of qubits in the program
    num_qubits: usize,

    /// Whether we've started processing
    started: bool,

    /// Tracking measurement result IDs for the current batch
    measurement_mapping: Vec<usize>,
}

impl QisControlEngine {
    /// Create a new QisControlEngine with the given runtime
    pub fn new(runtime: Box<dyn QisRuntime>) -> Self {
        Self {
            runtime,
            num_qubits: 0,
            started: false,
            measurement_mapping: Vec::new(),
        }
    }

    /// Load a QIS interface into the engine
    pub fn load_interface(&mut self, interface: QisInterface) -> Result<(), PecosError> {
        debug!("Loading QIS interface into QisControlEngine");

        // Count qubits
        self.num_qubits = interface.allocated_qubits.iter().max().map_or(0, |&q| q + 1);

        // Load into runtime
        self.runtime
            .load_interface(interface)
            .map_err(|e| PecosError::Generic(format!("Failed to load interface: {}", e)))?;

        Ok(())
    }

    /// Configure the engine with a program and return it
    ///
    /// This is a convenience method for use with the sim() API,
    /// following the same pattern as QASMEngine.
    ///
    /// # Example
    /// ```ignore
    /// let engine = qis_control_engine()
    ///     .runtime(native_runtime())
    ///     .build()?
    ///     .program(interface);
    ///
    /// let results = sim_builder()
    ///     .classical(engine)
    ///     .quantum(state_vector())
    ///     .run(100)?;
    /// ```
    pub fn program(mut self, interface: QisInterface) -> Self {
        // Best effort load - if it fails, the error will be caught later
        let _ = self.load_interface(interface);
        self
    }

    /// Convert quantum operations to ByteMessage
    fn operations_to_bytemessage(&mut self, ops: Vec<QuantumOp>) -> Result<ByteMessage, PecosError> {
        let mut builder = ByteMessageBuilder::new();
        let _ = builder.for_quantum_operations();

        // Clear previous measurement mapping
        self.measurement_mapping.clear();

        for op in ops {
            match op {
                // Single-qubit gates
                QuantumOp::H(q) => { builder.add_h(&[q]); }
                QuantumOp::X(q) => { builder.add_x(&[q]); }
                QuantumOp::Y(q) => { builder.add_y(&[q]); }
                QuantumOp::Z(q) => { builder.add_z(&[q]); }
                QuantumOp::S(q) => { builder.add_sz(&[q]); }
                QuantumOp::Sdg(q) => { builder.add_szdg(&[q]); }
                QuantumOp::T(q) => { builder.add_t(&[q]); }
                QuantumOp::Tdg(q) => { builder.add_tdg(&[q]); }

                // Rotation gates
                QuantumOp::RX(theta, q) => { builder.add_rx(theta, &[q]); }
                QuantumOp::RY(theta, q) => { builder.add_ry(theta, &[q]); }
                QuantumOp::RZ(theta, q) => { builder.add_rz(theta, &[q]); }

                // Hardware-native gates
                QuantumOp::RXY(theta, phi, q) => { builder.add_r1xy(theta, phi, &[q]); }

                // Two-qubit gates
                QuantumOp::CX(c, t) => { builder.add_cx(&[c], &[t]); }
                QuantumOp::CY(c, t) => { builder.add_cy(&[c], &[t]); }
                QuantumOp::CZ(c, t) => { builder.add_cz(&[c], &[t]); }
                QuantumOp::CH(c, t) => {
                    // Controlled-H decomposition
                    // CH = (I ⊗ Ry(-π/4)) · CX · (I ⊗ Ry(π/4))
                    builder
                        .add_ry(-std::f64::consts::PI / 4.0, &[t])
                        .add_cx(&[c], &[t])
                        .add_ry(std::f64::consts::PI / 4.0, &[t]);
                }
                QuantumOp::CRZ(theta, c, t) => {
                    // CRZ decomposition using CX and RZ
                    builder
                        .add_cx(&[c], &[t])
                        .add_rz(theta/2.0, &[t])
                        .add_cx(&[c], &[t])
                        .add_rz(-theta/2.0, &[t]);
                }

                // Three-qubit gates
                QuantumOp::CCX(c1, c2, t) => {
                    // Toffoli gate - decompose into basic gates
                    // This is a simplified decomposition
                    builder
                        .add_h(&[t])
                        .add_cx(&[c2], &[t])
                        .add_tdg(&[t])
                        .add_cx(&[c1], &[t])
                        .add_t(&[t])
                        .add_cx(&[c2], &[t])
                        .add_tdg(&[t])
                        .add_cx(&[c1], &[t])
                        .add_t(&[c2])
                        .add_t(&[t])
                        .add_h(&[t])
                        .add_cx(&[c1], &[c2])
                        .add_t(&[c1])
                        .add_tdg(&[c2])
                        .add_cx(&[c1], &[c2]);
                }

                // ZZ interaction
                QuantumOp::ZZ(q1, q2) => {
                    // ZZ gate is equivalent to CZ for |00⟩, |01⟩, |10⟩, |11⟩ basis
                    // ZZ = diag(1, -1, -1, 1) = CZ
                    builder.add_cz(&[q1], &[q2]);
                }
                QuantumOp::RZZ(theta, q1, q2) => {
                    // RZZ decomposition
                    builder
                        .add_cx(&[q1], &[q2])
                        .add_rz(theta, &[q2])
                        .add_cx(&[q1], &[q2]);
                }

                // Measurement
                QuantumOp::Measure(q, result_id) => {
                    // Track the result ID for this measurement
                    self.measurement_mapping.push(result_id);
                    builder.add_measurements(&[q]);
                }

                // Reset
                QuantumOp::Reset(q) => {
                    // Reset is prep in PECOS
                    builder.add_prep(&[q]);
                }
            }
        }

        Ok(builder.build())
    }

    /// Extract measurements from ByteMessage
    fn extract_measurements(&self, msg: ByteMessage) -> HashMap<usize, bool> {
        let mut measurements = HashMap::new();

        // Extract measurements from the ByteMessage
        if let Ok(outcomes) = msg.outcomes() {
            trace!("Extracted {} outcomes from ByteMessage", outcomes.len());
            trace!("Measurement mapping has {} entries", self.measurement_mapping.len());

            // Map outcomes to the result IDs we tracked
            for (idx, outcome) in outcomes.iter().enumerate() {
                if idx < self.measurement_mapping.len() {
                    let result_id = self.measurement_mapping[idx];
                    let value = *outcome != 0;
                    trace!("Mapping outcome[{}]={} to result_id={}", idx, outcome, result_id);
                    measurements.insert(result_id, value);
                }
            }
        } else {
            trace!("Failed to extract outcomes from ByteMessage");
        }

        trace!("Extracted measurements: {:?}", measurements);
        measurements
    }
}

impl Clone for QisControlEngine {
    fn clone(&self) -> Self {
        Self {
            runtime: dyn_clone::clone_box(&*self.runtime),
            num_qubits: self.num_qubits,
            started: self.started,
            measurement_mapping: self.measurement_mapping.clone(),
        }
    }
}

impl Engine for QisControlEngine {
    type Input = ();
    type Output = PecosShot;

    fn process(&mut self, _input: Self::Input) -> Result<Self::Output, PecosError> {
        debug!("QisControlEngine::process called");

        // Use the ControlEngine implementation for processing
        let mut stage = self.start(())?;

        loop {
            match stage {
                EngineStage::NeedsProcessing(_) => {
                    // In standalone mode, we can't actually execute quantum ops
                    // Just return empty measurements
                    let empty_msg = ByteMessage::builder().build();
                    stage = self.continue_processing(empty_msg)?;
                }
                EngineStage::Complete(output) => return Ok(output),
            }
        }
    }

    fn reset(&mut self) -> Result<(), PecosError> {
        self.runtime
            .reset()
            .map_err(|e| PecosError::Generic(format!("Failed to reset runtime: {}", e)))?;
        self.started = false;
        Ok(())
    }
}

impl ClassicalEngine for QisControlEngine {
    fn num_qubits(&self) -> usize {
        self.runtime.num_qubits()
    }

    fn generate_commands(&mut self) -> Result<ByteMessage, PecosError> {
        debug!("QisControlEngine::generate_commands called");

        // Get next batch of quantum operations from runtime
        match self.runtime.execute_until_quantum() {
            Ok(Some(ops)) => {
                trace!("Runtime returned {} operations", ops.len());
                for (i, op) in ops.iter().enumerate() {
                    trace!("  Op[{}]: {:?}", i, op);
                }
                self.operations_to_bytemessage(ops)
            }
            Ok(None) => {
                trace!("Runtime has no more operations");
                Ok(ByteMessage::builder().build())
            }
            Err(e) => Err(PecosError::Generic(format!(
                "Runtime execution failed: {}",
                e
            ))),
        }
    }

    fn handle_measurements(&mut self, message: ByteMessage) -> Result<(), PecosError> {
        debug!("QisControlEngine::handle_measurements called");

        let measurements = self.extract_measurements(message);
        trace!("Extracted {} measurements", measurements.len());

        self.runtime
            .provide_measurements(measurements)
            .map_err(|e| {
                PecosError::Generic(format!("Failed to provide measurements: {}", e))
            })?;

        Ok(())
    }

    fn get_results(&self) -> Result<PecosShot, PecosError> {
        debug!("QisControlEngine::get_results called");

        // Get results from runtime
        let runtime_shot = self.runtime.get_classical_state();

        // Convert to PECOS Shot format
        let mut shot = PecosShot::default();

        // Add measurements as U32 directly for compatibility
        for (result_id, value) in &runtime_shot.measurements {
            let val = if *value { 1u32 } else { 0u32 };
            shot.data.insert(
                format!("m{}", result_id),
                pecos_engines::shot_results::Data::U32(val),
            );
        }

        // Add classical registers
        for (name, bits) in &runtime_shot.registers {
            // Convert bits to u32 (limited to 32 bits)
            let mut val = 0u32;
            for (i, bit) in bits.iter().take(32).enumerate() {
                if *bit {
                    val |= 1 << i;
                }
            }
            shot.add_register(name, val, bits.len());
        }

        Ok(shot)
    }

    fn compile(&self) -> Result<(), PecosError> {
        // The program is already "compiled" when loaded into the runtime
        Ok(())
    }

    fn reset(&mut self) -> Result<(), PecosError> {
        Engine::reset(self)
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

impl ControlEngine for QisControlEngine {
    type Input = ();
    type Output = PecosShot;
    type EngineInput = ByteMessage;
    type EngineOutput = ByteMessage;

    fn start(
        &mut self,
        _input: Self::Input,
    ) -> Result<EngineStage<Self::EngineInput, Self::Output>, PecosError> {
        debug!("QisControlEngine::start called");

        // Start a new shot
        self.runtime
            .shot_start(0, None)
            .map_err(|e| PecosError::Generic(format!("Failed to start shot: {}", e)))?;

        self.started = true;

        // Generate initial commands
        let commands = self.generate_commands()?;

        if commands.is_empty()? {
            // No quantum operations needed
            Ok(EngineStage::Complete(self.get_results()?))
        } else {
            Ok(EngineStage::NeedsProcessing(commands))
        }
    }

    fn continue_processing(
        &mut self,
        measurements: Self::EngineOutput,
    ) -> Result<EngineStage<Self::EngineInput, Self::Output>, PecosError> {
        debug!("QisControlEngine::continue_processing called");

        // Handle measurements from quantum engine
        self.handle_measurements(measurements)?;

        // Generate next batch of commands
        let commands = self.generate_commands()?;

        if commands.is_empty()? {
            // Execution complete
            let shot = self.runtime
                .shot_end()
                .map_err(|e| PecosError::Generic(format!("Failed to end shot: {}", e)))?;

            // Convert runtime Shot to PECOS Shot
            let mut pecos_shot = PecosShot::default();
            for (result_id, value) in shot.measurements {
                let val = if value { 1u32 } else { 0u32 };
                // Insert as U32 directly for compatibility with existing tests
                pecos_shot.data.insert(
                    format!("m{}", result_id),
                    pecos_engines::shot_results::Data::U32(val),
                );
            }
            for (name, bits) in shot.registers {
                // Convert bits to u32
                let mut val = 0u32;
                for (i, bit) in bits.iter().take(32).enumerate() {
                    if *bit {
                        val |= 1 << i;
                    }
                }
                pecos_shot.add_register(&name, val, bits.len());
            }

            Ok(EngineStage::Complete(pecos_shot))
        } else {
            Ok(EngineStage::NeedsProcessing(commands))
        }
    }

    fn reset(&mut self) -> Result<(), PecosError> {
        Engine::reset(self)
    }
}