/// Derive macro for implementing Plugin traits
use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, DeriveInput, Expr, Lit, Meta};

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
        impl core::StructMetadata for #name {
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
