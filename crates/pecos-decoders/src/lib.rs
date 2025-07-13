//! Unified decoder library for PECOS
//!
//! This is a meta-crate that provides a unified interface to all PECOS decoders.
//! Enable the appropriate features to include specific decoder families.

// Re-export core traits
pub use pecos_decoder_core::{
    BatchDecoder, CssDecoder, Decoder, DecoderError, DecodingResultTrait, SoftDecoder,
};

// Re-export LDPC decoders when feature is enabled
#[cfg(feature = "ldpc")]
pub use pecos_ldpc_decoders::{
    BeliefFindDecoder,
    BpLsdDecoder,
    // Types
    BpMethod,
    // Decoders
    BpOsdDecoder,
    BpSchedule,
    ClusterStatistics,
    CssCode,
    DecodingResult,
    FlipDecoder,
    InputVectorType,
    // Errors
    LdpcError,
    LsdStatistics,
    MbpDecoder,
    OsdMethod,
    SoftInfoBpDecoder,
    SparseMatrix,
    UfMethod,
    UnionFindDecoder,
};
