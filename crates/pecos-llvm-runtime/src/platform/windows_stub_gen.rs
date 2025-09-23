//! Utility to generate Windows stub files dynamically

/// Information about an exported function
#[derive(Debug, Clone)]
pub struct ExportedFunction {
    pub name: &'static str,
    pub return_type: &'static str,
    pub params: &'static [(&'static str, &'static str)], // (type, name)
}

impl ExportedFunction {
    /// Generate C stub implementation
    fn generate_c_stub(&self) -> String {
        let params_str = if self.params.is_empty() {
            "void".to_string()
        } else {
            self.params
                .iter()
                .map(|(param_type, name)| format!("{param_type} {name}"))
                .collect::<Vec<_>>()
                .join(", ")
        };

        let body = match self.return_type {
            "int" | "usize" | "u32" | "i32" | "i1" => "{ return 0; }",
            "u64" => "{ return 0ULL; }",
            "i64" => "{ return 0LL; }",
            "void*" | "const unsigned char*" | "i8*" => "{ return &empty_commands; }",
            "void" if self.name == "panic" => "{ exit(0); }",
            _ => "{}",
        };

        format!(
            "__declspec(dllexport) {} {}({}) {}",
            self.return_type, self.name, params_str, body
        )
    }

    /// Generate DEF file entry
    fn generate_def_entry(&self) -> String {
        // Special case for main function
        if self.name == "main" {
            format!(
                "    {} @1 NONAME ; Export main function from LLVM IR program",
                self.name
            )
        } else {
            format!("    {}", self.name)
        }
    }
}

