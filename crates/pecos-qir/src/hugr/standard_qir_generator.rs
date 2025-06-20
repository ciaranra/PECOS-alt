/*!
Standard QIR Generator for HUGR

This module generates standard QIR format that is compatible with the existing
PECOS `QirEngine` infrastructure, rather than custom LLVM IR.

The output format matches examples/qir/bell.ll and works with `QirEngine::new()`.
*/

use anyhow;
use hugr_core::ops::ExtensionOp;
use hugr_core::{HugrView, Node};
use hugr_llvm::custom::{CodegenExtension, CodegenExtsBuilder};
use hugr_llvm::emit::{EmitOpArgs, func::EmitFuncContext};
use pecos_core::errors::PecosError;
use std::collections::HashMap;

/// Result name mapping for measurement outputs
pub type ResultNameMapping = HashMap<Node, String>;

/// Standard QIR quantum operation extension
///
/// This generates standard QIR format using opaque %Qubit* and %Result* types
/// that work with the existing PECOS `QirEngine`.
pub struct StandardQirExtension {
    result_names: ResultNameMapping,
}

impl StandardQirExtension {
    #[must_use]
    pub fn new(result_names: ResultNameMapping) -> Self {
        Self { result_names }
    }
}

impl CodegenExtension for StandardQirExtension {
    fn add_extension<'a, H: HugrView<Node = Node> + 'a>(
        self,
        builder: CodegenExtsBuilder<'a, H>,
    ) -> CodegenExtsBuilder<'a, H>
    where
        Self: 'a,
    {
        let result_names = std::rc::Rc::new(self.result_names);
        let ext_id = hugr_core::extension::ExtensionId::new("tket2.quantum").unwrap();

        builder
            .extension_op(ext_id.clone(), "QAlloc".into(), {
                move |ctx, args| emit_qalloc_standard(ctx, args).map_err(anyhow::Error::new)
            })
            .extension_op(ext_id.clone(), "H".into(), {
                move |ctx, args| {
                    emit_single_qubit_gate_standard(ctx, args, "__quantum__qis__h__body_i64")
                        .map_err(anyhow::Error::new)
                }
            })
            .extension_op(ext_id.clone(), "CX".into(), {
                move |ctx, args| {
                    emit_two_qubit_gate_standard(ctx, args, "__quantum__qis__cx__body_i64")
                        .map_err(anyhow::Error::new)
                }
            })
            .extension_op(ext_id.clone(), "MeasureFree".into(), {
                let names = result_names.clone();
                move |ctx, args| {
                    emit_measure_standard(ctx, args, &names).map_err(anyhow::Error::new)
                }
            })
            .extension_op(ext_id.clone(), "X".into(), {
                move |ctx, args| {
                    emit_single_qubit_gate_standard(ctx, args, "__quantum__qis__x__body_i64")
                        .map_err(anyhow::Error::new)
                }
            })
            .extension_op(ext_id.clone(), "Y".into(), {
                move |ctx, args| {
                    emit_single_qubit_gate_standard(ctx, args, "__quantum__qis__y__body_i64")
                        .map_err(anyhow::Error::new)
                }
            })
            .extension_op(ext_id.clone(), "Z".into(), {
                move |ctx, args| {
                    emit_single_qubit_gate_standard(ctx, args, "__quantum__qis__z__body_i64")
                        .map_err(anyhow::Error::new)
                }
            })
            // Rotation gates
            .extension_op(ext_id.clone(), "RX".into(), {
                move |ctx, args| {
                    emit_rotation_gate_standard(ctx, args, "__quantum__qis__rx__body")
                        .map_err(anyhow::Error::new)
                }
            })
            .extension_op(ext_id.clone(), "RY".into(), {
                move |ctx, args| {
                    emit_rotation_gate_standard(ctx, args, "__quantum__qis__ry__body")
                        .map_err(anyhow::Error::new)
                }
            })
            .extension_op(ext_id.clone(), "RZ".into(), {
                move |ctx, args| {
                    emit_rotation_gate_standard(ctx, args, "__quantum__qis__rz__body")
                        .map_err(anyhow::Error::new)
                }
            })
            // Pauli gates - S/SZ, T, and their adjoints
            .extension_op(ext_id.clone(), "S".into(), {
                move |ctx, args| {
                    emit_single_qubit_gate_standard(ctx, args, "__quantum__qis__s__body")
                        .map_err(anyhow::Error::new)
                }
            })
            .extension_op(ext_id.clone(), "Sdg".into(), {
                move |ctx, args| {
                    emit_single_qubit_gate_standard(ctx, args, "__quantum__qis__sdg__body")
                        .map_err(anyhow::Error::new)
                }
            })
            .extension_op(ext_id.clone(), "T".into(), {
                move |ctx, args| {
                    emit_single_qubit_gate_standard(ctx, args, "__quantum__qis__t__body")
                        .map_err(anyhow::Error::new)
                }
            })
            .extension_op(ext_id.clone(), "Tdg".into(), {
                move |ctx, args| {
                    emit_single_qubit_gate_standard(ctx, args, "__quantum__qis__tdg__body")
                        .map_err(anyhow::Error::new)
                }
            })
            // Two-qubit gates
            .extension_op(ext_id.clone(), "CY".into(), {
                move |ctx, args| {
                    emit_two_qubit_gate_standard(ctx, args, "__quantum__qis__cy__body")
                        .map_err(anyhow::Error::new)
                }
            })
            .extension_op(ext_id.clone(), "CZ".into(), {
                move |ctx, args| {
                    emit_two_qubit_gate_standard(ctx, args, "__quantum__qis__cz__body")
                        .map_err(anyhow::Error::new)
                }
            })
            .extension_op(ext_id.clone(), "CH".into(), {
                move |ctx, args| {
                    emit_two_qubit_gate_standard(ctx, args, "__quantum__qis__ch__body")
                        .map_err(anyhow::Error::new)
                }
            })
            // Controlled rotation gates
            .extension_op(ext_id.clone(), "CRZ".into(), {
                move |ctx, args| {
                    emit_controlled_rotation_gate_standard(ctx, args, "__quantum__qis__crz__body")
                        .map_err(anyhow::Error::new)
                }
            })
            .extension_op(ext_id.clone(), "Toffoli".into(), {
                move |ctx, args| {
                    emit_toffoli_gate_standard(ctx, args, "__quantum__qis__ccx__body")
                        .map_err(anyhow::Error::new)
                }
            })
    }
}

