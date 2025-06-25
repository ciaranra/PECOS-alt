/*!
Extension to handle tket2.bool types in HUGR-LLVM

This extension provides support for the tket2.bool opaque type used by Guppy 0.20.0.
It maps tket2.bool to LLVM i1 (boolean) type.
*/

use hugr_core::extension::ExtensionId;
use hugr_core::types::TypeName;
use hugr_core::{HugrView, Node};
use hugr_llvm::custom::{CodegenExtension, CodegenExtsBuilder};

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
        builder.custom_type((ext_id.clone(), bool_type_name), |ts, _hugr_type| {
            // Map tket2.bool to LLVM i1 (boolean)
            Ok(ts.iw_context().bool_type().into())
        })
    }
}
