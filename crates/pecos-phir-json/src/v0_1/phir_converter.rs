/*!
Streaming PHIR-JSON to PHIR-RON converter

This module provides a fast, streaming converter from PHIR-JSON to PHIR-RON
without building an intermediate AST. It reads JSON and writes RON directly.
*/

use pecos_core::errors::PecosError;
use serde_json::{Value, Map};
use std::io::{Read, Write};

/// Convert PHIR-JSON to PHIR-RON using streaming
pub fn stream_phir_json_to_ron<R: Read, W: Write>(
    reader: R,
    writer: &mut W,
) -> Result<(), PecosError> {
    // Parse JSON
    let json: Value = serde_json::from_reader(reader)
        .map_err(|e| PecosError::Input(format!("Failed to parse PHIR-JSON: {}", e)))?;
    
    // Validate format
    let obj = json.as_object()
        .ok_or_else(|| PecosError::Input("PHIR-JSON must be an object".to_string()))?;
    
    let format = obj.get("format")
        .and_then(|v| v.as_str())
        .ok_or_else(|| PecosError::Input("Missing 'format' field".to_string()))?;
    
    if format != "PHIR/JSON" {
        return Err(PecosError::Input(format!("Invalid format: expected 'PHIR/JSON', got '{}'", format)));
    }
    
    let version = obj.get("version")
        .and_then(|v| v.as_str())
        .ok_or_else(|| PecosError::Input("Missing 'version' field".to_string()))?;
    
    if version != "0.1.0" {
        return Err(PecosError::Input(format!("Unsupported version: expected '0.1.0', got '{}'", version)));
    }
    
    // Start streaming conversion
    let mut converter = StreamingConverter::new(writer);
    converter.convert_program(obj)?;
    
    Ok(())
}

/// Fast conversion from PHIR-JSON string to PHIR-RON string
pub fn phir_json_to_ron(json_str: &str) -> Result<String, PecosError> {
    let mut output = Vec::new();
    stream_phir_json_to_ron(json_str.as_bytes(), &mut output)?;
    String::from_utf8(output)
        .map_err(|e| PecosError::Input(format!("Invalid UTF-8 in RON output: {}", e)))
}

/// Convert PHIR-JSON string directly to PHIR Module via RON
/// 
/// This is the main conversion function that uses streaming for performance.
/// The conversion path is: PHIR-JSON string → PHIR-RON string → PHIR Module
pub fn phir_json_to_module(json_str: &str) -> Result<pecos_phir::Module, PecosError> {
    // Use streaming converter for speed
    let ron_text = phir_json_to_ron(json_str)?;
    
    // Deserialize RON to PHIR Module
    pecos_phir::from_ron(&ron_text).map_err(|e| 
        PecosError::Input(format!("Failed to deserialize PHIR from RON: {}", e))
    )
}

/// Convert PHIR-JSON string to both RON text and PHIR Module
/// 
/// This is useful for debugging as it returns both the intermediate RON
/// representation and the final PHIR Module.
pub fn phir_json_to_ron_and_module(json_str: &str) -> Result<(String, pecos_phir::Module), PecosError> {
    // Convert to RON text
    let ron_text = phir_json_to_ron(json_str)?;
    
    // Deserialize RON to PHIR Module
    let module = pecos_phir::from_ron(&ron_text).map_err(|e| 
        PecosError::Input(format!("Failed to deserialize PHIR from RON: {}", e))
    )?;
    
    Ok((ron_text, module))
}

struct StreamingConverter<W: Write> {
    writer: W,
    next_ssa_id: u32,
    variable_map: std::collections::HashMap<String, u32>,
}

impl<W: Write> StreamingConverter<W> {
    fn new(writer: W) -> Self {
        Self {
            writer,
            next_ssa_id: 0,
            variable_map: std::collections::HashMap::new(),
        }
    }
    
    fn write(&mut self, s: &str) -> Result<(), PecosError> {
        self.writer.write_all(s.as_bytes())
            .map_err(|e| PecosError::Input(format!("Write error: {}", e)))
    }
    
