fn main() {
    let x = -1.0_f64;
    let result = x.ln();
    println!("ln(-1) = {result}");
    println!("is NaN? {}", result.is_nan());
}
