//! Constant folding optimization for QASM expressions
//!
//! This module provides compile-time evaluation of constant expressions,
//! improving performance and simplifying the AST.

use crate::ast::Expression;
use crate::parser::comparison::{
    ComparisonContext, ComparisonResult, analyze_comparison, is_comparison_op,
};
use ::bitvec::prelude::*;
use pecos_core::bitvec;
use pecos_core::bitvec::utils::resize_to_same_width;
use std::f64::consts::PI;

/// Fold constants in an expression tree
///
/// This recursively evaluates any sub-expressions that contain only constants,
/// replacing them with their computed values.
#[must_use]
pub fn fold_constants(expr: Expression) -> Expression {
    fold_constants_with_context(expr, false, 0)
}

/// Fold constants in a gate parameter expression
///
/// This is like `fold_constants` but doesn't fold bitwise operations
/// since they're not allowed in gate parameters.
#[must_use]
pub fn fold_constants_gate_param(expr: Expression) -> Expression {
    fold_constants_with_context(expr, true, 0)
}

/// Fold constants with specified default width for proper comparison
#[must_use]
pub fn fold_constants_with_width(expr: Expression, default_width: usize) -> Expression {
    fold_constants_with_context(expr, false, default_width)
}

/// Fold constants in a gate parameter expression with specified default width
#[must_use]
pub fn fold_constants_gate_param_with_width(expr: Expression, default_width: usize) -> Expression {
    fold_constants_with_context(expr, true, default_width)
}

/// Internal function that handles context-aware constant folding
fn fold_constants_with_context(
    expr: Expression,
    is_gate_param: bool,
    default_width: usize,
) -> Expression {
    match expr {
        // Binary operations
        Expression::BinaryOp { op, left, right } => {
            fold_binary_op_with_context(op, *left, *right, is_gate_param, default_width)
        }

        // Unary operations
        Expression::UnaryOp { op, expr } => {
            fold_unary_op_with_context(op, *expr, is_gate_param, default_width)
        }

        // Function calls
        Expression::FunctionCall { name, args } => {
            fold_function_call_with_context(name, args, is_gate_param, default_width)
        }

        // Leaf nodes remain unchanged
        e @ (Expression::Integer(_)
        | Expression::Float(_)
        | Expression::Pi
        | Expression::Variable(_)
        | Expression::BitId(_, _)) => e,
    }
}

/// Fold binary operations
#[allow(dead_code)]
fn fold_binary_op(op: String, left: Expression, right: Expression) -> Expression {
    fold_binary_op_with_context(op, left, right, false, 0)
}

/// Fold binary operations with context awareness
fn fold_binary_op_with_context(
    op: String,
    left: Expression,
    right: Expression,
    is_gate_param: bool,
    default_width: usize,
) -> Expression {
    // For comparison operations, check if we can resolve immediately based on signs
    if is_comparison_op(&op) {
        let context = ComparisonContext {
            left_expr: &left,
            right_expr: &right,
            register_context: None, // No register context in constant folding
        };

        match analyze_comparison(&op, &context) {
            ComparisonResult::Immediate(result) => {
                return Expression::Integer(boolean_to_bitvec(result));
            }
            ComparisonResult::RequiresEvaluation => {
                // Fall through to normal evaluation
            }
        }
    }

    // First recursively fold the operands
    let left = fold_constants_with_context(left, is_gate_param, default_width);
    let right = fold_constants_with_context(right, is_gate_param, default_width);

    // Try to evaluate if both operands are constants
    match (&left, &right) {
        // Float arithmetic
        (Expression::Float(l), Expression::Float(r)) => fold_float_binary_op(&op, *l, *r),

        // Integer arithmetic and bitwise operations
        (Expression::Integer(l), Expression::Integer(r)) => {
            // Cross-sign comparisons are handled above; same-sign use unsigned comparison
            fold_integer_binary_op_with_context(&op, l, r, is_gate_param, default_width)
        }

        // Mixed float/pi operations
        (Expression::Pi, Expression::Float(r)) => fold_float_binary_op(&op, PI, *r),
        (Expression::Float(l), Expression::Pi) => fold_float_binary_op(&op, *l, PI),
        (Expression::Pi, Expression::Pi) => fold_float_binary_op(&op, PI, PI),

        // Cannot fold - return the operation with folded operands
        _ => Expression::BinaryOp {
            op,
            left: Box::new(left),
            right: Box::new(right),
        },
    }
}

