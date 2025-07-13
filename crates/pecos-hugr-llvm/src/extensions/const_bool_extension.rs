/*!
Extension to handle tket2.bool constants in HUGR-LLVM

This extension provides support for loading boolean constants from the tket2.bool extension.
*/

use hugr_core::{HugrView, Node};
use hugr_llvm::custom::{CodegenExtension, CodegenExtsBuilder};

/// Extension to handle tket2.bool constants
pub struct ConstBoolExtension;

impl Default for ConstBoolExtension {
    fn default() -> Self {
        Self::new()
    }
}

impl ConstBoolExtension {
    #[must_use]
    pub fn new() -> Self {
        Self
    }
}

// Define a simple ConstBool type that matches the tket2 structure
#[derive(Debug, Clone, PartialEq, Hash, serde::Serialize, serde::Deserialize)]
struct ConstBool(bool);

#[typetag::serde]
impl hugr_core::ops::constant::CustomConst for ConstBool {
    fn name(&self) -> hugr_core::ops::constant::ValueName {
        format!("ConstBool({})", self.0).into()
    }

    fn equal_consts(&self, other: &dyn hugr_core::ops::constant::CustomConst) -> bool {
        hugr_core::ops::constant::downcast_equal_consts(self, other)
    }

    fn get_type(&self) -> hugr_core::types::Type {
        // Create tket2.bool type
        use hugr_core::extension::ExtensionId;
        use hugr_core::types::{CustomType, TypeBound, TypeName};
        use std::sync::Weak;
        
        let ext_id = ExtensionId::new("tket2.bool").unwrap();
        let type_name: TypeName = "bool".into();
        let bool_type = CustomType::new(
            type_name,
            vec![],
            ext_id,
            TypeBound::Copyable,
            &Weak::new(), // Empty weak ref is fine for type creation
        );
        bool_type.into()
    }
}

impl CodegenExtension for ConstBoolExtension {
    fn add_extension<'a, H: HugrView<Node = Node> + 'a>(
        self,
        builder: CodegenExtsBuilder<'a, H>,
    ) -> CodegenExtsBuilder<'a, H>
    where
        Self: 'a,
    {
        // Register handler for ConstBool constants
        builder.custom_const::<ConstBool>(|context, konst| {
            // Create LLVM i1 constant with the boolean value
            let bool_type = context.iw_context().bool_type();
            let bool_value = bool_type.const_int(u64::from(konst.0), false);
            Ok(bool_value.into())
        })
    }
}