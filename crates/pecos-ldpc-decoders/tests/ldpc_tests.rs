//! LDPC decoder integration tests
//!
//! This file includes all LDPC-specific tests from the ldpc/ subdirectory.

#[path = "ldpc/belief_find_test.rs"]
mod belief_find_test;

#[path = "ldpc/decoder_tests.rs"]
mod decoder_tests;

#[path = "ldpc/exhaustive_error_tests.rs"]
mod exhaustive_error_tests;

#[path = "ldpc/integration_test.rs"]
mod integration_test;

#[path = "ldpc/monte_carlo_tests.rs"]
mod monte_carlo_tests;

#[path = "ldpc/new_decoders_test.rs"]
mod new_decoders_test;

#[path = "ldpc/pcm_matrices_test.rs"]
mod pcm_matrices_test;
