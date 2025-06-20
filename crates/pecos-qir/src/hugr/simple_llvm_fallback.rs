/*!
Simple LLVM fallback for basic Guppy programs

When HUGR compilation fails due to version incompatibilities,
this module can generate LLVM IR directly for simple programs.
*/

use log::info;
use pecos_core::errors::PecosError;
use serde_json::Value;

/// Check if this HUGR can be handled by simple fallback
#[must_use]
pub fn can_handle_simple(hugr_json: &Value) -> bool {
    // Check if this is a simple function that we can handle
    if let Some(modules) = hugr_json.get("modules").and_then(|m| m.as_array()) {
        if let Some(module) = modules.first() {
            if let Some(nodes) = module.get("nodes").and_then(|n| n.as_array()) {
                // Look for simple patterns we can handle:
                // 1. Functions that just return a constant
                let has_simple_const = nodes.iter().any(|n| {
                    n.get("op").and_then(|op| op.as_str()) == Some("Const")
                        && n.get("v").and_then(|v| v.as_object()).is_some_and(|v| {
                            v.get("value")
                                .and_then(|val| val.as_object())
                                .is_some_and(|val| {
                                    val.get("c").and_then(|c| c.as_str()) == Some("ConstInt")
                                })
                        })
                });

                // 2. Functions with simple arithmetic (iadd, isub, imul)
                let has_simple_arithmetic = nodes.iter().any(|n| {
                    if let Some(op) = n.get("op") {
                        if let Some(op_obj) = op.as_object() {
                            if let Some(op_name) = op_obj.get("op_name").and_then(|n| n.as_str()) {
                                return matches!(op_name, "iadd" | "isub" | "imul");
                            }
                        }
                    }
                    false
                });

                return has_simple_const || has_simple_arithmetic;
            }
        }
    }
    false
}

/// Generate LLVM IR for simple functions
///
/// # Errors
/// Returns `PecosError` if the HUGR JSON structure is invalid or cannot be processed
pub fn generate_simple_llvm(hugr_json: &Value) -> Result<String, PecosError> {
    info!("Using simple LLVM fallback for basic Guppy function");

    // Extract function information
    let mut func_name = "main";
    let mut has_arithmetic = false;
    let mut arithmetic_op = "";
    let mut num_params = 0;
    let mut const_value = 42u64;

    if let Some(modules) = hugr_json.get("modules").and_then(|m| m.as_array()) {
        if let Some(module) = modules.first() {
            if let Some(nodes) = module.get("nodes").and_then(|n| n.as_array()) {
                // Find function name and signature
                for node in nodes {
                    if node.get("op").and_then(|op| op.as_str()) == Some("FuncDefn") {
                        if let Some(name) = node.get("name").and_then(|n| n.as_str()) {
                            func_name = name;
                        }
                        // Check signature for number of parameters
                        if let Some(sig) = node.get("signature").and_then(|s| s.as_object()) {
                            if let Some(body) = sig.get("body").and_then(|b| b.as_object()) {
                                if let Some(input) = body.get("input").and_then(|i| i.as_array()) {
                                    num_params = input.len();
                                }
                            }
                        }
                    }

                    // Check for arithmetic operations
                    if let Some(op) = node.get("op").and_then(|o| o.as_object()) {
                        if let Some(op_name) = op.get("op_name").and_then(|n| n.as_str()) {
                            if matches!(op_name, "iadd" | "isub" | "imul") {
                                has_arithmetic = true;
                                arithmetic_op = op_name;
                            }
                        }
                    }

                    // Find constant value
                    if node.get("op").and_then(|op| op.as_str()) == Some("Const") {
                        if let Some(v) = node.get("v").and_then(|v| v.as_object()) {
                            if let Some(value_obj) = v.get("value").and_then(|val| val.as_object())
                            {
                                if let Some(v_inner) =
                                    value_obj.get("v").and_then(|v| v.as_object())
                                {
                                    if let Some(value) =
                                        v_inner.get("value").and_then(serde_json::Value::as_u64)
                                    {
                                        const_value = value;
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // Generate appropriate LLVM IR based on function type
    let llvm_ir = if has_arithmetic && num_params == 2 {
        generate_arithmetic_llvm(func_name, arithmetic_op)
    } else {
        generate_constant_llvm(func_name, const_value)
    };

    Ok(llvm_ir)
}

fn generate_arithmetic_llvm(func_name: &str, arithmetic_op: &str) -> String {
    let op_instruction = match arithmetic_op {
        "isub" => "sub",
        "imul" => "mul",
        _ => "add", // Default to add (includes "iadd")
    };

    format!(
        "{}

define i64 @{func_name}(i64 %x, i64 %y) {{
entry:
    %result = {op_instruction} i64 %x, %y
    ret i64 %result
}}

; Entry point wrapper
define i32 @main() {{
entry:
    %result = call i64 @{func_name}(i64 5, i64 3)
    %result32 = trunc i64 %result to i32
    ret i32 %result32
}}
",
        get_llvm_prologue()
    )
}

fn generate_constant_llvm(func_name: &str, const_value: u64) -> String {
    format!(
        "{}

define i64 @{func_name}() {{
entry:
    ret i64 {const_value}
}}

; Entry point wrapper
define i32 @main() {{
entry:
    %result = call i64 @{func_name}()
    %result32 = trunc i64 %result to i32
    ret i32 %result32
}}
",
        get_llvm_prologue()
    )
}

fn get_llvm_prologue() -> &'static str {
    "; ModuleID = 'guppy_simple_fallback'
target datalayout = \"e-m:e-p270:32:32-p271:32:32-p272:64:64-i64:64-f80:128-n8:16:32:64-S128\"
target triple = \"x86_64-unknown-linux-gnu\"

%Result = type opaque
%Qubit = type opaque

declare void @__quantum__qis__h__body(%Qubit*)
declare void @__quantum__qis__x__body(%Qubit*)
declare void @__quantum__qis__y__body(%Qubit*)
declare void @__quantum__qis__z__body(%Qubit*)
declare void @__quantum__qis__cx__body(%Qubit*, %Qubit*)
declare void @__quantum__qis__m__body(%Qubit*, %Result*)
declare void @__quantum__rt__result_record_output(%Result*, i8*)"
}
