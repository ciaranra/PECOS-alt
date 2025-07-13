//! Exhaustive error pattern testing for decoders
//! Tests all possible 1-bit and 2-bit error patterns on small codes

use ndarray::{Array1, Array2};
use pecos_ldpc_decoders::*;

/// Helper function for backtracking in `generate_error_patterns`
fn backtrack(
    pattern: &mut Vec<u8>,
    patterns: &mut Vec<Vec<u8>>,
    start: usize,
    remaining: usize,
    n: usize,
) {
    if remaining == 0 {
        patterns.push(pattern.clone());
        return;
    }

    for i in start..=n - remaining {
        pattern[i] = 1;
        backtrack(pattern, patterns, i + 1, remaining - 1, n);
        pattern[i] = 0;
    }
}

/// Helper to generate all n-bit error patterns with exactly k bits set
fn generate_error_patterns(n: usize, k: usize) -> Vec<Vec<u8>> {
    let mut patterns = Vec::new();
    let mut pattern = vec![0u8; n];

    backtrack(&mut pattern, &mut patterns, 0, k, n);
    patterns
}

/// Test helper that verifies a decoder can correct all k-bit errors
fn test_all_k_bit_errors<D, F>(
    code_name: &str,
    decoder_name: &str,
    pcm: &Array2<u8>,
    k: usize,
    mut create_decoder: F,
) where
    F: FnMut() -> D,
    D: DecoderTrait,
{
    let n = pcm.ncols();
    let error_patterns = generate_error_patterns(n, k);
    let mut failed = 0;

    for error in &error_patterns {
        let error_array = Array1::from_vec(error.clone());
        let syndrome = pcm.dot(&error_array).mapv(|x| x % 2);

        let mut decoder = create_decoder();
        let result = decoder.decode(&syndrome.view()).unwrap();

        // Check if decoder found a valid solution (syndrome matches)
        let result_syndrome = pcm.dot(&result.decoding).mapv(|x| x % 2);
        if result_syndrome != syndrome {
            failed += 1;
            eprintln!(
                "{} {} failed on error pattern {:?}: got {:?}, syndrome mismatch",
                code_name, decoder_name, error, result.decoding
            );
        }
    }

    assert_eq!(
        failed,
        0,
        "{} {} failed to correct {}/{} {}-bit error patterns",
        code_name,
        decoder_name,
        failed,
        error_patterns.len(),
        k
    );

    println!(
        "{} {} successfully corrected all {} {}-bit error patterns",
        code_name,
        decoder_name,
        error_patterns.len(),
        k
    );
}

mod exhaustive_tests {
    use super::*;

    /// Create a repetition code PCM
    fn repetition_code_pcm(n: usize) -> Array2<u8> {
        let mut pcm = Array2::zeros((n - 1, n));
        for i in 0..n - 1 {
            pcm[[i, i]] = 1;
            pcm[[i, i + 1]] = 1;
        }
        pcm
    }

    /// Create a ring code PCM (cyclic repetition code)
    fn ring_code_pcm(n: usize) -> Array2<u8> {
        let mut pcm = Array2::zeros((n, n));
        for i in 0..n {
            pcm[[i, i]] = 1;
            pcm[[i, (i + 1) % n]] = 1;
        }
        pcm
    }

    #[test]
    fn test_bp_osd_exhaustive_single_bit_errors() {
        let pcm = repetition_code_pcm(7);
        let sparse_pcm = SparseMatrix::from_dense(&pcm.view());
        let error_rate = 0.1;

        test_all_k_bit_errors("Rep(7)", "BP+OSD", &pcm, 1, || {
            BpOsdDecoder::new(
                &sparse_pcm,
                Some(error_rate),
                None,
                20,
                BpMethod::ProductSum,
                BpSchedule::Parallel,
                0.625,
                OsdMethod::OsdCs,
                10,
                InputVectorType::Syndrome,
                None,
                None,
                None,
            )
            .unwrap()
        });
    }

    #[test]
    fn test_bp_lsd_exhaustive_single_bit_errors() {
        let pcm = repetition_code_pcm(7);
        let sparse_pcm = SparseMatrix::from_dense(&pcm.view());
        let error_rate = 0.1;

        test_all_k_bit_errors("Rep(7)", "BP+LSD", &pcm, 1, || {
            BpLsdDecoder::new(
                &sparse_pcm,
                Some(error_rate),
                None,
                20,
                BpMethod::ProductSum,
                BpSchedule::Parallel,
                0.625,
                OsdMethod::OsdCs,
                10,
                1,
                InputVectorType::Syndrome,
                None,
                None,
                None,
            )
            .unwrap()
        });
    }

    #[test]
    fn test_flip_decoder_exhaustive_single_bit_errors() {
        let pcm = repetition_code_pcm(7);
        let sparse_pcm = SparseMatrix::from_dense(&pcm.view());

        test_all_k_bit_errors("Rep(7)", "Flip", &pcm, 1, || {
            FlipDecoder::new(&sparse_pcm, 100, 5, 42).unwrap()
        });
    }

