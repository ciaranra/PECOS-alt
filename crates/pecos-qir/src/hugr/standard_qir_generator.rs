/*!
Standard QIR Generator for HUGR

This module generates standard QIR format that is compatible with the existing
PECOS QirEngine infrastructure, rather than custom LLVM IR.

The output format matches examples/qir/bell.ll and works with QirEngine::new().
*/

use pecos_core::errors::PecosError;
use hugr_core::{HugrView, Node};
use hugr_core::ops::ExtensionOp;
use anyhow;
use hugr_llvm::custom::{CodegenExtension, CodegenExtsBuilder};
use hugr_llvm::emit::{EmitOpArgs, func::EmitFuncContext};
use hugr_llvm::inkwell::types::PointerType;
use hugr_llvm::inkwell::values::PointerValue;
use std::collections::HashMap;

/// Result name mapping for measurement outputs
pub type ResultNameMapping = HashMap<Node, String>;

/// Standard QIR quantum operation extension
/// 
/// This generates standard QIR format using opaque %Qubit* and %Result* types
/// that work with the existing PECOS QirEngine.
pub struct StandardQirExtension {
    result_names: ResultNameMapping,
}

impl StandardQirExtension {
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
                move |ctx, args| emit_qalloc_standard(ctx, args).map_err(|e| anyhow::Error::new(e))
            })
            .extension_op(ext_id.clone(), "H".into(), {
                move |ctx, args| emit_single_qubit_gate_standard(ctx, args, "__quantum__qis__h__body").map_err(|e| anyhow::Error::new(e))
            })
            .extension_op(ext_id.clone(), "CX".into(), {
                move |ctx, args| emit_two_qubit_gate_standard(ctx, args, "__quantum__qis__cx__body").map_err(|e| anyhow::Error::new(e))
            })
            .extension_op(ext_id.clone(), "MeasureFree".into(), {
                let names = result_names.clone();
                move |ctx, args| emit_measure_standard(ctx, args, &names).map_err(|e| anyhow::Error::new(e))
            })
            .extension_op(ext_id.clone(), "X".into(), {
                move |ctx, args| emit_single_qubit_gate_standard(ctx, args, "__quantum__qis__x__body").map_err(|e| anyhow::Error::new(e))
            })
            .extension_op(ext_id.clone(), "Y".into(), {
                move |ctx, args| emit_single_qubit_gate_standard(ctx, args, "__quantum__qis__y__body").map_err(|e| anyhow::Error::new(e))
            })
            .extension_op(ext_id.clone(), "Z".into(), {
                move |ctx, args| emit_single_qubit_gate_standard(ctx, args, "__quantum__qis__z__body").map_err(|e| anyhow::Error::new(e))
            })
    }
}

/// Get or create the opaque %Qubit type
fn get_qubit_type<'c>(context: &'c hugr_llvm::inkwell::context::Context) -> PointerType<'c> {
    // Create opaque struct type for Qubit
    let qubit_struct = context.opaque_struct_type("Qubit");
    qubit_struct.ptr_type(hugr_llvm::inkwell::AddressSpace::default())
}

/// Get or create the opaque %Result type  
fn get_result_type<'c>(context: &'c hugr_llvm::inkwell::context::Context) -> PointerType<'c> {
    // Create opaque struct type for Result
    let result_struct = context.opaque_struct_type("Result");
    result_struct.ptr_type(hugr_llvm::inkwell::AddressSpace::default())
}

/// Convert integer qubit ID to %Qubit* pointer (standard QIR uses inttoptr)
fn int_to_qubit_ptr<'c>(
    llvm_context: &'c hugr_llvm::inkwell::context::Context,
    builder: &hugr_llvm::inkwell::builder::Builder<'c>,
    qubit_id: hugr_llvm::inkwell::values::IntValue<'c>,
) -> Result<PointerValue<'c>, PecosError> {
    let qubit_type = get_qubit_type(llvm_context);
    
    // Convert integer to pointer: inttoptr i64 %id to %Qubit*
    let qubit_ptr = builder.build_int_to_ptr(qubit_id, qubit_type, "qubit_ptr")?;
    Ok(qubit_ptr)
}

