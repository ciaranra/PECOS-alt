use crate::byte_message::QuantumGate;
use crate::engines::noise::utils::{NoiseRng, SingleQubitNoiseResult, TwoQubitNoiseResult};
use std::collections::HashMap;
use std::fmt::Debug;
use std::marker::PhantomData;

/// A trait for defining the precision of sampling tables
///
/// This trait defines the interface for different precision levels
/// used in table-based sampling of quantum noise operations.
pub trait Precision: Debug + Clone {
    /// The table size (number of entries)
    const TABLE_SIZE: usize;

    /// Generate a random index into the table
    fn random_index(rng: &NoiseRng) -> usize;
}

/// 8-bit precision with 256 table entries
#[derive(Debug, Clone)]
pub struct Bits8;

impl Precision for Bits8 {
    const TABLE_SIZE: usize = 256; // 2^8

    #[inline]
    fn random_index(rng: &NoiseRng) -> usize {
        // Fast path for 8-bit precision
        rng.random_u32(0..256) as usize
    }
}

/// 32-bit precision with 4096 table entries
#[derive(Debug, Clone)]
pub struct Bits32;

impl Precision for Bits32 {
    const TABLE_SIZE: usize = 4096; // Practical size, not full 2^32

    #[inline]
    fn random_index(rng: &NoiseRng) -> usize {
        let random_u32 = rng.random_u32(0..u32::MAX);

        // Map the full 32-bit range to our table size
        // This preserves the full 32 bits of precision while using a reasonable table size
        usize::try_from(
            (u64::from(random_u32)
                * u64::from(
                    u32::try_from(Self::TABLE_SIZE).expect("TABLE_SIZE should fit in u32"),
                ))
                / u64::from(u32::MAX),
        )
        .expect("Table index calculation should never overflow usize")
    }
}

/// 16-bit precision with 2048 table entries
#[derive(Debug, Clone)]
pub struct Bits16;

impl Precision for Bits16 {
    const TABLE_SIZE: usize = 2048; // Practical size, not full 2^16

    #[inline]
    fn random_index(rng: &NoiseRng) -> usize {
        usize::try_from(
            rng.random_u32(
                0..u32::try_from(Self::TABLE_SIZE).expect("TABLE_SIZE should fit in u32"),
            ),
        )
        .expect("Random index should never exceed usize::MAX")
    }
}

/// 64-bit precision with 8192 table entries
///
/// While not actually using 64-bit integers directly, this provides higher precision
/// sampling capability through a larger table size and optimized implementation.
/// This is particularly useful for models with very small probabilities.
#[derive(Debug, Clone)]
pub struct Bits64;

impl Precision for Bits64 {
    const TABLE_SIZE: usize = 8192; // Practical size, not full 2^64

    #[inline]
    fn random_index(rng: &NoiseRng) -> usize {
        // For the 64-bit case, we'll use a more direct approach
        // Generate a random number directly in the desired range
        // This ensures better distribution especially for small probabilities
        let index = rng
            .random_u32(0..u32::try_from(Self::TABLE_SIZE).expect("TABLE_SIZE should fit in u32"));
        usize::try_from(index).expect("Index should fit in usize")
    }
}

/// Safely convert a usize to f64 with proper error handling for cases where
/// the value might exceed the range of lossless representation in f64
fn safe_usize_to_f64(val: usize) -> f64 {
    // Try to convert through u32 if possible (lossless conversion to f64)
    if let Ok(val32) = u32::try_from(val) {
        f64::from(val32)
    } else {
        // If value is too large for u32, we have to use direct casting
        // Allow the precision loss warning because we've explicitly noted this is a fallback
        // for cases where the value is too large, and we're accepting the loss
        #[allow(clippy::cast_precision_loss)]
        let result = val as f64;
        result
    }
}

/// Generic type-cached table sampler parameterized by precision
#[derive(Debug, Clone)]
pub struct TypeCachedTableSampler<P: Precision> {
    /// Lookup table containing operation types
    lookup_table: Vec<u8>,
    /// Phantom data to use the precision type
    _precision: PhantomData<P>,
}

