pub use crate::{PHIREngine, setup_phir_engine};

// Re-export common shot result types and formatters from pecos-engines
pub use pecos_engines::{
    BitVecDisplayFormat, Shot, ShotMap, ShotMapDisplayExt, ShotMapDisplayOptions, ShotVec,
};
