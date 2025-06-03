use ::bitvec::prelude::*;
use pecos_core::{bitvec, errors::PecosError};
use pest::iterators::Pair;

use crate::ast::Expression;
use crate::parser::Rule;

/// Parse an arbitrary-length decimal integer string into a `BitVec`
/// This only handles positive integers - negative signs should be handled as unary operations
///
/// # Errors
///
/// Returns an error if the string contains invalid decimal digits
pub fn parse_integer_to_bitvec(s: &str) -> Result<BitVec<u8, Lsb0>, PecosError> {
    bitvec::parse_decimal_string(s).map_err(PecosError::ParseInvalidNumber)
}

/// Simplified binary expression parser
///
/// # Errors
///
/// Returns an error if the expression cannot be parsed
pub fn parse_binary_expr(pair: Pair<Rule>) -> Result<Expression, PecosError> {
    let rule = pair.as_rule();
    let inner_pairs: Vec<Pair<Rule>> = pair.into_inner().collect();

    // Single element - no operator
    if inner_pairs.len() == 1 {
        return parse_expr(inner_pairs[0].clone());
    }

    // Get default operator for the current rule
    let default_op = match rule {
        Rule::b_or_expr => "|",
        Rule::b_xor_expr => "^",
        Rule::b_and_expr => "&",
        Rule::equality_expr => "==",
        Rule::relational_expr => "<",
        Rule::shift_expr => "<<",
        Rule::additive_expr => "+",
        Rule::multiplicative_expr => "*",
        Rule::power_expr => "**",
        _ => {
            return Err(PecosError::ParseInvalidExpression(
                "Unknown binary rule".to_string(),
            ));
        }
    };

    // Build expression tree
    let mut result = parse_expr(inner_pairs[0].clone())?;
    let mut i = 1;

    while i < inner_pairs.len() {
        let next_pair = &inner_pairs[i];

        let (op, right_expr) = match next_pair.as_rule() {
            // Explicit operator
            Rule::equality_op
            | Rule::relational_op
            | Rule::shift_op
            | Rule::add_op
            | Rule::mul_op
            | Rule::pow_op => {
                if i + 1 < inner_pairs.len() {
                    let op_str = next_pair.as_str();
                    let right = parse_expr(inner_pairs[i + 1].clone())?;
                    i += 2;
                    (op_str, right)
                } else {
                    return Err(PecosError::ParseInvalidExpression(
                        "Missing right operand".to_string(),
                    ));
                }
            }
            // Implicit operator
            _ => {
                let right = parse_expr(next_pair.clone())?;
                i += 1;
                (default_op, right)
            }
        };

        result = Expression::BinaryOp {
            op: op.to_string(),
            left: Box::new(result),
            right: Box::new(right_expr),
        };
    }

    Ok(result)
}

/// Main expression parser
///
/// # Errors
///
/// Returns an error if the expression cannot be parsed
///
/// # Panics
///
/// Panics if the parser encounters an unexpected structure in the parse tree
pub fn parse_expr(pair: Pair<Rule>) -> Result<Expression, PecosError> {
    match pair.as_rule() {
        Rule::expr => {
            let inner = pair.into_inner().next().ok_or_else(|| {
                PecosError::ParseInvalidExpression("Empty expression".to_string())
            })?;
            parse_expr(inner)
        }

        // Binary operations - use consolidated parser
        Rule::b_or_expr
        | Rule::b_xor_expr
        | Rule::b_and_expr
        | Rule::equality_expr
        | Rule::relational_expr
        | Rule::shift_expr
        | Rule::additive_expr
        | Rule::multiplicative_expr
        | Rule::power_expr => parse_binary_expr(pair),

        // Unary operations
        Rule::unary_expr => {
            let mut pairs = pair.into_inner();
            let mut ops = Vec::new();

            // Collect operators
            while let Some(pair) = pairs.peek() {
                if pair.as_rule() == Rule::unary_op {
                    ops.push(pairs.next().unwrap().as_str().to_string());
                } else {
                    break;
                }
            }

            // Get operand
            let operand_pair = pairs.next().ok_or_else(|| {
                PecosError::ParseInvalidExpression(
                    "Missing operand for unary operation".to_string(),
                )
            })?;
            let mut expr = parse_expr(operand_pair)?;

            // Apply operators in reverse order
            for op in ops.iter().rev() {
                match (&op[..], &expr) {
                    ("-", Expression::Integer(_)) => {
                        // Don't perform two's complement here - just mark it as a unary operation
                        // The actual negation will happen during evaluation with the proper register width
                        expr = Expression::UnaryOp {
                            op: "-".to_string(),
                            expr: Box::new(expr),
                        };
                    }
                    _ => {
                        expr = Expression::UnaryOp {
                            op: op.clone(),
                            expr: Box::new(expr),
                        };
                    }
                }
            }

            Ok(expr)
        }

        // Primary expressions
        Rule::primary_expr => {
            let inner = pair.into_inner().next().unwrap();
            parse_expr(inner)
        }

        // Atomic values
        Rule::pi_constant => Ok(Expression::Pi),
        Rule::number => {
            let num_str = pair.as_str();
            if num_str.contains('.') || num_str.contains('e') || num_str.contains('E') {
                Ok(Expression::Float(num_str.parse().map_err(|_| {
                    PecosError::ParseInvalidNumber(num_str.to_string())
                })?))
            } else {
                Ok(Expression::Integer(parse_integer_to_bitvec(num_str)?))
            }
        }
        Rule::int => {
            let int_str = pair.as_str();
            Ok(Expression::Integer(parse_integer_to_bitvec(int_str)?))
        }
        Rule::bit_id => {
            let bit_id = pair.as_str();
            let parts: Vec<&str> = bit_id.split('[').collect();
            let name = parts[0].to_string();
            let idx_str = parts[1].trim_end_matches(']');
            let idx = idx_str
                .parse()
                .map_err(|_| PecosError::ParseInvalidNumber(idx_str.to_string()))?;
            Ok(Expression::BitId(name, idx))
        }
        Rule::identifier => Ok(Expression::Variable(pair.as_str().to_string())),
        Rule::function_call => {
            let mut pairs = pair.into_inner();
            let name = pairs.next().unwrap().as_str().to_string();
            let args: Result<Vec<_>, _> = pairs.map(parse_expr).collect();
            Ok(Expression::FunctionCall { name, args: args? })
        }
        _ => Err(PecosError::ParseInvalidExpression(format!(
            "Unexpected rule in expression: {:?}",
            pair.as_rule()
        ))),
    }
}
