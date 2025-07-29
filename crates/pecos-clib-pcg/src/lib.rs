use std::cell::RefCell;

// Mirror the C struct pcg32_random_t
#[repr(C)]
struct PcgState {
    state: u64,
    inc: u64,
}

// FFI bindings to the C PCG library - now with thread-safe versions!
unsafe extern "C" {
    fn pcg32_random_r(rng: *mut PcgState) -> u32;
    fn pcg32_boundedrand_r(rng: *mut PcgState, bound: u32) -> u32;
    fn pcg32_srandom_r(rng: *mut PcgState, initstate: u64, initseq: u64);
}

// Thread-local state: each thread has its own PCG state
thread_local! {
    static THREAD_STATE: RefCell<ThreadRngState> = RefCell::new(ThreadRngState::new());
}

struct ThreadRngState {
    pcg: PcgState,
    seed: u64,
}

impl ThreadRngState {
    fn new() -> Self {
        let mut state = Self {
            pcg: PcgState { state: 0, inc: 0 },
            seed: 0,
        };
        // Initialize with default seed
        state.reseed(42);
        state
    }

    fn reseed(&mut self, seed: u64) {
        self.seed = seed;
        unsafe {
            // For PCG: initstate affects starting position, initseq selects sequence
            // Using seed for initseq (sequence selection) and a fixed initstate
            // matches the original behavior while allowing different sequences per thread
            pcg32_srandom_r(&raw mut self.pcg, 42, seed);
        }
    }
}

// Rust wrapper functions with safe interfaces
#[must_use]
pub fn random() -> u32 {
    THREAD_STATE.with(|state| {
        let mut state = state.borrow_mut();
        unsafe { pcg32_random_r(&raw mut state.pcg) }
    })
}

#[must_use]
pub fn boundedrand(bound: u32) -> u32 {
    THREAD_STATE.with(|state| {
        let mut state = state.borrow_mut();
        unsafe { pcg32_boundedrand_r(&raw mut state.pcg, bound) }
    })
}

#[must_use]
pub fn frandom() -> f64 {
    // The C code implements this as ldexp(pcg32_random(), -32)
    // which is equivalent to dividing by 2^32
    THREAD_STATE.with(|state| {
        let mut state = state.borrow_mut();
        let val = unsafe { pcg32_random_r(&raw mut state.pcg) };
        f64::from(val) * 2.0_f64.powi(-32)
    })
}

pub fn srandom(seq: u64) {
    THREAD_STATE.with(|state| state.borrow_mut().reseed(seq));
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