    #[test]
    fn test_belief_find_exhaustive_single_bit_errors() {
        let pcm = repetition_code_pcm(7);
        let sparse_pcm = SparseMatrix::from_dense(&pcm.view());
        let error_rate = 0.1;

        test_all_k_bit_errors("Rep(7)", "BeliefFind", &pcm, 1, || {
            BeliefFindDecoder::new(
                &sparse_pcm,
                Some(error_rate),
                None,
                20,
                BpMethod::ProductSum,
                0.625,
                BpSchedule::Parallel,
                None,
                None,
                None,
                UfMethod::Inversion,
                1,
            )
            .unwrap()
        });
    }

    #[test]
    fn test_union_find_exhaustive_single_bit_errors() {
        let pcm = repetition_code_pcm(7);
        let sparse_pcm = SparseMatrix::from_dense(&pcm.view());

        test_all_k_bit_errors("Rep(7)", "UnionFind", &pcm, 1, || {
            UnionFindDecoder::new(&sparse_pcm, UfMethod::Inversion).unwrap()
        });
    }

    #[test]
    fn test_bp_osd_exhaustive_two_bit_errors_small() {
        // Use smaller code for 2-bit errors to keep test time reasonable
        let pcm = repetition_code_pcm(5);
        let sparse_pcm = SparseMatrix::from_dense(&pcm.view());
        let error_rate = 0.1;

        test_all_k_bit_errors("Rep(5)", "BP+OSD", &pcm, 2, || {
            BpOsdDecoder::new(
                &sparse_pcm,
                Some(error_rate),
                None,
                20,
                BpMethod::ProductSum,
                BpSchedule::Parallel,
                0.625,
                OsdMethod::OsdCs,
                10,
                InputVectorType::Syndrome,
                None,
                None,
                None,
            )
            .unwrap()
        });
    }

    #[test]
    fn test_ring_code_exhaustive_single_bit() {
        // Test on ring code - harder than repetition code
        let pcm = ring_code_pcm(6);
        let sparse_pcm = SparseMatrix::from_dense(&pcm.view());
        let error_rate = 0.1;

        test_all_k_bit_errors("Ring(6)", "BP+OSD", &pcm, 1, || {
            BpOsdDecoder::new(
                &sparse_pcm,
                Some(error_rate),
                None,
                50, // More iterations for harder code
                BpMethod::ProductSum,
                BpSchedule::Parallel,
                0.625,
                OsdMethod::OsdCs,
                10,
                InputVectorType::Syndrome,
                None,
                None,
                None,
            )
            .unwrap()
        });
    }

    /// Test that decoders handle the zero syndrome correctly
    #[test]
    fn test_all_decoders_zero_syndrome() {
        let pcm = repetition_code_pcm(5);
        let sparse_pcm = SparseMatrix::from_dense(&pcm.view());
        let zero_syndrome = Array1::zeros(4);
        let error_rate = 0.1;

        // Test each decoder type
        let mut bp_osd = BpOsdDecoder::new(
            &sparse_pcm,
            Some(error_rate),
            None,
            20,
            BpMethod::ProductSum,
            BpSchedule::Parallel,
            0.625,
            OsdMethod::OsdCs,
            10,
            InputVectorType::Syndrome,
            None,
            None,
            None,
        )
        .unwrap();

        let result = bp_osd.decode(&zero_syndrome.view()).unwrap();
        assert_eq!(
            result.decoding.sum(),
            0,
            "BP+OSD should return zero for zero syndrome"
        );

        let mut flip = FlipDecoder::new(&sparse_pcm, 20, 5, 42).unwrap();
        let result = flip.decode(&zero_syndrome.view()).unwrap();
        assert_eq!(
            result.decoding.sum(),
            0,
            "Flip should return zero for zero syndrome"
        );
    }
}

// Trait to allow generic testing
trait DecoderTrait {
    fn decode(&mut self, syndrome: &ndarray::ArrayView1<u8>) -> Result<DecodingResult>;
}

// Implement trait for our decoders
impl DecoderTrait for BpOsdDecoder {
    fn decode(&mut self, syndrome: &ndarray::ArrayView1<u8>) -> Result<DecodingResult> {
        self.decode(syndrome)
    }
}

impl DecoderTrait for BpLsdDecoder {
    fn decode(&mut self, syndrome: &ndarray::ArrayView1<u8>) -> Result<DecodingResult> {
        self.decode(syndrome)
    }
}

impl DecoderTrait for FlipDecoder {
    fn decode(&mut self, syndrome: &ndarray::ArrayView1<u8>) -> Result<DecodingResult> {
        self.decode(syndrome)
    }
}

impl DecoderTrait for UnionFindDecoder {
    fn decode(&mut self, syndrome: &ndarray::ArrayView1<u8>) -> Result<DecodingResult> {
        self.decode(syndrome, &[], 0)
    }
}

impl DecoderTrait for BeliefFindDecoder {
    fn decode(&mut self, syndrome: &ndarray::ArrayView1<u8>) -> Result<DecodingResult> {
        self.decode(syndrome)
    }
}
