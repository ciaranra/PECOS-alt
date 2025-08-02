//! Integration tests for the unified program API
//!
//! These tests verify that engines can accept both shared program types
//! from pecos-programs and engine-specific types.

#[cfg(test)]
mod tests {
    use pecos_programs::{QasmProgram, LlvmProgram, HugrProgram};
    use pecos_qasm::qasm_engine;
    use pecos_llvm_sim::llvm_engine;
    use pecos_selene::selene_engine;
    use pecos_engines::sim;

    #[test]
    fn test_qasm_engine_accepts_shared_program() {
        // Create a QasmProgram
        let program = QasmProgram::from_string(
            "OPENQASM 2.0; include \"qelib1.inc\"; qreg q[1]; h q[0];"
        );
        
        // Verify it compiles with qasm_engine
        let _ = qasm_engine().program(program);
    }

    #[test]
    fn test_llvm_engine_accepts_shared_programs() {
        // Test with LlvmProgram
        let llvm_program = LlvmProgram::from_string(
            "define void @main() { ret void }"
        );
        let _ = llvm_engine().program(llvm_program);
        
        // Test with HugrProgram
        let hugr_program = HugrProgram::from_bytes(vec![1, 2, 3, 4]);
        let _ = llvm_engine().program(hugr_program);
    }

    #[test]
    fn test_selene_engine_accepts_shared_programs() {
        // Test with LlvmProgram
        let llvm_program = LlvmProgram::from_string(
            "define void @main() { ret void }"
        );
        let _ = selene_engine().program(llvm_program).qubits(1);
        
        // Test with HugrProgram
        let hugr_program = HugrProgram::from_bytes(vec![1, 2, 3, 4]);
        let _ = selene_engine().program(hugr_program).qubits(1);
    }

    #[test]
    fn test_sim_function_with_program_api() {
        // Test that sim() works with engine builders using program API
        let qasm_program = QasmProgram::from_string(
            "OPENQASM 2.0; include \"qelib1.inc\"; qreg q[1]; h q[0];"
        );
        
        let _ = sim(qasm_engine().program(qasm_program))
            .seed(42);
    }

    #[test]
    fn test_from_trait_implementations() {
        // Test From<Program> implementations
        let qasm_program = QasmProgram::from_string("OPENQASM 2.0;");
        let builder: pecos_qasm::QasmEngineBuilder = qasm_program.into();
        let _ = builder;

        let llvm_program = LlvmProgram::from_string("define void @main() {}");
        let builder: pecos_llvm_sim::LlvmEngineBuilder = llvm_program.into();
        let _ = builder;

        let hugr_program = HugrProgram::from_bytes(vec![1, 2, 3]);
        let builder: pecos_llvm_sim::LlvmEngineBuilder = hugr_program.into();
        let _ = builder;
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
        assert_eq!(format!("{}", qasm), "OPENQASM 2.0;");

        let llvm = LlvmProgram::from_string("define void @main() {}");
        assert_eq!(format!("{}", llvm), "define void @main() {}");

        let hugr = HugrProgram::from_bytes(vec![1, 2, 3]);
        assert_eq!(format!("{}", hugr), "HugrProgram(3 bytes)");
    }

    #[test]
    fn test_program_enum() {
        use pecos_programs::Program;

        let qasm = QasmProgram::from_string("OPENQASM 2.0;");
        let program: Program = qasm.into();
        assert_eq!(program.program_type(), "QASM");

        let llvm = LlvmProgram::from_string("define void @main() {}");
        let program: Program = llvm.into();
        assert_eq!(program.program_type(), "LLVM");

        let hugr = HugrProgram::from_bytes(vec![1, 2, 3]);
        let program: Program = hugr.into();
        assert_eq!(program.program_type(), "HUGR");
    }
}