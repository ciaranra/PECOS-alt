//! Core traits and utilities for PECOS decoders
//!
//! This crate defines the common traits and types that all decoder implementations
//! should use, enabling interoperability between different decoder types.
//!
//! # Structure
//!
//! - `errors` - Unified error types using thiserror
//! - `results` - Common result types and builders
//! - `config` - Configuration traits and validation utilities
//! - `matrix` - Common matrix types and check matrix traits
//! - `dem` - Detector error model traits and utilities
//! - `testing` - Testing utilities (requires `testing` feature)

pub mod advanced;
pub mod config;
pub mod dem;
pub mod errors;
pub mod matrix;
pub mod results;

use ndarray::ArrayView1;

// Re-export commonly used types
pub use advanced::{
    AdvancedDecoder, AdvancedDecodingResult, DecodingOptions, DecodingStats, DetailedDecoder,
    DynamicWeightDecoder, ErasureDecoder, MatchedEdge, MatchedPair,
};
pub use config::{
    BatchConfig, ConfigBuilder, DecoderConfig, DecodingMethod, PerformanceConfig, SolverType,
};
pub use dem::{DemConfig, DemConfigBuilder, DemDecoder, DemInfo};
pub use errors::{ConfigError, DecoderError, ErrorConvert, GraphError, MatrixError};
pub use matrix::{CheckMatrixConfig, CheckMatrixDecoder, SparseCheckMatrix};
pub use results::{
    BatchDecodingResult, DecodingResultTrait, ResultBuilder, StandardDecodingResult,
};

/// Core trait that all decoders must implement
pub trait Decoder {
    /// The result type for this decoder
    type Result: DecodingResultTrait;

    /// The error type for this decoder
    type Error: std::error::Error + Send + Sync + 'static;

    /// Decode a syndrome or received vector
    ///
    /// The exact interpretation of the input depends on the decoder type
    /// and configuration. For LDPC decoders, this is typically a syndrome
    /// vector when using syndrome-based decoding.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The input dimensions don't match the decoder's expectations
    /// - The decoding process fails to converge
    /// - Internal decoder errors occur
    fn decode(&mut self, input: &ArrayView1<u8>) -> Result<Self::Result, Self::Error>;

    /// Get the number of checks (rows in parity check matrix)
    fn check_count(&self) -> usize;

    /// Get the number of bits (columns in parity check matrix)
    fn bit_count(&self) -> usize;
}

/// Trait for decoders that support soft information (log-likelihood ratios)
pub trait SoftDecoder: Decoder {
    /// Decode using soft information (log-likelihood ratios)
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The LLR array dimensions don't match the decoder's expectations
    /// - The LLR values are invalid (e.g., NaN or Inf)
    /// - The soft decoding process fails
    fn decode_soft(&mut self, llrs: &ArrayView1<f64>) -> Result<Self::Result, Self::Error>;
}

/// Trait for quantum CSS code decoders
pub trait CssDecoder {
    /// The result type for this decoder
    type Result: DecodingResultTrait;

    /// The error type for this decoder
    type Error: std::error::Error + Send + Sync + 'static;

    /// Decode both X and Z syndromes for a CSS code
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Either syndrome has incorrect dimensions
    /// - The X or Z decoding fails
    /// - The decoder doesn't support CSS decoding
    fn decode_css(
        &mut self,
        x_syndrome: &ArrayView1<u8>,
        z_syndrome: &ArrayView1<u8>,
    ) -> Result<Self::Result, Self::Error>;

    /// Get the number of X checks
    fn x_check_count(&self) -> usize;

    /// Get the number of Z checks
    fn z_check_count(&self) -> usize;

    /// Get the number of qubits
    fn qubit_count(&self) -> usize;
}

/// Trait for decoders that support batch decoding
pub trait BatchDecoder: Decoder {
    /// Decode multiple inputs in a batch
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Any input has incorrect dimensions
    /// - Any individual decoding fails
    /// - The batch is too large for the decoder to handle
    fn decode_batch(&mut self, inputs: &[ArrayView1<u8>])
    -> Result<Vec<Self::Result>, Self::Error>;
}

// ============================================================================
// Testing Utilities
// ============================================================================

/// Common testing utilities for decoder implementations
#[cfg(feature = "testing")]
pub mod testing {
    use super::{Decoder, ndarray};
    use ndarray::Array1;
    use std::sync::{Arc, Mutex};
    use std::thread;
    use std::time::{Duration, Instant};