/// Get the list of exported functions
/// This list must be kept in sync with runtime.rs
pub const EXPORTED_FUNCTIONS: &[ExportedFunction] = &[
    // QIS Setup/Teardown
    ExportedFunction {
        name: "setup",
        return_type: "void",
        params: &[("i64", "seed")],
    },
    ExportedFunction {
        name: "teardown",
        return_type: "i64",
        params: &[],
    },
    // QIS Memory Management
    ExportedFunction {
        name: "___qalloc",
        return_type: "i64",
        params: &[],
    },
    ExportedFunction {
        name: "___qfree",
        return_type: "void",
        params: &[("i64", "qubit")],
    },
    ExportedFunction {
        name: "___reset",
        return_type: "void",
        params: &[("i64", "qubit")],
    },
    // QIS Measurement Functions
    ExportedFunction {
        name: "___measure",
        return_type: "i1",
        params: &[("i64", "qubit")],
    },
    ExportedFunction {
        name: "___lazy_measure",
        return_type: "i64",
        params: &[("i64", "qubit")],
    },
    ExportedFunction {
        name: "___lazy_measure_leaked",
        return_type: "i64",
        params: &[("i64", "qubit")],
    },
    ExportedFunction {
        name: "___lazy_measure_reset",
        return_type: "i64",
        params: &[("i64", "qubit")],
    },
    // QIS Gate Functions
    ExportedFunction {
        name: "___rxy",
        return_type: "void",
        params: &[("i64", "qubit"), ("double", "theta"), ("double", "phi")],
    },
    ExportedFunction {
        name: "___rz",
        return_type: "void",
        params: &[("i64", "qubit"), ("double", "theta")],
    },
    ExportedFunction {
        name: "___rzz",
        return_type: "void",
        params: &[("i64", "qubit1"), ("i64", "qubit2"), ("double", "theta")],
    },
    // QIS Future Reference Management
    ExportedFunction {
        name: "___inc_future_refcount",
        return_type: "void",
        params: &[("i64", "reference")],
    },
    ExportedFunction {
        name: "___dec_future_refcount",
        return_type: "void",
        params: &[("i64", "reference")],
    },
    ExportedFunction {
        name: "___read_future_bool",
        return_type: "i1",
        params: &[("i64", "reference")],
    },
    ExportedFunction {
        name: "___read_future_uint",
        return_type: "u64",
        params: &[("i64", "reference")],
    },
    // QIS Error Handling
    ExportedFunction {
        name: "panic",
        return_type: "void",
        params: &[("i32", "code"), ("i8*", "message")],
    },
    // QIR runtime API (kept for internal use)
    ExportedFunction {
        name: "llvm_runtime_reset",
        return_type: "void",
        params: &[],
    },
    ExportedFunction {
        name: "llvm_runtime_get_binary_commands",
        return_type: "void*",
        params: &[],
    },
    ExportedFunction {
        name: "llvm_runtime_free_binary_commands",
        return_type: "void",
        params: &[("void*", "cmds")],
    },
    ExportedFunction {
        name: "llvm_runtime_update_measurement_results",
        return_type: "void",
        params: &[("const u32*", "results_ptr"), ("usize", "results_len")],
    },
    ExportedFunction {
        name: "llvm_runtime_finalize_shot",
        return_type: "void",
        params: &[],
    },
    ExportedFunction {
        name: "llvm_runtime_get_shot_results",
        return_type: "void*",
        params: &[],
    },
    ExportedFunction {
        name: "llvm_runtime_free_shot_data",
        return_type: "void",
        params: &[("void*", "data")],
    },
    ExportedFunction {
        name: "llvm_runtime_get_measurement_result_ids",
        return_type: "void*",
        params: &[],
    },
    ExportedFunction {
        name: "llvm_runtime_free_result_ids",
        return_type: "void",
        params: &[("void*", "data")],
    },
    ExportedFunction {
        name: "llvm_runtime_get_measurements_executed",
        return_type: "usize",
        params: &[],
    },
    // Quantum instruction set
    ExportedFunction {
        name: "__quantum__qis__s__body",
        return_type: "void",
        params: &[("i64", "qubit")],
    },
    ExportedFunction {
        name: "__quantum__qis__sdg__body",
        return_type: "void",
        params: &[("i64", "qubit")],
    },
    ExportedFunction {
        name: "__quantum__qis__t__body",
        return_type: "void",
        params: &[("i64", "qubit")],
    },
    ExportedFunction {
        name: "__quantum__qis__tdg__body",
        return_type: "void",
        params: &[("i64", "qubit")],
    },
    ExportedFunction {
        name: "__quantum__qis__reset__body",
        return_type: "void",
        params: &[("i64", "qubit")],
    },
    ExportedFunction {
        name: "__quantum__rt__qubit_release",
        return_type: "void",
        params: &[("i64", "qubit")],
    },
    ExportedFunction {
        name: "__quantum__qis__rx__body",
        return_type: "void",
        params: &[("double", "theta"), ("i64", "qubit")],
    },
    ExportedFunction {
        name: "__quantum__qis__ry__body",
        return_type: "void",
        params: &[("double", "theta"), ("i64", "qubit")],
    },
    ExportedFunction {
        name: "__quantum__qis__cz__body",
        return_type: "void",
        params: &[("i64", "control"), ("i64", "target")],
    },
    ExportedFunction {
        name: "__quantum__qis__cy__body",
        return_type: "void",
        params: &[("i64", "control"), ("i64", "target")],
    },
    ExportedFunction {
        name: "__quantum__qis__ch__body",
        return_type: "void",
        params: &[("i64", "control"), ("i64", "target")],
    },
    ExportedFunction {
        name: "__quantum__qis__crz__body",
        return_type: "void",
        params: &[("double", "theta"), ("i64", "control"), ("i64", "target")],
    },
    ExportedFunction {
        name: "__quantum__qis__ccx__body",
        return_type: "void",
        params: &[("i64", "control1"), ("i64", "control2"), ("i64", "target")],
    },
    ExportedFunction {
        name: "__quantum__qis__zz__body",
        return_type: "void",
        params: &[("i64", "qubit1"), ("i64", "qubit2")],
    },
    // Runtime management
    ExportedFunction {
        name: "__quantum__rt__result_allocate",
        return_type: "i64",
        params: &[],
    },
    ExportedFunction {
        name: "__quantum__rt__qubit_allocate",
        return_type: "i64",
        params: &[],
    },
    ExportedFunction {
        name: "__quantum__rt__message",
        return_type: "void",
        params: &[("const char*", "msg")],
    },
    ExportedFunction {
        name: "__quantum__rt__record",
        return_type: "void",
        params: &[("const char*", "data")],
    },
    ExportedFunction {
        name: "__quantum__rt__result_get_one",
        return_type: "int32_t",
        params: &[("i64", "result")],
    },
    // Main function (exported from QIR program, not runtime)
    ExportedFunction {
        name: "main",
        return_type: "void",
        params: &[],
    },
    // Quantum gate functions
    ExportedFunction {
        name: "__quantum__qis__h__body",
        return_type: "void",
        params: &[("i64", "qubit")],
    },
    ExportedFunction {
        name: "__quantum__qis__x__body",
        return_type: "void",
        params: &[("i64", "qubit")],
    },
    ExportedFunction {
        name: "__quantum__qis__y__body",
        return_type: "void",
        params: &[("i64", "qubit")],
    },
    ExportedFunction {
        name: "__quantum__qis__z__body",
        return_type: "void",
        params: &[("i64", "qubit")],
    },
    ExportedFunction {
        name: "__quantum__qis__cx__body",
        return_type: "void",
        params: &[("i64", "control"), ("i64", "target")],
    },
    ExportedFunction {
        name: "__quantum__qis__cnot__body",
        return_type: "void",
        params: &[("i64", "control"), ("i64", "target")],
    },
    ExportedFunction {
        name: "__quantum__qis__rz__body",
        return_type: "void",
        params: &[("double", "theta"), ("i64", "qubit")],
    },
    ExportedFunction {
        name: "__quantum__qis__r1xy__body",
        return_type: "void",
        params: &[("double", "theta"), ("double", "phi"), ("i64", "qubit")],
    },
    ExportedFunction {
        name: "__quantum__qis__m__body",
        return_type: "i32",
        params: &[("i64", "qubit"), ("i64", "result")],
    },
    ExportedFunction {
        name: "__quantum__rt__result_record_output",
        return_type: "void",
        params: &[("i64", "result"), ("i8*", "name")],
    },
];

