//! HUGR to QIS LLVM IR compiler
//!
//! This module provides HUGR to LLVM IR compilation that generates
//! Selene QIS-compatible LLVM IR. It follows the same approach as
//! Selene's hugr-qis compiler.

// array module is declared in lib.rs

use anyhow::{Result, anyhow};
use pecos_core::errors::PecosError;
use std::rc::Rc;

use itertools::Itertools;
use tket::extension::rotation::ROTATION_EXTENSION;
use tket::extension::{TKET_EXTENSION, TKET1_EXTENSION};
use tket::hugr::extension::{ExtensionRegistry, prelude};
#[allow(deprecated)]
use tket::hugr::llvm::extension::int::IntCodegenExtension;
use tket::hugr::llvm::inkwell::OptimizationLevel;
use tket::hugr::llvm::inkwell::context::Context;
use tket::hugr::llvm::inkwell::module::Module;
use tket::hugr::llvm::inkwell::passes::PassBuilderOptions;
use tket::hugr::llvm::inkwell::targets::{
    CodeModel, InitializationConfig, RelocMode, Target, TargetMachine,
};
use tket::hugr::llvm::utils::fat::FatExt as _;
use tket::hugr::llvm::utils::inline_constant_functions;
use tket::hugr::llvm::{
    CodegenExtsBuilder,
    custom::CodegenExtsMap,
    emit::{EmitHugr, Namer},
};
use tket::hugr::ops::DataflowParent;
use tket::hugr::package::Package;
use tket::hugr::std_extensions::arithmetic::{
    conversions, float_ops, float_types, int_ops, int_types,
};
use tket::hugr::std_extensions::{collections, logic, ptr};
use tket::hugr::{Hugr, HugrView, Node};
use tket::llvm::rotation::RotationCodegenExtension;
use tket_qsystem::QSystemPass;
use tket_qsystem::extension::{futures as qsystem_futures, qsystem, result as qsystem_result};
use tket_qsystem::llvm::array_utils::ArrayLowering;
use tket_qsystem::llvm::futures::FuturesCodegenExtension;
use tket_qsystem::llvm::{
    debug::DebugCodegenExtension, prelude::QISPreludeCodegen, qsystem::QSystemCodegenExtension,
    random::RandomCodegenExtension, result::ResultsCodegenExtension, utils::UtilsCodegenExtension,
};

const LLVM_MAIN: &str = "qmain";
const METADATA: &[(&str, &[&str])] = &[("name", &["mainlib"])];

/// Extension registry with all required extensions for HUGR compilation
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
        TKET_EXTENSION.to_owned(),
        TKET1_EXTENSION.to_owned(),
        tket::extension::bool::BOOL_EXTENSION.to_owned(),
        tket::extension::debug::DEBUG_EXTENSION.to_owned(),
        tket_qsystem::extension::gpu::EXTENSION.to_owned(),
        tket_qsystem::extension::wasm::EXTENSION.to_owned(),
    ])
});

/// Read HUGR from bytes (handles both JSON and binary envelope formats)
fn read_hugr_envelope(bytes: &[u8]) -> Result<Hugr> {
    // Check if input is JSON format (starts with '{') vs binary envelope format
    if bytes.is_empty() {
        return Err(anyhow!("Empty HUGR input"));
    }

    // Check magic number for format detection
    if bytes[0] == b'{' {
        // JSON format - wrap it in a binary envelope so HUGR can load it
        // This allows us to store human-readable JSON in git but still load it
        let json_str =
            std::str::from_utf8(bytes).map_err(|e| anyhow!("Invalid UTF-8 in JSON HUGR: {e}"))?;

        // Create a binary envelope with JSON content
        // The envelope format is: MAGIC_HEADER + JSON_CONTENT
        // HUGR expects: "HUGRiHJv" (8 bytes) + format byte + compression byte + JSON
        let mut envelope = Vec::new();

        // Magic header for HUGR envelope
        envelope.extend_from_slice(b"HUGRiHJv");

        // Format byte: 0x3F (63) for JSON format (EnvelopeFormat::JSON)
        envelope.push(0x3F);

        // Compression byte: 0x40 (64) - this is what HUGR expects
        envelope.push(0x40);

        // Append the JSON content
        envelope.extend_from_slice(json_str.as_bytes());

        // Now load using the envelope
        let mut cursor = std::io::Cursor::new(&envelope);
        if let Ok(hugr) = Hugr::load(&mut cursor, Some(&REGISTRY)) {
            Ok(hugr)
        } else {
            // If direct HUGR loading fails, try Package loading
            let mut cursor = std::io::Cursor::new(&envelope);
            match Package::load(&mut cursor, Some(&REGISTRY)) {
                Ok(package) => {
                    // Extract the main HUGR from the package
                    if let Some(hugr) = package.modules.first() {
                        Ok(hugr.clone())
                    } else {
                        Err(anyhow!("Package contains no HUGR modules"))
                    }
                }
                Err(e) => Err(anyhow!("Failed to load JSON HUGR as envelope: {e}")),
            }
        }
    } else {
        // Binary envelope format - use TKET's loading mechanism directly
        let mut cursor = std::io::Cursor::new(bytes);
        let hugr = Hugr::load(&mut cursor, Some(&REGISTRY))
            .map_err(|e| anyhow!("Failed to load HUGR envelope: {e}"))?;

        Ok(hugr)
    }
}

