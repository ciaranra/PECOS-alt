//! Tests using pre-computed PCM matrices from the Python test suite

use ndarray::arr1;
use pecos_ldpc_decoders::{
    BpLsdDecoder, BpMethod, BpOsdDecoder, BpSchedule, InputVectorType, OsdMethod, SparseMatrix,
};

fn create_surface_code_3() -> SparseMatrix {
    // Surface code distance 3 (simplified version)
    let row_indices = vec![0, 0, 1, 1, 2, 2, 3, 3, 4, 4, 4, 4, 5, 5, 5, 5, 6, 6, 7, 7];
    let col_indices = vec![0, 1, 2, 3, 0, 2, 1, 3, 0, 1, 4, 5, 2, 3, 4, 5, 4, 6, 5, 6];
    SparseMatrix::from_coo(8, 7, row_indices, col_indices).unwrap()
}

fn create_hamming_7_4() -> SparseMatrix {
    let mut row_indices = Vec::new();
    let mut col_indices = Vec::new();

    // Standard Hamming(7,4) parity check matrix
    row_indices.extend(&[0, 0, 0, 0]);
    col_indices.extend(&[3, 4, 5, 6]);

    row_indices.extend(&[1, 1, 1, 1]);
    col_indices.extend(&[1, 2, 5, 6]);

    row_indices.extend(&[2, 2, 2, 2]);
    col_indices.extend(&[0, 2, 4, 6]);

    SparseMatrix::from_coo(3, 7, row_indices, col_indices).unwrap()
}

#[cfg(test)]
mod pcm_matrix_tests {
    use super::*;

    #[test]
    fn test_surface_code_basic() {
        let pcm = create_surface_code_3();
        assert_eq!(pcm.rows, 8);
        assert_eq!(pcm.cols, 7);

        let error_rate = 0.1;
        let syndrome = arr1(&[1, 0, 0, 0, 1, 0, 0, 0]);

        let mut decoder = BpOsdDecoder::new(
            &pcm,
            Some(error_rate),
            None,
            50,
            BpMethod::MinimumSum,
            BpSchedule::Parallel,
            0.625,
            OsdMethod::OsdCs,
            3,
            InputVectorType::Syndrome,
            None,
            None,
            None,
        )
        .unwrap();

        let result = decoder.decode(&syndrome.view()).unwrap();
        println!(
            "Surface code decoding: converged = {}, iterations = {}",
            result.converged, result.iterations
        );
    }

    #[test]
    #[allow(clippy::cast_precision_loss)] // Acceptable for computing average iterations
    fn test_performance_comparison() {
        // Test decoding performance with different decoder configurations
        let pcm = create_surface_code_3();
        let error_rate = 0.05;

        // Generate multiple random-like syndromes
        let test_syndromes = vec![
            arr1(&[1, 0, 0, 0, 0, 0, 0, 0]),
            arr1(&[0, 1, 0, 1, 0, 0, 0, 0]),
            arr1(&[1, 1, 0, 0, 1, 0, 0, 0]),
            arr1(&[0, 0, 1, 1, 0, 1, 0, 0]),
            arr1(&[1, 0, 1, 0, 1, 0, 1, 0]),
        ];

        for (method_name, bp_method, bp_schedule) in [
            (
                "Product-Sum Parallel",
                BpMethod::ProductSum,
                BpSchedule::Parallel,
            ),
            (
                "Min-Sum Parallel",
                BpMethod::MinimumSum,
                BpSchedule::Parallel,
            ),
            ("Min-Sum Serial", BpMethod::MinimumSum, BpSchedule::Serial),
        ] {
            let mut total_iterations = 0;
            let mut converged_count = 0;

            for syndrome in &test_syndromes {
                let mut decoder = BpOsdDecoder::new(
                    &pcm,
                    Some(error_rate),
                    None,
                    20,
                    bp_method,
                    bp_schedule,
                    0.625,
                    OsdMethod::Off,
                    0,
                    InputVectorType::Syndrome,
                    None,
                    None,
                    None,
                )
                .unwrap();

                let result = decoder.decode(&syndrome.view()).unwrap();
                total_iterations += result.iterations;
                if result.converged {
                    converged_count += 1;
                }
            }

            println!(
                "{}: {}/{} converged, avg iterations: {:.1}",
                method_name,
                converged_count,
                test_syndromes.len(),
                total_iterations as f64 / test_syndromes.len() as f64
            );
        }
    }

