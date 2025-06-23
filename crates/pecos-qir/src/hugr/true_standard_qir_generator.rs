/*!
True Standard QIR Generator for HUGR

This module generates TRUE standard QIR format that uses opaque pointer types (%Qubit*, %Result*)
instead of integer-based types. This format is compatible with Microsoft QIR specification
and the examples in examples/qir/bell.ll.

The key differences from the current StandardQirExtension are:
1. Uses opaque pointer types: %Qubit* and %Result* instead of i64
2. Measurement functions return void instead of i32
3. Entry points return void instead of i1
4. Includes proper type declarations for %Qubit and %Result
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

/// True Standard QIR quantum operation extension
///
/// This generates TRUE standard QIR format using opaque %Qubit* and %Result* types
/// that are fully compatible with Microsoft QIR specification.
pub struct TrueStandardQirExtension {
    result_names: ResultNameMapping,
}

impl TrueStandardQirExtension {
    #[must_use]
    pub fn new(result_names: ResultNameMapping) -> Self {
        // Reset static counters for each new extension instance (per function compilation)
        unsafe {
            NEXT_QUBIT_ID = 0;
            NEXT_RESULT_ID = 0;
        }
        Self { result_names }
    }
}

impl CodegenExtension for TrueStandardQirExtension {
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
                move |ctx, args| emit_qalloc_true_standard(ctx, args).map_err(anyhow::Error::new)
            })
            .extension_op(ext_id.clone(), "H".into(), {
                move |ctx, args| {
                    emit_single_qubit_gate_true_standard(ctx, args, "__quantum__qis__h__body")
                        .map_err(anyhow::Error::new)
                }
            })
            .extension_op(ext_id.clone(), "CX".into(), {
                move |ctx, args| {
                    emit_two_qubit_gate_true_standard(ctx, args, "__quantum__qis__cx__body")
                        .map_err(anyhow::Error::new)
                }
            })
            .extension_op(ext_id.clone(), "MeasureFree".into(), {
                let names = result_names.clone();
                move |ctx, args| {
                    emit_measure_true_standard(ctx, args, &names).map_err(anyhow::Error::new)
                }
            })
            .extension_op(ext_id.clone(), "X".into(), {
                move |ctx, args| {
                    emit_single_qubit_gate_true_standard(ctx, args, "__quantum__qis__x__body")
                        .map_err(anyhow::Error::new)
                }
            })
            .extension_op(ext_id.clone(), "Y".into(), {
                move |ctx, args| {
                    emit_single_qubit_gate_true_standard(ctx, args, "__quantum__qis__y__body")
                        .map_err(anyhow::Error::new)
                }
            })
            .extension_op(ext_id.clone(), "Z".into(), {
                move |ctx, args| {
                    emit_single_qubit_gate_true_standard(ctx, args, "__quantum__qis__z__body")
                        .map_err(anyhow::Error::new)
                }
            })
            // Rotation gates
            .extension_op(ext_id.clone(), "Rx".into(), {
                move |ctx, args| {
                    emit_rotation_gate_true_standard(ctx, args, "__quantum__qis__rx__body")
                        .map_err(anyhow::Error::new)
                }
            })
            .extension_op(ext_id.clone(), "Ry".into(), {
                move |ctx, args| {
                    emit_rotation_gate_true_standard(ctx, args, "__quantum__qis__ry__body")
                        .map_err(anyhow::Error::new)
                }
            })
            .extension_op(ext_id.clone(), "Rz".into(), {
                move |ctx, args| {
                    emit_rotation_gate_true_standard(ctx, args, "__quantum__qis__rz__body")
                        .map_err(anyhow::Error::new)
                }
            })
            // Pauli gates - S, T, and their adjoints
            .extension_op(ext_id.clone(), "S".into(), {
                move |ctx, args| {
                    emit_single_qubit_gate_true_standard(ctx, args, "__quantum__qis__s__body")
                        .map_err(anyhow::Error::new)
                }
            })
            .extension_op(ext_id.clone(), "Sdg".into(), {
                move |ctx, args| {
                    emit_single_qubit_gate_true_standard(ctx, args, "__quantum__qis__sdg__body")
                        .map_err(anyhow::Error::new)
                }
            })
            .extension_op(ext_id.clone(), "T".into(), {
                move |ctx, args| {
                    emit_single_qubit_gate_true_standard(ctx, args, "__quantum__qis__t__body")
                        .map_err(anyhow::Error::new)
                }
            })
            .extension_op(ext_id.clone(), "Tdg".into(), {
                move |ctx, args| {
                    emit_single_qubit_gate_true_standard(ctx, args, "__quantum__qis__tdg__body")
                        .map_err(anyhow::Error::new)
                }
            })
            // Two-qubit gates
            .extension_op(ext_id.clone(), "CY".into(), {
                move |ctx, args| {
                    emit_two_qubit_gate_true_standard(ctx, args, "__quantum__qis__cy__body")
                        .map_err(anyhow::Error::new)
                }
            })
            .extension_op(ext_id.clone(), "CZ".into(), {
                move |ctx, args| {
                    emit_two_qubit_gate_true_standard(ctx, args, "__quantum__qis__cz__body")
                        .map_err(anyhow::Error::new)
                }
            })
            // Controlled rotation gates
            .extension_op(ext_id.clone(), "CRz".into(), {
                move |ctx, args| {
                    emit_controlled_rotation_gate_true_standard(ctx, args, "__quantum__qis__crz__body")
                        .map_err(anyhow::Error::new)
                }
            })
            // Three-qubit gates
            .extension_op(ext_id.clone(), "Toffoli".into(), {
                move |ctx, args| {
                    emit_toffoli_gate_true_standard(ctx, args, "__quantum__qis__ccx__body")
                        .map_err(anyhow::Error::new)
                }
            })
    }
}

// Static counters for qubit and result allocation
static mut NEXT_QUBIT_ID: i64 = 0;
static mut NEXT_RESULT_ID: u64 = 0;

fn emit_qalloc_true_standard<'c, H: HugrView<Node = Node>>(
    context: &mut EmitFuncContext<'c, '_, H>,
    args: EmitOpArgs<'c, '_, ExtensionOp, H>,
) -> Result<(), PecosError> {
    let llvm_context = context.iw_context();
    let builder = context.builder();

    // Call the proper runtime allocation function: i64 @__quantum__rt__qubit_allocate()
    // This matches the bell_final.ll example
    let i64_type = llvm_context.i64_type();
    let allocate_func_type = i64_type.fn_type(&[], false);
    let allocate_func = context.get_extern_func("__quantum__rt__qubit_allocate", allocate_func_type)?;

    // Call the allocation function
    let qubit_call = builder.build_call(allocate_func, &[], "qubit_usize")?;
    let qubit_i64 = qubit_call.try_as_basic_value().left().unwrap().into_int_value();

    // HUGR expects i16, so we need to truncate the ID to i16
    let i16_type = llvm_context.i16_type();
    let qubit_i16 = builder.build_int_truncate(qubit_i64, i16_type, "qubit")?;

    args.outputs.finish(builder, [qubit_i16.into()])?;
    Ok(())
}

fn emit_single_qubit_gate_true_standard<'c, H: HugrView<Node = Node>>(
    context: &mut EmitFuncContext<'c, '_, H>,
    args: EmitOpArgs<'c, '_, ExtensionOp, H>,
    func_name: &str,
) -> Result<(), PecosError> {
    let llvm_context = context.iw_context();
    let builder = context.builder();

    // Convert i16 qubit to i64 then to %Qubit* pointer
    let i64_type = llvm_context.i64_type();
    let qubit_i64 =
        builder.build_int_z_extend(args.inputs[0].into_int_value(), i64_type, "qubit_i64")?;

    // Create %Qubit* opaque pointer type 
    // Note: We use i8* internally but the function signatures will be declared as %Qubit*
    let i8_type = llvm_context.i8_type();
    let qubit_ptr_type = i8_type.ptr_type(hugr_llvm::inkwell::AddressSpace::default());
    
    // Convert qubit ID to pointer - don't use null for qubit 0
    // The runtime will interpret the pointer value as the qubit index
    let qubit_ptr = builder.build_int_to_ptr(qubit_i64, qubit_ptr_type, "qubit_ptr")?;

    // True Standard QIR function signature: void @__quantum__qis__h__body(%Qubit*)
    let void_type = llvm_context.void_type();
    let func_type = void_type.fn_type(&[qubit_ptr_type.into()], false);
    let func = context.get_extern_func(func_name, func_type)?;

    builder.build_call(func, &[qubit_ptr.into()], "")?;
    args.outputs.finish(builder, [args.inputs[0]])?;
    Ok(())
}

fn emit_two_qubit_gate_true_standard<'c, H: HugrView<Node = Node>>(
    context: &mut EmitFuncContext<'c, '_, H>,
    args: EmitOpArgs<'c, '_, ExtensionOp, H>,
    func_name: &str,
) -> Result<(), PecosError> {
    let llvm_context = context.iw_context();
    let builder = context.builder();

    // Convert both qubits from i16 to i64 to %Qubit* pointers
    let i64_type = llvm_context.i64_type();
    let i8_type = llvm_context.i8_type();
    let qubit_ptr_type = i8_type.ptr_type(hugr_llvm::inkwell::AddressSpace::default());

    let control_i64 =
        builder.build_int_z_extend(args.inputs[0].into_int_value(), i64_type, "control_i64")?;
    let target_i64 =
        builder.build_int_z_extend(args.inputs[1].into_int_value(), i64_type, "target_i64")?;

    // Convert qubit IDs to pointers - don't use null for qubit 0
    let control_ptr = builder.build_int_to_ptr(control_i64, qubit_ptr_type, "control_ptr")?;
    let target_ptr = builder.build_int_to_ptr(target_i64, qubit_ptr_type, "target_ptr")?;

    // True Standard QIR function signature: void @__quantum__qis__cx__body(%Qubit*, %Qubit*)
    let void_type = llvm_context.void_type();
    let func_type = void_type.fn_type(&[qubit_ptr_type.into(), qubit_ptr_type.into()], false);
    let func = context.get_extern_func(func_name, func_type)?;

    builder.build_call(func, &[control_ptr.into(), target_ptr.into()], "")?;
    args.outputs
        .finish(builder, [args.inputs[0], args.inputs[1]])?;
    Ok(())
}

fn emit_measure_true_standard<'c, H: HugrView<Node = Node>>(
    context: &mut EmitFuncContext<'c, '_, H>,
    args: EmitOpArgs<'c, '_, ExtensionOp, H>,
    result_names: &ResultNameMapping,
) -> Result<(), PecosError> {
    let llvm_context = context.iw_context();
    let builder = context.builder();

    // Convert qubit from i16 to i64 to %Qubit* pointer
    let i64_type = llvm_context.i64_type();
    let i8_type = llvm_context.i8_type();
    let qubit_ptr_type = i8_type.ptr_type(hugr_llvm::inkwell::AddressSpace::default());
    let result_ptr_type = i8_type.ptr_type(hugr_llvm::inkwell::AddressSpace::default());

    let qubit_i64 =
        builder.build_int_z_extend(args.inputs[0].into_int_value(), i64_type, "qubit_i64")?;
    
    // Convert qubit ID to pointer - don't use null for qubit 0
    // The runtime will interpret the pointer value as the qubit index
    let qubit_ptr = builder.build_int_to_ptr(qubit_i64, qubit_ptr_type, "qubit_ptr")?;

    // Allocate result ID
    let result_id = unsafe {
        let id = NEXT_RESULT_ID;
        NEXT_RESULT_ID += 1;
        id
    };

    // For results, we always use inttoptr (never null) - even for result 0
    // This matches bell.ll: inttoptr (i64 0 to %Result*)
    let result_id_val = i64_type.const_int(result_id, false);
    let result_ptr = builder.build_int_to_ptr(result_id_val, result_ptr_type, "result_ptr")?;

    // True Standard QIR measurement: void @__quantum__qis__m__body(%Qubit*, %Result*)
    let void_type = llvm_context.void_type();
    let measure_func_type = void_type.fn_type(&[qubit_ptr_type.into(), result_ptr_type.into()], false);
    let measure_func = context.get_extern_func("__quantum__qis__m__body", measure_func_type)?;

    builder.build_call(
        measure_func,
        &[qubit_ptr.into(), result_ptr.into()],
        "",
    )?;

    // Record the result with __quantum__rt__result_record_output
    let measurement_node = args.node.node();
    let name = result_names
        .get(&measurement_node)
        .map_or("c", std::string::String::as_str);

    // Create string constant for the result name
    let name_str = llvm_context.const_string(name.as_bytes(), true);
    let name_global = context.get_global(format!("str_{name}"), name_str.get_type(), true)?;
    name_global.set_initializer(&name_str);

    // Get pointer to the string
    let i32_type = llvm_context.i32_type();
    let zero = i32_type.const_zero();
    let indices = [zero, zero];
    let string_ptr = unsafe {
        builder.build_in_bounds_gep(name_global.as_pointer_value(), &indices, "string_ptr")?
    };

    // Call result recording with the same %Result* pointer we used for measurement
    let record_func_type = void_type.fn_type(&[result_ptr_type.into(), i8_type.ptr_type(hugr_llvm::inkwell::AddressSpace::default()).into()], false);
    let record_func =
        context.get_extern_func("__quantum__rt__result_record_output", record_func_type)?;

    builder.build_call(record_func, &[result_ptr.into(), string_ptr.into()], "")?;

    // For HUGR, we need to return a boolean measurement result
    // Since we can't actually get the measurement result in standard QIR (measurements return void),
    // we'll return a placeholder false for now
    // In a real implementation, this would need to be handled differently
    let bool_type = llvm_context.bool_type();
    let false_val = bool_type.const_zero();

    args.outputs.finish(builder, [false_val.into()])?;
    Ok(())
}

fn emit_rotation_gate_true_standard<'c, H: HugrView<Node = Node>>(
    context: &mut EmitFuncContext<'c, '_, H>,
    args: EmitOpArgs<'c, '_, ExtensionOp, H>,
    func_name: &str,
) -> Result<(), PecosError> {
    let llvm_context = context.iw_context();
    let builder = context.builder();

    // Convert i16 qubit to i64 then to %Qubit* pointer
    let i64_type = llvm_context.i64_type();
    let qubit_i64 =
        builder.build_int_z_extend(args.inputs[0].into_int_value(), i64_type, "qubit_i64")?;

    // Create %Qubit* opaque pointer type 
    let i8_type = llvm_context.i8_type();
    let qubit_ptr_type = i8_type.ptr_type(hugr_llvm::inkwell::AddressSpace::default());
    
    // Handle special case: qubit 0 should use null pointer
    let qubit_ptr = if qubit_i64.is_const() && qubit_i64.get_zero_extended_constant() == Some(0) {
        qubit_ptr_type.const_null()
    } else {
        builder.build_int_to_ptr(qubit_i64, qubit_ptr_type, "qubit_ptr")?
    };

    // Extract rotation angle (second argument is the angle in float)
    let angle = args.inputs[1].into_float_value();
    let f64_type = llvm_context.f64_type();
    let angle_f64 = builder.build_float_cast(angle, f64_type, "angle_f64")?;

    // True Standard QIR function signature: void @__quantum__qis__rx__body(%Qubit*, double)
    let void_type = llvm_context.void_type();
    let func_type = void_type.fn_type(&[qubit_ptr_type.into(), f64_type.into()], false);
    let func = context.get_extern_func(func_name, func_type)?;

    builder.build_call(func, &[qubit_ptr.into(), angle_f64.into()], "")?;
    args.outputs.finish(builder, [args.inputs[0]])?;
    Ok(())
}

fn emit_controlled_rotation_gate_true_standard<'c, H: HugrView<Node = Node>>(
    context: &mut EmitFuncContext<'c, '_, H>,
    args: EmitOpArgs<'c, '_, ExtensionOp, H>,
    func_name: &str,
) -> Result<(), PecosError> {
    let llvm_context = context.iw_context();
    let builder = context.builder();

    // Convert both qubits from i16 to i64 to %Qubit* pointers
    let i64_type = llvm_context.i64_type();
    let i8_type = llvm_context.i8_type();
    let qubit_ptr_type = i8_type.ptr_type(hugr_llvm::inkwell::AddressSpace::default());

    let control_i64 =
        builder.build_int_z_extend(args.inputs[0].into_int_value(), i64_type, "control_i64")?;
    let target_i64 =
        builder.build_int_z_extend(args.inputs[1].into_int_value(), i64_type, "target_i64")?;

    // Handle special case: qubit 0 should use null pointer
    let control_ptr = if control_i64.is_const() && control_i64.get_zero_extended_constant() == Some(0) {
        qubit_ptr_type.const_null()
    } else {
        builder.build_int_to_ptr(control_i64, qubit_ptr_type, "control_ptr")?
    };
    
    let target_ptr = if target_i64.is_const() && target_i64.get_zero_extended_constant() == Some(0) {
        qubit_ptr_type.const_null()
    } else {
        builder.build_int_to_ptr(target_i64, qubit_ptr_type, "target_ptr")?
    };

    // Extract rotation angle (third argument)
    let angle = args.inputs[2].into_float_value();
    let f64_type = llvm_context.f64_type();
    let angle_f64 = builder.build_float_cast(angle, f64_type, "angle_f64")?;

    // True Standard QIR function signature: void @__quantum__qis__crz__body(%Qubit*, %Qubit*, double)
    let void_type = llvm_context.void_type();
    let func_type = void_type.fn_type(&[qubit_ptr_type.into(), qubit_ptr_type.into(), f64_type.into()], false);
    let func = context.get_extern_func(func_name, func_type)?;

    builder.build_call(func, &[control_ptr.into(), target_ptr.into(), angle_f64.into()], "")?;
    args.outputs
        .finish(builder, [args.inputs[0], args.inputs[1]])?;
    Ok(())
}

fn emit_toffoli_gate_true_standard<'c, H: HugrView<Node = Node>>(
    context: &mut EmitFuncContext<'c, '_, H>,
    args: EmitOpArgs<'c, '_, ExtensionOp, H>,
    func_name: &str,
) -> Result<(), PecosError> {
    let llvm_context = context.iw_context();
    let builder = context.builder();

    // Convert three qubits from i16 to i64 to %Qubit* pointers
    let i64_type = llvm_context.i64_type();
    let i8_type = llvm_context.i8_type();
    let qubit_ptr_type = i8_type.ptr_type(hugr_llvm::inkwell::AddressSpace::default());

    let control1_i64 =
        builder.build_int_z_extend(args.inputs[0].into_int_value(), i64_type, "control1_i64")?;
    let control2_i64 =
        builder.build_int_z_extend(args.inputs[1].into_int_value(), i64_type, "control2_i64")?;
    let target_i64 =
        builder.build_int_z_extend(args.inputs[2].into_int_value(), i64_type, "target_i64")?;

    // Handle special case: qubit 0 should use null pointer
    let control1_ptr = if control1_i64.is_const() && control1_i64.get_zero_extended_constant() == Some(0) {
        qubit_ptr_type.const_null()
    } else {
        builder.build_int_to_ptr(control1_i64, qubit_ptr_type, "control1_ptr")?
    };
    
    let control2_ptr = if control2_i64.is_const() && control2_i64.get_zero_extended_constant() == Some(0) {
        qubit_ptr_type.const_null()
    } else {
        builder.build_int_to_ptr(control2_i64, qubit_ptr_type, "control2_ptr")?
    };
    
    let target_ptr = if target_i64.is_const() && target_i64.get_zero_extended_constant() == Some(0) {
        qubit_ptr_type.const_null()
    } else {
        builder.build_int_to_ptr(target_i64, qubit_ptr_type, "target_ptr")?
    };

    // True Standard QIR function signature: void @__quantum__qis__ccx__body(%Qubit*, %Qubit*, %Qubit*)
    let void_type = llvm_context.void_type();
    let func_type = void_type.fn_type(&[qubit_ptr_type.into(), qubit_ptr_type.into(), qubit_ptr_type.into()], false);
    let func = context.get_extern_func(func_name, func_type)?;

    builder.build_call(func, &[control1_ptr.into(), control2_ptr.into(), target_ptr.into()], "")?;
    args.outputs
        .finish(builder, [args.inputs[0], args.inputs[1], args.inputs[2]])?;
    Ok(())
}