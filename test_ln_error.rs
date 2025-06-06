use pecos_qasm::QASMParser;

fn main() {
    let qasm = r"
        OPENQASM 2.0;
        qreg q[1];
        U(ln(-1), 0, 0) q[0];
    ";

    match QASMParser::parse_str_raw(qasm) {
        Ok(program) => {
            println!("Parsing succeeded!");
            println!("Operations: {:?}", program.operations);
        }
        Err(e) => {
            println!("Parsing failed with error: {}", e);
        }
    }
}