/// Process HUGR by applying required passes
fn process_hugr(hugr: &mut Hugr) -> Result<()> {
    QSystemPass::default().run(hugr)?;
    inline_constant_functions(hugr)?;
    Ok(())
}

/// Build codegen extensions for LLVM generation
#[allow(deprecated)]
fn codegen_extensions() -> CodegenExtsMap<'static, Hugr> {
    use crate::array::SeleneHeapArrayCodegen;
    let pcg = QISPreludeCodegen;

    CodegenExtsBuilder::default()
        .add_prelude_extensions(pcg.clone())
        .add_extension(IntCodegenExtension::new(pcg.clone()))
        .add_float_extensions()
        .add_conversion_extensions()
        .add_logic_extensions()
        .add_extension(SeleneHeapArrayCodegen::LOWERING.codegen_extension())
        .add_default_static_array_extensions()
        .add_extension(FuturesCodegenExtension)
        .add_extension(QSystemCodegenExtension::from(pcg.clone()))
        .add_extension(RandomCodegenExtension)
        .add_extension(ResultsCodegenExtension::new(
            SeleneHeapArrayCodegen::LOWERING,
        ))
        .add_extension(RotationCodegenExtension::new(pcg))
        .add_extension(UtilsCodegenExtension)
        .add_extension(DebugCodegenExtension::new(SeleneHeapArrayCodegen::LOWERING))
        .finish()
}

/// Get the entry point name from the HUGR
fn get_entry_point_name(namer: &Namer, hugr: &impl HugrView<Node = Node>) -> Result<String> {
    const HUGR_MAIN: &str = "main";

    let (name, entry_point_node) = if hugr.entrypoint_optype().is_module() {
        // For backwards compatibility: assume entrypoint is "main" function in module
        let node = hugr
            .children(hugr.module_root())
            .filter(|&n| {
                hugr.get_optype(n)
                    .as_func_defn()
                    .is_some_and(|f| f.func_name() == HUGR_MAIN)
            })
            .exactly_one()
            .map_err(|_| {
                anyhow!("Module entrypoint must have a single function named {HUGR_MAIN} as child")
            })?;
        (HUGR_MAIN, node)
    } else {
        let func_defn = hugr
            .entrypoint_optype()
            .as_func_defn()
            .ok_or_else(|| anyhow!("Entry point node is not a function definition"))?;

        if func_defn.inner_signature().input_count() != 0 {
            return Err(anyhow!(
                "Entry point function must have no input parameters (found {})",
                func_defn.inner_signature().input_count()
            ));
        }
        (func_defn.func_name().as_ref(), hugr.entrypoint())
    };

    Ok(namer.name_func(name, entry_point_node))
}

/// Wrap the HUGR entry point with setup/teardown calls
fn wrap_main<'c>(
    ctx: &'c Context,
    module: &Module<'c>,
    hugr_entry: &str,
    module_entry: &str,
) -> Result<()> {
    let entry_ty = ctx.i64_type().fn_type(&[ctx.i64_type().into()], false);
    let entry_fun = module.add_function(module_entry, entry_ty, None);

    // Add EntryPoint attribute to the function
    entry_fun.add_attribute(
        tket::hugr::llvm::inkwell::attributes::AttributeLoc::Function,
        ctx.create_string_attribute("EntryPoint", ""),
    );

    let setup_type = ctx.void_type().fn_type(&[ctx.i64_type().into()], false);
    let setup = module.add_function("setup", setup_type, None);

    let teardown_type = ctx.i64_type().fn_type(&[], false);
    let teardown = module.add_function("teardown", teardown_type, None);

    let block = ctx.append_basic_block(entry_fun, "entry");
    let builder = ctx.create_builder();
    builder.position_at_end(block);

    let initial_tc = entry_fun.get_nth_param(0).unwrap().into_int_value();
    let hugr_main = module
        .get_function(hugr_entry)
        .ok_or_else(|| anyhow!("Entrypoint function '{hugr_entry}' not found in Module"))?;

    builder.build_call(setup, &[initial_tc.into()], "")?;
    builder.build_call(hugr_main, &[], "")?;
    let tc = builder
        .build_call(teardown, &[], "")?
        .try_as_basic_value()
        .left()
        .ok_or_else(|| anyhow!("teardown has no return value"))?;
    builder.build_return(Some(&tc))?;

    Ok(())
}

