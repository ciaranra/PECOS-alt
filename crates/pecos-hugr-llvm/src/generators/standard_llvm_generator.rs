/*!
HUGR LLVM Generator

This module generates HUGR-style LLVM IR format using integer-based parameters
that is compatible with the PECOS `LlvmEngine` infrastructure.

The output format matches examples/llvm/bell.ll and works with `LlvmEngine::new()`.
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

/// Standard LLVM quantum operation extension
///
/// This generates HUGR-style LLVM IR format using integer types
/// that work with the PECOS `LlvmEngine`.
pub struct StandardLlvmExtension {
    result_names: ResultNameMapping,
}

impl StandardLlvmExtension {
    #[must_use]
    pub fn new(result_names: ResultNameMapping) -> Self {
        Self { result_names }
    }

    fn get_function_name(base_name: &str) -> String {
        // Use the base name directly without any suffix
        base_name.to_string()
    }
}

impl CodegenExtension for StandardLlvmExtension {
    #[allow(clippy::too_many_lines)]
    #[allow(clippy::similar_names)]
    fn add_extension<'a, H: HugrView<Node = Node> + 'a>(
        self,
        builder: CodegenExtsBuilder<'a, H>,
    ) -> CodegenExtsBuilder<'a, H>
    where
        Self: 'a,
    {
        let ext_id = hugr_core::extension::ExtensionId::new("tket2.quantum").unwrap();

        // Pre-compute all function names before moving self.result_names
        let h_func = Self::get_function_name("__quantum__qis__h__body");
        let x_func = Self::get_function_name("__quantum__qis__x__body");
        let y_func = Self::get_function_name("__quantum__qis__y__body");
        let z_func = Self::get_function_name("__quantum__qis__z__body");
        let cx_func = Self::get_function_name("__quantum__qis__cx__body");

        // These names are intentionally similar as they represent rotation functions for different axes
        let rotation_x_func = Self::get_function_name("__quantum__qis__rx__body");
        let rotation_y_func = Self::get_function_name("__quantum__qis__ry__body");
        let rotation_z_func = Self::get_function_name("__quantum__qis__rz__body");

        let result_names = std::rc::Rc::new(self.result_names);

        builder
            .extension_op(ext_id.clone(), "QAlloc".into(), {
                move |ctx, args| emit_qalloc_standard(ctx, args).map_err(anyhow::Error::new)
            })
            .extension_op(ext_id.clone(), "H".into(), {
                move |ctx, args| {
                    emit_single_qubit_gate_standard(ctx, args, &h_func).map_err(anyhow::Error::new)
                }
            })
            .extension_op(ext_id.clone(), "CX".into(), {
                move |ctx, args| {
                    emit_two_qubit_gate_standard(ctx, args, &cx_func).map_err(anyhow::Error::new)
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
                    emit_single_qubit_gate_standard(ctx, args, &x_func).map_err(anyhow::Error::new)
                }
            })
            .extension_op(ext_id.clone(), "Y".into(), {
                move |ctx, args| {
                    emit_single_qubit_gate_standard(ctx, args, &y_func).map_err(anyhow::Error::new)
                }
            })
            .extension_op(ext_id.clone(), "Z".into(), {
                move |ctx, args| {
                    emit_single_qubit_gate_standard(ctx, args, &z_func).map_err(anyhow::Error::new)
                }
            })
            // Rotation gates
            .extension_op(ext_id.clone(), "Rx".into(), {
                move |ctx, args| {
                    emit_rotation_gate_standard(ctx, args, &rotation_x_func)
                        .map_err(anyhow::Error::new)
                }
            })
            .extension_op(ext_id.clone(), "Ry".into(), {
                move |ctx, args| {
                    emit_rotation_gate_standard(ctx, args, &rotation_y_func)
                        .map_err(anyhow::Error::new)
                }
            })
            .extension_op(ext_id.clone(), "Rz".into(), {
                move |ctx, args| {
                    emit_rotation_gate_standard(ctx, args, &rotation_z_func)
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
                move |ctx, args| emit_ch_decomposed(ctx, args).map_err(anyhow::Error::new)
            })
            // Controlled rotation gates
            .extension_op(ext_id.clone(), "CRz".into(), {
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

// Removed static counter - using runtime allocation instead

fn emit_qalloc_standard<'c, H: HugrView<Node = Node>>(
    context: &mut EmitFuncContext<'c, '_, H>,
    args: EmitOpArgs<'c, '_, ExtensionOp, H>,
) -> Result<(), PecosError> {
    let llvm_context = context.iw_context();
    let builder = context.builder();

    // Call the proper runtime allocation function: i64 @__quantum__rt__qubit_allocate()
    let i64_type = llvm_context.i64_type();
    let allocate_func_type = i64_type.fn_type(&[], false);
    let allocate_func =
        context.get_extern_func("__quantum__rt__qubit_allocate", allocate_func_type)?;

    // Call the allocation function
    let qubit_call = builder.build_call(allocate_func, &[], "qubit_usize")?;
    let qubit_i64 = qubit_call
        .try_as_basic_value()
        .left()
        .unwrap()
        .into_int_value();

    // HUGR expects i16, so we need to truncate the ID to i16
    let i16_type = llvm_context.i16_type();
    let qubit_i16 = builder.build_int_truncate(qubit_i64, i16_type, "qubit")?;

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

// Removed static counter - using runtime allocation instead

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

    builder.build_call(
        func,
        &[angle.into(), control_i64.into(), target_i64.into()],
        "",
    )?;
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

    builder.build_call(
        func,
        &[control1_i64.into(), control2_i64.into(), target_i64.into()],
        "",
    )?;
    args.outputs
        .finish(builder, [args.inputs[0], args.inputs[1], args.inputs[2]])?;
    Ok(())
}

fn emit_ch_decomposed<'c, H: HugrView<Node = Node>>(
    context: &mut EmitFuncContext<'c, '_, H>,
    args: EmitOpArgs<'c, '_, ExtensionOp, H>,
) -> Result<(), PecosError> {
    let llvm_context = context.iw_context();
    let builder = context.builder();

    // CH gate decomposition: Ry(-π/4) on target, CZ, Ry(π/4) on target
    // Convert qubits from i16 to i64
    let i64_type = llvm_context.i64_type();
    let control_i64 =
        builder.build_int_z_extend(args.inputs[0].into_int_value(), i64_type, "control_i64")?;
    let target_i64 =
        builder.build_int_z_extend(args.inputs[1].into_int_value(), i64_type, "target_i64")?;

    // Create angle values for Ry gates
    let f64_type = llvm_context.f64_type();
    // -π/4 radians for the first Ry
    let neg_pi_4 = f64_type.const_float(-std::f64::consts::PI / 4.0);
    // π/4 radians for the second Ry
    let pi_4 = f64_type.const_float(std::f64::consts::PI / 4.0);

    // First Ry(-π/4) on target
    let ry_func_type = llvm_context
        .void_type()
        .fn_type(&[f64_type.into(), i64_type.into()], false);
    let ry_func = context.get_extern_func("__quantum__qis__ry__body", ry_func_type)?;
    builder.build_call(ry_func, &[neg_pi_4.into(), target_i64.into()], "")?;

    // CZ on control and target
    let cz_func_type = llvm_context
        .void_type()
        .fn_type(&[i64_type.into(), i64_type.into()], false);
    let cz_func = context.get_extern_func("__quantum__qis__cz__body", cz_func_type)?;
    builder.build_call(cz_func, &[control_i64.into(), target_i64.into()], "")?;

    // Second Ry(π/4) on target
    builder.build_call(ry_func, &[pi_4.into(), target_i64.into()], "")?;

    // Return the original qubits
    args.outputs
        .finish(builder, [args.inputs[0], args.inputs[1]])?;
    Ok(())
}

fn emit_measure_standard<'c, H: HugrView<Node = Node>>(
    context: &mut EmitFuncContext<'c, '_, H>,
    args: EmitOpArgs<'c, '_, ExtensionOp, H>,
    result_names: &ResultNameMapping,
) -> Result<(), PecosError> {
    let llvm_context = context.iw_context();
    let builder = context.builder();

    // Convert qubit from i16 to i64
    let i64_type = llvm_context.i64_type();
    let qubit_i64 =
        builder.build_int_z_extend(args.inputs[0].into_int_value(), i64_type, "qubit_i64")?;

    // Allocate result ID using HUGR runtime allocation
    // Call __quantum__rt__result_allocate() which returns i64
    let allocate_result_func_type = i64_type.fn_type(&[], false);
    let allocate_result_func =
        context.get_extern_func("__quantum__rt__result_allocate", allocate_result_func_type)?;

    let result_call = builder.build_call(allocate_result_func, &[], "result_id")?;
    let result_id_val = result_call
        .try_as_basic_value()
        .left()
        .unwrap()
        .into_int_value();

    // PECOS QIR measurement: i32 @__quantum__qis__m__body(i64, i64)
    let i32_type = llvm_context.i32_type();
    let measure_func_type = i32_type.fn_type(&[i64_type.into(), i64_type.into()], false);
    let measure_func = context.get_extern_func("__quantum__qis__m__body", measure_func_type)?;

    let measurement_result = builder.build_call(
        measure_func,
        &[qubit_i64.into(), result_id_val.into()],
        "measurement_result",
    )?;

    // IMPORTANT: Record the result with __quantum__rt__result_record_output
    // Get the result name for this measurement node, fallback to "c" if not found
    let measurement_node = args.node.node();
    let name = result_names
        .get(&measurement_node)
        .map_or("c", std::string::String::as_str);

    // Create string constant for the result name
    let i8_type = llvm_context.i8_type();
    let name_str = llvm_context.const_string(name.as_bytes(), true);
    let name_global = context.get_global(format!("str_{name}"), name_str.get_type(), true)?;
    name_global.set_initializer(&name_str);

    // Get pointer to the string
    let zero = i32_type.const_zero();
    let indices = [zero, zero];
    let string_ptr = unsafe {
        builder.build_in_bounds_gep(name_global.as_pointer_value(), &indices, "string_ptr")?
    };

    // Call result recording with the result ID as a pointer (cast i64 to i8*)
    let void_type = llvm_context.void_type();
    let i8_ptr_type = i8_type.ptr_type(hugr_llvm::inkwell::AddressSpace::default());
    let record_func_type = void_type.fn_type(&[i8_ptr_type.into(), i8_ptr_type.into()], false);
    let record_func =
        context.get_extern_func("__quantum__rt__result_record_output", record_func_type)?;

    // Cast the result ID to a pointer (this is how the PECOS runtime expects it)
    let result_ptr = builder.build_int_to_ptr(result_id_val, i8_ptr_type, "result_ptr")?;

    builder.build_call(record_func, &[result_ptr.into(), string_ptr.into()], "")?;

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