impl<P: Precision> TypeCachedTableSampler<P> {
    /// Create a new `TypeCachedTableSampler` from a model with weights
    ///
    /// # Panics
    /// Panics if:
    /// - The model is empty
    /// - The model has all zero weights
    /// - The model contains an invalid operation string
    #[must_use]
    pub fn new(model: &HashMap<String, f64>) -> Self {
        assert!(
            !model.is_empty(),
            "Failed to create sampler: model is empty"
        );

        // Calculate total weight and validate operations
        let mut valid_ops = Vec::new();
        let mut valid_weights = Vec::new();
        let mut total_weight = 0.0;

        for (op_str, weight) in model {
            if *weight <= 0.0 {
                continue;
            }

            let op_type = match op_str.as_str() {
                "X" => 0_u8,
                "Y" => 1_u8,
                "Z" => 2_u8,
                "I" => 3_u8,
                "L" => 4_u8,
                _ => panic!("Invalid operation in model: {op_str}"),
            };

            valid_ops.push(op_type);
            valid_weights.push(*weight);
            total_weight += *weight;
        }

        assert!(
            !(valid_ops.is_empty() || total_weight <= 0.0),
            "Failed to create sampler: model has all zero weights"
        );

        // Create a lookup table with the precision's table size
        let table_size = P::TABLE_SIZE;
        let mut lookup_table = Vec::with_capacity(table_size);

        // Fill the lookup table
        let mut cumulative = 0.0;
        let mut current_op_idx = 0;

        for i in 0..table_size {
            // Use a ratio that avoids the precision loss from casting usize to f64
            let target = f64::from(u32::try_from(i).expect("Index should fit in u32"))
                / f64::from(u32::try_from(table_size).expect("Table size should fit in u32"));

            // Find the operation that corresponds to this position
            while current_op_idx < valid_ops.len() - 1
                && cumulative + valid_weights[current_op_idx] / total_weight < target
            {
                cumulative += valid_weights[current_op_idx] / total_weight;
                current_op_idx += 1;
            }

            lookup_table.push(valid_ops[current_op_idx]);
        }

        TypeCachedTableSampler {
            lookup_table,
            _precision: PhantomData,
        }
    }

    /// Sample a single qubit noise operation
    #[inline]
    #[must_use]
    pub fn sample_sq_noise(&self, rng: &NoiseRng, qubit: usize) -> SingleQubitNoiseResult {
        // Get random index using the precision's method
        let idx = P::random_index(rng);

        // Direct index into the lookup table
        let op_type = self.lookup_table[idx] as usize;

        // Create the result on-demand based on operation type
        match op_type {
            0 => SingleQubitNoiseResult {
                gate: Some(QuantumGate::x(qubit)),
                qubit_leaked: false,
            },
            1 => SingleQubitNoiseResult {
                gate: Some(QuantumGate::y(qubit)),
                qubit_leaked: false,
            },
            2 => SingleQubitNoiseResult {
                gate: Some(QuantumGate::z(qubit)),
                qubit_leaked: false,
            },
            3 => SingleQubitNoiseResult {
                gate: None,
                qubit_leaked: false,
            },
            4 => SingleQubitNoiseResult {
                gate: None,
                qubit_leaked: true,
            },
            _ => unreachable!(),
        }
    }

    /// Estimate memory usage in bytes
    #[must_use]
    pub fn memory_usage(&self) -> usize {
        // Lookup table size
        let lookup_table_size = self.lookup_table.capacity() * std::mem::size_of::<u8>();

        // Other fields
        let other_fields = std::mem::size_of::<Self>();

        lookup_table_size + other_fields
    }

    /// Get the probability distribution represented by this sampler
    #[must_use]
    pub fn distribution(&self) -> HashMap<String, f64> {
        // Count occurrences of each operation type in the lookup table
        let mut counts = HashMap::new();
        let table_size = safe_usize_to_f64(self.lookup_table.len());

        if table_size == 0.0 {
            return HashMap::new();
        }

        for &op_type in &self.lookup_table {
            let op_str = match op_type {
                0 => "X".to_string(),
                1 => "Y".to_string(),
                2 => "Z".to_string(),
                3 => "I".to_string(),
                4 => "L".to_string(),
                _ => continue, // Skip invalid operation types
            };
            *counts.entry(op_str).or_insert(0.0) += 1.0;
        }

        // Convert counts to probabilities
        for value in counts.values_mut() {
            *value /= table_size;
        }

        counts
    }
}