/// Get the native target machine for LLVM
fn get_native_target_machine(opt_level: OptimizationLevel) -> Result<TargetMachine> {
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

/// Optimize the module using LLVM passes
fn optimize_module(
    module: &Module,
    target_machine: &TargetMachine,
    opt_level: OptimizationLevel,
) -> Result<()> {
    let opt_str = match opt_level {
        OptimizationLevel::Aggressive => "default<O3>",
        OptimizationLevel::Less => "default<O1>",
        OptimizationLevel::None => "default<O0>",
        OptimizationLevel::Default => "default<O2>",
    };

    module
        .run_passes(opt_str, target_machine, PassBuilderOptions::create())
        .map_err(|e| anyhow!("Failed to run optimization passes: {e}"))?;
    Ok(())
}

/// Generate LLVM module from HUGR
fn get_hugr_llvm_module<'c>(
    context: &'c Context,
    namer: Rc<Namer>,
    hugr: &Hugr,
    module_name: &str,
    exts: Rc<CodegenExtsMap<'static, Hugr>>,
) -> Result<Module<'c>> {
    let module = context.create_module(module_name);
    let emit = EmitHugr::new(context, module, namer, exts);
    Ok(emit
        .emit_module(hugr.try_fat(hugr.module_root()).unwrap())?
        .finish())
}

/// Main compilation function
fn compile_hugr<'c>(
    hugr: &mut Hugr,
    ctx: &'c Context,
    module_name: &str,
    opt_level: OptimizationLevel,
) -> Result<Module<'c>> {
    let target_machine = get_native_target_machine(opt_level)?;
    let namer = Rc::new(Namer::new("__hugr__.", true));

    // Find the entry point
    let hugr_entry = get_entry_point_name(&namer, hugr)?;

    // Process the HUGR
    process_hugr(hugr)?;

    // Generate LLVM module
    let module =
        get_hugr_llvm_module(ctx, namer, hugr, module_name, Rc::new(codegen_extensions()))?;

    // Set target-specific information
    module.set_triple(&target_machine.get_triple());
    module.set_data_layout(&target_machine.get_target_data().get_data_layout());

    // Wrap with setup/teardown
    wrap_main(ctx, &module, &hugr_entry, LLVM_MAIN)?;

    // Add metadata
    for (key, values) in METADATA {
        let md_vec = values
            .iter()
            .map(|v| ctx.metadata_string(v).into())
            .collect::<Vec<_>>();
        let node = ctx.metadata_node(md_vec.as_slice());
        module
            .add_global_metadata(key, &node)
            .map_err(|e| anyhow!("Failed to add metadata: {e}"))?;
    }

    // Optimize
    optimize_module(&module, &target_machine, opt_level)?;

    // Verify
    module
        .verify()
        .map_err(|e| anyhow!("Module verification failed: {e}"))?;

    // Ensure the EntryPoint attribute is properly applied
    // This is a workaround - re-add the attribute after optimization
    if let Some(entry_fun) = module.get_function(LLVM_MAIN) {
        entry_fun.add_attribute(
            tket::hugr::llvm::inkwell::attributes::AttributeLoc::Function,
            ctx.create_string_attribute("EntryPoint", ""),
        );
    }

    Ok(module)
}

/// Compile HUGR bytes to LLVM IR string
///
/// This is the main entry point for the compiler.
pub fn compile_hugr_bytes_to_string(hugr_bytes: &[u8]) -> Result<String, PecosError> {
    log::info!("Compiling HUGR to LLVM IR");

    // Read HUGR
    let mut hugr = read_hugr_envelope(hugr_bytes)
        .map_err(|e| PecosError::Generic(format!("Failed to read HUGR: {e}")))?;

    // Create LLVM context
    let context = Context::create();

    // Compile
    let module = compile_hugr(&mut hugr, &context, "hugr", OptimizationLevel::Default)
        .map_err(|e| PecosError::Generic(format!("Compilation failed: {e}")))?;

    // Get the module string
    let mut llvm_str = module.to_string();

    // Workaround: Manually add the EntryPoint attribute if it's missing
    // This is needed because inkwell sometimes doesn't properly serialize string attributes
    if !llvm_str.contains("\"EntryPoint\"") && llvm_str.contains("define i64 @qmain") {
        // Find where qmain is defined and add an attribute reference
        llvm_str = llvm_str.replace(
            "define i64 @qmain(i64 %0) local_unnamed_addr {",
            "define i64 @qmain(i64 %0) local_unnamed_addr #1 {",
        );
        // Add the attribute definition at the end
        if !llvm_str.contains("attributes #1") {
            llvm_str.push_str("\nattributes #1 = { \"EntryPoint\" }\n");
        }
    }

    Ok(llvm_str)
}
