//! Test that the bitvec arithmetic fix works correctly

use pecos_core::bitvec;

#[test]
fn test_subtract_different_widths() {
    // Test case that was previously broken: 10 - 7 should equal 3
    let a = bitvec::parse_decimal_string("10").unwrap();
    let b = bitvec::parse_decimal_string("7").unwrap();

    println!("a (10): {} bits", a.len());
    println!("b (7): {} bits", b.len());

    let result = bitvec::subtract(&a, &b);
    let result_str = bitvec::to_decimal_string(&result);

    println!("10 - 7 = {result_str}");
    assert_eq!(result_str, "3", "10 - 7 should equal 3");
}

#[test]
fn test_add_different_widths() {
    // Test addition with different widths
    let a = bitvec::parse_decimal_string("15").unwrap();
    let b = bitvec::parse_decimal_string("1").unwrap();

    println!(
        "a (15): {} bits, binary: {}",
        a.len(),
        bitvec::to_bitstring(&a)
    );
    println!(
        "b (1): {} bits, binary: {}",
        b.len(),
        bitvec::to_bitstring(&b)
    );

    let result = bitvec::add(&a, &b);
    let result_str = bitvec::to_decimal_string(&result);

    println!("15 + 1 = {} ({} bits)", result_str, result.len());
    // 15 needs 5 bits (01111), so 15 + 1 = 16 fits in 5 bits
    assert_eq!(result_str, "16", "15 + 1 should equal 16");
}

#[test]
fn test_multiply_different_widths() {
    // Test multiplication with different widths
    let a = bitvec::parse_decimal_string("5").unwrap();
    let b = bitvec::parse_decimal_string("3").unwrap();

    println!(
        "a (5): {} bits, binary: {}",
        a.len(),
        bitvec::to_bitstring(&a)
    );
    println!(
        "b (3): {} bits, binary: {}",
        b.len(),
        bitvec::to_bitstring(&b)
    );

    let result = bitvec::multiply(&a, &b);
    let result_str = bitvec::to_decimal_string(&result);

    println!("5 * 3 = {} ({} bits)", result_str, result.len());
    // 5 needs 3 bits, but parse_decimal_string adds a leading 0 to make it positive
    // So it's actually 4 bits (0101), and 5 * 3 = 15 fits in 4 bits (1111)
    assert_eq!(result_str, "15", "5 * 3 should equal 15");
}

#[test]
fn test_divide_different_widths() {
    // Test division with different widths
    let a = bitvec::parse_decimal_string("20").unwrap();
    let b = bitvec::parse_decimal_string("3").unwrap();

    let result = bitvec::divide(&a, &b);
    let result_str = bitvec::to_decimal_string(&result);

    println!("20 / 3 = {result_str}");
    assert_eq!(result_str, "6", "20 / 3 should equal 6 (integer division)");
}

#[test]
fn test_negative_arithmetic() {
    // Test with negative numbers (using two's complement)
    // Create -3 by negating 3
    let pos_3 = bitvec::parse_decimal_string("3").unwrap();
    let neg_3 = bitvec::negate(&pos_3);
    let pos_5 = bitvec::parse_decimal_string("5").unwrap();

    println!(
        "pos_3: {} bits, binary: {}",
        pos_3.len(),
        bitvec::to_bitstring(&pos_3)
    );
    println!(
        "neg_3: {} bits, binary: {}",
        neg_3.len(),
        bitvec::to_bitstring(&neg_3)
    );
    println!(
        "pos_5: {} bits, binary: {}",
        pos_5.len(),
        bitvec::to_bitstring(&pos_5)
    );

    // -3 + 5 = 2
    let result = bitvec::add(&neg_3, &pos_5);
    let result_str = bitvec::to_decimal_string(&result);

    println!(
        "-3 + 5 = {} ({} bits, binary: {})",
        result_str,
        result.len(),
        bitvec::to_bitstring(&result)
    );
    // With proper sign extension, -3 + 5 = 2
    assert_eq!(result_str, "2");
}
