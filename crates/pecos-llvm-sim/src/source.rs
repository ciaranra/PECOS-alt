use hugr_core::Hugr;
use pecos_core::errors::PecosError;
use std::path::PathBuf;

/// Represents different input sources for LLVM simulation.
#[derive(Debug, Clone)]
pub enum LlvmSource {
    /// LLVM IR as a string (text format)
    LlvmIr(String),
    /// LLVM bitcode as bytes (binary format)
    LlvmBitcode(Vec<u8>),
    /// Path to an LLVM file (auto-detects .ll or .bc)
    LlvmFile(PathBuf),
    /// Path to an LLVM IR text file (.ll)
    LlvmIrFile(PathBuf),
    /// Path to an LLVM bitcode file (.bc)
    LlvmBitcodeFile(PathBuf),
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

            Self::LlvmBitcode(bitcode) => {
                // Convert bitcode to LLVM IR text using llvm-dis
                use std::fs;
                use std::process::Command;
                use tempfile::TempDir;
                
                // Create temporary directory
                let temp_dir = TempDir::new()
                    .map_err(|e| PecosError::with_context(e, "Failed to create temp directory for bitcode conversion"))?;
                
                // Write bitcode to temporary file
                let bc_file = temp_dir.path().join("input.bc");
                fs::write(&bc_file, &bitcode)
                    .map_err(|e| PecosError::with_context(e, "Failed to write bitcode to temp file"))?;
                
                // Use llvm-dis to convert bitcode to IR
                let ir_file = temp_dir.path().join("output.ll");
                let output = Command::new("llvm-dis")
                    .arg("-o")
                    .arg(&ir_file)
                    .arg(&bc_file)
                    .output()
                    .map_err(|e| PecosError::with_context(e, "Failed to execute llvm-dis. Make sure LLVM tools are installed"))?;
                
                if !output.status.success() {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    return Err(PecosError::Input(
                        format!("llvm-dis failed to convert bitcode: {}", stderr)
                    ));
                }
                
                // Read the resulting IR
                fs::read_to_string(&ir_file)
                    .map_err(|e| PecosError::with_context(e, "Failed to read converted LLVM IR"))
            }
            
            Self::LlvmFile(path) => {
                // Auto-detect based on extension
                match path.extension().and_then(|s| s.to_str()) {
                    Some("ll") => Self::LlvmIrFile(path).to_llvm_ir(),
                    Some("bc") => Self::LlvmBitcodeFile(path).to_llvm_ir(),
                    _ => {
                        // Default to trying as text
                        std::fs::read_to_string(&path).map_err(|e| {
                            PecosError::with_context(
                                e,
                                format!("Failed to read LLVM file: {}. Expected .ll or .bc extension.", path.display()),
                            )
                        })
                    }
                }
            }
            
            Self::LlvmIrFile(path) => std::fs::read_to_string(&path).map_err(|e| {
                PecosError::with_context(
                    e,
                    format!("Failed to read LLVM IR file: {}", path.display()),
                )
            }),
            
            Self::LlvmBitcodeFile(path) => {
                // Read bitcode file and convert to IR
                let bitcode = std::fs::read(&path).map_err(|e| {
                    PecosError::with_context(
                        e,
                        format!("Failed to read LLVM bitcode file: {}", path.display()),
                    )
                })?;
                
                // Convert using the same logic as in-memory bitcode
                Self::LlvmBitcode(bitcode).to_llvm_ir()
            }

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