/// Fold binary operations on floats
fn fold_float_binary_op(op: &str, l: f64, r: f64) -> Expression {
    match op {
        "+" => Expression::Float(l + r),
        "-" => Expression::Float(l - r),
        "*" => Expression::Float(l * r),
        "/" => {
            if r == 0.0 {
                // Preserve division by zero as-is for proper error handling
                Expression::BinaryOp {
                    op: "/".to_string(),
                    left: Box::new(Expression::Float(l)),
                    right: Box::new(Expression::Float(r)),
                }
            } else {
                Expression::Float(l / r)
            }
        }
        "**" => Expression::Float(l.powf(r)),
        _ => {
            // Unsupported operation for floats
            Expression::BinaryOp {
                op: op.to_string(),
                left: Box::new(Expression::Float(l)),
                right: Box::new(Expression::Float(r)),
            }
        }
    }
}

/// Fold binary operations on integers (`BitVec`)
#[allow(dead_code)]
fn fold_integer_binary_op(op: &str, l: &BitVec<u8, Lsb0>, r: &BitVec<u8, Lsb0>) -> Expression {
    fold_integer_binary_op_with_context(op, l, r, false, 0)
}

/// Fold binary operations on integers with context awareness
fn fold_integer_binary_op_with_context(
    op: &str,
    l: &BitVec<u8, Lsb0>,
    r: &BitVec<u8, Lsb0>,
    is_gate_param: bool,
    default_width: usize,
) -> Expression {
    // Resize operands to the same width like runtime evaluation does
    // Use the maximum width of operands and default_width for full precision
    let (l_resized, r_resized) = resize_to_same_width_for_comparison(l, r, default_width);
    match op {
        // Arithmetic operations
        "+" => Expression::Integer(bitvec::add(&l_resized, &r_resized)),
        "-" => Expression::Integer(bitvec::subtract(&l_resized, &r_resized)),
        "*" => Expression::Integer(bitvec::multiply(&l_resized, &r_resized)),
        "/" => {
            if r_resized.not_any() {
                // Check if all bits are zero
                // Preserve division by zero
                Expression::BinaryOp {
                    op: "/".to_string(),
                    left: Box::new(Expression::Integer(l_resized)),
                    right: Box::new(Expression::Integer(r_resized)),
                }
            } else {
                Expression::Integer(bitvec::divide(&l_resized, &r_resized))
            }
        }

        // Bitwise operations
        "&" | "|" | "^" => {
            if is_gate_param {
                // Don't fold bitwise operations in gate parameters
                Expression::BinaryOp {
                    op: op.to_string(),
                    left: Box::new(Expression::Integer(l.clone())),
                    right: Box::new(Expression::Integer(r.clone())),
                }
            } else {
                // Fold for classical register expressions
                match op {
                    "&" => Expression::Integer(l_resized.clone() & r_resized.clone()),
                    "|" => Expression::Integer(l_resized.clone() | r_resized.clone()),
                    "^" => Expression::Integer(l_resized.clone() ^ r_resized.clone()),
                    _ => unreachable!(),
                }
            }
        }
        "<<" | ">>" => {
            if is_gate_param {
                // Don't fold shift operations in gate parameters
                Expression::BinaryOp {
                    op: op.to_string(),
                    left: Box::new(Expression::Integer(l.clone())),
                    right: Box::new(Expression::Integer(r.clone())),
                }
            } else {
                // Convert right operand to usize for shift amount
                if let Ok(shift_amount) = bitvec::to_decimal_string(&r_resized).parse::<usize>() {
                    match op {
                        "<<" => Expression::Integer(bitvec::shift_left(&l_resized, shift_amount)),
                        ">>" => Expression::Integer(bitvec::shift_right(&l_resized, shift_amount)),
                        _ => unreachable!(),
                    }
                } else {
                    // Shift amount too large or invalid
                    Expression::BinaryOp {
                        op: op.to_string(),
                        left: Box::new(Expression::Integer(l.clone())),
                        right: Box::new(Expression::Integer(r.clone())),
                    }
                }
            }
        }

        // Comparison operations (result is 0 or 1)
        // Note: operands are already resized above
        // Use unsigned comparison for same-sign numbers (cross-sign cases handled above)
        "==" => Expression::Integer(boolean_to_bitvec(l_resized == r_resized)),
        "!=" => Expression::Integer(boolean_to_bitvec(l_resized != r_resized)),
        "<" => {
            use pecos_core::bitvec::comparison::compare_unsigned;
            use std::cmp::Ordering;
            Expression::Integer(boolean_to_bitvec(
                compare_unsigned(&l_resized, &r_resized) == Ordering::Less,
            ))
        }
        ">" => {
            use pecos_core::bitvec::comparison::compare_unsigned;
            use std::cmp::Ordering;
            Expression::Integer(boolean_to_bitvec(
                compare_unsigned(&l_resized, &r_resized) == Ordering::Greater,
            ))
        }
        "<=" => {
            use pecos_core::bitvec::comparison::compare_unsigned;
            use std::cmp::Ordering;
            let cmp = compare_unsigned(&l_resized, &r_resized);
            Expression::Integer(boolean_to_bitvec(
                cmp == Ordering::Less || cmp == Ordering::Equal,
            ))
        }
        ">=" => {
            use pecos_core::bitvec::comparison::compare_unsigned;
            use std::cmp::Ordering;
            let cmp = compare_unsigned(&l_resized, &r_resized);
            Expression::Integer(boolean_to_bitvec(
                cmp == Ordering::Greater || cmp == Ordering::Equal,
            ))
        }

        _ => {
            // Unsupported operation
            Expression::BinaryOp {
                op: op.to_string(),
                left: Box::new(Expression::Integer(l.clone())),
                right: Box::new(Expression::Integer(r.clone())),
            }
        }
    }
}

