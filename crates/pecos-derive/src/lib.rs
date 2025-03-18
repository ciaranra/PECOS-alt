/// Derive macro for implementing Plugin traits
///
/// This macro automatically implements the `StructMetadata` trait for a struct,
/// extracting documentation comments to use as the description.
///
/// # Example
///
/// ```
/// // Mock the StructMetadata trait to make the example self-contained
/// extern crate pecos_derive;
///
/// // Define a mock StructMetadata trait that matches the real one
/// pub trait StructMetadata {
///     fn name(&self) -> &str;
///     fn description(&self) -> &str;
/// }
///
/// // Make it available in the expected namespace
/// mod pecos_core {
///     pub use super::StructMetadata;
/// }
///
/// use pecos_derive::StructMetadata;
///
/// /// My custom struct with metadata
/// #[derive(StructMetadata)]
/// struct MyStruct;
///
/// // Test that it works
/// impl MyStruct {
///     fn new() -> Self { MyStruct }
/// }
///
/// fn main() {
///     let my_struct = MyStruct::new();
///     // These would call the trait methods if we could actually run the test
///     // println!("Name: {}", my_struct.name());
///     // println!("Description: {}", my_struct.description());
/// }
/// ```
use proc_macro::TokenStream;
use quote::quote;
use syn::{DeriveInput, Expr, Lit, Meta, parse_macro_input};

#[proc_macro_derive(StructMetadata)]
pub fn derive_metadata(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;

    // Extract doc comment if it exists
    let doc = input
        .attrs
        .iter()
        .filter(|attr| attr.path().is_ident("doc"))
        .find_map(|attr| {
            if let Meta::NameValue(meta) = &attr.meta {
                if let Expr::Lit(expr_lit) = &meta.value {
                    if let Lit::Str(lit) = &expr_lit.lit {
                        return Some(lit.value());
                    }
                }
            }
            None
        })
        .unwrap_or_else(|| format!("A {} struct", name));

    let expanded = quote! {
        impl pecos_core::StructMetadata for #name {
            fn name(&self) -> &str {
                stringify!(#name)
            }

            fn description(&self) -> &str {
                #doc
            }
        }
    };

    TokenStream::from(expanded)
}

/// Attribute macro for dynamic plugin libraries
///
/// This macro generates the necessary boilerplate code for registering
/// a plugin with the plugin registry.
///
/// # Example
///
/// ```text
/// // This is a syntax example, not actual code
///
/// #[plugin_library]
/// #[derive(Debug, Clone)]
/// struct MyCustomPlugin {
///     // Plugin implementation
/// }
///
/// // The macro would generate code similar to:
/// #[no_mangle]
/// pub extern "C" fn register_plugin(registry: &mut plugin_system::PluginRegistry) -> Result<(), Box<dyn std::error::Error>> {
///     registry.register_rust_coprocessor(Box::new(MyCustomPlugin::new()))?;
///     Ok(())
/// }
/// ```
#[proc_macro_attribute]
pub fn plugin_library(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as DeriveInput);
    let name = &input.ident;

    let expanded = quote! {
        #[no_mangle]
        pub extern "C" fn register_plugin(registry: &mut plugin_system::PluginRegistry) -> Result<(), Box<dyn std::error::Error>> {
            registry.register_rust_coprocessor(Box::new(#name::new()))?;
            Ok(())
        }
    };

    TokenStream::from(expanded)
}

/*
Example usage:

// For regular python-plugins:
/// My plugin that does something cool
#[derive(Debug, Clone, Plugin)]
pub struct MyPlugin;

// For dynamic libraries:
use plugin_system_macros::plugin_library;
use plugin_system::prelude::*;

#[plugin_library]
#[derive(Debug, Clone)]
pub struct MyCustomPlugin {
    // ... plugin implementation
}
*/