/// Convert integer result ID to %Result* pointer
fn int_to_result_ptr<'c>(
    llvm_context: &'c hugr_llvm::inkwell::context::Context,
    builder: &hugr_llvm::inkwell::builder::Builder<'c>,
    result_id: hugr_llvm::inkwell::values::IntValue<'c>,
) -> Result<PointerValue<'c>, PecosError> {
    let result_type = get_result_type(llvm_context);
    
    // Convert integer to pointer: inttoptr i64 %id to %Result*
    let result_ptr = builder.build_int_to_ptr(result_id, result_type, "result_ptr")?;
    Ok(result_ptr)
}

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
    static mut NEXT_QUBIT_ID: i64 = 0;
    
    let qubit_id = unsafe {
        let id = NEXT_QUBIT_ID;
        NEXT_QUBIT_ID += 1;
        id
    };
    
    let i64_type = llvm_context.i64_type();
    let qubit_id_val = i64_type.const_int(qubit_id as u64, false);
    
    // Convert to %Qubit* pointer (not actually used in this function)
    let _qubit_ptr = int_to_qubit_ptr(llvm_context, builder, qubit_id_val)?;
    
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
    
    // Convert i16 qubit to i64 then to %Qubit*
    let i64_type = llvm_context.i64_type();
    let qubit_i64 = builder.build_int_z_extend(
        args.inputs[0].into_int_value(),
        i64_type,
        "qubit_i64"
    )?;
    
    let qubit_ptr = int_to_qubit_ptr(llvm_context, builder, qubit_i64)?;
    
    // Standard QIR function signature: void @__quantum__qis__h__body(%Qubit*)
    let void_type = llvm_context.void_type();
    let qubit_type = get_qubit_type(llvm_context);
    let func_type = void_type.fn_type(&[qubit_type.into()], false);
    let func = context.get_extern_func(func_name, func_type)?;
    
    builder.build_call(func, &[qubit_ptr.into()], "")?;
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
    
    // Convert both qubits from i16 to %Qubit*
    let i64_type = llvm_context.i64_type();
    
    let control_i64 = builder.build_int_z_extend(
        args.inputs[0].into_int_value(),
        i64_type,
        "control_i64"
    )?;
    let target_i64 = builder.build_int_z_extend(
        args.inputs[1].into_int_value(),
        i64_type,
        "target_i64"
    )?;
    
    let control_ptr = int_to_qubit_ptr(llvm_context, builder, control_i64)?;
    let target_ptr = int_to_qubit_ptr(llvm_context, builder, target_i64)?;
    
    // Standard QIR function signature: void @__quantum__qis__cx__body(%Qubit*, %Qubit*)
    let void_type = llvm_context.void_type();
    let qubit_type = get_qubit_type(llvm_context);
    let func_type = void_type.fn_type(&[qubit_type.into(), qubit_type.into()], false);
    let func = context.get_extern_func(func_name, func_type)?;
    
    builder.build_call(func, &[control_ptr.into(), target_ptr.into()], "")?;
    args.outputs.finish(builder, [args.inputs[0], args.inputs[1]])?;
    Ok(())
}

fn emit_measure_standard<'c, H: HugrView<Node = Node>>(
    context: &mut EmitFuncContext<'c, '_, H>,
    args: EmitOpArgs<'c, '_, ExtensionOp, H>,
    result_names: &ResultNameMapping,
) -> Result<(), PecosError> {
    let llvm_context = context.iw_context();
    let builder = context.builder();
    
    // Convert qubit from i16 to %Qubit*
    let i64_type = llvm_context.i64_type();
    let qubit_i64 = builder.build_int_z_extend(
        args.inputs[0].into_int_value(),
        i64_type,
        "qubit_i64"
    )?;
    let qubit_ptr = int_to_qubit_ptr(llvm_context, builder, qubit_i64)?;
    
    // Allocate result ID (for now, use static allocation like qubits)
    static mut NEXT_RESULT_ID: i64 = 0;
    let result_id = unsafe {
        let id = NEXT_RESULT_ID;
        NEXT_RESULT_ID += 1;
        id
    };
    
    let result_id_val = i64_type.const_int(result_id as u64, false);
    let result_ptr = int_to_result_ptr(llvm_context, builder, result_id_val)?;
    
    // Standard QIR measurement: void @__quantum__qis__m__body(%Qubit*, %Result*)
    let void_type = llvm_context.void_type();
    let qubit_type = get_qubit_type(llvm_context);
    let result_type = get_result_type(llvm_context);
    let measure_func_type = void_type.fn_type(&[qubit_type.into(), result_type.into()], false);
    let measure_func = context.get_extern_func("__quantum__qis__m__body", measure_func_type)?;
    
    builder.build_call(measure_func, &[qubit_ptr.into(), result_ptr.into()], "")?;
    
    // Record the result with a name
    let measurement_node = args.node.node();
    let name = result_names.get(&measurement_node)
        .map(|s| s.as_str())
        .unwrap_or("c");
    
    let name_str = llvm_context.const_string(name.as_bytes(), true);
    let name_global = context.get_global(&format!("str_{}", name), name_str.get_type(), true)?;
    name_global.set_initializer(&name_str);
    
    // Get pointer to the string
    let i32_type = llvm_context.i32_type();
    let zero = i32_type.const_zero();
    let indices = [zero, zero];
    let name_ptr = unsafe {
        builder.build_in_bounds_gep(name_global.as_pointer_value(), &indices, "name_ptr")?
    };
    
    // Standard QIR result recording: void @__quantum__rt__result_record_output(%Result*, i8*)
    let i8_type = llvm_context.i8_type();
    let i8_ptr_type = i8_type.ptr_type(hugr_llvm::inkwell::AddressSpace::default());
    let record_func_type = void_type.fn_type(&[result_type.into(), i8_ptr_type.into()], false);
    let record_func = context.get_extern_func("__quantum__rt__result_record_output", record_func_type)?;
    
    builder.build_call(record_func, &[result_ptr.into(), name_ptr.into()], "")?;
    
    // For HUGR, we need to return a boolean result
    // In standard QIR, the measurement result is recorded, not returned
    // But HUGR expects a boolean return value
    // 
    // For now, return a placeholder boolean (this might need extension)
    let bool_type = llvm_context.bool_type();
    let placeholder_result = bool_type.const_zero(); // false
    
    args.outputs.finish(builder, [placeholder_result.into()])?;
    Ok(())
}