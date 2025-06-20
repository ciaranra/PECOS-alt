/*!
Quantum Operation Extension for HUGR→LLVM compilation

This module provides configurable quantum operation extensions that map HUGR quantum operations
to LLVM IR function calls compatible with the PECOS QIR runtime.

Based on the working implementation from quantum-compilation-examples.
*/

use anyhow;
use hugr_core::extension::ExtensionId;
use hugr_core::ops::ExtensionOp;
use hugr_core::{HugrView, Node};
use hugr_llvm::custom::{CodegenExtension, CodegenExtsBuilder};
use hugr_llvm::emit::{EmitOpArgs, func::EmitFuncContext};
use pecos_core::errors::PecosError;
use std::collections::HashMap;

/// Result name mapping for measurement outputs
pub type ResultNameMapping = HashMap<Node, String>;

/// Configuration for quantum operation naming
#[derive(Debug, Clone)]
pub struct QuantumNamingConfig {
    pub qalloc: String,
    pub h: String,
    pub cx: String,
    pub measure: String,
    pub x: String,
    pub y: String,
    pub z: String,
    pub rx: String,
    pub ry: String,
    pub rz: String,
    /// Whether to use opaque pointer types (%Qubit*, %Result*) or integer types (i64)
    pub use_opaque_types: bool,
}

impl QuantumNamingConfig {
    /// Standard QIR naming convention with opaque types
    #[must_use]
    pub fn qir() -> Self {
        Self {
            qalloc: "__quantum__rt__qubit_allocate".to_string(),
            h: "__quantum__qis__h__body".to_string(),
            cx: "__quantum__qis__cx__body".to_string(),
            measure: "__quantum__qis__m__body".to_string(),
            x: "__quantum__qis__x__body".to_string(),
            y: "__quantum__qis__y__body".to_string(),
            z: "__quantum__qis__z__body".to_string(),
            rx: "__quantum__qis__rx__body".to_string(),
            ry: "__quantum__qis__ry__body".to_string(),
            rz: "__quantum__qis__rz__body".to_string(),
            use_opaque_types: true, // Standard QIR uses %Qubit*, %Result*
        }
    }

    /// PECOS integer-based naming convention (for compatibility with current runtime)
    #[must_use]
    pub fn pecos() -> Self {
        Self {
            qalloc: "__quantum__rt__qubit_allocate".to_string(),
            h: "__quantum__qis__h__body".to_string(),
            cx: "__quantum__qis__cx__body".to_string(),
            measure: "__quantum__qis__m__body".to_string(),
            x: "__quantum__qis__x__body".to_string(),
            y: "__quantum__qis__y__body".to_string(),
            z: "__quantum__qis__z__body".to_string(),
            rx: "__quantum__qis__rx__body".to_string(),
            ry: "__quantum__qis__ry__body".to_string(),
            rz: "__quantum__qis__rz__body".to_string(),
            use_opaque_types: false, // PECOS runtime uses i64/usize
        }
    }

    /// PECOS alternative naming convention
    #[must_use]
    pub fn pecos_alt() -> Self {
        Self {
            qalloc: "__hugr__quantum__qalloc".to_string(),
            h: "__hugr__quantum__h".to_string(),
            cx: "__hugr__quantum__cx".to_string(),
            measure: "__hugr__quantum__measure_free".to_string(),
            x: "__hugr__quantum__x".to_string(),
            y: "__hugr__quantum__y".to_string(),
            z: "__hugr__quantum__z".to_string(),
            rx: "__hugr__quantum__rx".to_string(),
            ry: "__hugr__quantum__ry".to_string(),
            rz: "__hugr__quantum__rz".to_string(),
            use_opaque_types: false, // Alternative naming also uses integers
        }
    }

    /// Custom naming with a prefix
    #[must_use]
    pub fn with_prefix(prefix: &str) -> Self {
        Self {
            qalloc: format!("{prefix}_qalloc"),
            h: format!("{prefix}_h"),
            cx: format!("{prefix}_cx"),
            measure: format!("{prefix}_measure"),
            x: format!("{prefix}_x"),
            y: format!("{prefix}_y"),
            z: format!("{prefix}_z"),
            rx: format!("{prefix}_rx"),
            ry: format!("{prefix}_ry"),
            rz: format!("{prefix}_rz"),
            use_opaque_types: false, // Custom prefixes typically use integers
        }
    }
}

/// Configurable quantum operation codegen extension
pub struct ConfigurableQuantumExtension {
    config: QuantumNamingConfig,
    result_names: ResultNameMapping,
}