// Static counter for qubit allocation
static mut NEXT_QUBIT_ID: i64 = 0;

fn emit_qalloc_standard<'c, H: HugrView<Node = Node>>(
    context: &mut EmitFuncContext<'c, '_, H>,
    args: EmitOpArgs<'c, '_, ExtensionOp, H>,
) -> Result<(), PecosError> {
    let llvm_context = context.iw_context();
    let builder = context.builder();

    // For now, allocate static qubit IDs (0, 1, 2, ...)
    // In a full implementation, we could use __quantum__rt__qubit_allocate()
    // but the standard examples use static IDs

    // Use a simple counter for qubit allocation
    // This should be properly managed, but for now use static allocation

    let qubit_id = unsafe {
        let id = NEXT_QUBIT_ID;
        NEXT_QUBIT_ID += 1;
        id
    };

    let i64_type = llvm_context.i64_type();
    let qubit_id_val = i64_type.const_int(qubit_id.try_into().unwrap_or(0), false);

    // HUGR expects i16, so we need to return the ID as i16
    let i16_type = llvm_context.i16_type();
    let qubit_i16 = builder.build_int_truncate(qubit_id_val, i16_type, "qubit")?;

    args.outputs.finish(builder, [qubit_i16.into()])?;
    Ok(())
}

