use std::cmp::Ordering;

fn main() {
    let nan = (-1.0_f64).ln();
    println!("ln(-1) = {nan}");
    println!("is_nan = {}", nan.is_nan());

    // Check if NaN passes the <= 0.0 check
    println!("nan <= 0.0 = {}", nan <= 0.0);
    // Use partial_cmp for clearer comparison with NaN handling
    let cmp_result = nan.partial_cmp(&0.0);
    println!("nan.partial_cmp(&0.0) = {cmp_result:?}");
    println!(
        "nan is not greater than 0.0 = {}",
        matches!(cmp_result, None | Some(Ordering::Less | Ordering::Equal))
    );
}