impl ConfigurableQuantumExtension {
    /// Create new quantum extension with custom configuration
    #[must_use]
    pub fn new(config: QuantumNamingConfig, result_names: ResultNameMapping) -> Self {
        Self {
            config,
            result_names,
        }
    }

    /// Create quantum extension with QIR naming
    #[must_use]
    pub fn qir(result_names: ResultNameMapping) -> Self {
        Self::new(QuantumNamingConfig::qir(), result_names)
    }

    /// Create quantum extension with PECOS alternative naming
    #[must_use]
    pub fn pecos_alt(result_names: ResultNameMapping) -> Self {
        Self::new(QuantumNamingConfig::pecos_alt(), result_names)
    }
}

impl CodegenExtension for ConfigurableQuantumExtension {
    fn add_extension<'a, H: HugrView<Node = Node> + 'a>(
        self,
        builder: CodegenExtsBuilder<'a, H>,
    ) -> CodegenExtsBuilder<'a, H>
    where
        Self: 'a,
    {
        let config = std::rc::Rc::new(self.config);
        let result_names = std::rc::Rc::new(self.result_names);
        let ext_id = ExtensionId::new("tket2.quantum").unwrap();

        builder
            .extension_op(ext_id.clone(), "QAlloc".into(), {
                let cfg = config.clone();
                move |ctx, args| emit_qalloc(ctx, args, &cfg.qalloc).map_err(anyhow::Error::new)
            })
            .extension_op(ext_id.clone(), "H".into(), {
                let cfg = config.clone();
                move |ctx, args| {
                    emit_single_qubit_gate(ctx, args, &cfg.h).map_err(anyhow::Error::new)
                }
            })
            .extension_op(ext_id.clone(), "CX".into(), {
                let cfg = config.clone();
                move |ctx, args| emit_two_qubit_gate(ctx, args, &cfg.cx).map_err(anyhow::Error::new)
            })
            .extension_op(ext_id.clone(), "MeasureFree".into(), {
                let cfg = config.clone();
                let names = result_names.clone();
                move |ctx, args| {
                    emit_measure(ctx, args, &cfg.measure, &names).map_err(anyhow::Error::new)
                }
            })
            .extension_op(ext_id.clone(), "X".into(), {
                let cfg = config.clone();
                move |ctx, args| {
                    emit_single_qubit_gate(ctx, args, &cfg.x).map_err(anyhow::Error::new)
                }
            })
            .extension_op(ext_id.clone(), "Y".into(), {
                let cfg = config.clone();
                move |ctx, args| {
                    emit_single_qubit_gate(ctx, args, &cfg.y).map_err(anyhow::Error::new)
                }
            })
            .extension_op(ext_id.clone(), "Z".into(), {
                let cfg = config.clone();
                move |ctx, args| {
                    emit_single_qubit_gate(ctx, args, &cfg.z).map_err(anyhow::Error::new)
                }
            })
    }
}

fn emit_qalloc<'c, H: HugrView<Node = Node>>(
    context: &mut EmitFuncContext<'c, '_, H>,
    args: EmitOpArgs<'c, '_, ExtensionOp, H>,
    func_name: &str,
) -> Result<(), PecosError> {
    let llvm_context = context.iw_context();
    let builder = context.builder();

    // PECOS returns usize, but HUGR expects i16
    let usize_type = llvm_context.i64_type(); // assuming 64-bit system
    let i16_type = llvm_context.i16_type();

    let func_type = usize_type.fn_type(&[], false);
    let func = context.get_extern_func(func_name, func_type)?;

    let qubit_usize = builder
        .build_call(func, &[], "qubit_usize")?
        .try_as_basic_value()
        .unwrap_left();

    // Convert from usize to i16 for HUGR
    let qubit_i16 = builder.build_int_truncate(qubit_usize.into_int_value(), i16_type, "qubit")?;

    args.outputs.finish(builder, [qubit_i16.into()])?;
    Ok(())
}

fn emit_single_qubit_gate<'c, H: HugrView<Node = Node>>(
    context: &mut EmitFuncContext<'c, '_, H>,
    args: EmitOpArgs<'c, '_, ExtensionOp, H>,
    func_name: &str,
) -> Result<(), PecosError> {
    let llvm_context = context.iw_context();
    let builder = context.builder();

    let usize_type = llvm_context.i64_type(); // assuming 64-bit system

    // Convert qubit from i16 to usize
    let qubit_usize =
        builder.build_int_z_extend(args.inputs[0].into_int_value(), usize_type, "qubit_usize")?;

    let func_type = llvm_context
        .void_type()
        .fn_type(&[usize_type.into()], false);
    let func = context.get_extern_func(func_name, func_type)?;

    builder.build_call(func, &[qubit_usize.into()], "")?;
    args.outputs.finish(builder, [args.inputs[0]])?;
    Ok(())
}

