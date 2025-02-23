use log::debug;
use pecos_core::types::{GateType, ShotResult};
use pecos_noise::NoiseModel;

use super::{ClassicalEngine, QuantumEngine};
use crate::channels::{CommandChannel, MessageChannel};
use crate::errors::QueueError;
use parking_lot::Mutex;
use std::sync::Arc;
use std::thread;

/// HybridEngine coordinates between classical and quantum components via message passing
pub struct HybridEngine<C, M>
where
    C: CommandChannel + Send + Sync + 'static,
    M: MessageChannel + Send + Sync + 'static,
{
    classical: Box<dyn ClassicalEngine>,
    quantum: Arc<Mutex<Box<dyn QuantumEngine>>>,
    cmd_writer: C,
    cmd_reader: C,
    meas_writer: M,
    meas_reader: M,
    noise_model: Option<Box<dyn NoiseModel>>,
}

impl<C, M> HybridEngine<C, M>
where
    C: CommandChannel + Send + Sync + 'static + Clone,
    M: MessageChannel + Send + Sync + 'static + Clone,
{
    pub fn new(
        classical: Box<dyn ClassicalEngine>,
        quantum: Box<dyn QuantumEngine>,
        cmd_channel: C,
        meas_channel: M,
    ) -> Self {
        let cmd_writer = cmd_channel.clone();
        let cmd_reader = cmd_channel;
        let meas_writer = meas_channel.clone();
        let meas_reader = meas_channel;

        Self {
            classical,
            quantum: Arc::new(Mutex::new(quantum)),
            cmd_writer,
            cmd_reader,
            meas_writer,
            meas_reader,
            noise_model: None,
        }
    }

    pub fn set_noise_model(&mut self, noise_model: Option<Box<dyn NoiseModel>>) {
        self.noise_model = noise_model;
    }

    /// Executes a single quantum circuit shot and returns the result.
    pub fn run_shot(&mut self) -> Result<ShotResult, QueueError> {
        // Reset quantum engine at start of shot
        debug!("Resetting quantum engine");
        self.quantum.lock().reset()?;

        // Get commands from classical engine
        let commands = self.classical.process_program()?;
        debug!("Classical engine generated {} commands", commands.len());

        // Apply noise model if configured
        let commands = if let Some(noise_model) = &self.noise_model {
            debug!("Applying noise model to commands");
            noise_model.clone_box().apply_noise(commands)
        } else {
            commands
        };

        // Send commands through channel
        debug!("Sending {} commands to quantum thread", commands.len());
        for cmd in &commands {
            self.cmd_writer.send_command(cmd)?;
        }
        debug!("Signaling end of commands");
        self.cmd_writer.flush()?;

        // Process commands and collect measurements in quantum thread
        let mut measurements = Vec::new();
        {
            debug!("Processing commands in quantum thread");
            let mut quantum = self.quantum.lock();

            while let Some(cmd) = self.cmd_reader.receive_command()? {
                debug!("Processing quantum command: {:?}", cmd);
                if let Some(measurement) = quantum.process(cmd)? {
                    debug!("Generated measurement: {}", measurement);
                    measurements.push(measurement);
                }
            }
        }

        // Send measurements back
        debug!("Sending {} measurements", measurements.len());
        for measurement in measurements {
            self.meas_writer.send_measurement(measurement)?;
            self.classical.handle_measurement(measurement)?;
        }
        debug!("Signaling end of measurements");
        self.meas_writer.flush()?;

        // Get final results
        debug!("Getting final results");
        let results = self.classical.get_results()?;
        debug!(
            "Shot complete with {} measurements",
            results.measurements.len()
        );
        Ok(results)
    }
}
