use pecos_clib_pcg::{boundedrand, frandom, random, srandom};

#[test]
fn test_pcg_random_generates_values() {
    // Test that random() generates different values
    let val1 = random();
    let val2 = random();

    // It's extremely unlikely that two consecutive calls return the same value
    // (probability is 1 in 2^32)
    assert_ne!(
        val1, val2,
        "Two consecutive random() calls should generate different values"
    );
}

#[test]
fn test_pcg_bounded_random() {
    // Test bounded random with various bounds
    let bounds = [1, 2, 10, 100, 1000];

    for bound in bounds {
        for _ in 0..10 {
            let val = boundedrand(bound);
            assert!(
                val < bound,
                "boundedrand({bound}) returned {val}, which is >= {bound}"
            );
        }
    }
}

#[test]
fn test_pcg_frandom_range() {
    // Test that frandom returns values in [0.0, 1.0)
    for _ in 0..100 {
        let val = frandom();
        assert!(val >= 0.0, "frandom() returned {val}, which is < 0.0");
        assert!(val < 1.0, "frandom() returned {val}, which is >= 1.0");
    }
}

#[test]
fn test_pcg_seeding() {
    // Test that seeding produces deterministic sequences
    // Note: PCG uses global state, so we test determinism directly

    // Test that the same seed produces the same first few values
    srandom(12345);
    let first_val_1 = random();
    let second_val_1 = random();

    srandom(12345);
    let first_val_2 = random();
    let second_val_2 = random();

    assert_eq!(
        first_val_1, first_val_2,
        "First value after seeding should be deterministic"
    );
    assert_eq!(
        second_val_1, second_val_2,
        "Second value after seeding should be deterministic"
    );

    // Test that different seeds produce different values
    srandom(54321);
    let different_first = random();

    assert_ne!(
        first_val_1, different_first,
        "Different seeds should produce different values"
    );
}

#[test]
fn test_pcg_deterministic_behavior() {
    // Test that the RNG is deterministic after seeding
    srandom(999);
    let first_value = random();

    srandom(999);
    let second_value = random();

    assert_eq!(
        first_value, second_value,
        "First value after seeding should be deterministic"
    );
}
