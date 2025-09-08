/*!
Extension to handle tket.rotation types in HUGR-LLVM

This extension provides support for the tket.rotation opaque type used by newer Guppy versions.
It maps rotation angles to LLVM f64 (double) type and handles rotation operations.
*/

use anyhow::Result;
use hugr_core::extension::ExtensionId;
use hugr_core::ops::ExtensionOp;
use hugr_core::types::TypeName;
use hugr_core::{HugrView, Node};
use hugr_llvm::custom::{CodegenExtension, CodegenExtsBuilder};
use hugr_llvm::emit::{EmitOpArgs, func::EmitFuncContext};

/// Extension to handle tket.rotation types (newer Guppy versions)
#[derive(Clone)]
pub struct TketRotationExtension;

impl Default for TketRotationExtension {
    fn default() -> Self {
        Self::new()
    }
}

impl TketRotationExtension {
    #[must_use]
    pub fn new() -> Self {
        Self
    }

    fn emit_from_halfturns_unchecked<'c, H: HugrView<Node = Node>>(
        context: &mut EmitFuncContext<'c, '_, H>,
        args: EmitOpArgs<'c, '_, ExtensionOp, H>,
    ) -> Result<()> {
        // Simple passthrough - the input float IS the rotation value
        let [input] = args
            .inputs
            .try_into()
            .map_err(|_| anyhow::anyhow!("from_halfturns_unchecked expects 1 input"))?;
        args.outputs.finish(context.builder(), vec![input])
    }

    fn emit_to_halfturns<'c, H: HugrView<Node = Node>>(
        context: &mut EmitFuncContext<'c, '_, H>,
        args: EmitOpArgs<'c, '_, ExtensionOp, H>,
    ) -> Result<()> {
        // Simple passthrough - the rotation value IS a float
        let [input] = args
            .inputs
            .try_into()
            .map_err(|_| anyhow::anyhow!("to_halfturns expects 1 input"))?;
        args.outputs.finish(context.builder(), vec![input])
    }
}

impl CodegenExtension for TketRotationExtension {
    fn add_extension<'a, H: HugrView<Node = Node> + 'a>(
        self,
        builder: CodegenExtsBuilder<'a, H>,
    ) -> CodegenExtsBuilder<'a, H>
    where
        Self: 'a,
    {
        // Use tket.rotation instead of tket2.rotation for newer Guppy
        let ext_id = ExtensionId::new("tket.rotation").unwrap();
        let rotation_type_name = TypeName::new_inline("rotation");

        // Register the rotation type
        builder
            .custom_type((ext_id.clone(), rotation_type_name), |ts, _hugr_type| {
                // Map tket.rotation to LLVM f64 (double precision float)
                // This represents rotation angles in halfturns
                Ok(ts.iw_context().f64_type().into())
            })
            // Register rotation operations
            .extension_op(ext_id.clone(), "from_halfturns_unchecked".into(), {
                |context, args| Self::emit_from_halfturns_unchecked(context, args)
            })
            .extension_op(ext_id.clone(), "to_halfturns".into(), {
                |context, args| Self::emit_to_halfturns(context, args)
            })
    }
}