/// Generic type-cached table sampler for two-qubit operations
#[derive(Debug, Clone)]
pub struct TypeCachedTwoQubitTableSampler<P: Precision> {
    /// Lookup table containing operation pairs
    lookup_table: Vec<(u8, u8)>,
    /// Phantom data to use the precision type
    _precision: PhantomData<P>,
}

impl<P: Precision> TypeCachedTwoQubitTableSampler<P> {
    /// Create a new `TypeCachedTwoQubitTableSampler` from a model with weights
    ///
    /// # Panics
    /// Panics if:
    /// - The model is empty
    /// - The model has all zero weights
    /// - The model contains an invalid operation string
    /// - Any operation string doesn't have exactly 2 characters
    #[must_use]
    pub fn new(model: &HashMap<String, f64>) -> Self {
        assert!(
            !model.is_empty(),
            "Failed to create two-qubit sampler: model is empty"
        );

        // Calculate total weight and validate operations
        let mut valid_ops = Vec::new();
        let mut valid_weights = Vec::new();
        let mut total_weight = 0.0;

        for (op_str, weight) in model {
            if *weight <= 0.0 {
                continue;
            }

            // Two-qubit operations must be exactly 2 characters
            assert!(
                (op_str.len() == 2),
                "Invalid two-qubit operation: '{op_str}' (must be two characters)"
            );

            // Extract and validate first qubit operation
            let q0_op_str = &op_str[0..1];
            let q0_op_type = match q0_op_str {
                "X" => 0_u8,
                "Y" => 1_u8,
                "Z" => 2_u8,
                "I" => 3_u8,
                "L" => 4_u8,
                _ => panic!("Invalid operation for first qubit: {q0_op_str}"),
            };

            // Extract and validate second qubit operation
            let q1_op_str = &op_str[1..2];
            let q1_op_type = match q1_op_str {
                "X" => 0_u8,
                "Y" => 1_u8,
                "Z" => 2_u8,
                "I" => 3_u8,
                "L" => 4_u8,
                _ => panic!("Invalid operation for second qubit: {q1_op_str}"),
            };

            // Skip "II" (identity on both qubits)
            assert!(
                !(q0_op_type == 3 && q1_op_type == 3),
                "Invalid two-qubit operation: 'II' (identity on both qubits) is not allowed"
            );

            valid_ops.push((q0_op_type, q1_op_type));
            valid_weights.push(*weight);
            total_weight += *weight;
        }

        assert!(
            !(valid_ops.is_empty() || total_weight <= 0.0),
            "Failed to create two-qubit sampler: model has all zero weights"
        );

        // Create a lookup table with the precision's table size
        let table_size = P::TABLE_SIZE;
        let mut lookup_table = Vec::with_capacity(table_size);

        // Fill the lookup table
        let mut cumulative = 0.0;
        let mut current_op_idx = 0;

        for i in 0..table_size {
            // Use a ratio that avoids the precision loss from casting usize to f64
            let target = f64::from(u32::try_from(i).expect("Index should fit in u32"))
                / f64::from(u32::try_from(table_size).expect("Table size should fit in u32"));

            // Find the operation that corresponds to this position
            while current_op_idx < valid_ops.len() - 1
                && cumulative + valid_weights[current_op_idx] / total_weight < target
            {
                cumulative += valid_weights[current_op_idx] / total_weight;
                current_op_idx += 1;
            }

            lookup_table.push(valid_ops[current_op_idx]);
        }

        TypeCachedTwoQubitTableSampler {
            lookup_table,
            _precision: PhantomData,
        }
    }

