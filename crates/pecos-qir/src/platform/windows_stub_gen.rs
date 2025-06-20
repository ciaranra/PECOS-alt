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
            "i64" => "{ return 0LL; }",
            "void*" => "{ return &empty_commands; }",
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
                "    {} @1 NONAME ; Export main function from QIR program",
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
    // QIR runtime API
    ExportedFunction {
        name: "qir_runtime_reset",
        return_type: "void",
        params: &[],
    },
    ExportedFunction {
        name: "qir_runtime_get_binary_commands",
        return_type: "void*",
        params: &[],
    },
    ExportedFunction {
        name: "qir_runtime_free_binary_commands",
        return_type: "void",
        params: &[("void*", "cmds")],
    },
    ExportedFunction {
        name: "qir_runtime_update_measurement_results",
        return_type: "void",
        params: &[("const u32*", "results_ptr"), ("usize", "results_len")],
    },
    ExportedFunction {
        name: "qir_runtime_finalize_shot",
        return_type: "void",
        params: &[],
    },
    ExportedFunction {
        name: "qir_runtime_get_shot_results",
        return_type: "void*",
        params: &[],
    },
    ExportedFunction {
        name: "qir_runtime_free_shot_data",
        return_type: "void",
        params: &[("void*", "data")],
    },
    // Quantum instruction set
    ExportedFunction {
        name: "__quantum__qis__rz__body",
        return_type: "void",
        params: &[("double", "theta"), ("int", "qubit")],
    },
    ExportedFunction {
        name: "__quantum__qis__r1xy__body",
        return_type: "void",
        params: &[("double", "theta"), ("double", "phi"), ("int", "qubit")],
    },
    ExportedFunction {
        name: "__quantum__qis__rxy__body",
        return_type: "void",
        params: &[("double", "theta"), ("double", "phi"), ("int", "qubit")],
    },
    ExportedFunction {
        name: "__quantum__qis__h__body",
        return_type: "void",
        params: &[("const unsigned char*", "qubit")],
    },
    ExportedFunction {
        name: "__quantum__qis__x__body",
        return_type: "void",
        params: &[("int", "qubit")],
    },
    ExportedFunction {
        name: "__quantum__qis__y__body",
        return_type: "void",
        params: &[("int", "qubit")],
    },
    ExportedFunction {
        name: "__quantum__qis__z__body",
        return_type: "void",
        params: &[("int", "qubit")],
    },
    ExportedFunction {
        name: "__quantum__qis__s__body",
        return_type: "void",
        params: &[("int", "qubit")],
    },
    ExportedFunction {
        name: "__quantum__qis__sdg__body",
        return_type: "void",
        params: &[("int", "qubit")],
    },
    ExportedFunction {
        name: "__quantum__qis__t__body",
        return_type: "void",
        params: &[("int", "qubit")],
    },
    ExportedFunction {
        name: "__quantum__qis__tdg__body",
        return_type: "void",
        params: &[("int", "qubit")],
    },
    ExportedFunction {
        name: "__quantum__qis__rx__body",
        return_type: "void",
        params: &[("double", "theta"), ("int", "qubit")],
    },
    ExportedFunction {
        name: "__quantum__qis__ry__body",
        return_type: "void",
        params: &[("double", "theta"), ("int", "qubit")],
    },
    ExportedFunction {
        name: "__quantum__qis__cx__body",
        return_type: "void",
        params: &[("const unsigned char*", "control"), ("const unsigned char*", "target")],
    },
    ExportedFunction {
        name: "__quantum__qis__cz__body",
        return_type: "void",
        params: &[("int", "control"), ("int", "target")],
    },
    ExportedFunction {
        name: "__quantum__qis__cy__body",
        return_type: "void",
        params: &[("int", "control"), ("int", "target")],
    },
    ExportedFunction {
        name: "__quantum__qis__ch__body",
        return_type: "void",
        params: &[("int", "control"), ("int", "target")],
    },
    ExportedFunction {
        name: "__quantum__qis__szz__body",
        return_type: "void",
        params: &[("int", "q1"), ("int", "q2")],
    },
    ExportedFunction {
        name: "__quantum__qis__zz__body",
        return_type: "void",
        params: &[("int", "q1"), ("int", "q2")],
    },
    ExportedFunction {
        name: "__quantum__qis__rzz__body",
        return_type: "void",
        params: &[("double", "theta"), ("int", "q1"), ("int", "q2")],
    },
    ExportedFunction {
        name: "__quantum__qis__crz__body",
        return_type: "void",
        params: &[("double", "theta"), ("int", "control"), ("int", "target")],
    },
    ExportedFunction {
        name: "__quantum__qis__ccx__body",
        return_type: "void",
        params: &[("int", "control1"), ("int", "control2"), ("int", "target")],
    },
    ExportedFunction {
        name: "__quantum__qis__m__body",
        return_type: "void",
        params: &[("const unsigned char*", "qubit"), ("const unsigned char*", "result")],
    },
    ExportedFunction {
        name: "__quantum__qis__reset__body",
        return_type: "void",
        params: &[("int", "qubit")],
    },
    // Runtime management
    ExportedFunction {
        name: "__quantum__rt__initialize",
        return_type: "void",
        params: &[("void*", "config")],
    },
    ExportedFunction {
        name: "__quantum__rt__qubit_allocate",
        return_type: "int",
        params: &[],
    },
    ExportedFunction {
        name: "__quantum__rt__result_allocate",
        return_type: "int",
        params: &[],
    },
    ExportedFunction {
        name: "__quantum__rt__qubit_release",
        return_type: "void",
        params: &[("int", "qubit")],
    },
    ExportedFunction {
        name: "__quantum__rt__result_release",
        return_type: "void",
        params: &[("int", "result")],
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
        name: "__quantum__rt__result_record_output",
        return_type: "void",
        params: &[("const unsigned char*", "result"), ("const char*", "name")],
    },
    // Main function (exported from QIR program, not runtime)
    ExportedFunction {
        name: "main",
        return_type: "void",
        params: &[],
    },
    // Integer-based HUGR variants
    ExportedFunction {
        name: "__quantum__qis__h__body_i64",
        return_type: "void",
        params: &[("i64", "qubit")],
    },
    ExportedFunction {
        name: "__quantum__qis__x__body_i64",
        return_type: "void",
        params: &[("i64", "qubit")],
    },
    ExportedFunction {
        name: "__quantum__qis__y__body_i64",
        return_type: "void",
        params: &[("i64", "qubit")],
    },
    ExportedFunction {
        name: "__quantum__qis__z__body_i64",
        return_type: "void",
        params: &[("i64", "qubit")],
    },
    ExportedFunction {
        name: "__quantum__qis__cx__body_i64",
        return_type: "void",
        params: &[("i64", "control"), ("i64", "target")],
    },
    ExportedFunction {
        name: "__quantum__qis__m__body_i64",
        return_type: "u32",
        params: &[("i64", "qubit"), ("i64", "result")],
    },
    // Pointer-based wrapper functions for standard QIR format
    ExportedFunction {
        name: "__quantum__qis__h__body__ptr",
        return_type: "void",
        params: &[("const unsigned char*", "qubit")],
    },
    ExportedFunction {
        name: "__quantum__qis__x__body__ptr",
        return_type: "void",
        params: &[("const unsigned char*", "qubit")],
    },
    ExportedFunction {
        name: "__quantum__qis__y__body__ptr",
        return_type: "void",
        params: &[("const unsigned char*", "qubit")],
    },
    ExportedFunction {
        name: "__quantum__qis__z__body__ptr",
        return_type: "void",
        params: &[("const unsigned char*", "qubit")],
    },
    ExportedFunction {
        name: "__quantum__qis__rz__body__ptr",
        return_type: "void",
        params: &[("double", "theta"), ("const unsigned char*", "qubit")],
    },
    ExportedFunction {
        name: "__quantum__qis__rxy__body__ptr",
        return_type: "void",
        params: &[
            ("double", "theta"),
            ("double", "phi"),
            ("const unsigned char*", "qubit"),
        ],
    },
    ExportedFunction {
        name: "__quantum__qis__cx__body__ptr",
        return_type: "void",
        params: &[
            ("const unsigned char*", "control"),
            ("const unsigned char*", "target"),
        ],
    },
    ExportedFunction {
        name: "__quantum__qis__cz__body__ptr",
        return_type: "void",
        params: &[
            ("const unsigned char*", "control"),
            ("const unsigned char*", "target"),
        ],
    },
    ExportedFunction {
        name: "__quantum__qis__zz__body__ptr",
        return_type: "void",
        params: &[
            ("double", "theta"),
            ("const unsigned char*", "q1"),
            ("const unsigned char*", "q2"),
        ],
    },
    ExportedFunction {
        name: "__quantum__qis__m__body__ptr",
        return_type: "void*",
        params: &[
            ("const unsigned char*", "qubit"),
            ("const unsigned char*", "result"),
        ],
    },
    ExportedFunction {
        name: "__quantum__rt__result_record_output__ptr",
        return_type: "void",
        params: &[("const unsigned char*", "result"), ("const char*", "name")],
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

// Define type aliases
typedef uint32_t u32;
typedef size_t usize;
typedef int32_t i32;
typedef int64_t i64;
typedef unsigned char i1;

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
