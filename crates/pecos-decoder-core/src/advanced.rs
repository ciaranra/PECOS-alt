//! Advanced decoding features and traits
//!
//! This module provides traits for advanced decoder capabilities like
//! erasure decoding, dynamic weights, and detailed matching information.

use crate::errors::DecoderError;
use crate::results::StandardDecodingResult;
use ndarray::ArrayView1;

/// Trait for decoders that support erasure information
pub trait ErasureDecoder: super::Decoder {
    /// Decode with erasure information
    ///
    /// - `syndrome`: The syndrome or detection events
    /// - `erasures`: Indices of known erasure locations
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The syndrome dimensions don't match the decoder's expectations
    /// - Any erasure index is out of bounds
    /// - The decoding process fails to converge
    fn decode_with_erasures(
        &mut self,
        syndrome: &ArrayView1<u8>,
        erasures: &[usize],
    ) -> Result<Self::Result, Self::Error>;
}

/// Trait for decoders that support dynamic edge weights
pub trait DynamicWeightDecoder: super::Decoder {
    /// Update edge weights dynamically
    ///
    /// - `edges`: List of (node1, node2) pairs
    /// - `weights`: New weights for each edge
    ///
    /// # Errors
    ///
    /// Returns [`DecoderError`] if:
    /// - The number of edges and weights don't match
    /// - Any edge refers to invalid node indices
    /// - Any weight is invalid (e.g., negative or NaN)
    fn update_edge_weights(
        &mut self,
        edges: &[(usize, usize)],
        weights: &[f64],
    ) -> Result<(), DecoderError>;

    /// Reset all weights to their initial values
    ///
    /// # Errors
    ///
    /// Returns [`DecoderError`] if the decoder doesn't support weight reset
    fn reset_weights(&mut self) -> Result<(), DecoderError>;

    /// Decode with temporary weight modifications
    ///
    /// Weights are automatically reset after decoding
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Weight update fails (see [`update_edge_weights`](Self::update_edge_weights))
    /// - Decoding fails with the modified weights
    /// - Weight reset fails after decoding
    fn decode_with_weights(
        &mut self,
        syndrome: &ArrayView1<u8>,
        edges: &[(usize, usize)],
        weights: &[f64],
    ) -> Result<Self::Result, Self::Error>
    where
        Self::Error: From<DecoderError>,
    {
        self.update_edge_weights(edges, weights)?;
        let result = self.decode(syndrome);
        self.reset_weights()?;
        result
    }
}

/// Information about a matched edge in the decoding
#[derive(Debug, Clone, PartialEq)]
pub struct MatchedEdge {
    /// First node index
    pub node1: usize,
    /// Second node index (or boundary marker)
    pub node2: usize,
    /// Weight of the edge
    pub weight: f64,
    /// Observable flips associated with this edge
    pub observables: Vec<usize>,
}

/// Information about a matched pair of detectors
#[derive(Debug, Clone, PartialEq)]
pub struct MatchedPair {
    /// First detector index
    pub detector1: usize,
    /// Second detector index (None for boundary)
    pub detector2: Option<usize>,
    /// Weight/cost of the matching
    pub weight: f64,
}

/// Trait for decoders that can provide detailed matching information
pub trait DetailedDecoder: super::Decoder {
    /// Decode and return matched edges
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The syndrome dimensions are invalid
    /// - The decoding process fails
    /// - The decoder cannot provide edge information
    fn decode_to_edges(
        &mut self,
        syndrome: &ArrayView1<u8>,
    ) -> Result<Vec<MatchedEdge>, Self::Error>;

    /// Decode and return matched detector pairs
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The syndrome dimensions are invalid
    /// - The decoding process fails
    /// - The decoder cannot provide pair information
    fn decode_to_pairs(
        &mut self,
        syndrome: &ArrayView1<u8>,
    ) -> Result<Vec<MatchedPair>, Self::Error>;

    /// Get detailed statistics about the last decoding
    fn get_stats(&self) -> DecodingStats;
}

/// Statistics about a decoding operation
#[derive(Debug, Clone, Default)]
pub struct DecodingStats {
    /// Number of iterations performed (if applicable)
    pub iterations: Option<usize>,
    /// Time taken for decoding
    pub time_taken: Option<std::time::Duration>,
    /// Number of nodes explored (for search-based decoders)
    pub nodes_explored: Option<usize>,
    /// Number of blossoms formed (for matching decoders)
    pub blossoms_formed: Option<usize>,
    /// Whether the decoder converged
    pub converged: bool,
    /// Confidence in the result (0.0 to 1.0)
    pub confidence: Option<f64>,
}

/// Options for advanced decoding
#[derive(Debug, Clone, Default)]
pub struct DecodingOptions {
    /// Return detailed matching information
    pub return_details: bool,
    /// Maximum iterations (overrides decoder default)
    pub max_iterations: Option<usize>,
    /// Early termination threshold
    pub early_termination_threshold: Option<f64>,
    /// Enable verbose logging
    pub verbose: bool,
    /// Custom erasure locations
    pub erasures: Option<Vec<usize>>,
    /// Dynamic edge weights
    pub edge_weights: Option<Vec<(usize, usize, f64)>>,
}

/// Trait for decoders that support advanced options
pub trait AdvancedDecoder: super::Decoder {
    /// Decode with advanced options
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The syndrome dimensions are invalid
    /// - Any option values are invalid (e.g., negative `max_iterations`)
    /// - Erasure indices are out of bounds
    /// - Edge weight updates fail
    /// - The decoding process fails
    fn decode_advanced(
        &mut self,
        syndrome: &ArrayView1<u8>,
        options: DecodingOptions,
    ) -> Result<AdvancedDecodingResult<Self::Result>, Self::Error>;
}

/// Result type for advanced decoding
#[derive(Debug, Clone)]
pub struct AdvancedDecodingResult<R> {
    /// The basic decoding result
    pub result: R,
    /// Detailed statistics
    pub stats: DecodingStats,
    /// Matched edges (if requested)
    pub matched_edges: Option<Vec<MatchedEdge>>,
    /// Matched pairs (if requested)
    pub matched_pairs: Option<Vec<MatchedPair>>,
}

impl<R> AdvancedDecodingResult<R> {
    /// Create a new advanced result from a basic result
    pub fn from_basic(result: R) -> Self {
        Self {
            result,
            stats: DecodingStats::default(),
            matched_edges: None,
            matched_pairs: None,
        }
    }

    /// Add statistics
    #[must_use]
    pub fn with_stats(mut self, stats: DecodingStats) -> Self {
        self.stats = stats;
        self
    }

    /// Add matched edges
    #[must_use]
    pub fn with_edges(mut self, edges: Vec<MatchedEdge>) -> Self {
        self.matched_edges = Some(edges);
        self
    }

    /// Add matched pairs
    #[must_use]
    pub fn with_pairs(mut self, pairs: Vec<MatchedPair>) -> Self {
        self.matched_pairs = Some(pairs);
        self
    }
}

/// Convert advanced result to standard result
impl<R: crate::results::DecodingResultTrait> From<AdvancedDecodingResult<R>>
    for StandardDecodingResult
{
    fn from(advanced: AdvancedDecodingResult<R>) -> Self {
        let basic = advanced.result.to_standard();
        StandardDecodingResult {
            observable: basic.observable,
            weight: basic.weight,
            converged: Some(advanced.stats.converged),
            iterations: advanced.stats.iterations,
            confidence: advanced.stats.confidence,
        }
    }
}
