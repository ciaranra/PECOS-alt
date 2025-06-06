fn main() {
    let nan = (-1.0_f64).ln();
    println!("ln(-1) = {}", nan);
    println!("is_nan = {}", nan.is_nan());

    // Check if NaN passes the <= 0.0 check
    println!("nan <= 0.0 = {}", nan <= 0.0);
    println!("!(nan > 0.0) = {}", !(nan > 0.0));
}