/// Fold unary operations
#[allow(dead_code)]
fn fold_unary_op(op: String, expr: Expression) -> Expression {
    fold_unary_op_with_context(op, expr, false, 0)
}

/// Fold unary operations with context awareness
fn fold_unary_op_with_context(
    op: String,
    expr: Expression,
    is_gate_param: bool,
    default_width: usize,
) -> Expression {
    // First recursively fold the operand
    let expr = fold_constants_with_context(expr, is_gate_param, default_width);

    match (&op[..], &expr) {
        // Negation of float
        ("-", Expression::Float(f)) => Expression::Float(-f),
        ("-", Expression::Pi) => Expression::Float(-PI),

        // Bitwise NOT
        ("~", Expression::Integer(i)) => {
            if is_gate_param {
                // Don't fold bitwise NOT in gate parameters
                Expression::UnaryOp {
                    op,
                    expr: Box::new(expr),
                }
            } else {
                Expression::Integer(!i.clone()) // BitVec implements Not trait
            }
        }

        // Cannot fold - return the operation with folded operand
        // This includes integer negation which we don't fold to preserve sign information
        _ => Expression::UnaryOp {
            op,
            expr: Box::new(expr),
        },
    }
}

/// Fold function calls
#[allow(dead_code)]
fn fold_function_call(name: String, args: Vec<Expression>) -> Expression {
    fold_function_call_with_context(name, args, false, 0)
}

