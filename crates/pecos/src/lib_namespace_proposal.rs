// Proposal for namespace organization in the pecos crate

// Current flat exports (harder to discover):
// use pecos::{qasm_engine, qis_engine, sparse_stabilizer, depolarizing_noise};

// Proposed namespace organization:
pub mod engines {
    //! Classical control engines

    // Re-export engine builders
    pub use pecos_qasm::{qasm_engine, QasmEngine, QasmEngineBuilder};
    pub use pecos_qis_sim::{qis_engine, QisEngine, QisEngineBuilder};
    pub use pecos_selene_engine::{selene_engine, SeleneEngine, SeleneEngineBuilder};

    // Re-export the trait
    pub use pecos_engines::ClassicalControlEngine;
}

pub mod quantum {
    //! Quantum simulation backends

    pub use pecos_engines::quantum_engine_builder::{
        state_vector,
        sparse_stabilizer,
        sparse_stab, // alias
        StateVectorEngineBuilder,
        SparseStabilizerEngineBuilder,
        QuantumEngineBuilder,
        IntoQuantumEngine,
    };

    pub use pecos_engines::quantum::{QuantumEngine, StateVecEngine, SparseStabEngine};
}

pub mod noise {
    //! Noise models and builders

    pub use pecos_engines::noise::{
        // Free functions (when implemented)
        general_noise,
        depolarizing_noise,
        biased_depolarizing_noise,

        // Builder types
        GeneralNoiseModelBuilder,
        DepolarizingNoiseModelBuilder,
        BiasedDepolarizingNoiseModelBuilder,

        // Model types
        NoiseModel,
        PassThroughNoiseModel,
        DepolarizingNoiseModel,
        BiasedDepolarizingNoiseModel,

        // Traits
        IntoNoiseModel,
    };
}

pub mod programs {
    //! Program types for different engines

    pub use pecos_programs::{
        QasmProgram,
        LlvmProgram,
        HugrProgram,
        Program, // trait
    };
}

pub mod sim {
    //! Simulation builders and runners

    pub use pecos_engines::{
        sim,
        SimBuilder,
        Simulation,
        SimConfig,

        // Re-export engine builders for convenience
        sim_builder::QuantumEngineType,
    };
}

pub mod results {
    //! Simulation results and data types

    pub use pecos_core::shot_results::{
        ShotVec,
        ShotMap,
        Shot,
        Data,
    };
}

// Keep flat exports for backward compatibility
pub use engines::*;
pub use quantum::*;
pub use noise::*;
pub use programs::*;
pub use sim::*;
pub use results::*;

// Usage examples:
/*
use pecos::engines;
use pecos::quantum;
use pecos::noise;

// Clear and organized
let results = engines::qasm()
    .program(program)
    .to_sim()
    .quantum_engine(quantum::sparse_stabilizer())
    .noise(noise::depolarizing()
        .with_p1_probability(0.01))
    .run(1000)?;

// Or with specific imports
use pecos::{engines::qasm_engine, quantum::sparse_stab, noise::DepolarizingNoiseModelBuilder};
*/