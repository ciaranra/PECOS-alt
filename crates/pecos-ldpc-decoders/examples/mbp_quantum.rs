//! Example of using the MBP decoder for quantum CSS codes

use ndarray::Array1;
use pecos_ldpc_decoders::{BpMethod, CssCode, MbpDecoder, SparseMatrix};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create a simple CSS code (repetition code)
    // HX checks X errors: [1 1 1 0 0 0]
    //                     [0 0 0 1 1 1]
    let hx = SparseMatrix::from_coo(
        2,
        6, // 2 X checks, 6 qubits
        vec![0, 0, 0, 1, 1, 1],
        vec![0, 1, 2, 3, 4, 5],
    )?;

    // HZ checks Z errors: [1 1 0 0 0 0]
    //                     [0 1 1 0 0 0]
    //                     [0 0 0 1 1 0]
    //                     [0 0 0 0 1 1]
    let hz = SparseMatrix::from_coo(
        4,
        6, // 4 Z checks, 6 qubits
        vec![0, 0, 1, 1, 2, 2, 3, 3],
        vec![0, 1, 1, 2, 3, 4, 4, 5],
    )?;

    // Create CSS code
    let css = CssCode::new(hx.clone(), hz.clone())?;
    println!("Created CSS code with {} qubits", css.n);
    println!("X stabilizers: {}", css.mx);
    println!("Z stabilizers: {}", css.mz);

    // Create MBP decoder
    let mut decoder = MbpDecoder::new(
        &hx,
        &hz,
        0.1,                  // Physical error rate
        [1.0, 1.0, 1.0],      // Equal XYZ bias
        20,                   // Max iterations
        BpMethod::ProductSum, // BP method
        1.0,                  // MS scaling (not used for product-sum)
        Some(1),              // Single thread
    )?;

    // Example 1: No errors (all-zero syndrome)
    let syndrome = Array1::zeros(6); // 4 Z checks + 2 X checks
    let result = decoder.decode(&syndrome.view())?;
    println!("\nNo errors:");
    println!("Syndrome: {syndrome:?}");
    println!("Decoded: {result:?}");

    // Example 2: Single X error on qubit 1
    // This triggers Z stabilizer 0 and 1
    let mut syndrome = Array1::zeros(6);
    syndrome[0] = 1; // Z check 0
    syndrome[1] = 1; // Z check 1
    let result = decoder.decode(&syndrome.view())?;
    println!("\nX error on qubit 1:");
    println!("Syndrome: {syndrome:?}");
    println!("Decoded: {result:?}");

    // Example 3: Single Z error on qubit 2
    // This triggers X stabilizer 0
    let mut syndrome = Array1::zeros(6);
    syndrome[4] = 1; // X check 0 (index 4 = mz + 0)
    let result = decoder.decode(&syndrome.view())?;
    println!("\nZ error on qubit 2:");
    println!("Syndrome: {syndrome:?}");
    println!("Decoded: {result:?}");

    // Example 4: Y error on qubit 3 (both X and Z)
    // Triggers Z check 2 and X check 1
    let mut syndrome = Array1::zeros(6);
    syndrome[2] = 1; // Z check 2
    syndrome[5] = 1; // X check 1 (index 5 = mz + 1)
    let result = decoder.decode(&syndrome.view())?;
    println!("\nY error on qubit 3:");
    println!("Syndrome: {syndrome:?}");
    println!("Decoded: {result:?}");

    // Get GF(4) representation
    let gf4_result = decoder.decode_gf4(&syndrome.view())?;
    println!("GF(4) decoding: {gf4_result:?}");
    println!("(0=I, 1=X, 2=Y, 3=Z)");

    Ok(())
}
