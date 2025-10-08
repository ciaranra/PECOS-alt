use log::debug;
use pecos::DynamicEngineBuilder;
#[cfg(feature = "phir")]
use pecos::phir_json_engine;
use pecos::prelude::*;
use pecos::qis_engine;
use std::path::Path;

/// Sets up a classical engine for the CLI based on the program type
///
/// This function handles all engine types including QIR, PHIR, and QASM.
pub fn setup_cli_engine(
    program_path: &Path,
    _shots: Option<usize>,
    use_jit: bool,
) -> Result<Box<dyn ClassicalControlEngine>, PecosError> {
    debug!("Setting up engine for path: {}", program_path.display());

    // Create build directory for engine outputs
    let build_dir = program_path
        .parent()
        .ok_or_else(|| {
            PecosError::Input(format!(
                "Cannot determine parent directory for path: {}",
                program_path.display()
            ))
        })?
        .join("build");
    debug!("Build directory: {}", build_dir.display());
    std::fs::create_dir_all(&build_dir).map_err(PecosError::IO)?;

    // The detect_program_type function now includes proper context in errors
    let program_type = detect_program_type(program_path)?;

    match program_type {
        ProgramType::QIR => {
            debug!("Setting up QIR engine");

            if use_jit {
                // Explicit JIT interface requested
                debug!("Using explicit JIT interface for QIR engine");
                #[cfg(feature = "jit")]
                {
                    let qis_program = QisProgram::from_file(program_path)?;

                    let engine = qis_engine()
                        .runtime(native_runtime())
                        .interface(jit_interface_builder())
                        .try_program(qis_program)?
                        .build()?;

                    Ok(Box::new(engine) as Box<dyn ClassicalControlEngine>)
                }
                #[cfg(not(feature = "jit"))]
                {
                    Err(PecosError::Processing(
                        "JIT interface not available. Please enable the 'jit' feature.".to_string(),
                    ))
                }
            } else {
                // Use default Selene interface and runtime
                // This will use the Helios interface with Selene simple runtime
                // Users can override by using --jit flag
                Err(PecosError::Processing(
                    "Default QIS execution requires Selene runtime.\n\
                    \n\
                    To use JIT interface instead, run with --jit flag:\n\
                    pecos run --jit program.ll"
                        .to_string(),
                ))
            }
        }
        ProgramType::PHIR => {
            debug!("Setting up PHIR-JSON engine");
            setup_phir_json_engine(program_path)
        }
        ProgramType::QASM => {
            debug!("Setting up QASM engine");
            setup_qasm_engine(program_path, None)
        }
    }
}

/// Sets up a classical engine builder for the CLI based on the program type
///
/// This function returns a `DynamicEngineBuilder` that can be used with `sim_builder`
pub fn setup_cli_engine_builder(
    program_path: &Path,
    use_jit: bool,
) -> Result<DynamicEngineBuilder, PecosError> {
    debug!(
        "Setting up engine builder for path: {}",
        program_path.display()
    );

    let program_type = detect_program_type(program_path)?;

    match program_type {
        ProgramType::QIR => {
            debug!("Setting up QIR engine builder");
            #[cfg(feature = "llvm")]
            {
                let qis_program = QisProgram::from_file(program_path)?;

                let engine_builder = if use_jit {
                    // Explicit JIT interface requested
                    debug!("Using explicit JIT interface for QIR engine builder");
                    #[cfg(feature = "jit")]
                    {
                        qis_engine()
                            .runtime(native_runtime())
                            .interface(jit_interface_builder())
                            .try_program(qis_program.clone())?
                    }
                    #[cfg(not(feature = "jit"))]
                    {
                        return Err(PecosError::Processing(
                            "JIT interface not available. Please enable the 'jit' feature."
                                .to_string(),
                        ));
                    }
                } else {
                    // Use Selene interface (default) - fail with helpful message if not available
                    qis_engine().try_program(qis_program)?
                };

                Ok(DynamicEngineBuilder::new(engine_builder))
            }
            #[cfg(not(feature = "llvm"))]
            {
                Err(PecosError::Input(
                    "LLVM support not compiled in".to_string(),
                ))
            }
        }
        ProgramType::PHIR => {
            debug!("Setting up PHIR-JSON engine builder");
            #[cfg(feature = "phir")]
            {
                Ok(DynamicEngineBuilder::new(
                    phir_json_engine().file(program_path)?,
                ))
            }
            #[cfg(not(feature = "phir"))]
            {
                Err(PecosError::Input(
                    "PHIR support not compiled in".to_string(),
                ))
            }
        }
        ProgramType::QASM => {
            debug!("Setting up QASM engine builder");
            #[cfg(feature = "qasm")]
            {
                use pecos::qasm_engine;
                let qasm_content = std::fs::read_to_string(program_path)
                    .map_err(|e| PecosError::Input(format!("Failed to read QASM file: {e}")))?;
                Ok(DynamicEngineBuilder::new(qasm_engine().qasm(qasm_content)))
            }
            #[cfg(not(feature = "qasm"))]
            {
                Err(PecosError::Input(
                    "QASM support not compiled in".to_string(),
                ))
            }
        }
    }
}
