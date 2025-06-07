fn main() {
    // Check what -1.0 <= 0.0 evaluates to
    let x = -1.0;
    println!("-1.0 <= 0.0 = {}", x <= 0.0);

    // Check behavior with very small negative number
    let epsilon = -0.000_000_1;
    println!("{} <= 0.0 = {}", epsilon, epsilon <= 0.0);
}