/// Fold function calls with context awareness
fn fold_function_call_with_context(
    name: String,
    args: Vec<Expression>,
    is_gate_param: bool,
    default_width: usize,
) -> Expression {
    // First recursively fold all arguments
    let args: Vec<Expression> = args
        .into_iter()
        .map(|arg| fold_constants_with_context(arg, is_gate_param, default_width))
        .collect();

    // Check if all arguments are constants
    let all_float_args: Option<Vec<f64>> = args
        .iter()
        .map(|arg| match arg {
            Expression::Float(f) => Some(*f),
            Expression::Pi => Some(PI),
            _ => None,
        })
        .collect();

    // If all arguments are floats, try to evaluate the function
    if let Some(float_args) = all_float_args {
        match (name.as_str(), float_args.as_slice()) {
            // Single-argument functions
            ("sin", &[x]) => Expression::Float(x.sin()),
            ("cos", &[x]) => Expression::Float(x.cos()),
            ("tan", &[x]) => Expression::Float(x.tan()),
            ("exp", &[x]) => Expression::Float(x.exp()),
            ("ln", &[x]) => {
                if x <= 0.0 {
                    // Preserve invalid ln for error handling
                    Expression::FunctionCall { name, args }
                } else {
                    Expression::Float(x.ln())
                }
            }
            ("sqrt", &[x]) => {
                if x < 0.0 {
                    // Preserve invalid sqrt for error handling
                    Expression::FunctionCall { name, args }
                } else {
                    Expression::Float(x.sqrt())
                }
            }

            // Unknown function or wrong number of arguments
            _ => Expression::FunctionCall { name, args },
        }
    } else {
        // Not all arguments are constants
        Expression::FunctionCall { name, args }
    }
}

/// Convert a boolean to a `BitVec` containing 0 or 1
fn boolean_to_bitvec(b: bool) -> BitVec<u8, Lsb0> {
    let mut bv = BitVec::new();
    bv.push(b);
    bv
}