fn emit_single_qubit_gate_standard<'c, H: HugrView<Node = Node>>(
    context: &mut EmitFuncContext<'c, '_, H>,
    args: EmitOpArgs<'c, '_, ExtensionOp, H>,
    func_name: &str,
) -> Result<(), PecosError> {
    let llvm_context = context.iw_context();
    let builder = context.builder();

    // Convert i16 qubit to i64 (compatible with usize on 64-bit systems)
    let i64_type = llvm_context.i64_type();
    let qubit_i64 =
        builder.build_int_z_extend(args.inputs[0].into_int_value(), i64_type, "qubit_i64")?;

    // PECOS QIR function signature: void @__quantum__qis__h__body(i64)
    let void_type = llvm_context.void_type();
    let func_type = void_type.fn_type(&[i64_type.into()], false);
    let func = context.get_extern_func(func_name, func_type)?;

    builder.build_call(func, &[qubit_i64.into()], "")?;
    args.outputs.finish(builder, [args.inputs[0]])?;
    Ok(())
}

fn emit_two_qubit_gate_standard<'c, H: HugrView<Node = Node>>(
    context: &mut EmitFuncContext<'c, '_, H>,
    args: EmitOpArgs<'c, '_, ExtensionOp, H>,
    func_name: &str,
) -> Result<(), PecosError> {
    let llvm_context = context.iw_context();
    let builder = context.builder();

    // Convert both qubits from i16 to i64 (compatible with usize on 64-bit systems)
    let i64_type = llvm_context.i64_type();

    let control_i64 =
        builder.build_int_z_extend(args.inputs[0].into_int_value(), i64_type, "control_i64")?;
    let target_i64 =
        builder.build_int_z_extend(args.inputs[1].into_int_value(), i64_type, "target_i64")?;

    // PECOS QIR function signature: void @__quantum__qis__cx__body(i64, i64)
    let void_type = llvm_context.void_type();
    let func_type = void_type.fn_type(&[i64_type.into(), i64_type.into()], false);
    let func = context.get_extern_func(func_name, func_type)?;

    builder.build_call(func, &[control_i64.into(), target_i64.into()], "")?;
    args.outputs
        .finish(builder, [args.inputs[0], args.inputs[1]])?;
    Ok(())
}

// Static counter for result allocation
static mut NEXT_RESULT_ID: u64 = 0;

fn emit_rotation_gate_standard<'c, H: HugrView<Node = Node>>(
    context: &mut EmitFuncContext<'c, '_, H>,
    args: EmitOpArgs<'c, '_, ExtensionOp, H>,
    func_name: &str,
) -> Result<(), PecosError> {
    let llvm_context = context.iw_context();
    let builder = context.builder();

    // Rotation gates take a qubit and an angle (float)
    // Convert i16 qubit to i64
    let i64_type = llvm_context.i64_type();
    let qubit_i64 =
        builder.build_int_z_extend(args.inputs[0].into_int_value(), i64_type, "qubit_i64")?;

    // Get the angle parameter (should be a float)
    let angle = args.inputs[1].into_float_value();
    let f64_type = llvm_context.f64_type();

    // PECOS QIR function signature: void @__quantum__qis__rx__body(f64, i64)
    let void_type = llvm_context.void_type();
    let func_type = void_type.fn_type(&[f64_type.into(), i64_type.into()], false);
    let func = context.get_extern_func(func_name, func_type)?;

    builder.build_call(func, &[angle.into(), qubit_i64.into()], "")?;
    args.outputs.finish(builder, [args.inputs[0]])?;
    Ok(())
}