    /// Generate a random syndrome with specified density
    #[must_use]
    #[allow(clippy::cast_precision_loss)]
    pub fn generate_random_syndrome(size: usize, density: f64, seed: u64) -> Vec<u8> {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut syndrome = vec![0u8; size];
        for (i, syndrome_bit) in syndrome.iter_mut().enumerate().take(size) {
            let mut hasher = DefaultHasher::new();
            (seed, i).hash(&mut hasher);
            let hash = hasher.finish();
            if (hash as f64 / u64::MAX as f64) < density {
                *syndrome_bit = 1;
            }
        }
        syndrome
    }

    /// Test sequential determinism for any decoder
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Any decoding operation fails
    /// - The results are not identical across runs
    pub fn test_sequential_determinism<D: Decoder>(
        mut decoder_factory: impl FnMut() -> D + Copy,
        syndrome: &[u8],
        runs: usize,
    ) -> Result<(), Box<dyn std::error::Error>>
    where
        D::Result: PartialEq + std::fmt::Debug,
    {
        let syndrome_array = Array1::from_vec(syndrome.to_vec());
        let syndrome_view = syndrome_array.view();
        let mut results = Vec::new();

        for _ in 0..runs {
            let mut decoder = decoder_factory();
            let result = decoder.decode(&syndrome_view)?;
            results.push(result);
        }

        // All results should be identical
        let first = &results[0];
        for (i, result) in results.iter().enumerate() {
            if result != first {
                return Err(
                    format!("Run {i} gave different result: {result:?} != {first:?}").into(),
                );
            }
        }

        Ok(())
    }

    /// Test parallel independence for any decoder
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Any decoding operation fails
    /// - The results differ between parallel executions
    ///
    /// # Panics
    ///
    /// Panics if a thread fails to acquire the mutex lock
    pub fn test_parallel_independence<D: Decoder + Send + 'static>(
        decoder_factory: impl Fn() -> D + Send + Sync + Clone + 'static,
        syndrome: Vec<u8>,
        num_threads: usize,
        iterations_per_thread: usize,
    ) -> Result<(), Box<dyn std::error::Error>>
    where
        D::Result: PartialEq + std::fmt::Debug + Send + 'static,
        D::Error: Send + 'static,
    {
        let results = Arc::new(Mutex::new(Vec::new()));
        let factory = Arc::new(decoder_factory);
        let syndrome = Arc::new(syndrome);

        let mut handles = vec![];

        for thread_id in 0..num_threads {
            let results_clone = Arc::clone(&results);
            let factory_clone = Arc::clone(&factory);
            let syndrome_clone = Arc::clone(&syndrome);

            let handle = thread::spawn(move || {
                for iteration in 0..iterations_per_thread {
                    let mut decoder = factory_clone();
                    let syndrome_array = Array1::from_vec(syndrome_clone.to_vec());
                    let syndrome_view = syndrome_array.view();

                    match decoder.decode(&syndrome_view) {
                        Ok(result) => {
                            results_clone
                                .lock()
                                .unwrap()
                                .push((thread_id, iteration, result));
                        }
                        Err(e) => {
                            log::error!("Thread {thread_id} iteration {iteration} failed: {e:?}");
                            return Err(e);
                        }
                    }

                    thread::sleep(Duration::from_millis(1));
                }
                Ok(())
            });

            handles.push(handle);
        }

        // Wait for all threads
        for handle in handles {
            handle.join().map_err(|_| "Thread panicked")??;
        }

        let final_results = results.lock().unwrap();

        // Check that all results are consistent
        if let Some((_, _, first_result)) = final_results.first() {
            for (thread_id, iteration, result) in final_results.iter() {
                if result != first_result {
                    return Err(format!(
                        "Thread {thread_id} iteration {iteration} gave different result: {result:?} != {first_result:?}"
                    )
                    .into());
                }
            }
        }

        Ok(())
    }

    /// Benchmark a decoder's performance
    ///
    /// # Errors
    ///
    /// Returns an error if any decoding operation fails during benchmarking
    pub fn benchmark_decoder<D: Decoder>(
        mut decoder: D,
        syndrome: &[u8],
        iterations: usize,
    ) -> Result<Duration, Box<dyn std::error::Error>> {
        let syndrome_array = Array1::from_vec(syndrome.to_vec());
        let syndrome_view = syndrome_array.view();

        let start = Instant::now();
        for _ in 0..iterations {
            decoder.decode(&syndrome_view)?;
        }
        let elapsed = start.elapsed();

        Ok(elapsed / u32::try_from(iterations).unwrap_or(u32::MAX))
    }
}

// ============================================================================
// Re-exports
// ============================================================================

/// Re-export common types
pub use ndarray;
pub use thiserror;
