//! Runtime plugins for PECOS-Selene integration
//!
//! This crate provides:
//! - ByteMessage simulator plugin that collects quantum operations
//! - HUGR to LLVM compilation utilities

pub mod communication;

use selene_core::{
    export_runtime_plugin,
    runtime::{BatchOperation, Operation, RuntimeInterface, interface::RuntimeInterfaceFactory},
    utils::MetricValue,
    time::Instant,
};
use std::collections::{VecDeque, HashMap};
use anyhow::{Result, anyhow};
use communication::{FileChannel, COMM_DIR_ENV};

/// A simulator that collects quantum operations for ByteMessage conversion
pub struct ByteMessageSimulator {
    /// Queue of operations to be returned
    operation_queue: VecDeque<BatchOperation>,
    /// Accumulated operations for the current batch
    current_batch: Vec<Operation>,
    /// Store measurement results received from PECOS
    measurement_results: HashMap<u64, bool>,
    /// Next result ID
    next_result_id: u64,
    /// Active qubit allocations
    allocated_qubits: Vec<u64>,
    /// Next qubit ID
    next_qubit_id: u64,
    /// Start time for batches
    start_time: Instant,
}

impl ByteMessageSimulator {
    pub fn new(start: Instant) -> Self {
        #[cfg(feature = "logging")]
        log::debug!("Creating new ByteMessageSimulator");
        
        Self {
            operation_queue: VecDeque::new(),
            current_batch: Vec::new(),
            measurement_results: HashMap::new(),
            next_result_id: 0,
            allocated_qubits: Vec::new(),
            next_qubit_id: 0,
            start_time: start,
        }
    }
    
    /// Flush current batch of operations
    fn flush_batch(&mut self) {
        if !self.current_batch.is_empty() {
            let batch = BatchOperation::new(
                std::mem::take(&mut self.current_batch),
                self.start_time,
                Default::default(),
            );
            self.operation_queue.push_back(batch);
        }
    }
}

impl RuntimeInterface for ByteMessageSimulator {
    fn exit(&mut self) -> Result<()> {
        #[cfg(feature = "logging")]
        log::debug!("ByteMessageSimulator: Exiting");
        Ok(())
    }

    fn get_next_operations(&mut self) -> Result<Option<BatchOperation>> {
        // Flush any pending operations
        self.flush_batch();
        
        // Return next batch from queue
        Ok(self.operation_queue.pop_front())
    }
    
    fn shot_start(&mut self, _shot_id: u64, _seed: u64) -> Result<()> {
        #[cfg(feature = "logging")]
        log::debug!("ByteMessageSimulator: Starting shot {}", _shot_id);
        
        // Clear state for new shot
        self.operation_queue.clear();
        self.current_batch.clear();
        self.measurement_results.clear();
        self.allocated_qubits.clear();
        self.next_result_id = 0;
        self.next_qubit_id = 0;
        Ok(())
    }
    
    fn shot_end(&mut self) -> Result<()> {
        #[cfg(feature = "logging")]
        log::debug!("ByteMessageSimulator: Ending shot");
        
        // Flush any remaining operations
        self.flush_batch();
        Ok(())
    }
    
    fn custom_call(&mut self, _tag: u64, _data: &[u8]) -> Result<u64> {
        Err(anyhow!("Custom calls not supported by ByteMessageSimulator"))
    }
    
    fn get_metric(&mut self, _nth_metric: u8) -> Result<Option<(String, MetricValue)>> {
        Ok(None)
    }
    
    fn qalloc(&mut self) -> Result<u64> {
        let qubit_id = self.next_qubit_id;
        self.next_qubit_id += 1;
        self.allocated_qubits.push(qubit_id);
        
        #[cfg(feature = "logging")]
        log::debug!("ByteMessageSimulator: Allocated qubit {}", qubit_id);
        
        Ok(qubit_id)
    }
    
    fn qfree(&mut self, qubit_id: u64) -> Result<()> {
        if let Some(pos) = self.allocated_qubits.iter().position(|&q| q == qubit_id) {
            self.allocated_qubits.remove(pos);
            #[cfg(feature = "logging")]
            log::debug!("ByteMessageSimulator: Freed qubit {}", qubit_id);
            Ok(())
        } else {
            Err(anyhow!("Attempted to free unallocated qubit {}", qubit_id))
        }
    }
    