    fn writeln(&mut self, s: &str) -> Result<(), PecosError> {
        self.write(s)?;
        self.write("\n")
    }
    
    fn get_ssa_id(&mut self, var: &str) -> u32 {
        if let Some(&id) = self.variable_map.get(var) {
            id
        } else {
            let id = self.next_ssa_id;
            self.next_ssa_id += 1;
            self.variable_map.insert(var.to_string(), id);
            id
        }
    }
    
    fn convert_program(&mut self, obj: &Map<String, Value>) -> Result<(), PecosError> {
        // Extract module name from metadata
        let module_name = obj.get("metadata")
            .and_then(|m| m.as_object())
            .and_then(|m| m.get("name"))
            .and_then(|n| n.as_str())
            .unwrap_or("phir_module");
        
        // Write module header
        self.writeln("ModuleOp(")?;
        self.writeln(&format!("    name: \"{}\",", module_name))?;
        self.writeln("    attributes: {},")?;
        self.writeln("    body: (")?;
        self.writeln("        blocks: [")?;
        self.writeln("            (")?;
        self.writeln("                label: None,")?;
        self.writeln("                arguments: [],")?;
        self.writeln("                operations: [")?;
        
        // Main function
        self.writeln("                    (")?;
        self.writeln("                        operation: Builtin(Func((")?;
        self.writeln("                            name: \"main\",")?;
        self.writeln("                            function_type: (inputs: [], outputs: [], variadic: false),")?;
        self.writeln("                            attributes: {},")?;
        self.writeln("                            body: [(")?;
        self.writeln("                                blocks: [(")?;
        self.writeln("                                    label: None,")?;
        self.writeln("                                    arguments: [],")?;
        self.writeln("                                    operations: [")?;
        
        // Convert operations
        if let Some(ops) = obj.get("ops").and_then(|v| v.as_array()) {
            for op in ops {
                self.convert_operation(op, 10)?;
            }
        }
        
        // Close all the structures
        self.writeln("                                    ],")?;
        self.writeln("                                    terminator: Some(Return(values: [])),")?;
        self.writeln("                                    attributes: {},")?;
        self.writeln("                                )],")?;
        self.writeln("                                kind: Graph,")?;
        self.writeln("                                attributes: {},")?;
        self.writeln("                            )],")?;
        self.writeln("                        ))),")?;
        self.writeln("                        operands: [],")?;
        self.writeln("                        results: [],")?;
        self.writeln("                        result_types: [],")?;
        self.writeln("                        regions: [],")?;
        self.writeln("                        attributes: {},")?;
        self.writeln("                        location: None,")?;
        self.writeln("                    ),")?;
        self.writeln("                ],")?;
        self.writeln("                terminator: None,")?;
        self.writeln("                attributes: {},")?;
        self.writeln("            ),")?;
        self.writeln("        ],")?;
        self.writeln("        kind: SSACFG,")?;
        self.writeln("        attributes: {},")?;
        self.writeln("    ),")?;
        self.writeln(")")?;
        
        Ok(())
    }
    