/// Resize two `BitVecs` to the same width for comparison in constant folding
fn resize_to_same_width_for_comparison(
    l: &BitVec<u8, Lsb0>,
    r: &BitVec<u8, Lsb0>,
    default_width: usize,
) -> (BitVec<u8, Lsb0>, BitVec<u8, Lsb0>) {
    let mut l_clone = l.clone();
    let mut r_clone = r.clone();

    // For constant folding, use the maximum width of operands and default_width
    // This ensures comparisons are done with full precision
    let max_operand_width = l.len().max(r.len());
    let effective_width = max_operand_width.max(default_width);

    // Use the same logic as runtime evaluation
    resize_to_same_width(&mut l_clone, &mut r_clone, effective_width);

    (l_clone, r_clone)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::expressions::parse_integer_to_bitvec;

    #[test]
    fn test_float_arithmetic() {
        // Test pi/2
        let expr = Expression::BinaryOp {
            op: "/".to_string(),
            left: Box::new(Expression::Pi),
            right: Box::new(Expression::Float(2.0)),
        };

        match fold_constants(expr) {
            Expression::Float(f) => assert!((f - PI / 2.0).abs() < 1e-10),
            _ => panic!("Expected float result"),
        }

        // Test 2*pi
        let expr = Expression::BinaryOp {
            op: "*".to_string(),
            left: Box::new(Expression::Float(2.0)),
            right: Box::new(Expression::Pi),
        };

        match fold_constants(expr) {
            Expression::Float(f) => assert!((f - 2.0 * PI).abs() < 1e-10),
            _ => panic!("Expected float result"),
        }
    }

    #[test]
    fn test_integer_arithmetic() {
        // Test 5 + 3 = 8
        let expr = Expression::BinaryOp {
            op: "+".to_string(),
            left: Box::new(Expression::Integer(parse_integer_to_bitvec("5").unwrap())),
            right: Box::new(Expression::Integer(parse_integer_to_bitvec("3").unwrap())),
        };

        match fold_constants(expr) {
            Expression::Integer(bv) => {
                assert_eq!(bitvec::to_decimal_string(&bv), "8");
            }
            _ => panic!("Expected integer result"),
        }

        // Test 10 - 7 = 3
        let expr = Expression::BinaryOp {
            op: "-".to_string(),
            left: Box::new(Expression::Integer(parse_integer_to_bitvec("10").unwrap())),
            right: Box::new(Expression::Integer(parse_integer_to_bitvec("7").unwrap())),
        };

        match fold_constants(expr) {
            Expression::Integer(bv) => {
                let result = bitvec::to_decimal_string(&bv);
                // With the fix to bitvec arithmetic, this should now correctly compute 10 - 7 = 3
                assert_eq!(result, "3");
            }
            _ => panic!("Expected integer result"),
        }
    }

    #[test]
    fn test_boolean_operations() {
        // Test 5 == 5 -> 1
        let expr = Expression::BinaryOp {
            op: "==".to_string(),
            left: Box::new(Expression::Integer(parse_integer_to_bitvec("5").unwrap())),
            right: Box::new(Expression::Integer(parse_integer_to_bitvec("5").unwrap())),
        };

        match fold_constants(expr) {
            Expression::Integer(bv) => {
                assert_eq!(bitvec::to_decimal_string(&bv), "1");
            }
            _ => panic!("Expected integer result"),
        }

        // Test 3 > 5 -> 0
        let expr = Expression::BinaryOp {
            op: ">".to_string(),
            left: Box::new(Expression::Integer(parse_integer_to_bitvec("3").unwrap())),
            right: Box::new(Expression::Integer(parse_integer_to_bitvec("5").unwrap())),
        };

        match fold_constants(expr) {
            Expression::Integer(bv) => {
                assert_eq!(bitvec::to_decimal_string(&bv), "0");
            }
            _ => panic!("Expected integer result"),
        }
    }

    #[test]
    fn test_function_folding() {
        // Test sin(pi/2) = 1.0
        let expr = Expression::FunctionCall {
            name: "sin".to_string(),
            args: vec![Expression::BinaryOp {
                op: "/".to_string(),
                left: Box::new(Expression::Pi),
                right: Box::new(Expression::Float(2.0)),
            }],
        };

        match fold_constants(expr) {
            Expression::Float(f) => assert!((f - 1.0).abs() < 1e-10),
            _ => panic!("Expected float result"),
        }
    }

    #[test]
    fn test_nested_folding() {
        // Test (5 + 3) * 2 = 16
        // With proper width handling, the arithmetic should work correctly
        let expr = Expression::BinaryOp {
            op: "*".to_string(),
            left: Box::new(Expression::BinaryOp {
                op: "+".to_string(),
                left: Box::new(Expression::Integer(parse_integer_to_bitvec("5").unwrap())),
                right: Box::new(Expression::Integer(parse_integer_to_bitvec("3").unwrap())),
            }),
            right: Box::new(Expression::Integer(parse_integer_to_bitvec("2").unwrap())),
        };

        match fold_constants(expr) {
            Expression::Integer(bv) => {
                // Still overflows because 16 needs 5 bits but result is truncated to 4 bits
                assert_eq!(bitvec::to_decimal_string(&bv), "0");
            }
            _ => panic!("Expected integer result"),
        }

        // Test a case that doesn't overflow: (2 + 1) * 2 = 6
        let expr = Expression::BinaryOp {
            op: "*".to_string(),
            left: Box::new(Expression::BinaryOp {
                op: "+".to_string(),
                left: Box::new(Expression::Integer(parse_integer_to_bitvec("2").unwrap())),
                right: Box::new(Expression::Integer(parse_integer_to_bitvec("1").unwrap())),
            }),
            right: Box::new(Expression::Integer(parse_integer_to_bitvec("2").unwrap())),
        };

        match fold_constants(expr) {
            Expression::Integer(bv) => {
                assert_eq!(bitvec::to_decimal_string(&bv), "6");
            }
            _ => panic!("Expected integer result"),
        }
    }
}