    fn rxy_gate(&mut self, qubit_id: u64, theta: f64, phi: f64) -> Result<()> {
        #[cfg(feature = "logging")]
        log::debug!("ByteMessageSimulator: RXY gate on qubit {} with theta={}, phi={}", qubit_id, theta, phi);
        
        self.current_batch.push(Operation::RXYGate { qubit_id, theta, phi });
        Ok(())
    }
    
    fn rzz_gate(&mut self, qubit_id_1: u64, qubit_id_2: u64, theta: f64) -> Result<()> {
        #[cfg(feature = "logging")]
        log::debug!("ByteMessageSimulator: RZZ gate on qubits {} and {} with theta={}", qubit_id_1, qubit_id_2, theta);
        
        self.current_batch.push(Operation::RZZGate { qubit_id_1, qubit_id_2, theta });
        Ok(())
    }
    
    fn rz_gate(&mut self, qubit_id: u64, theta: f64) -> Result<()> {
        #[cfg(feature = "logging")]
        log::debug!("ByteMessageSimulator: RZ gate on qubit {} with theta={}", qubit_id, theta);
        
        self.current_batch.push(Operation::RZGate { qubit_id, theta });
        Ok(())
    }
    
    fn measure(&mut self, qubit_id: u64) -> Result<u64> {
        let result_id = self.next_result_id;
        self.next_result_id += 1;
        
        #[cfg(feature = "logging")]
        log::debug!("ByteMessageSimulator: Measure qubit {} -> result {}", qubit_id, result_id);
        
        self.current_batch.push(Operation::Measure { result_id, qubit_id });
        Ok(result_id)
    }
    
    fn reset(&mut self, qubit_id: u64) -> Result<()> {
        #[cfg(feature = "logging")]
        log::debug!("ByteMessageSimulator: Reset qubit {}", qubit_id);
        
        self.current_batch.push(Operation::Reset { qubit_id });
        Ok(())
    }
    
    fn force_result(&mut self, _result_id: u64) -> Result<()> {
        #[cfg(feature = "logging")]
        log::debug!("ByteMessageSimulator: Force result {}", _result_id);
        
        // Flush operations to ensure measurement is processed
        self.flush_batch();
        Ok(())
    }
    
    fn increment_future_refcount(&mut self, _future: u64) -> Result<()> {
        // No-op for our simple implementation
        Ok(())
    }
    
    fn decrement_future_refcount(&mut self, _future: u64) -> Result<()> {
        // No-op for our simple implementation
        Ok(())
    }
    
    fn local_barrier(&mut self, _qubit_ids: &[u64], _tag: u64) -> Result<()> {
        // No-op for our simple implementation
        Ok(())
    }
    
    fn global_barrier(&mut self, _tag: u64) -> Result<()> {
        // No-op for our simple implementation
        Ok(())
    }
    
    fn get_bool_result(&mut self, result_id: u64) -> Result<Option<bool>> {
        Ok(self.measurement_results.get(&result_id).copied())
    }
    
    fn get_u64_result(&mut self, _result_id: u64) -> Result<Option<u64>> {
        // We don't use u64 results in our simple simulator
        Ok(None)
    }
    
    fn set_bool_result(&mut self, result_id: u64, result: bool) -> Result<()> {
        #[cfg(feature = "logging")]
        log::debug!("ByteMessageSimulator: Set result {} = {}", result_id, result);
        
        self.measurement_results.insert(result_id, result);
        Ok(())
    }
    
    fn set_u64_result(&mut self, _result_id: u64, _result: u64) -> Result<()> {
        // We don't use u64 results in our simple simulator
        Ok(())
    }
    
    fn measure_leaked(&mut self, _qubit: u64) -> Result<u64> {
        // Return 0 for non-leaked state (we don't model leakage)
        Ok(0)
    }
}

/// Factory for creating ByteMessageSimulator instances
#[derive(Default)]
pub struct ByteMessageSimulatorFactory;

impl RuntimeInterfaceFactory for ByteMessageSimulatorFactory {
    type Interface = ByteMessageSimulator;
    
    fn init(
        self: std::sync::Arc<Self>,
        _n_qubits: u64,
        start: Instant,
        _args: &[impl AsRef<str>],
    ) -> Result<Box<Self::Interface>> {
        Ok(Box::new(ByteMessageSimulator::new(start)))
    }
}

// Export modules
pub mod hugr_compiler;

// Export the plugin
export_runtime_plugin!(crate::ByteMessageSimulatorFactory);