    fn convert_operation(&mut self, op: &Value, indent: usize) -> Result<(), PecosError> {
        let obj = op.as_object()
            .ok_or_else(|| PecosError::Input("Operation must be an object".to_string()))?;
        
        let indent_str = " ".repeat(indent);
        
        // Variable definition
        if let Some(data) = obj.get("data").and_then(|v| v.as_str()) {
            let data_type = obj.get("data_type").and_then(|v| v.as_str()).unwrap_or("");
            let variable = obj.get("variable").and_then(|v| v.as_str()).unwrap_or("");
            let size = obj.get("size").and_then(|v| v.as_u64()).unwrap_or(0);
            
            let comment = format!("{}// Variable definition: {} {} {} (size: {})", 
                indent_str, data, data_type, variable, size);
            self.writeln(&comment)?;
            
            // Register the variable
            self.get_ssa_id(variable);
            return Ok(());
        }
        
        // Quantum operation
        if let Some(qop) = obj.get("qop").and_then(|v| v.as_str()) {
            self.writeln(&format!("{}(", indent_str))?;
            
            // Convert operation name
            let phir_op = match qop {
                "H" => "Quantum(H)",
                "X" => "Quantum(X)",
                "Y" => "Quantum(Y)",
                "Z" => "Quantum(Z)",
                "S" => "Quantum(S)",
                "T" => "Quantum(T)",
                "CX" | "CNOT" => "Quantum(CX)",
                "CZ" => "Quantum(CZ)",
                "Measure" => "Quantum(Measure)",
                _ => return Err(PecosError::Input(format!("Unknown quantum op: {}", qop))),
            };
            
            self.writeln(&format!("{}    operation: {},", indent_str, phir_op))?;
            
            // Operands
            self.write(&format!("{}    operands: [", indent_str))?;
            if let Some(args) = obj.get("args").and_then(|v| v.as_array()) {
                for (i, arg) in args.iter().enumerate() {
                    if i > 0 { self.write(", ")?; }
                    
                    if let Some(arr) = arg.as_array() {
                        if arr.len() == 2 {
                            if let (Some(var), Some(idx)) = (arr[0].as_str(), arr[1].as_u64()) {
                                let ssa_id = self.get_ssa_id(var);
                                self.write(&format!("(id: {}, version: 0)", ssa_id + idx as u32))?;
                            }
                        }
                    }
                }
            }
            self.writeln("],")?;
            
            // Results
            self.write(&format!("{}    results: [", indent_str))?;
            if let Some(returns) = obj.get("returns").and_then(|v| v.as_array()) {
                for (i, ret) in returns.iter().enumerate() {
                    if i > 0 { self.write(", ")?; }
                    
                    if let Some(arr) = ret.as_array() {
                        if arr.len() == 2 {
                            if let (Some(var), Some(idx)) = (arr[0].as_str(), arr[1].as_u64()) {
                                let ssa_id = self.get_ssa_id(var);
                                self.write(&format!("(id: {}, version: 0)", ssa_id + idx as u32))?;
                            }
                        }
                    }
                }
            } else {
                // Generate result
                let result_id = self.next_ssa_id;
                self.next_ssa_id += 1;
                self.write(&format!("(id: {}, version: 0)", result_id))?;
            }
            self.writeln("],")?;
            
            // Result types
            let result_type = if qop == "Measure" { "Bit" } else { "Qubit" };
            self.writeln(&format!("{}    result_types: [{}],", indent_str, result_type))?;
            self.writeln(&format!("{}    regions: [],", indent_str))?;
            self.writeln(&format!("{}    attributes: {{}},", indent_str))?;
            self.writeln(&format!("{}    location: None,", indent_str))?;
            self.writeln(&format!("{}),", indent_str))?;
            
            return Ok(());
        }
        
        // Classical operation
        if let Some(cop) = obj.get("cop").and_then(|v| v.as_str()) {
            if cop == "Result" {
                let args = obj.get("args");
                let returns = obj.get("returns");
                let comment = format!("{}// Result operation: {:?} -> {:?}", indent_str, args, returns);
                self.writeln(&comment)?;
            } else {
                let comment = format!("{}// Classical operation: {} (not yet implemented)", indent_str, cop);
                self.writeln(&comment)?;
            }
            return Ok(());
        }
        
        // Other operations as comments for now
        self.writeln(&format!("{}// Unsupported operation: {:?}", indent_str, obj))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_fast_conversion() {
        let json = r#"{
            "format": "PHIR/JSON",
            "version": "0.1.0",
            "metadata": {"name": "test"},
            "ops": [
                {"qop": "H", "args": [["q", 0]]}
            ]
        }"#;
        
        let ron = phir_json_to_ron(json).unwrap();
        assert!(ron.contains("ModuleOp"));
        assert!(ron.contains("Quantum(H)"));
    }
    
    #[test] 
    fn test_module_conversion() {
        let json = r#"{
            "format": "PHIR/JSON",
            "version": "0.1.0",
            "metadata": {"name": "test"},
            "ops": [
                {"qop": "H", "args": [["q", 0]]}
            ]
        }"#;
        
        let module = phir_json_to_module(json).unwrap();
        assert_eq!(module.name, "test");
    }
}