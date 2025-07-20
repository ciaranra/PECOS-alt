"""Tests for quantum engine builders in the unified API."""

import pytest
from pecos_rslib import (
    state_vector,
    sparse_stabilizer, 
    sparse_stab,
    StateVectorEngineBuilder,
    SparseStabilizerEngineBuilder,
)
from pecos_rslib.sim import (
    qasm_engine,
    llvm_engine,
    depolarizing_noise,
)
from pecos_rslib.programs import QasmProgram, LlvmProgram


class TestQuantumEngineBuilders:
    """Test quantum engine builders and factory functions."""
    
    def test_factory_functions_exist(self):
        """Test that factory functions are available."""
        # These should all be callable
        assert callable(state_vector)
        assert callable(sparse_stabilizer)
        assert callable(sparse_stab)
        
    def test_builder_classes_exist(self):
        """Test that builder classes are available."""
        # These should be classes
        assert hasattr(StateVectorEngineBuilder, '__name__')
        assert hasattr(SparseStabilizerEngineBuilder, '__name__')
        
    def test_state_vector_builder(self):
        """Test creating state vector engine builder."""
        # Using factory function
        builder1 = state_vector()
        assert builder1 is not None
        
        # Using class directly
        builder2 = StateVectorEngineBuilder()
        assert builder2 is not None
        
        # Test with qubits
        builder3 = state_vector().qubits(10)
        assert builder3 is not None
        
    def test_sparse_stabilizer_builder(self):
        """Test creating sparse stabilizer engine builder."""
        # Using factory function
        builder1 = sparse_stabilizer()
        assert builder1 is not None
        
        # Using class directly  
        builder2 = SparseStabilizerEngineBuilder()
        assert builder2 is not None
        
        # Test with qubits
        builder3 = sparse_stabilizer().qubits(5)
        assert builder3 is not None
        
    def test_sparse_stab_alias(self):
        """Test that sparse_stab is an alias for sparse_stabilizer."""
        builder1 = sparse_stab()
        builder2 = sparse_stabilizer()
        # Both should create the same type of builder
        assert type(builder1) == type(builder2)
        
    def test_unified_api_with_quantum_engine(self):
        """Test using quantum engine builders in the unified API."""
        qasm = """
        OPENQASM 2.0;
        include "qelib1.inc";
        qreg q[2];
        creg c[2];
        h q[0];
        cx q[0], q[1];
        measure q -> c;
        """
        
        # Test with state vector engine
        sim = (
            qasm_engine()
            .program(QasmProgram.from_string(qasm))
            .to_sim()
            .quantum(state_vector())
            .seed(42)
        )
        results = sim.run(100)
        assert "c" in results
        assert len(results["c"]) == 100
        
        # Test with sparse stabilizer engine
        sim2 = (
            qasm_engine()
            .program(QasmProgram.from_string(qasm))
            .to_sim()
            .quantum(sparse_stabilizer())
            .seed(42)
        )
        results2 = sim2.run(100)
        assert "c" in results2
        assert len(results2["c"]) == 100
        
    def test_quantum_engine_with_noise(self):
        """Test using quantum engines with noise models."""
        qasm = """
        OPENQASM 2.0;
        include "qelib1.inc";
        qreg q[1];
        creg c[1];
        h q[0];
        measure q[0] -> c[0];
        """
        
        # Create noise model
        noise = depolarizing_noise().with_p1_probability(0.01)
        
        # Test with state vector engine and noise
        sim = (
            qasm_engine()
            .program(QasmProgram.from_string(qasm))
            .to_sim()
            .quantum(state_vector())
            .noise(noise)
            .seed(42)
        )
        results = sim.run(1000)
        assert "c" in results
        assert len(results["c"]) == 1000
        
    @pytest.mark.skip(reason="LLVM runtime not available in test environment")
    def test_llvm_with_quantum_engine(self):
        """Test LLVM engine with quantum engine builders."""
        llvm_ir = """
        declare void @__quantum__qis__h__body(i64)
        declare i32 @__quantum__qis__m__body(i64, i64)
        
        define void @test() #0 {
            call void @__quantum__qis__h__body(i64 0)
            %r = call i32 @__quantum__qis__m__body(i64 0, i64 0)
            ret void
        }
        
        attributes #0 = { "EntryPoint" }
        """
        
        sim = (
            llvm_engine()
            .program(LlvmProgram.from_ir(llvm_ir))
            .to_sim()
            .quantum(state_vector())
            .seed(42)
        )
        results = sim.run(100)
        assert results is not None