fn emit_controlled_rotation_gate_standard<'c, H: HugrView<Node = Node>>(
    context: &mut EmitFuncContext<'c, '_, H>,
    args: EmitOpArgs<'c, '_, ExtensionOp, H>,
    func_name: &str,
) -> Result<(), PecosError> {
    let llvm_context = context.iw_context();
    let builder = context.builder();

    // Controlled rotation gates take two qubits and an angle
    let i64_type = llvm_context.i64_type();
    let control_i64 =
        builder.build_int_z_extend(args.inputs[0].into_int_value(), i64_type, "control_i64")?;
    let target_i64 =
        builder.build_int_z_extend(args.inputs[1].into_int_value(), i64_type, "target_i64")?;
    
    // Get the angle parameter
    let angle = args.inputs[2].into_float_value();
    let f64_type = llvm_context.f64_type();

    // PECOS QIR function signature: void @__quantum__qis__crz__body(f64, i64, i64)
    let void_type = llvm_context.void_type();
    let func_type = void_type.fn_type(&[f64_type.into(), i64_type.into(), i64_type.into()], false);
    let func = context.get_extern_func(func_name, func_type)?;

    builder.build_call(func, &[angle.into(), control_i64.into(), target_i64.into()], "")?;
    args.outputs
        .finish(builder, [args.inputs[0], args.inputs[1]])?;
    Ok(())
}

fn emit_toffoli_gate_standard<'c, H: HugrView<Node = Node>>(
    context: &mut EmitFuncContext<'c, '_, H>,
    args: EmitOpArgs<'c, '_, ExtensionOp, H>,
    func_name: &str,
) -> Result<(), PecosError> {
    let llvm_context = context.iw_context();
    let builder = context.builder();

    // Toffoli takes three qubits
    let i64_type = llvm_context.i64_type();
    let control1_i64 =
        builder.build_int_z_extend(args.inputs[0].into_int_value(), i64_type, "control1_i64")?;
    let control2_i64 =
        builder.build_int_z_extend(args.inputs[1].into_int_value(), i64_type, "control2_i64")?;
    let target_i64 =
        builder.build_int_z_extend(args.inputs[2].into_int_value(), i64_type, "target_i64")?;

    // PECOS QIR function signature: void @__quantum__qis__ccx__body(i64, i64, i64)
    let void_type = llvm_context.void_type();
    let func_type = void_type.fn_type(&[i64_type.into(), i64_type.into(), i64_type.into()], false);
    let func = context.get_extern_func(func_name, func_type)?;

    builder.build_call(func, &[control1_i64.into(), control2_i64.into(), target_i64.into()], "")?;
    args.outputs
        .finish(builder, [args.inputs[0], args.inputs[1], args.inputs[2]])?;
    Ok(())
}

fn emit_measure_standard<'c, H: HugrView<Node = Node>>(
    context: &mut EmitFuncContext<'c, '_, H>,
    args: EmitOpArgs<'c, '_, ExtensionOp, H>,
    _result_names: &ResultNameMapping,
) -> Result<(), PecosError> {
    let llvm_context = context.iw_context();
    let builder = context.builder();

    // Convert qubit from i16 to i64
    let i64_type = llvm_context.i64_type();
    let qubit_i64 =
        builder.build_int_z_extend(args.inputs[0].into_int_value(), i64_type, "qubit_i64")?;

    // Allocate result ID (for now, use static allocation like qubits)
    let result_id = unsafe {
        let id = NEXT_RESULT_ID;
        NEXT_RESULT_ID += 1;
        id
    };

    let result_id_val = i64_type.const_int(result_id, false);

    // PECOS QIR measurement: i32 @__quantum__qis__m__body(i64, i64)
    let i32_type = llvm_context.i32_type();
    let measure_func_type = i32_type.fn_type(&[i64_type.into(), i64_type.into()], false);
    let measure_func = context.get_extern_func("__quantum__qis__m__body", measure_func_type)?;

    let measurement_result = builder.build_call(
        measure_func,
        &[qubit_i64.into(), result_id_val.into()],
        "measurement_result",
    )?;

    // Convert i32 result to bool for HUGR
    let measurement_i32 = measurement_result
        .try_as_basic_value()
        .left()
        .unwrap()
        .into_int_value();
    let zero_i32 = i32_type.const_zero();
    let is_one = builder.build_int_compare(
        hugr_llvm::inkwell::IntPredicate::NE,
        measurement_i32,
        zero_i32,
        "is_one",
    )?;

    args.outputs.finish(builder, [is_one.into()])?;
    Ok(())
}