fn emit_two_qubit_gate<'c, H: HugrView<Node = Node>>(
    context: &mut EmitFuncContext<'c, '_, H>,
    args: EmitOpArgs<'c, '_, ExtensionOp, H>,
    func_name: &str,
) -> Result<(), PecosError> {
    let llvm_context = context.iw_context();
    let builder = context.builder();

    let usize_type = llvm_context.i64_type(); // assuming 64-bit system

    // Convert both qubits from i16 to usize
    let control_usize =
        builder.build_int_z_extend(args.inputs[0].into_int_value(), usize_type, "control_usize")?;

    let target_usize =
        builder.build_int_z_extend(args.inputs[1].into_int_value(), usize_type, "target_usize")?;

    let func_type = llvm_context
        .void_type()
        .fn_type(&[usize_type.into(), usize_type.into()], false);
    let func = context.get_extern_func(func_name, func_type)?;

    builder.build_call(func, &[control_usize.into(), target_usize.into()], "")?;
    args.outputs
        .finish(builder, [args.inputs[0], args.inputs[1]])?;
    Ok(())
}

fn emit_measure<'c, H: HugrView<Node = Node>>(
    context: &mut EmitFuncContext<'c, '_, H>,
    args: EmitOpArgs<'c, '_, ExtensionOp, H>,
    func_name: &str,
    result_names: &ResultNameMapping,
) -> Result<(), PecosError> {
    let llvm_context = context.iw_context();
    let builder = context.builder();

    // PECOS expects __quantum__qis__m__body(qubit: usize, result: usize) -> u32
    // We need to allocate a result ID and convert types
    let i32_type = llvm_context.i32_type();
    let i8_type = llvm_context.i8_type();
    let usize_type = llvm_context.i64_type(); // assuming 64-bit system

    // First allocate a result
    let alloc_func_type = usize_type.fn_type(&[], false);
    let alloc_func = context.get_extern_func("__quantum__rt__result_allocate", alloc_func_type)?;
    let result_id = builder
        .build_call(alloc_func, &[], "result_id")?
        .try_as_basic_value()
        .unwrap_left();

    // Convert qubit from i16 to usize
    let qubit_usize =
        builder.build_int_z_extend(args.inputs[0].into_int_value(), usize_type, "qubit_usize")?;

    // Call measurement with (qubit, result_id)
    let measure_func_type = i32_type.fn_type(&[usize_type.into(), usize_type.into()], false);
    let measure_func = context.get_extern_func(func_name, measure_func_type)?;

    let measurement = builder
        .build_call(
            measure_func,
            &[qubit_usize.into(), result_id.into()],
            "measurement",
        )?
        .try_as_basic_value()
        .unwrap_left();

    // IMMEDIATELY record the result with the same result_id used for measurement
    // This follows QIR standard: measurement result should be recorded with same result pointer

    // Get the result name for this measurement node, fallback to "c" if not found
    let measurement_node = args.node.node();
    let name = result_names
        .get(&measurement_node)
        .map_or("c", std::string::String::as_str);

    let name_str = llvm_context.const_string(name.as_bytes(), true);
    let name_global = context.get_global(format!("str_{name}"), name_str.get_type(), true)?;
    name_global.set_initializer(&name_str);

    // Get pointer to the string
    let zero = i32_type.const_zero();
    let indices = [zero, zero];
    let string_ptr = unsafe {
        builder.build_in_bounds_gep(name_global.as_pointer_value(), &indices, "string_ptr")?
    };

    // Call result recording with the SAME result_id used for measurement
    let void_type = llvm_context.void_type();
    let i8_ptr_type = i8_type.ptr_type(hugr_llvm::inkwell::AddressSpace::default());
    let record_func_type = void_type.fn_type(&[usize_type.into(), i8_ptr_type.into()], false);
    let record_func =
        context.get_extern_func("__quantum__rt__result_record_output", record_func_type)?;

    builder.build_call(record_func, &[result_id.into(), string_ptr.into()], "")?;

    // Convert u32 result to bool (0 -> false, 1 -> true)
    let zero = i32_type.const_zero();
    let bool_result = builder.build_int_compare(
        hugr_llvm::inkwell::IntPredicate::NE,
        measurement.into_int_value(),
        zero,
        "bool_result",
    )?;

    args.outputs.finish(builder, [bool_result.into()])?;
    Ok(())
}
