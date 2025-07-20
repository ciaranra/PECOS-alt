//! Integration tests for the unified simulation API
//!
//! These tests verify that the unified API works consistently across engine types.

#[cfg(test)]
mod tests {
    #[test]
    fn test_unified_api_compiles() {
        // This test verifies that the unified API syntax compiles correctly
        // We don't run it because it would require actual quantum circuits
        
        // The fact that this compiles proves the API is consistent
        let _ = || {
            use pecos_qasm::{qasm_engine};
            use pecos_llvm_sim::{llvm_engine};
            use pecos_selene_ceng::{selene_engine};
            use pecos_engines::{ClassicalControlEngineBuilder, DepolarizingNoise, state_vector, sparse_stabilizer};
            use pecos_programs::QasmProgram;
            
            // QASM engine with unified API
            let _results = qasm_engine()
                .program(QasmProgram::from_string("OPENQASM 2.0; include \"qelib1.inc\"; qreg q[2]; h q[0];"))
                .to_sim()
                .seed(42)
                .workers(4)
                .noise(DepolarizingNoise { p: 0.01 })
                .qubits(2)
                .quantum(state_vector())
                .run(1000);
            
            // LLVM engine with unified API
            use pecos_programs::LlvmProgram;
            let _results = llvm_engine()
                .program(LlvmProgram::from_string("define void @main() { ret void }"))
                .to_sim()
                .seed(42)
                .auto_workers()
                .noise(DepolarizingNoise { p: 0.01 })
                .qubits(1)
                .quantum(sparse_stabilizer())
                .run(1000);
            
            // Selene engine with unified API
            let _results = selene_engine()
                .program(LlvmProgram::from_string("define void @main() { ret void }"))
                .qubits(2)
                .to_sim()
                .seed(42)
                .workers(8)
                .noise(DepolarizingNoise { p: 0.01 })
                .verbose(true)
                .run(1000);
        };
    }
    
    #[test]
    fn test_consistent_method_names() {
        // Verify all builders have consistent input methods
        let _ = || {
            use pecos_qasm::{qasm_engine};
            use pecos_llvm_sim::{llvm_engine};
            use pecos_selene_ceng::{selene_engine};
            use pecos_engines::ClassicalControlEngineBuilder;
            
            // QASM-specific inputs
            use pecos_programs::QasmProgram;
            let _q1 = qasm_engine().program(QasmProgram::from_string("..."));
            // Note: from_file returns Result, so in real code you'd handle the error
            // let _q2 = qasm_engine().program(QasmProgram::from_file("circuit.qasm")?);
            
            // LLVM-specific inputs
            use pecos_programs::LlvmProgram;
            let _l1 = llvm_engine().program(LlvmProgram::from_string("..."));
            let _l2 = llvm_engine().program(LlvmProgram::from_bitcode(vec![]));
            // Note: from_file returns Result, so in real code you'd handle the error
            // let _l3 = llvm_engine().program(LlvmProgram::from_file("circuit.ll")?);
            
            // Selene inputs (supports multiple formats)
            let _s1 = selene_engine().program(LlvmProgram::from_string("...")).qubits(1);
            let _s2 = selene_engine().program(LlvmProgram::from_bitcode(vec![])).qubits(1);
            // Note: from_file returns Result, so in real code you'd handle the error
            // let _s3 = selene_engine().program(LlvmProgram::from_file("circuit.ll")?).qubits(1);
            
            // Common simulation methods (via to_sim())
            use pecos_engines::{PassThroughNoise, BiasedDepolarizingNoise};
            
            let _sim1 = qasm_engine().program(QasmProgram::from_string("...")).to_sim()
                .seed(42)
                .workers(4)
                .noise(PassThroughNoise);
                
            let _sim2 = llvm_engine().program(LlvmProgram::from_string("...")).to_sim()
                .seed(123)
                .auto_workers()
                .noise(BiasedDepolarizingNoise { p: 0.02 })
                .qubits(20);
        };
    }
}