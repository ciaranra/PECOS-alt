//! Common matrix types and traits for decoders
//!
//! This module provides standardized matrix representations that decoders
//! can use for parity check matrices and related structures.

use crate::errors::{DecoderError, MatrixError};
use ndarray::{Array2, ArrayView2};

/// Common trait for decoders that can be constructed from check matrices
pub trait CheckMatrixDecoder: super::Decoder {
    /// Configuration type for check matrix construction
    type CheckMatrixConfig: Default;

    /// Create decoder from a dense check matrix
    ///
    /// # Errors
    ///
    /// Returns [`DecoderError`] if:
    /// - The matrix dimensions are invalid (e.g., empty)
    /// - The matrix values are invalid (only 0 and 1 allowed)
    /// - The decoder cannot be constructed from the matrix
    fn from_dense_matrix(check_matrix: &ArrayView2<u8>) -> Result<Self, DecoderError>
    where
        Self: Sized,
    {
        Self::from_dense_matrix_with_config(check_matrix, Default::default())
    }

    /// Create decoder from a dense check matrix with configuration
    ///
    /// # Errors
    ///
    /// Returns [`DecoderError`] if:
    /// - The matrix dimensions are invalid
    /// - The matrix values are invalid
    /// - The configuration is invalid
    /// - The decoder cannot be constructed with the given parameters
    fn from_dense_matrix_with_config(
        check_matrix: &ArrayView2<u8>,
        config: Self::CheckMatrixConfig,
    ) -> Result<Self, DecoderError>
    where
        Self: Sized;

    /// Create decoder from a sparse check matrix (COO format)
    ///
    /// # Errors
    ///
    /// Returns [`DecoderError`] if:
    /// - The rows and cols vectors have different lengths
    /// - Any index is out of bounds for the given shape
    /// - The shape dimensions are invalid
    /// - The decoder cannot be constructed from the matrix
    fn from_sparse_matrix(
        rows: Vec<usize>,
        cols: Vec<usize>,
        shape: (usize, usize),
    ) -> Result<Self, DecoderError>
    where
        Self: Sized,
    {
        Self::from_sparse_matrix_with_config(rows, cols, shape, Default::default())
    }

    /// Create decoder from a sparse check matrix with configuration
    ///
    /// # Errors
    ///
    /// Returns [`DecoderError`] if:
    /// - The sparse matrix format is invalid
    /// - The configuration is invalid
    /// - The decoder cannot be constructed with the given parameters
    fn from_sparse_matrix_with_config(
        rows: Vec<usize>,
        cols: Vec<usize>,
        shape: (usize, usize),
        config: Self::CheckMatrixConfig,
    ) -> Result<Self, DecoderError>
    where
        Self: Sized;
}

/// Common sparse matrix representation (COO format)
#[derive(Debug, Clone, PartialEq)]
pub struct SparseCheckMatrix {
    /// Row indices of non-zero entries
    pub rows: Vec<usize>,
    /// Column indices of non-zero entries
    pub cols: Vec<usize>,
    /// Values of non-zero entries (optional, defaults to 1)
    pub values: Option<Vec<u8>>,
    /// Shape of the matrix (rows, cols)
    pub shape: (usize, usize),
}

impl SparseCheckMatrix {
    /// Create a new sparse check matrix
    ///
    /// # Errors
    ///
    /// Returns [`MatrixError`] if:
    /// - The rows and cols vectors have different lengths
    /// - Any index is out of bounds for the given shape
    pub fn new(
        rows: Vec<usize>,
        cols: Vec<usize>,
        shape: (usize, usize),
    ) -> Result<Self, MatrixError> {
        if rows.len() != cols.len() {
            return Err(MatrixError::InvalidDimensions {
                rows: rows.len(),
                cols: cols.len(),
            });
        }

        // Validate indices
        for (&r, &c) in rows.iter().zip(cols.iter()) {
            if r >= shape.0 || c >= shape.1 {
                return Err(MatrixError::IndexOutOfBounds {
                    row: r,
                    col: c,
                    rows: shape.0,
                    cols: shape.1,
                });
            }
        }

        Ok(Self {
            rows,
            cols,
            values: None,
            shape,
        })
    }

    /// Create with explicit values
    ///
    /// # Errors
    ///
    /// Returns [`MatrixError`] if:
    /// - The rows, cols, and values vectors have different lengths
    /// - Any index is out of bounds for the given shape
    pub fn with_values(
        rows: Vec<usize>,
        cols: Vec<usize>,
        values: Vec<u8>,
        shape: (usize, usize),
    ) -> Result<Self, MatrixError> {
        if rows.len() != cols.len() || rows.len() != values.len() {
            return Err(MatrixError::InvalidDimensions {
                rows: rows.len(),
                cols: cols.len(),
            });
        }

        let mut matrix = Self::new(rows, cols, shape)?;
        matrix.values = Some(values);
        Ok(matrix)
    }

