use pecos_core::errors::PecosError;
use pest::iterators::Pair;

use crate::parser::gates::{parse_gate_definition, parse_opaque_def};
use crate::parser::operations::{parse_classical_operation, parse_if_statement, parse_quantum_op};
use crate::parser::registers::parse_register;
use crate::parser::{Program, Rule};

/// Parse a statement in the QASM program
///
/// # Errors
///
/// Returns an error if the statement is invalid
pub fn parse_statement(pair: Pair<Rule>, program: &mut Program) -> Result<(), PecosError> {
    for inner_pair in pair.into_inner() {
        match inner_pair.as_rule() {
            Rule::register_decl => parse_register(inner_pair, program)?,
            Rule::quantum_op => {
                if let Some(op) = parse_quantum_op(inner_pair, program)? {
                    program.operations.push(op);
                }
            }
            Rule::classical_op => {
                if let Some(op) = parse_classical_operation(inner_pair)? {
                    program.operations.push(op);
                }
            }
            Rule::if_stmt => {
                if let Some(op) = parse_if_statement(inner_pair, program)? {
                    program.operations.push(op);
                }
            }
            Rule::gate_def => {
                parse_gate_definition(inner_pair, program)?;
            }
            Rule::include => {
                return Err(PecosError::ParseSyntax {
                    language: "QASM".to_string(),
                    message: "Include statements should be preprocessed before parsing".to_string(),
                });
            }
            Rule::opaque_def => {
                if let Some(op) = parse_opaque_def(inner_pair)? {
                    program.operations.push(op);
                }
            }
            _ => {}
        }
    }
    Ok(())
}
