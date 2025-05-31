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
            "int" | "usize" | "u32" => "{ return 0; }",
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
        name: "__quantum__qis__h__body",
        return_type: "void",
        params: &[("int", "qubit")],
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
        name: "__quantum__qis__cx__body",
        return_type: "void",
        params: &[("int", "control"), ("int", "target")],
    },
    ExportedFunction {
        name: "__quantum__qis__cz__body",
        return_type: "void",
        params: &[("int", "control"), ("int", "target")],
    },
    ExportedFunction {
        name: "__quantum__qis__szz__body",
        return_type: "void",
        params: &[("int", "q1"), ("int", "q2")],
    },
    ExportedFunction {
        name: "__quantum__qis__rzz__body",
        return_type: "void",
        params: &[("double", "theta"), ("int", "q1"), ("int", "q2")],
    },
    ExportedFunction {
        name: "__quantum__qis__m__body",
        return_type: "int",
        params: &[("int", "qubit"), ("int", "result")],
    },
    ExportedFunction {
        name: "__quantum__qis__reset__body",
        return_type: "void",
        params: &[("int", "qubit")],
    },
    // Runtime management
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
        params: &[("int", "result"), ("const char*", "name")],
    },
    // Main function (exported from QIR program, not runtime)
    ExportedFunction {
        name: "main",
        return_type: "void",
        params: &[],
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
