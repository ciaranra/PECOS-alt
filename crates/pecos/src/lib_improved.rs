//! PECOS - Performance Engineering for Quantum Computing
//! 
//! This is the main entry point for the PECOS framework. It re-exports
//! functionality from various sub-crates in an organized, discoverable way.
//! 
//! # Architecture
//! 
//! PECOS is organized as a workspace with specialized crates:
//! - `pecos-core`: Core types and traits
//! - `pecos-engines`: Engine traits and quantum backends
//! - `pecos-qasm`: QASM parser and engine
//! - `pecos-llvm-sim`: LLVM-based simulation engine
//! - `pecos-selene`: Selene integration
//! - `pecos-programs`: Program types
//! - `pecos`: This meta-crate providing unified API
//! 
//! # Examples
//! 
//! ```rust
//! use pecos::prelude::*;
//! use pecos::{engines, quantum, noise};
//! 
//! // Create and run a quantum simulation
//! let results = engines::qasm()
//!     .program(QasmProgram::from_string(qasm_code))
//!     .to_sim()
//!     .quantum_engine(quantum::sparse_stabilizer())
//!     .noise(noise::depolarizing().with_p1_probability(0.01))
//!     .run(1000)?;
//! ```

#![doc(html_root_url = "https://docs.rs/pecos")]

// ============================================================================
// Namespace modules
// ============================================================================

/// Classical control engines for quantum circuit execution
pub mod engines {
    //! Classical control engines that parse and execute quantum programs.
    //! 
    //! # Available Engines
    //! 
    //! - **QASM**: OpenQASM 2.0 support via [`qasm_engine()`]
    //! - **LLVM**: LLVM IR quantum programs via [`llvm_engine()`]  
    //! - **Selene**: High-performance engine via [`selene_engine()`]
    
    pub use pecos_qasm::{qasm_engine, QasmEngine, QasmEngineBuilder};
    pub use pecos_llvm_sim::{llvm_engine, LlvmEngine, LlvmEngineBuilder};
    pub use pecos_selene::{selene_engine, SeleneEngine, SeleneEngineBuilder};
    
    // Export the main trait
    pub use pecos_engines::ClassicalControlEngine;
}

/// Quantum simulation backends
pub mod quantum {
    //! Quantum state simulation backends.
    //! 
    //! # Available Backends
    //! 
    //! - **State Vector**: Full quantum state simulation via [`state_vector()`]
    //! - **Sparse Stabilizer**: Efficient Clifford simulation via [`sparse_stabilizer()`]
    
    // Builders and factory functions
    pub use pecos_engines::quantum_engine_builder::{
        state_vector,
        sparse_stabilizer,
        sparse_stab, // convenient alias
        StateVectorEngineBuilder,
        SparseStabilizerEngineBuilder,
        QuantumEngineBuilder,
        IntoQuantumEngine,
    };
    
    // Engine implementations
    pub use pecos_engines::quantum::{
        QuantumEngine,
        StateVecEngine,
        SparseStabEngine,
    };
}

/// Noise models for quantum simulations
pub mod noise {
    //! Noise models for realistic quantum simulations.
    //! 
    //! # Available Models
    //! 
    //! - **General**: Flexible noise model via [`general()`]
    //! - **Depolarizing**: Symmetric depolarizing noise via [`depolarizing()`]
    //! - **Biased Depolarizing**: Asymmetric noise via [`biased_depolarizing()`]
    
    // Builder types
    pub use pecos_engines::noise::{
        GeneralNoiseModelBuilder,
        DepolarizingNoiseModelBuilder,
        BiasedDepolarizingNoiseModelBuilder,
        
        // Traits
        IntoNoiseModel,
        NoiseModel,
    };
    
    // Model implementations
    pub use pecos_engines::noise::{
        PassThroughNoiseModel,
        DepolarizingNoiseModel,
        BiasedDepolarizingNoiseModel,
    };
    
    // Convenience functions (once implemented in pecos-engines)
    // pub use pecos_engines::noise::{general, depolarizing, biased_depolarizing};
    
    // For now, provide convenience functions here
    /// Create a general noise model builder
    pub fn general() -> GeneralNoiseModelBuilder {
        GeneralNoiseModelBuilder::new()
    }
    
    /// Create a depolarizing noise model builder
    pub fn depolarizing() -> DepolarizingNoiseModelBuilder {
        DepolarizingNoiseModelBuilder::new()
    }
    
    /// Create a biased depolarizing noise model builder
    pub fn biased_depolarizing() -> BiasedDepolarizingNoiseModelBuilder {
        BiasedDepolarizingNoiseModelBuilder::new()
    }
}

/// Program types for quantum circuits
pub mod programs {
    //! Program representations for different quantum computing frameworks.
    
    pub use pecos_programs::{
        QasmProgram,
        LlvmProgram,
        HugrProgram,
        Program, // trait
    };
}

/// Simulation results and data types
pub mod results {
    //! Types for representing simulation results.
    
    pub use pecos_core::shot_results::{
        ShotVec,
        ShotMap,
        Shot,
        Data,
    };
}

/// Error types
pub mod errors {
    //! Error types used throughout PECOS.
    
    pub use pecos_core::errors::{PecosError, PecosResult};
}

// ============================================================================
// Prelude module for common imports
// ============================================================================

/// Common imports for PECOS users
pub mod prelude {
    //! The PECOS prelude - common types for most use cases.
    //! 
    //! # Example
    //! ```rust
    //! use pecos::prelude::*;
    //! ```
    
    // Core traits
    pub use pecos_engines::ClassicalControlEngine;
    pub use pecos_engines::quantum::QuantumEngine;
    pub use pecos_engines::noise::NoiseModel;
    
    // Program types
    pub use pecos_programs::{QasmProgram, LlvmProgram, HugrProgram};
    
    // Result types
    pub use pecos_core::shot_results::{ShotVec, ShotMap};
    pub use pecos_core::errors::{PecosError, PecosResult};
    
    // Builder functions
    pub use pecos_engines::sim;
}

// ============================================================================
// Top-level re-exports for backward compatibility
// ============================================================================

// Re-export everything from modules for backward compatibility
// This allows both:
//   use pecos::engines::qasm_engine;  // New namespace style
//   use pecos::qasm_engine;           // Old flat style

pub use engines::*;
pub use quantum::*;
pub use noise::*;
pub use programs::*;
pub use results::*;
pub use errors::*;

// Also export the sim builder at top level
pub use pecos_engines::{sim, Simulation};