    #[test]
    fn test_lsd_on_structured_codes() {
        let pcm = create_surface_code_3();
        let error_rate = 0.1;

        // Test syndrome that might benefit from LSD clustering
        let syndrome = arr1(&[1, 1, 0, 0, 1, 1, 0, 0]);

        // Compare BP+OSD vs BP+LSD
        let mut bp_osd = BpOsdDecoder::new(
            &pcm,
            Some(error_rate),
            None,
            20,
            BpMethod::MinimumSum,
            BpSchedule::Parallel,
            0.625,
            OsdMethod::OsdCs,
            3,
            InputVectorType::Syndrome,
            None,
            None,
            None,
        )
        .unwrap();

        let mut lsd_decoder = BpLsdDecoder::new(
            &pcm,
            Some(error_rate),
            None,
            20,
            BpMethod::MinimumSum,
            BpSchedule::Parallel,
            0.625,
            OsdMethod::OsdCs,
            3,
            1,
            InputVectorType::Syndrome,
            None,
            None,
            None,
        )
        .unwrap();

        lsd_decoder.set_do_stats(true);

        let osd_result = bp_osd.decode(&syndrome.view()).unwrap();
        let lsd_result = lsd_decoder.decode(&syndrome.view()).unwrap();

        println!(
            "BP+OSD: converged = {}, iterations = {}",
            osd_result.converged, osd_result.iterations
        );
        println!(
            "BP+LSD: converged = {}, iterations = {}",
            lsd_result.converged, lsd_result.iterations
        );

        if lsd_result.converged {
            let stats = lsd_decoder.get_statistics_json().unwrap();
            println!("LSD statistics preview: {}", &stats[..stats.len().min(200)]);
        }
    }
}

#[cfg(test)]
mod quantum_code_tests {
    use super::*;

    #[test]
    fn test_css_code_simulation() {
        // Simulate a simple CSS code scenario
        // In practice, we would have separate Hx and Hz matrices
        let hx = create_hamming_7_4();
        let hz = create_hamming_7_4(); // Same for simplicity

        // Test X-type errors (use Hx)
        let syndrome_x = arr1(&[1, 0, 1]);

        let mut decoder_x = BpOsdDecoder::new(
            &hx,
            Some(0.01),
            None,
            20,
            BpMethod::MinimumSum,
            BpSchedule::Parallel,
            0.625,
            OsdMethod::OsdCs,
            2,
            InputVectorType::Syndrome,
            None,
            None,
            None,
        )
        .unwrap();

        let result_x = decoder_x.decode(&syndrome_x.view()).unwrap();

        // Test Z-type errors (use Hz)
        let syndrome_z = arr1(&[0, 1, 1]);

        let mut decoder_z = BpOsdDecoder::new(
            &hz,
            Some(0.01),
            None,
            20,
            BpMethod::MinimumSum,
            BpSchedule::Parallel,
            0.625,
            OsdMethod::OsdCs,
            2,
            InputVectorType::Syndrome,
            None,
            None,
            None,
        )
        .unwrap();

        let result_z = decoder_z.decode(&syndrome_z.view()).unwrap();

        println!("X-error decoding: converged = {}", result_x.converged);
        println!("Z-error decoding: converged = {}", result_z.converged);
    }
}

#[cfg(test)]
mod stress_tests {
    use super::*;
    use std::time::Instant;

    #[test]
    fn test_repeated_decoding() {
        // Test memory stability with repeated decodings
        let pcm = create_hamming_7_4();
        let error_rate = 0.1;

        let mut decoder = BpOsdDecoder::new(
            &pcm,
            Some(error_rate),
            None,
            10,
            BpMethod::MinimumSum,
            BpSchedule::Parallel,
            0.625,
            OsdMethod::Off,
            0,
            InputVectorType::Syndrome,
            None,
            None,
            None,
        )
        .unwrap();

        // Decode many times with different syndromes
        for i in 0..100 {
            let syndrome = match i % 4 {
                0 => arr1(&[0, 0, 0]),
                1 => arr1(&[1, 0, 0]),
                2 => arr1(&[0, 1, 1]),
                _ => arr1(&[1, 1, 1]),
            };

            let result = decoder.decode(&syndrome.view()).unwrap();
            assert_eq!(result.decoding.len(), 7);
        }
    }

    #[test]
    fn test_decoding_speed() {
        let pcm = create_surface_code_3();
        let error_rate = 0.1;
        let syndrome = arr1(&[1, 0, 1, 0, 1, 0, 1, 0]);

        let mut decoder = BpOsdDecoder::new(
            &pcm,
            Some(error_rate),
            None,
            20,
            BpMethod::MinimumSum,
            BpSchedule::Parallel,
            0.625,
            OsdMethod::Off,
            0,
            InputVectorType::Syndrome,
            None,
            None,
            None,
        )
        .unwrap();

        let num_runs = 10000;
        let start = Instant::now();

        for _ in 0..num_runs {
            let _ = decoder.decode(&syndrome.view()).unwrap();
        }

        let elapsed = start.elapsed();
        let decodings_per_second = f64::from(num_runs) / elapsed.as_secs_f64();

        println!(
            "Decoded {} syndromes in {:.2}s",
            num_runs,
            elapsed.as_secs_f64()
        );
        println!("Speed: {decodings_per_second:.0} decodings/second");

        // Basic performance assertion
        assert!(
            decodings_per_second > 1000.0,
            "Decoding speed too slow: {decodings_per_second} decodings/second"
        );
    }
}