    /// Convert to dense representation
    #[must_use]
    pub fn to_dense(&self) -> Array2<u8> {
        let mut dense = Array2::zeros(self.shape);

        if let Some(values) = &self.values {
            for ((&r, &c), &v) in self.rows.iter().zip(&self.cols).zip(values) {
                dense[[r, c]] = v;
            }
        } else {
            for (&r, &c) in self.rows.iter().zip(&self.cols) {
                dense[[r, c]] = 1;
            }
        }

        dense
    }

    /// Get the number of non-zero entries
    #[must_use]
    pub fn nnz(&self) -> usize {
        self.rows.len()
    }

    /// Get the density of the matrix (nnz / `total_elements`)
    #[must_use]
    #[allow(clippy::cast_precision_loss)]
    pub fn density(&self) -> f64 {
        let total = self.shape.0 * self.shape.1;
        if total == 0 {
            0.0
        } else {
            self.nnz() as f64 / total as f64
        }
    }
}

/// Configuration for check matrix decoders
#[derive(Debug, Clone, PartialEq, Default)]
pub struct CheckMatrixConfig {
    /// Edge weights for the decoder graph
    pub weights: Option<Vec<f64>>,
    /// Measurement error probabilities
    pub measurement_error_probs: Option<Vec<f64>>,
    /// Timelike weights for spacetime codes
    pub timelike_weights: Option<Vec<f64>>,
    /// Number of repetitions (for repetition codes)
    pub repetitions: Option<usize>,
    /// Use virtual boundary nodes
    pub use_virtual_boundary: bool,
    /// Custom observable count (if different from matrix)
    pub num_observables: Option<usize>,
}

/// Utility functions for matrix operations
pub mod utils {
    use super::{ArrayView2, MatrixError, SparseCheckMatrix};

    /// Convert dense matrix to sparse COO format
    #[must_use]
    pub fn dense_to_sparse(matrix: &ArrayView2<u8>) -> SparseCheckMatrix {
        let mut rows = Vec::new();
        let mut cols = Vec::new();
        let mut values = Vec::new();

        for ((r, c), &v) in matrix.indexed_iter() {
            if v != 0 {
                rows.push(r);
                cols.push(c);
                values.push(v);
            }
        }

        let shape = (matrix.nrows(), matrix.ncols());
        SparseCheckMatrix {
            rows,
            cols,
            values: Some(values),
            shape,
        }
    }

    /// Validate that a matrix is a valid parity check matrix
    ///
    /// # Errors
    ///
    /// Returns [`MatrixError`] if:
    /// - The matrix is empty
    /// - The matrix contains values other than 0 and 1
    pub fn validate_check_matrix(matrix: &ArrayView2<u8>) -> Result<(), MatrixError> {
        if matrix.is_empty() {
            return Err(MatrixError::EmptyMatrix);
        }

        // Check that all values are 0 or 1
        for &value in matrix {
            if value > 1 {
                return Err(MatrixError::InvalidFormat(
                    "Check matrix should only contain 0 and 1".to_string(),
                ));
            }
        }

        Ok(())
    }

    /// Calculate the row and column weights of a check matrix
    #[must_use]
    pub fn matrix_weights(matrix: &ArrayView2<u8>) -> (Vec<usize>, Vec<usize>) {
        let row_weights: Vec<usize> = (0..matrix.nrows())
            .map(|r| matrix.row(r).iter().filter(|&&v| v != 0).count())
            .collect();

        let col_weights: Vec<usize> = (0..matrix.ncols())
            .map(|c| matrix.column(c).iter().filter(|&&v| v != 0).count())
            .collect();

        (row_weights, col_weights)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sparse_matrix_creation() {
        let rows = vec![0, 1, 2];
        let cols = vec![1, 2, 0];
        let matrix = SparseCheckMatrix::new(rows, cols, (3, 3)).unwrap();

        assert_eq!(matrix.nnz(), 3);
        assert_eq!(matrix.shape, (3, 3));
    }

    #[test]
    fn test_sparse_to_dense_conversion() {
        let rows = vec![0, 1, 1];
        let cols = vec![0, 1, 2];
        let matrix = SparseCheckMatrix::new(rows, cols, (2, 3)).unwrap();

        let dense = matrix.to_dense();
        assert_eq!(dense[[0, 0]], 1);
        assert_eq!(dense[[1, 1]], 1);
        assert_eq!(dense[[1, 2]], 1);
        assert_eq!(dense[[0, 1]], 0);
    }

    #[test]
    fn test_matrix_validation() {
        let valid = Array2::from_shape_vec((2, 3), vec![1, 0, 1, 0, 1, 0]).unwrap();
        assert!(utils::validate_check_matrix(&valid.view()).is_ok());

        let invalid = Array2::from_shape_vec((2, 3), vec![1, 0, 2, 0, 1, 0]).unwrap();
        assert!(utils::validate_check_matrix(&invalid.view()).is_err());
    }
}