/// Generate Windows DEF file content
pub fn generate_def_file() -> String {
    let exports: Vec<String> = EXPORTED_FUNCTIONS
        .iter()
        .map(ExportedFunction::generate_def_entry)
        .collect();

    format!("EXPORTS\n{}\n", exports.join("\n"))
}

/// Generate Windows C stub content
pub fn generate_c_stub() -> String {
    // Filter out main (it's defined in the QIR program)
    let stub_functions: Vec<String> = EXPORTED_FUNCTIONS
        .iter()
        .filter(|f| f.name != "main")
        .map(ExportedFunction::generate_c_stub)
        .collect();

    format!(
        r"#include <stdlib.h>
#include <stdint.h>
#include <stdbool.h>

// Define type aliases
typedef uint32_t u32;
typedef uint64_t u64;
typedef size_t usize;
typedef int32_t i32;
typedef int64_t i64;
typedef unsigned char i1;
typedef char i8;

// Define a minimal binary command structure
typedef struct {{
    int command_count;
    unsigned char* data;
    size_t data_size;
}} BinaryCommands;

// Static data for commands - empty but valid
static unsigned char empty_data[] = {{0}};
static BinaryCommands empty_commands = {{0, empty_data, 1}};

// Required Windows DLL entry point
__declspec(dllexport) int _DllMainCRTStartup(void* hinst, unsigned long reason, void* reserved) {{
    return 1;
}}

// QIR runtime stubs
{}
",
        stub_functions.join("\n")
    )
}
