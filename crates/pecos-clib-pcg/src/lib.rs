// FFI bindings to the C PCG library
unsafe extern "C" {
    fn pcg32_random() -> u32;
    fn pcg32_boundedrand(bound: u32) -> u32;
    fn pcg32_frandom() -> f64;
    fn pcg32_srandom(seq: u64);
}

// Rust wrapper functions with safe interfaces
#[must_use]
pub fn random() -> u32 {
    unsafe { pcg32_random() }
}

#[must_use]
pub fn boundedrand(bound: u32) -> u32 {
    unsafe { pcg32_boundedrand(bound) }
}

#[must_use]
pub fn frandom() -> f64 {
    unsafe { pcg32_frandom() }
}

pub fn srandom(seq: u64) {
    unsafe { pcg32_srandom(seq) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pcg_functions() {
        // Set seed
        srandom(12345);

        // Test basic random
        let r1 = random();
        assert!(r1 > 0);

        // Test bounded random
        let bound = 100;
        let r2 = boundedrand(bound);
        assert!(r2 < bound);

        // Test float random
        let r3 = frandom();
        assert!((0.0..1.0).contains(&r3));
    }
}