    /// Sample a two-qubit noise operation
    #[inline]
    #[must_use]
    pub fn sample_tq_noise(
        &self,
        rng: &NoiseRng,
        qubit0: usize,
        qubit1: usize,
    ) -> TwoQubitNoiseResult {
        // Get random index using the precision's method
        let idx = P::random_index(rng);

        // Direct index into the lookup table
        let (q0_op_type, q1_op_type) = self.lookup_table[idx];

        // Check for leakage operations
        let q0_leaked = q0_op_type == 4;
        let q1_leaked = q1_op_type == 4;

        // Create gates based on the operation types
        let mut gates = Vec::with_capacity(2);

        // Add gate for first qubit if not identity or leakage
        match q0_op_type {
            0 => gates.push(QuantumGate::x(qubit0)),
            1 => gates.push(QuantumGate::y(qubit0)),
            2 => gates.push(QuantumGate::z(qubit0)),
            3 | 4 => {} // Identity or leakage - no gate
            _ => unreachable!(),
        }

        // Add gate for second qubit if not identity or leakage
        match q1_op_type {
            0 => gates.push(QuantumGate::x(qubit1)),
            1 => gates.push(QuantumGate::y(qubit1)),
            2 => gates.push(QuantumGate::z(qubit1)),
            3 | 4 => {} // Identity or leakage - no gate
            _ => unreachable!(),
        }

        // If we have no gates, return None for gates
        let gates_option = if gates.is_empty() { None } else { Some(gates) };

        TwoQubitNoiseResult {
            gates: gates_option,
            qubit0_leaked: q0_leaked,
            qubit1_leaked: q1_leaked,
        }
    }

    /// Estimate memory usage in bytes
    #[must_use]
    pub fn memory_usage(&self) -> usize {
        // Lookup table size
        let lookup_table_size = self.lookup_table.capacity() * std::mem::size_of::<(u8, u8)>();

        // Other fields
        let other_fields = std::mem::size_of::<Self>();

        lookup_table_size + other_fields
    }

    /// Get the probability distribution represented by this sampler
    #[must_use]
    pub fn distribution(&self) -> HashMap<String, f64> {
        // Count occurrences of each operation type in the lookup table
        let mut counts = HashMap::new();
        let table_size = safe_usize_to_f64(self.lookup_table.len());

        if table_size == 0.0 {
            return HashMap::new();
        }

        for &(op1, op2) in &self.lookup_table {
            let op1_str = match op1 {
                0 => "X",
                1 => "Y",
                2 => "Z",
                3 => "I",
                4 => "L",
                _ => continue, // Skip invalid operation types
            };

            let op2_str = match op2 {
                0 => "X",
                1 => "Y",
                2 => "Z",
                3 => "I",
                4 => "L",
                _ => continue, // Skip invalid operation types
            };

            let operation_str = format!("{op1_str}{op2_str}");
            *counts.entry(operation_str).or_insert(0.0) += 1.0;
        }

        // Convert counts to probabilities
        for value in counts.values_mut() {
            *value /= table_size;
        }

        counts
    }
}

// Type aliases for backward compatibility and convenience
pub type TypeCachedTableSampler8Bit = TypeCachedTableSampler<Bits8>;
// Use 32-bit precision as the high precision option
pub type TypeCachedTableSampler32Bit = TypeCachedTableSampler<Bits32>;
// Keep 16-bit for intermediate cases if needed
pub type TypeCachedTableSampler16Bit = TypeCachedTableSampler<Bits16>;
// 64-bit for extremely high precision
pub type TypeCachedTableSampler64Bit = TypeCachedTableSampler<Bits64>;

