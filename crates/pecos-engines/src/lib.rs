pub mod byte_message;
pub mod classical;
pub mod engine;
pub mod engine_builder;
pub mod engine_system;
pub mod hybrid;
pub mod monte_carlo;
pub mod noise;
pub mod prelude;
pub mod quantum;
pub mod quantum_engine_builder;
pub mod quantum_system;
pub mod shot_results;
pub mod sim_builder;

#[cfg(test)]
mod tests;

pub use byte_message::{ByteMessage, ByteMessageBuilder, Gate, GateType};
pub use engine::Engine;
pub use engine_system::{
    ClassicalControlEngine, ClassicalEngine, ControlEngine, EngineStage, EngineSystem,
};
pub use hybrid::HybridEngine;
pub use monte_carlo::MonteCarloEngine;
pub use noise::{
    DepolarizingNoiseModel, NoiseModel, PassThroughNoiseModel, PassThroughNoiseModelBuilder,
    GeneralNoiseModel, GeneralNoiseModelBuilder,
};
pub use pecos_core::errors::PecosError;
pub use quantum::QuantumEngine;
pub use quantum_engine_builder::{
    QuantumEngineBuilder, IntoQuantumEngineBuilder,
    StateVectorEngineBuilder, SparseStabilizerEngineBuilder,
    state_vector, sparse_stabilizer, sparse_stab,
};
pub use quantum_system::QuantumSystem;
pub use shot_results::data_vec::DataVecType;
pub use shot_results::{
    BitVecDisplayFormat, Data, DataVec, Shot, ShotMap, ShotMapDisplay, ShotMapDisplayExt,
    ShotMapDisplayOptions, ShotVec,
};
pub use engine_builder::{ClassicalControlEngineBuilder, SimInput};
pub use sim_builder::{
    SimBuilder, SimConfig, sim_builder, sim,
    PassThroughNoise, DepolarizingNoise, BiasedDepolarizingNoise,
    shots_to_columnar,
};
