
use pecos_qasm::parser::QASMParser;
use pecos_qasm::ast::Expression;

fn main() {
    // First test: Just parse the expression
    println!("=== Testing expression parsing and evaluation ===");
    let expr = Expression::FunctionCall {
        name: "ln".to_string(),
        args: vec![Expression::Float(-1.0)],
    };

    match expr.evaluate(None) {
        Ok(v) => println!("Direct evaluation succeeded: {}", v),
        Err(e) => println!("Direct evaluation failed: {}", e),
    }

    // Second test: Parse QASM
    println!("\n=== Testing QASM parsing ===");
    let qasm = r#"
        OPENQASM 2.0;
        qreg q[1];
        U(ln(-1), 0, 0) q[0];
    "#;

    match QASMParser::parse_str_raw(qasm) {
        Ok(program) => {
            println!("QASM parsing succeeded!");
            for (i, op) in program.operations.iter().enumerate() {
                println!("  Operation {}: {:?}", i, op);
            }
        }
        Err(e) => {
            println!("QASM parsing failed: {}", e);
            println!("Error type: {:?}", e);
        }
    }
}
