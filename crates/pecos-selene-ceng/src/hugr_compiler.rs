use anyhow::{Result, anyhow};
use hugr::llvm::CodegenExtsBuilder;
use hugr::llvm::custom::CodegenExtsMap;
use hugr::llvm::emit::{EmitHugr, Namer};
#[allow(deprecated)]
use hugr::llvm::extension::int::IntCodegenExtension;
use hugr::llvm::utils::fat::FatExt as _;
use hugr::llvm::utils::inline_constant_functions;
use inkwell::OptimizationLevel;
use inkwell::context::Context;
use inkwell::module::Module;
use inkwell::passes::PassBuilderOptions;
use inkwell::targets::{
    CodeModel, InitializationConfig, RelocMode, Target, TargetMachine,
};
use std::rc::Rc;
use tket2::extension::TKET2_EXTENSION;
use tket2::extension::rotation::ROTATION_EXTENSION;
use tket2::hugr::extension::{ExtensionRegistry, prelude};
use tket2::hugr::std_extensions::arithmetic::{
    conversions, float_ops, float_types, int_ops, int_types,
};
use tket2::hugr::std_extensions::{collections, logic, ptr};
use tket2::hugr::{self, llvm::inkwell};
use tket2::hugr::{Hugr, HugrView};
use tket2::llvm::rotation::RotationCodegenExtension;
use tket2_hseries::QSystemPass;
use tket2_hseries::extension::{futures as qsystem_futures, qsystem, result as qsystem_result};
use tket2_hseries::llvm::array_utils::{ArrayLowering, DEFAULT_STACK_ARRAY_LOWERING};
pub use tket2_hseries::llvm::futures::FuturesCodegenExtension;
use tket2_hseries::llvm::{
    debug::DebugCodegenExtension, prelude::QISPreludeCodegen, qsystem::QSystemCodegenExtension,
    random::RandomCodegenExtension, result::ResultsCodegenExtension, utils::UtilsCodegenExtension,
};

// const LLVM_MAIN: &str = "qmain";  // Reserved for future use

static REGISTRY: std::sync::LazyLock<ExtensionRegistry> = std::sync::LazyLock::new(|| {
    ExtensionRegistry::new([
        prelude::PRELUDE.to_owned(),
        int_types::EXTENSION.to_owned(),
        int_ops::EXTENSION.to_owned(),
        float_types::EXTENSION.to_owned(),
        float_ops::EXTENSION.to_owned(),
        conversions::EXTENSION.to_owned(),
        logic::EXTENSION.to_owned(),
        ptr::EXTENSION.to_owned(),
        collections::list::EXTENSION.to_owned(),
        collections::array::EXTENSION.to_owned(),
        collections::static_array::EXTENSION.to_owned(),
        collections::value_array::EXTENSION.to_owned(),
        qsystem_futures::EXTENSION.to_owned(),
        qsystem_result::EXTENSION.to_owned(),
        qsystem::EXTENSION.to_owned(),
        ROTATION_EXTENSION.to_owned(),
        TKET2_EXTENSION.to_owned(),
        tket2::extension::bool::BOOL_EXTENSION.to_owned(),
        tket2::extension::debug::DEBUG_EXTENSION.to_owned(),
    ])
});

/// Process a HUGR for quantum compilation
pub fn process_hugr(hugr: &mut Hugr) -> Result<()> {
    QSystemPass::default().run(hugr)?;
    inline_constant_functions(hugr)?;
    Ok(())
}