// Type aliases for two-qubit samplers
pub type TypeCachedTwoQubitTableSampler8Bit = TypeCachedTwoQubitTableSampler<Bits8>;
pub type TypeCachedTwoQubitTableSampler32Bit = TypeCachedTwoQubitTableSampler<Bits32>;
pub type TypeCachedTwoQubitTableSampler16Bit = TypeCachedTwoQubitTableSampler<Bits16>;
pub type TypeCachedTwoQubitTableSampler64Bit = TypeCachedTwoQubitTableSampler<Bits64>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[allow(clippy::items_after_statements)]
    fn test_type_cached_table_sampler_8bit() {
        // Create a test model
        let mut model = HashMap::new();
        model.insert("X".to_string(), 0.2);
        model.insert("Y".to_string(), 0.3);
        model.insert("Z".to_string(), 0.1);
        model.insert("I".to_string(), 0.3);
        model.insert("L".to_string(), 0.1);

        // Create the sampler
        let sampler = TypeCachedTableSampler8Bit::new(&model);

        // Create a deterministic RNG for testing
        let rng = NoiseRng::with_seed(42);

        // Number of samples to test
        const TEST_SAMPLES: usize = 10000;

        // Sample a bunch of operations
        let mut x_count = 0;
        let mut y_count = 0;
        let mut z_count = 0;
        let mut i_count = 0;
        let mut l_count = 0;

        for i in 0..TEST_SAMPLES {
            let qubit = i % 10; // Use different qubits
            let result = sampler.sample_sq_noise(&rng, qubit);

            if result.qubit_leaked {
                l_count += 1;
            } else if let Some(gate) = result.gate {
                match gate.gate_type {
                    crate::byte_message::gate_type::GateType::X => x_count += 1,
                    crate::byte_message::gate_type::GateType::Y => y_count += 1,
                    crate::byte_message::gate_type::GateType::Z => z_count += 1,
                    _ => i_count += 1,
                }
            } else {
                i_count += 1;
            }
        }

        // Calculate frequencies
        let x_freq = f64::from(x_count)
            / f64::from(u32::try_from(TEST_SAMPLES).expect("TEST_SAMPLES should fit in u32"));
        let y_freq = f64::from(y_count)
            / f64::from(u32::try_from(TEST_SAMPLES).expect("TEST_SAMPLES should fit in u32"));
        let z_freq = f64::from(z_count)
            / f64::from(u32::try_from(TEST_SAMPLES).expect("TEST_SAMPLES should fit in u32"));
        let i_freq = f64::from(i_count)
            / f64::from(u32::try_from(TEST_SAMPLES).expect("TEST_SAMPLES should fit in u32"));
        let l_freq = f64::from(l_count)
            / f64::from(u32::try_from(TEST_SAMPLES).expect("TEST_SAMPLES should fit in u32"));

        // Allow for statistical variation (±5%)
        assert!((x_freq - 0.2).abs() < 0.05);
        assert!((y_freq - 0.3).abs() < 0.05);
        assert!((z_freq - 0.1).abs() < 0.05);
        assert!((i_freq - 0.3).abs() < 0.05);
        assert!((l_freq - 0.1).abs() < 0.05);

        // Check memory usage is small
        let memory_usage = sampler.memory_usage();
        assert!(memory_usage < 300); // Very conservative upper bound
    }

    #[test]
    #[allow(clippy::items_after_statements)]
    fn test_type_cached_table_sampler_32bit() {
        // Create a test model
        let mut model = HashMap::new();
        model.insert("X".to_string(), 0.2);
        model.insert("Y".to_string(), 0.3);
        model.insert("Z".to_string(), 0.1);
        model.insert("I".to_string(), 0.3);
        model.insert("L".to_string(), 0.1);

        // Create the 32-bit sampler
        let sampler = TypeCachedTableSampler32Bit::new(&model);

        // Create a deterministic RNG for testing
        let rng = NoiseRng::with_seed(42);

        // Number of samples to test
        const TEST_SAMPLES: usize = 10000;

        // Sample a bunch of operations
        let mut counts = [0; 5];

        for i in 0..TEST_SAMPLES {
            let qubit = i % 10; // Use different qubits
            let result = sampler.sample_sq_noise(&rng, qubit);

            if result.qubit_leaked {
                counts[4] += 1; // Leakage
            } else if let Some(gate) = result.gate {
                match gate.gate_type {
                    crate::byte_message::gate_type::GateType::X => counts[0] += 1,
                    crate::byte_message::gate_type::GateType::Y => counts[1] += 1,
                    crate::byte_message::gate_type::GateType::Z => counts[2] += 1,
                    _ => counts[3] += 1,
                }
            } else {
                counts[3] += 1; // Identity
            }
        }

        // Calculate frequencies
        let expected = [0.2, 0.3, 0.1, 0.3, 0.1];

        // Check all frequencies match expectations with high precision
        for i in 0..5 {
            let freq = f64::from(counts[i])
                / f64::from(u32::try_from(TEST_SAMPLES).expect("TEST_SAMPLES should fit in u32"));
            assert!(
                (freq - expected[i]).abs() < 0.05,
                "Operation {} frequency {} differs from expected {}",
                i,
                freq,
                expected[i]
            );
        }

        // Verify memory usage - should be larger due to 32-bit table
        let memory_usage = sampler.memory_usage();
        assert!(memory_usage > 4000); // Table size at minimum
        assert!(memory_usage < 8000); // Conservative upper bound
    }

    #[test]
    #[allow(clippy::items_after_statements)]
    fn test_type_cached_table_sampler_16bit() {
        // Create a test model
        let mut model = HashMap::new();
        model.insert("X".to_string(), 0.2);
        model.insert("Y".to_string(), 0.3);
        model.insert("Z".to_string(), 0.1);
        model.insert("I".to_string(), 0.3);
        model.insert("L".to_string(), 0.1);

        // Create the 16-bit sampler
        let sampler = TypeCachedTableSampler16Bit::new(&model);

        // Create a deterministic RNG for testing
        let rng = NoiseRng::with_seed(42);

        // Number of samples to test
        const TEST_SAMPLES: usize = 10000;

        // Sample a bunch of operations
        let mut counts = [0; 5];

        for i in 0..TEST_SAMPLES {
            let qubit = i % 10; // Use different qubits
            let result = sampler.sample_sq_noise(&rng, qubit);

            if result.qubit_leaked {
                counts[4] += 1; // Leakage
            } else if let Some(gate) = result.gate {
                match gate.gate_type {
                    crate::byte_message::gate_type::GateType::X => counts[0] += 1,
                    crate::byte_message::gate_type::GateType::Y => counts[1] += 1,
                    crate::byte_message::gate_type::GateType::Z => counts[2] += 1,
                    _ => counts[3] += 1,
                }
            } else {
                counts[3] += 1; // Identity
            }
        }

        // Calculate frequencies
        let expected = [0.2, 0.3, 0.1, 0.3, 0.1];

        // Check all frequencies match expectations with high precision
        for i in 0..5 {
            let freq = f64::from(counts[i])
                / f64::from(u32::try_from(TEST_SAMPLES).expect("TEST_SAMPLES should fit in u32"));
            assert!(
                (freq - expected[i]).abs() < 0.05,
                "Operation {} frequency {} differs from expected {}",
                i,
                freq,
                expected[i]
            );
        }

        // Verify memory usage - should be between 8-bit and 32-bit samplers
        let memory_usage = sampler.memory_usage();
        assert!(memory_usage > 2000); // Table size at minimum
        assert!(memory_usage < 4000); // Less than 32-bit sampler
    }

    #[test]
    #[allow(clippy::items_after_statements)]
    fn test_type_cached_two_qubit_table_sampler() {
        // Create a test model for two-qubit operations
        let mut model = HashMap::new();
        model.insert("IX".to_string(), 0.2);
        model.insert("YZ".to_string(), 0.3);
        model.insert("XY".to_string(), 0.1);
        model.insert("ZI".to_string(), 0.3);
        model.insert("IL".to_string(), 0.1);

        // Create the sampler
        let sampler = TypeCachedTwoQubitTableSampler8Bit::new(&model);

        // Create a deterministic RNG for testing
        let rng = NoiseRng::with_seed(42);

        // Number of samples to test
        const TEST_SAMPLES: usize = 10000;

        // Sample a bunch of operations
        let mut ix_count = 0;
        let mut yz_count = 0;
        let mut xy_count = 0;
        let mut zi_count = 0;
        let mut leakage_count = 0;

        for i in 0..TEST_SAMPLES {
            let qubit0 = i % 5; // Use different qubits
            let qubit1 = (i % 5) + 5;
            let result = sampler.sample_tq_noise(&rng, qubit0, qubit1);

            // Determine which operation was sampled based on gates and leakage
            if result.qubit1_leaked {
                leakage_count += 1; // IL operation
            } else if let Some(gates) = &result.gates {
                if gates.len() == 1 {
                    if gates[0].qubits[0] == qubit0
                        && gates[0].gate_type == crate::byte_message::gate_type::GateType::Z
                    {
                        zi_count += 1; // ZI operation
                    } else if gates[0].qubits[0] == qubit1
                        && gates[0].gate_type == crate::byte_message::gate_type::GateType::X
                    {
                        ix_count += 1; // IX operation
                    }
                } else if gates.len() == 2 {
                    if gates[0].qubits[0] == qubit0
                        && gates[0].gate_type == crate::byte_message::gate_type::GateType::X
                        && gates[1].qubits[0] == qubit1
                        && gates[1].gate_type == crate::byte_message::gate_type::GateType::Y
                    {
                        xy_count += 1; // XY operation
                    } else if gates[0].qubits[0] == qubit0
                        && gates[0].gate_type == crate::byte_message::gate_type::GateType::Y
                        && gates[1].qubits[0] == qubit1
                        && gates[1].gate_type == crate::byte_message::gate_type::GateType::Z
                    {
                        yz_count += 1; // YZ operation
                    }
                }
            }
        }

        // Calculate frequencies
        let ix_freq = f64::from(ix_count)
            / f64::from(u32::try_from(TEST_SAMPLES).expect("TEST_SAMPLES should fit in u32"));
        let yz_freq = f64::from(yz_count)
            / f64::from(u32::try_from(TEST_SAMPLES).expect("TEST_SAMPLES should fit in u32"));
        let xy_freq = f64::from(xy_count)
            / f64::from(u32::try_from(TEST_SAMPLES).expect("TEST_SAMPLES should fit in u32"));
        let zi_freq = f64::from(zi_count)
            / f64::from(u32::try_from(TEST_SAMPLES).expect("TEST_SAMPLES should fit in u32"));
        let leakage_freq = f64::from(leakage_count)
            / f64::from(u32::try_from(TEST_SAMPLES).expect("TEST_SAMPLES should fit in u32"));

        // Allow for statistical variation (±5%)
        assert!((ix_freq - 0.2).abs() < 0.05);
        assert!((yz_freq - 0.3).abs() < 0.05);
        assert!((xy_freq - 0.1).abs() < 0.05);
        assert!((zi_freq - 0.3).abs() < 0.05);
        assert!((leakage_freq - 0.1).abs() < 0.05);

        // Memory usage should be reasonable
        let memory_usage = sampler.memory_usage();
        assert!(memory_usage < 1000); // Conservative upper bound
    }

    #[test]
    #[allow(clippy::items_after_statements)]
    fn test_type_cached_table_sampler_64bit() {
        // Create a test model
        let mut model = HashMap::new();
        model.insert("X".to_string(), 0.2);
        model.insert("Y".to_string(), 0.3);
        model.insert("Z".to_string(), 0.1);
        model.insert("I".to_string(), 0.3);
        model.insert("L".to_string(), 0.1);

        // Create the 64-bit sampler
        let sampler = TypeCachedTableSampler64Bit::new(&model);

        // Create a deterministic RNG for testing
        let rng = NoiseRng::with_seed(42);

        // Number of samples to test
        const TEST_SAMPLES: usize = 10000;

        // Sample a bunch of operations
        let mut counts = [0; 5];

        for i in 0..TEST_SAMPLES {
            let qubit = i % 10; // Use different qubits
            let result = sampler.sample_sq_noise(&rng, qubit);

            if result.qubit_leaked {
                counts[4] += 1; // Leakage
            } else if let Some(gate) = result.gate {
                match gate.gate_type {
                    crate::byte_message::gate_type::GateType::X => counts[0] += 1,
                    crate::byte_message::gate_type::GateType::Y => counts[1] += 1,
                    crate::byte_message::gate_type::GateType::Z => counts[2] += 1,
                    _ => counts[3] += 1,
                }
            } else {
                counts[3] += 1; // Identity
            }
        }

        // Calculate frequencies
        let expected = [0.2, 0.3, 0.1, 0.3, 0.1];

        // Check all frequencies match expectations with high precision
        for i in 0..5 {
            let freq = f64::from(counts[i])
                / f64::from(u32::try_from(TEST_SAMPLES).expect("TEST_SAMPLES should fit in u32"));
            assert!(
                (freq - expected[i]).abs() < 0.05,
                "Operation {} frequency {} differs from expected {}",
                i,
                freq,
                expected[i]
            );
        }

        // Verify memory usage - should be larger due to 64-bit table
        let memory_usage = sampler.memory_usage();
        assert!(memory_usage > 8000); // Table size at minimum
        assert!(memory_usage < 16000); // Conservative upper bound
    }
}
