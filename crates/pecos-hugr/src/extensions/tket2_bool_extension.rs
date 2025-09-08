/*!
Extension to handle tket2.bool types and operations in HUGR-LLVM

This extension provides support for the tket2.bool opaque type used by Guppy 0.20.0.
It maps tket2.bool to LLVM i1 (boolean) type and implements operations like read.
*/

use anyhow;
use hugr_core::extension::ExtensionId;
use hugr_core::ops::ExtensionOp;
use hugr_core::types::TypeName;
use hugr_core::{HugrView, Node};
use hugr_llvm::custom::{CodegenExtension, CodegenExtsBuilder};
use hugr_llvm::emit::{EmitOpArgs, func::EmitFuncContext};
use pecos_core::errors::PecosError;

/// Extension to handle tket2.bool types
pub struct Tket2BoolExtension;

impl Default for Tket2BoolExtension {
    fn default() -> Self {
        Self::new()
    }
}

impl Tket2BoolExtension {
    #[must_use]
    pub fn new() -> Self {
        Self
    }
}

impl CodegenExtension for Tket2BoolExtension {
    fn add_extension<'a, H: HugrView<Node = Node> + 'a>(
        self,
        builder: CodegenExtsBuilder<'a, H>,
    ) -> CodegenExtsBuilder<'a, H>
    where
        Self: 'a,
    {
        let ext_id = ExtensionId::new("tket2.bool").unwrap();
        let bool_type_name = TypeName::new_inline("bool");

        // Register the bool type
        let builder = builder.custom_type((ext_id.clone(), bool_type_name), |ts, _hugr_type| {
            // Map tket2.bool to LLVM i1 (boolean)
            Ok(ts.iw_context().bool_type().into())
        });

        // Register the read operation
        builder
            .extension_op(ext_id.clone(), "read".into(), {
                move |ctx, args| emit_bool_read(ctx, args).map_err(anyhow::Error::new)
            })
            .extension_op(ext_id.clone(), "make_opaque".into(), {
                move |ctx, args| emit_bool_make_opaque(ctx, args).map_err(anyhow::Error::new)
            })
            .extension_op(ext_id.clone(), "not".into(), {
                move |ctx, args| emit_bool_not(ctx, args).map_err(anyhow::Error::new)
            })
            .extension_op(ext_id.clone(), "eq".into(), {
                move |ctx, args| emit_bool_eq(ctx, args).map_err(anyhow::Error::new)
            })
    }
}

/// Emit the tket2.bool.read operation
/// Converts from opaque bool (i1) to unit sum (i1 for now to maintain compatibility)
fn emit_bool_read<'c, H: HugrView<Node = Node>>(
    context: &mut EmitFuncContext<'c, '_, H>,
    args: EmitOpArgs<'c, '_, ExtensionOp, H>,
) -> Result<(), PecosError> {
    let builder = context.builder();

    // Input is i1 (opaque bool)
    let bool_value = args.inputs[0].into_int_value();

    // For now, keep as i1 to avoid type mismatches in conditionals
    // This is a workaround until the HUGR type system is updated
    args.outputs.finish(builder, [bool_value.into()])?;
    Ok(())
}

/// Emit the `tket2.bool.make_opaque` operation
/// Converts from unit sum to opaque bool (both i1 for now)
fn emit_bool_make_opaque<'c, H: HugrView<Node = Node>>(
    context: &mut EmitFuncContext<'c, '_, H>,
    args: EmitOpArgs<'c, '_, ExtensionOp, H>,
) -> Result<(), PecosError> {
    let builder = context.builder();

    // Input should already be i1 with our workaround
    let bool_value = args.inputs[0].into_int_value();

    // Pass through as-is
    args.outputs.finish(builder, [bool_value.into()])?;
    Ok(())
}

/// Emit the tket2.bool.not operation
/// Logical NOT operation on a boolean value
fn emit_bool_not<'c, H: HugrView<Node = Node>>(
    context: &mut EmitFuncContext<'c, '_, H>,
    args: EmitOpArgs<'c, '_, ExtensionOp, H>,
) -> Result<(), PecosError> {
    let builder = context.builder();

    // Input is i1 (boolean)
    let bool_value = args.inputs[0].into_int_value();

    // Build NOT operation (XOR with true)
    let true_val = context.iw_context().bool_type().const_int(1, false);
    let not_value = builder.build_xor(bool_value, true_val, "bool_not")?;

    args.outputs.finish(builder, [not_value.into()])?;
    Ok(())
}

/// Emit the tket2.bool.eq operation
/// Equality comparison between two boolean values
fn emit_bool_eq<'c, H: HugrView<Node = Node>>(
    context: &mut EmitFuncContext<'c, '_, H>,
    args: EmitOpArgs<'c, '_, ExtensionOp, H>,
) -> Result<(), PecosError> {
    let builder = context.builder();

    // Inputs are two i1 (boolean) values
    let bool1 = args.inputs[0].into_int_value();
    let bool2 = args.inputs[1].into_int_value();

    // Build equality comparison (XNOR - equivalent values)
    // a == b is equivalent to NOT(a XOR b)
    let xor_value = builder.build_xor(bool1, bool2, "bool_xor")?;
    let true_val = context.iw_context().bool_type().const_int(1, false);
    let eq_value = builder.build_xor(xor_value, true_val, "bool_eq")?;

    args.outputs.finish(builder, [eq_value.into()])?;
    Ok(())
}