#[allow(deprecated)]
fn codegen_extensions() -> CodegenExtsMap<'static, Hugr> {
    let pcg = QISPreludeCodegen;
    CodegenExtsBuilder::default()
        .add_prelude_extensions(pcg.clone())
        .add_extension(IntCodegenExtension::new(pcg.clone()))
        .add_float_extensions()
        .add_conversion_extensions()
        .add_logic_extensions()
        // TODO: Replace with heap array lowering
        .add_extension(DEFAULT_STACK_ARRAY_LOWERING.codegen_extension())
        .add_default_static_array_extensions()
        .add_extension(FuturesCodegenExtension)
        .add_extension(QSystemCodegenExtension::from(pcg.clone()))
        .add_extension(RandomCodegenExtension)
        .add_extension(ResultsCodegenExtension::new(DEFAULT_STACK_ARRAY_LOWERING))
        .add_extension(RotationCodegenExtension::new(pcg))
        .add_extension(UtilsCodegenExtension)
        .add_extension(DebugCodegenExtension::new(DEFAULT_STACK_ARRAY_LOWERING))
        .finish()
}

/// Create an LLVM module from HUGR
fn get_hugr_llvm_module<'c, 'hugr, 'a: 'c>(
    context: &'c Context,
    namer: Rc<Namer>,
    hugr: &'hugr Hugr,
    module_name: impl AsRef<str>,
    exts: Rc<CodegenExtsMap<'a, Hugr>>,
) -> Result<Module<'c>> {
    let module = context.create_module(module_name.as_ref());
    let emit = EmitHugr::new(context, module, namer, exts);
    Ok(emit
        .emit_module(hugr.try_fat(hugr.module_root()).unwrap())?
        .finish())
}

/// Configuration for HUGR compilation
pub struct CompileConfig {
    /// Entry point symbol
    pub entry: Option<String>,
    /// LLVM module name
    pub name: String,
    /// Optimization level
    pub opt_level: OptimizationLevel,
}

impl Default for CompileConfig {
    fn default() -> Self {
        Self {
            entry: None,
            name: "hugr_module".to_string(),
            opt_level: OptimizationLevel::Default,
        }
    }
}

/// Compile HUGR to LLVM Module
pub fn compile_hugr_to_llvm<'c>(
    context: &'c Context,
    hugr: &mut Hugr,
    config: &CompileConfig,
    target_machine: &TargetMachine,
) -> Result<Module<'c>> {
    // Process the HUGR
    process_hugr(hugr)?;
    
    // Create namer
    let namer = Rc::new(Namer::new("__hugr__.", true));
    
    // Generate LLVM module
    let module = get_hugr_llvm_module(
        context,
        namer,
        hugr,
        &config.name,
        Rc::new(codegen_extensions()),
    )?;
    
    // Set target information
    let (data_layout, triple) = {
        (
            target_machine.get_target_data().get_data_layout(),
            target_machine.get_triple(),
        )
    };
    module.set_triple(&triple);
    module.set_data_layout(&data_layout);
    
    // Optimize
    let opt_str = match config.opt_level {
        OptimizationLevel::Aggressive => "default<O3>",
        OptimizationLevel::Less => "default<O1>",
        OptimizationLevel::None => "default<O0>",
        OptimizationLevel::Default => "default<O2>",
    };
    module
        .run_passes(opt_str, target_machine, PassBuilderOptions::create())
        .map_err(|e| anyhow!("Optimization failed: {}", e))?;
    
    // Verify
    module.verify().map_err(|e| anyhow!("Module verification failed: {}", e))?;
    
    Ok(module)
}

/// Get the Inkwell TargetMachine for the current platform
pub fn get_native_target_machine(opt_level: OptimizationLevel) -> Result<TargetMachine> {
    let reloc_mode = RelocMode::PIC;
    let code_model = CodeModel::Default;
    Target::initialize_native(&InitializationConfig::default()).unwrap();
    let triple = TargetMachine::get_default_triple();
    let target = Target::from_triple(&triple).map_err(|e| anyhow!("{e}"))?;
    target
        .create_target_machine(
            &triple,
            &TargetMachine::get_host_cpu_name().to_string_lossy(),
            &TargetMachine::get_host_cpu_features().to_string_lossy(),
            opt_level,
            reloc_mode,
            code_model,
        )
        .ok_or_else(|| anyhow!("Failed to create target machine"))
}

/// Get the extension registry for HUGR operations
pub fn get_extension_registry() -> &'static ExtensionRegistry {
    &REGISTRY
}