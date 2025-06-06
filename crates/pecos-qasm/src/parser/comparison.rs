//! Shared comparison logic for both constant folding and runtime evaluation
//!
//! This module provides unified comparison logic that can be used by both
//! compile-time constant folding and runtime expression evaluation.

use crate::ast::Expression;
use crate::bitvec_expression::BitVecExpressionContext;

/// Context for expression comparison operations
pub struct ComparisonContext<'a> {
    pub left_expr: &'a Expression,
    pub right_expr: &'a Expression,
    pub register_context: Option<&'a dyn BitVecExpressionContext>,
}

/// Result of a comparison analysis
#[derive(Debug, Clone)]
pub enum ComparisonResult {
    /// Comparison can be resolved immediately based on signs
    Immediate(bool),
    /// Operands need to be evaluated before comparison
    RequiresEvaluation,
}

/// Check if an expression represents a negative value
///
/// This uses a heuristic that considers:
/// 1. Direct negation (-x) - always negative
/// 2. Register MSB - but distinguishes between intentional negatives and large positives
#[must_use]
pub fn is_negative_expression(
    expr: &Expression,
    context: Option<&dyn BitVecExpressionContext>,
) -> bool {
    match expr {
        // Direct negation - this is the clearest signal for negative values
        Expression::UnaryOp { op, .. } if op == "-" => true,

        // For register variables, check MSB but be conservative about very large numbers
        Expression::Variable(name) => {
            if let Some(ctx) = context {
                if let Some(bitvec) = ctx.get_register(name) {
                    // Only treat as negative if MSB is set AND the number isn't suspiciously large
                    // This handles the case where MSB indicates intended negative vs very large positive
                    let msb_set = bitvec.last().as_deref().copied().unwrap_or(false);
                    if !msb_set {
                        return false;
                    }

                    // If MSB is set, check if this looks like an intended negative number
                    // vs a very large positive number (like 2^64 + something)
                    let bit_count = bitvec.count_ones();
                    let register_width = bitvec.len();

                    // Heuristic: if MSB is set but most other high bits are also set,
                    // this might be a large positive number with sign extension
                    // If only MSB + a few low bits are set, more likely intentional negative
                    if register_width > 64 && bit_count > register_width / 2 {
                        // Looks like sign extension of a large positive number
                        false
                    } else {
                        // Looks like an intended negative number
                        msb_set
                    }
                } else {
                    false
                }
            } else {
                false
            }
        }

        // Integer literals are never negative (negation is represented as UnaryOp)
        _ => false,
    }
}

/// Check if an operation is a comparison operator
#[must_use]
pub fn is_comparison_op(op: &str) -> bool {
    matches!(op, "==" | "!=" | "<" | ">" | "<=" | ">=")
}

/// Analyze a comparison operation and determine if it can be resolved immediately
/// based on the signs of the operands (cross-sign comparisons)
#[must_use]
pub fn analyze_comparison(op: &str, context: &ComparisonContext) -> ComparisonResult {
    if !is_comparison_op(op) {
        return ComparisonResult::RequiresEvaluation;
    }

    let left_is_negative = is_negative_expression(context.left_expr, context.register_context);
    let right_is_negative = is_negative_expression(context.right_expr, context.register_context);

    // Handle cross-sign comparisons directly - these can be resolved immediately
    match (left_is_negative, right_is_negative, op) {
        // negative < positive is always true, positive > negative is always true
        (true, false, "<" | "<=") | (false, true, ">" | ">=") => ComparisonResult::Immediate(true),

        // negative > positive is always false, positive < negative is always false
        (true, false, ">" | ">=") | (false, true, "<" | "<=") => ComparisonResult::Immediate(false),

        // Same sign or equality operators - need to evaluate operands
        _ => ComparisonResult::RequiresEvaluation,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::expressions::parse_integer_to_bitvec;

    #[test]
    fn test_is_comparison_op() {
        assert!(is_comparison_op("=="));
        assert!(is_comparison_op("!="));
        assert!(is_comparison_op("<"));
        assert!(is_comparison_op(">"));
        assert!(is_comparison_op("<="));
        assert!(is_comparison_op(">="));

        assert!(!is_comparison_op("+"));
        assert!(!is_comparison_op("-"));
        assert!(!is_comparison_op("*"));
        assert!(!is_comparison_op("/"));
    }

    #[test]
    fn test_is_negative_expression_literals() {
        // Integer literals are never negative
        let expr = Expression::Integer(parse_integer_to_bitvec("5").unwrap());
        assert!(!is_negative_expression(&expr, None));

        // Direct negation is always negative
        let expr = Expression::UnaryOp {
            op: "-".to_string(),
            expr: Box::new(Expression::Integer(parse_integer_to_bitvec("5").unwrap())),
        };
        assert!(is_negative_expression(&expr, None));
    }

    #[test]
    fn test_analyze_comparison_cross_sign() {
        let left = Expression::UnaryOp {
            op: "-".to_string(),
            expr: Box::new(Expression::Integer(parse_integer_to_bitvec("1").unwrap())),
        };
        let right = Expression::Integer(parse_integer_to_bitvec("8").unwrap());

        let context = ComparisonContext {
            left_expr: &left,
            right_expr: &right,
            register_context: None,
        };

        // -1 > 8 should be immediately false
        match analyze_comparison(">", &context) {
            ComparisonResult::Immediate(false) => (), // Expected
            other => panic!("Expected Immediate(false), got {other:?}"),
        }
    }

    #[test]
    fn test_analyze_comparison_same_sign() {
        let left = Expression::Integer(parse_integer_to_bitvec("5").unwrap());
        let right = Expression::Integer(parse_integer_to_bitvec("8").unwrap());

        let context = ComparisonContext {
            left_expr: &left,
            right_expr: &right,
            register_context: None,
        };

        // 5 > 8 requires evaluation (both positive)
        match analyze_comparison(">", &context) {
            ComparisonResult::RequiresEvaluation => (), // Expected
            ComparisonResult::Immediate(result) => {
                panic!("Expected RequiresEvaluation, got Immediate({result})")
            }
        }
    }
}
