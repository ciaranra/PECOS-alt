use hugr_core::Hugr;
use pecos_core::errors::PecosError;
use std::path::PathBuf;

/// Represents different input sources for LLVM simulation.
#[derive(Debug, Clone)]
pub enum LlvmSource {
    /// LLVM IR as a string
    LlvmIr(String),
    /// Path to an LLVM IR file
    LlvmFile(PathBuf),
    /// In-memory HUGR
    Hugr(Box<Hugr>),
    /// HUGR as serialized bytes
    HugrBytes(Vec<u8>),
    /// Path to a HUGR file
    HugrFile(PathBuf),
}

impl LlvmSource {
    /// Convert the source to LLVM IR string.
    ///
    /// This handles all necessary compilation steps:
    /// - Reading files if needed
    /// - Compiling HUGR to LLVM IR
    pub fn to_llvm_ir(self) -> Result<String, PecosError> {
        match self {
            Self::LlvmIr(ir) => Ok(ir),

            Self::LlvmFile(path) => std::fs::read_to_string(&path).map_err(|e| {
                PecosError::with_context(
                    e,
                    format!("Failed to read LLVM IR file: {}", path.display()),
                )
            }),

            Self::Hugr(hugr) => {
                // Create a Package from the HUGR and serialize it
                use hugr_core::envelope::{EnvelopeConfig, write_envelope};
                use hugr_core::package::Package;

                let package = Package::new(vec![*hugr]);

                // Write package to bytes using envelope format
                let mut buffer = Vec::new();
                write_envelope(&mut buffer, &package, EnvelopeConfig::default())
                    .map_err(|e| PecosError::with_context(e, "Failed to serialize HUGR package"))?;

                compile_hugr_bytes(buffer)
            }

            Self::HugrBytes(bytes) => compile_hugr_bytes(bytes),

            Self::HugrFile(path) => {
                // Use pecos-hugr-llvm to compile file directly
                use pecos_hugr_llvm::compile_hugr_to_llvm;
                let temp_output = tempfile::NamedTempFile::new()
                    .map_err(|e| PecosError::with_context(e, "Failed to create temp file"))?;

                let output_path =
                    compile_hugr_to_llvm(&path, Some(temp_output.path().to_path_buf()))?;

                std::fs::read_to_string(&output_path).map_err(|e| {
                    PecosError::with_context(
                        e,
                        format!("Failed to read compiled LLVM IR: {}", output_path.display()),
                    )
                })
            }
        }
    }
}

/// Compile HUGR bytes to LLVM IR string
fn compile_hugr_bytes(hugr_bytes: Vec<u8>) -> Result<String, PecosError> {
    use pecos_hugr_llvm::compile_hugr_bytes_to_string;
    compile_hugr_bytes_to_string(&hugr_bytes)
}
