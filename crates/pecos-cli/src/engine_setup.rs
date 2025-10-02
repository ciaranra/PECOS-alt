use log::debug;
use pecos::DynamicEngineBuilder;
use pecos::prelude::*;
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
            use pecos::{qis_control_engine, qis_jit_interface, native_runtime, QisProgram};

            if use_jit {
                // Explicit JIT interface requested
                debug!("Using explicit JIT interface for QIR engine");
                let qis_program = QisProgram::from_file(program_path)?;
                let interface_builder = qis_jit_interface();
                let interface = interface_builder.build_from_qis_program(qis_program)?;

                let engine_builder = qis_control_engine()
                    .runtime(native_runtime())
                    .program(interface);

                Ok(Box::new(engine_builder.build()?) as Box<dyn ClassicalControlEngine>)
            } else {
                // Use Selene interface (default) - fail with helpful message if not available
                setup_qis_control_engine(program_path)
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
pub fn setup_cli_engine_builder(program_path: &Path, use_jit: bool) -> Result<DynamicEngineBuilder, PecosError> {
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
                use pecos::prelude::*;
                let qis_program = QisProgram::from_file(program_path)?;

                let engine_builder = if use_jit {
                    // Explicit JIT interface requested
                    debug!("Using explicit JIT interface for QIR engine builder");
                    let interface_builder = qis_jit_interface();
                    let interface = interface_builder.build_from_qis_program(qis_program)?;

                    qis_control_engine()
                        .runtime(native_runtime())
                        .program(interface)
                } else {
                    // Use Selene interface (default) - fail with helpful message if not available
                    qis_control_engine().try_program(qis_program)?
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
                use pecos::phir_json_engine;
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
