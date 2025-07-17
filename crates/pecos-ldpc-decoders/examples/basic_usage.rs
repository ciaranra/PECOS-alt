//! Basic usage example for LDPC decoders

use ndarray::{arr1, arr2};
use pecos_ldpc_decoders::{
    BeliefFindDecoder, BpLsdDecoder, BpMethod, BpOsdDecoder, BpSchedule, FlipDecoder,
    InputVectorType, OsdMethod, SoftInfoBpDecoder, SparseMatrix, UfMethod, UnionFindDecoder,
};

fn hamming_code() -> SparseMatrix {
    // Hamming(7,4) code parity check matrix
    let dense = arr2(&[
        [1, 0, 1, 0, 1, 0, 1],
        [0, 1, 1, 0, 0, 1, 1],
        [0, 0, 0, 1, 1, 1, 1],
    ]);

    SparseMatrix::from_dense(&dense.view())
}

#[allow(clippy::too_many_lines)]
#[allow(clippy::similar_names)] // bposd and bplsd are reasonable names
fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("LDPC Rust Example");
    println!("=================\n");

    // Create parity check matrix
    let pcm = hamming_code();
    println!(
        "Created Hamming(7,4) code with {}x{} parity check matrix",
        pcm.rows, pcm.cols
    );
    println!("Number of non-zero elements: {}\n", pcm.nnz());

    // Set up decoder parameters
    let error_rate = 0.1;
    let max_iter = 10;

    // Example syndrome (corresponding to error in positions 1 and 4)
    let syndrome = arr1(&[1, 1, 0]);

    // Test BP+OSD decoder
    println!("Testing BP+OSD decoder:");
    println!("----------------------");
    let mut bposd = BpOsdDecoder::new(
        &pcm,
        Some(error_rate),
        None,
        max_iter,
        BpMethod::MinimumSum,
        BpSchedule::Parallel,
        0.625,
        OsdMethod::OsdCs,
        2,
        InputVectorType::Syndrome,
        None, // omp_thread_count
        None, // serial_schedule_order
        None, // random_schedule_seed
    )?;

    let result_osd = bposd.decode(&syndrome.view())?;
    println!("Syndrome: {syndrome:?}");
    println!("Decoded error: {:?}", result_osd.decoding);
    println!("Converged: {}", result_osd.converged);
    println!("Iterations: {}\n", result_osd.iterations);

    // Test BP+LSD decoder
    println!("Testing BP+LSD decoder:");
    println!("----------------------");
    let mut bplsd = BpLsdDecoder::new(
        &pcm,
        Some(error_rate),
        None,
        max_iter,
        BpMethod::MinimumSum,
        BpSchedule::Parallel,
        0.625,
        OsdMethod::OsdCs,
        0,
        1,
        InputVectorType::Syndrome,
        None, // omp_thread_count
        None, // serial_schedule_order
        None, // random_schedule_seed
    )?;

    let result_lsd = bplsd.decode(&syndrome.view())?;
    println!("Syndrome: {syndrome:?}");
    println!("Decoded error: {:?}", result_lsd.decoding);
    println!("Converged: {}", result_lsd.converged);
    println!("Iterations: {}\n", result_lsd.iterations);

    // Test Soft Information BP decoder
    println!("Testing Soft Information BP decoder:");
    println!("-----------------------------------");
    let mut soft_bp = SoftInfoBpDecoder::new(
        &pcm,
        Some(error_rate),
        None,
        max_iter,
        BpMethod::MinimumSum,
        0.625,
        None, // omp_thread_count
        None, // serial_schedule_order
        None, // random_schedule_seed
    )?;

    // Create a soft syndrome (log-likelihood ratios)
    // Negative values indicate evidence for bit = 1
    let soft_syndrome = vec![
        -2.5, // Strong evidence for syndrome bit = 1
        -1.8, // Moderate evidence for syndrome bit = 1
        0.9,  // Weak evidence for syndrome bit = 0
    ];

    let cutoff = 1.0;
    let sigma = 1.0;

    let result_soft = soft_bp.decode(&soft_syndrome, cutoff, sigma)?;
    println!("Soft syndrome: {soft_syndrome:?}");
    println!("Decoded error: {:?}", result_soft.decoding);
    println!("Converged: {}", result_soft.converged);
    println!("Iterations: {}", result_soft.iterations);

    // Get log-probability ratios
    let llrs = soft_bp.log_prob_ratios();
    println!("Log-probability ratios: {llrs:?}\n");

    // Test Flip decoder
    println!("Testing Flip decoder:");
    println!("--------------------");
    let mut flip_decoder = FlipDecoder::new(
        &pcm, 20, // max_iter
        5,  // pfreq (perturb every 5 iterations)
        42, // random seed
    )?;

    let result_flip = flip_decoder.decode(&syndrome.view())?;
    println!("Syndrome: {syndrome:?}");
    println!("Decoded error: {:?}", result_flip.decoding);
    println!("Converged: {}", result_flip.converged);
    println!("Iterations: {}\n", result_flip.iterations);

    // Test Union Find decoder
    println!("Testing Union Find decoder:");
    println!("--------------------------");
    let mut uf_decoder = UnionFindDecoder::new(&pcm, UfMethod::Inversion)?;

    let empty_llrs: Vec<f64> = vec![];
    let result_uf = uf_decoder.decode(&syndrome.view(), &empty_llrs, 0)?;
    println!("Syndrome: {syndrome:?}");
    println!("Decoded error: {:?}", result_uf.decoding);
    println!("Converged: {}", result_uf.converged);
    println!("Iterations: {}\n", result_uf.iterations);

    // Test BeliefFind decoder
    println!("Testing BeliefFind decoder:");
    println!("---------------------------");
    let mut belief_find_decoder = BeliefFindDecoder::new(
        &pcm,
        Some(error_rate),
        None,
        max_iter,
        BpMethod::MinimumSum,
        0.625,
        BpSchedule::Parallel,
        None, // omp_thread_count
        None, // serial_schedule_order
        None, // random_schedule_seed
        UfMethod::Inversion,
        0, // bits_per_step (0 = all)
    )?;

    let result_bf = belief_find_decoder.decode(&syndrome.view())?;
    println!("Syndrome: {syndrome:?}");
    println!("Decoded error: {:?}", result_bf.decoding);
    println!("Converged: {}", result_bf.converged);
    println!("Iterations: {} (BP iterations)", result_bf.iterations);

    Ok(())
}
