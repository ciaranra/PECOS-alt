//! Integration tests for the unified program API
//!
//! These tests verify that engines can accept both shared program types
//! from pecos-programs and engine-specific types.

#[cfg(test)]
mod tests {
    use pecos_engines::sim;
    use pecos_programs::{HugrProgram, QasmProgram, QisProgram};
    use pecos_qasm::qasm_engine;
    use pecos::qis_control_engine;

    #[test]
    fn test_qasm_engine_accepts_shared_program() {
        // Create a QasmProgram
        let program =
            QasmProgram::from_string("OPENQASM 2.0; include \"qelib1.inc\"; qreg q[1]; h q[0];");

        // Verify it compiles with qasm_engine
        let _ = qasm_engine().program(program);
    }

    #[test]
    fn test_qis_engine_accepts_shared_programs() {
        use pecos_qis_ccengine::qis_jit_interface;

        // Test with QisProgram - use JIT interface for tests (no external dependencies)
        let llvm_program = QisProgram::from_string("define void @main() {\nentry:\n  ret void\n}");
        let _ = qis_control_engine().interface(qis_jit_interface()).program(llvm_program);

        // Test with HugrProgram - use try_program to handle invalid test data gracefully
        let hugr_program = HugrProgram::from_bytes(vec![1, 2, 3, 4]);
        let result = qis_control_engine().interface(qis_jit_interface()).try_program(hugr_program);
        // We expect this to fail since the data is invalid
        assert!(result.is_err());
    }


    #[test]
    fn test_sim_function_with_program_api() {
        // Test that sim() works with engine builders using program API
        let qasm_program =
            QasmProgram::from_string("OPENQASM 2.0; include \"qelib1.inc\"; qreg q[1]; h q[0];");

        let _ = sim(qasm_engine().program(qasm_program)).seed(42);
    }

    #[test]
    fn test_from_trait_implementations() {
        // Test From<Program> implementations
        let qasm_program = QasmProgram::from_string("OPENQASM 2.0;");
        let builder: pecos_qasm::QasmEngineBuilder = qasm_program.into();
        let _ = builder;

        // Create a simpler LLVM program to debug the issue
        let llvm_program = QisProgram::from_string("define void @main() {\nentry:\n  ret void\n}");

        // Debug: Print what gets stored in the QisProgram
        match &llvm_program.content {
            pecos_programs::QisContent::Ir(ir) => {
                println!("Stored IR in QisProgram:");
                for (i, line) in ir.lines().enumerate() {
                    println!("{}: {}", i + 1, line);
                }
            }
            _ => println!("Not IR content"),
        }


        // Use JIT interface for tests (no external dependencies)
        use pecos_qis_ccengine::qis_jit_interface;
        let result = qis_control_engine().interface(qis_jit_interface()).try_program(llvm_program);
        match result {
            Ok(builder) => {
                let _ = builder;
            }
            Err(e) => {
                panic!("Failed to create QIS builder: {}", e);
            }
        }

        // Note: We don't test HugrProgram::into() because it would require valid HUGR data
        // The From trait is implemented generically for all IntoQisInterface types
        // and is tested in the pecos-qis-ccengine crate with proper error handling
    }

    #[test]
    fn test_file_loading() -> Result<(), std::io::Error> {
        use std::io::Write;
        use tempfile::NamedTempFile;

        // Create temporary QASM file
        let mut temp_file = NamedTempFile::new()?;
        writeln!(temp_file, "OPENQASM 2.0;")?;
        writeln!(temp_file, "include \"qelib1.inc\";")?;
        writeln!(temp_file, "qreg q[2];")?;
        writeln!(temp_file, "h q[0];")?;
        temp_file.flush()?;

        // Load and use the program
        let program = QasmProgram::from_file(temp_file.path())?;
        let _ = qasm_engine().program(program);

        Ok(())
    }

    #[test]
    fn test_program_display() {
        let qasm = QasmProgram::from_string("OPENQASM 2.0;");
        assert_eq!(format!("{qasm}"), "OPENQASM 2.0;");

        let llvm = QisProgram::from_string("define void @main() {\nentry:\n  ret void\n}");
        assert_eq!(format!("{llvm}"), "define void @main() {\nentry:\n  ret void\n}");

        let hugr = HugrProgram::from_bytes(vec![1, 2, 3]);
        assert_eq!(format!("{hugr}"), "HugrProgram(3 bytes)");
    }

    #[test]
    fn test_program_enum() {
        use pecos_programs::Program;

        let qasm = QasmProgram::from_string("OPENQASM 2.0;");
        let program: Program = qasm.into();
        assert_eq!(program.program_type(), "QASM");

        let qis = QisProgram::from_string("define void @main() {\nentry:\n  ret void\n}");
        let program: Program = qis.into();
        assert_eq!(program.program_type(), "QIS");

        let hugr = HugrProgram::from_bytes(vec![1, 2, 3]);
        let program: Program = hugr.into();
        assert_eq!(program.program_type(), "HUGR");
    }
}
