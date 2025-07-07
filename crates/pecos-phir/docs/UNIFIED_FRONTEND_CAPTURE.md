# Unified Frontend Capture: From Any Source to PHIR/MLIR

## Overview

PHIR/MLIR can capture the complete semantics of any source language - quantum or classical - by representing source constructs as operations in an AST-like form. This document shows how different source languages map to PHIR's unified representation.

## The Universal Pattern

```
┌─────────────┐     ┌──────────────────┐     ┌─────────────────┐     ┌──────────────┐
│   Source    │────▶│  Source-Specific │────▶│  Unified PHIR   │────▶│  Target IR   │
│  Language   │     │   AST Operations │     │  (Normalized)   │     │ (LLVM/QIR)   │
└─────────────┘     └──────────────────┘     └─────────────────┘     └──────────────┘
    QASM 3.0             qasm3.*                  quantum.*              llvm.*
    Guppy                guppy.*                  + arith.*              + qir.*
    HUGR                 hugr.*                   + scf.*
    LLVM IR              llvm_ast.*               + cf.*
```

## Source Language Examples

### 1. OpenQASM 3.0

OpenQASM has both quantum gates and classical control flow:

```qasm
// Original QASM 3.0 source
include "stdgates.inc";

def bell(qubit[2] q) -> bit[2] {
    h q[0];
    cx q[0], q[1];
    bit[2] c;
    c[0] = measure q[0];
    c[1] = measure q[1];
    return c;
}

qubit[4] q;
bit[4] results;

for i in [0:2:3] {  // 0, 2
    bell(q[i:i+1]) -> results[i:i+1];
}

if (results[0] == 1) {
    x q[3];
}
```

PHIR AST-like capture:

```mlir
module @qasm_program {
  // Include directives preserved
  "qasm3.include"() {file = "stdgates.inc"} : () -> ()
  
  // Function definition with QASM-specific syntax
  "qasm3.defcal"() {name = "bell", return_type = "bit[2]"} ({
    "qasm3.param"() {name = "q", type = "qubit[2]"} : () -> ()
  }, {
    // Body preserves QASM structure
    "qasm3.gate_call"() {gate = "h", qubits = ["q[0]"]} : () -> ()
    "qasm3.gate_call"() {gate = "cx", qubits = ["q[0]", "q[1]"]} : () -> ()
    
    "qasm3.creg_decl"() {name = "c", size = 2} : () -> ()
    "qasm3.measure_assign"() {qubit = "q[0]", creg = "c[0]"} : () -> ()
    "qasm3.measure_assign"() {qubit = "q[1]", creg = "c[1]"} : () -> ()
    "qasm3.return"() {value = "c"} : () -> ()
  }) : () -> ()
  
  // Global declarations
  "qasm3.qreg_decl"() {name = "q", size = 4} : () -> ()
  "qasm3.creg_decl"() {name = "results", size = 4} : () -> ()
  
  // QASM's range-based for loop
  "qasm3.for_range"() {var = "i", start = 0, stop = 3, step = 2} ({
    // Slice notation preserved
    "qasm3.call_assign"() {
      func = "bell",
      args = ["q[i:i+1]"],
      results = ["results[i:i+1]"]
    } : () -> ()
  }) : () -> ()
  
  // Classical control
  "qasm3.if"() ({
    "qasm3.binary_op"() {op = "==", lhs = "results[0]", rhs = 1} : () -> !qasm3.bool
  }, {
    "qasm3.gate_call"() {gate = "x", qubits = ["q[3]"]} : () -> ()
  }) : () -> ()
}
```

### 2. Guppy (Python-like Quantum Language)

Guppy has Python syntax with quantum extensions:

```python
# Original Guppy source
from guppylang import qubit, quantum

@quantum
def teleport(msg: qubit, alice: qubit, bob: qubit) -> qubit:
    # Create Bell pair
    alice = h(alice)
    alice, bob = cx(alice, bob)
    
    # Bell measurement
    msg, alice = cx(msg, alice)
    msg = h(msg)
    
    # Classical communication
    m1 = measure(msg)
    m2 = measure(alice)
    
    # Conditional corrections
    if m2:
        bob = x(bob)
    if m1:
        bob = z(bob)
        
    return bob

# List comprehension with quantum operations
qubits = [qubit() for _ in range(10)]
results = [h(q) | measure(q) for q in qubits]
```

PHIR AST-like capture:

```mlir
module @guppy_program {
  // Import statements
  "guppy.import"() {module = "guppylang", names = ["qubit", "quantum"]} : () -> ()
  
  // Function with decorator
  "guppy.function"() {
    name = "teleport",
    decorators = ["quantum"],
    params = [("msg", "qubit"), ("alice", "qubit"), ("bob", "qubit")],
    return_type = "qubit"
  } ({
    // Python-style comments preserved as attributes
    "guppy.comment"() {text = "Create Bell pair"} : () -> ()
    
    // Assignment with function call
    "guppy.assign"() {target = "alice"} ({
      "guppy.call"() {func = "h", args = ["alice"]} : () -> !guppy.value
    }) : () -> ()
    
    // Tuple assignment
    "guppy.tuple_assign"() {targets = ["alice", "bob"]} ({
      "guppy.call"() {func = "cx", args = ["alice", "bob"]} : () -> !guppy.tuple
    }) : () -> ()
    
    // Python if statements
    "guppy.if"() ({
      "guppy.name"() {id = "m2"} : () -> !guppy.value
    }, {
      "guppy.assign"() {target = "bob"} ({
        "guppy.call"() {func = "x", args = ["bob"]} : () -> !guppy.value
      }) : () -> ()
    }) : () -> ()
    
    "guppy.return"() {value = "bob"} : () -> ()
  }) : () -> ()
  
  // List comprehension - complex Python construct
  "guppy.list_comp"() {
    target = "qubits",
    element = "guppy.call"() {func = "qubit"} : () -> !guppy.value,
    iter = "guppy.call"() {func = "range", args = [10]} : () -> !guppy.value
  } : () -> ()
  
  // Comprehension with quantum operations
  "guppy.list_comp"() {
    target = "results",
    element = "guppy.binary_op"() {op = "|"} ({
      "guppy.call"() {func = "h", args = ["q"]} : () -> !guppy.value,
      "guppy.call"() {func = "measure", args = ["q"]} : () -> !guppy.value
    }) : () -> !guppy.value,
    iter_var = "q",
    iter_source = "qubits"
  } : () -> ()
}
```

### 3. HUGR (Hierarchical Unified Graph Representation)

HUGR uses graph-based representation with ports and edges:

```yaml
# Original HUGR (simplified notation)
nodes:
  - id: entry
    op: FuncDefn
    signature: [Qubit, Qubit] -> [Bit, Bit]
    
  - id: h_gate
    op: quantum.H
    parent: entry
    
  - id: cnot_gate  
    op: quantum.CNOT
    parent: entry
    
  - id: measure_0
    op: quantum.Measure
    parent: entry

edges:
  - [entry.input[0], h_gate.input]
  - [h_gate.output, cnot_gate.control]
  - [entry.input[1], cnot_gate.target]
```

PHIR AST-like capture:

```mlir
module @hugr_graph {
  // HUGR's hierarchical structure maps naturally to regions
  "hugr.func_defn"() {
    id = "entry",
    signature = "[Qubit, Qubit] -> [Bit, Bit]"
  } ({
    // Nodes become operations
    %h_out = "hugr.node"() {
      id = "h_gate",
      op_type = "quantum.H"
    } : () -> !hugr.wire<qubit>
    
    %cnot_out:2 = "hugr.node"() {
      id = "cnot_gate", 
      op_type = "quantum.CNOT"
    } : () -> (!hugr.wire<qubit>, !hugr.wire<qubit>)
    
    %m0 = "hugr.node"() {
      id = "measure_0",
      op_type = "quantum.Measure"
    } : () -> !hugr.wire<bit>
    
    // Edges as explicit connection operations
    "hugr.edge"() {
      source = "entry.input[0]",
      target = "h_gate.input"
    } : () -> ()
    
    "hugr.edge"() {
      source = "h_gate.output",
      target = "cnot_gate.control"
    } : () -> ()
    
    // Dataflow order can be preserved or reconstructed
  }) : () -> ()
  
  // HUGR's module system
  "hugr.module"() {
    name = "quantum_algorithms",
    exports = ["bell_pair", "qft"]
  } ({
    // Nested function definitions
  }) : () -> ()
}
```

### 4. LLVM IR

Even LLVM IR can be captured at AST level before lowering:

```llvm
; Original LLVM IR
define i32 @quantum_simulate(i32 %n) {
entry:
  %cmp = icmp sgt i32 %n, 0
  br i1 %cmp, label %loop.header, label %exit

loop.header:
  %i = phi i32 [ 0, %entry ], [ %i.next, %loop.body ]
  %sum = phi double [ 0.0, %entry ], [ %sum.next, %loop.body ]
  
  ; Quantum operation via intrinsic
  %qubit = call i8* @__quantum_allocate_qubit()
  call void @__quantum_h(i8* %qubit)
  %result = call i1 @__quantum_measure(i8* %qubit)
  
  br label %loop.body

loop.body:
  %val = sitofp i1 %result to double
  %sum.next = fadd double %sum, %val
  %i.next = add i32 %i, 1
  %done = icmp eq i32 %i.next, %n
  br i1 %done, label %exit, label %loop.header

exit:
  %ret = phi double [ 0.0, %entry ], [ %sum.next, %loop.body ]
  ret double %ret
}
```

PHIR AST-like capture:

```mlir
module @llvm_ir_ast {
  "llvm_ast.define"() {
    name = "@quantum_simulate",
    return_type = "i32",
    params = [("%n", "i32")],
    linkage = "external"
  } ({
    "llvm_ast.label"() {name = "entry"} ({
      %cmp = "llvm_ast.icmp"() {pred = "sgt", lhs = "%n", rhs = 0} : () -> !llvm_ast.i1
      "llvm_ast.br"() {
        cond = %cmp,
        true_label = "loop.header",
        false_label = "exit"
      } : () -> ()
    }) : () -> ()
    
    "llvm_ast.label"() {name = "loop.header"} ({
      // Phi nodes captured explicitly
      %i = "llvm_ast.phi"() {
        type = "i32",
        incoming = [("0", "entry"), ("%i.next", "loop.body")]
      } : () -> !llvm_ast.i32
      
      %sum = "llvm_ast.phi"() {
        type = "double",
        incoming = [("0.0", "entry"), ("%sum.next", "loop.body")]
      } : () -> !llvm_ast.double
      
      // Quantum intrinsics preserved
      %qubit = "llvm_ast.call"() {
        func = "@__quantum_allocate_qubit",
        args = []
      } : () -> !llvm_ast.ptr
      
      "llvm_ast.call"() {
        func = "@__quantum_h",
        args = [%qubit]
      } : () -> ()
      
      "llvm_ast.br"() {target = "loop.body"} : () -> ()
    }) : () -> ()
    
    // ... rest of blocks
  }) : () -> ()
}
```

## Unified Lowering Pipeline

After capturing source-specific constructs, we progressively lower to unified PHIR:

### Stage 1: Source-Specific → Generic AST

```rust
// Pattern matching to normalize different source constructs
rewrite_patterns! {
    // QASM gate calls → generic quantum ops
    "qasm3.gate_call"() {gate = %g, qubits = %q} 
      => "ast.quantum_gate"() {gate = %g, qubits = normalize_qubits(%q)}
    
    // Guppy function calls → generic calls  
    "guppy.call"() {func = %f, args = %a}
      => "ast.call"() {func = %f, args = %a}
      
    // HUGR nodes → operations
    "hugr.node"() {op_type = %op}
      => create_op_from_hugr_type(%op)
      
    // LLVM intrinsics → quantum ops
    "llvm_ast.call"() {func = "@__quantum_h", args = [%q]}
      => "ast.quantum_gate"() {gate = "H", qubits = [%q]}
}
```

### Stage 2: Generic AST → Typed PHIR

```mlir
// After type resolution and normalization
module @unified {
  func @quantum_simulate(%n: i32) -> f64 {
    %zero = arith.constant 0 : i32
    %zero_f = arith.constant 0.0 : f64
    %cmp = arith.cmpi sgt, %n, %zero : i32
    
    cf.cond_br %cmp, ^loop_header(%zero, %zero_f : i32, f64), ^exit(%zero_f : f64)
    
  ^loop_header(%i: i32, %sum: f64):
    %qubit = quantum.alloc : !quantum.qubit
    quantum.h %qubit : !quantum.qubit
    %result = quantum.measure %qubit : !quantum.qubit -> i1
    
    %val = arith.uitofp %result : i1 to f64
    %sum_next = arith.addf %sum, %val : f64
    %i_next = arith.addi %i, %c1 : i32
    %done = arith.cmpi eq, %i_next, %n : i32
    
    cf.cond_br %done, ^exit(%sum_next : f64), ^loop_header(%i_next, %sum_next : i32, f64)
    
  ^exit(%ret: f64):
    return %ret : f64
  }
}
```

## Benefits of Unified Capture

### 1. **Language Interoperability**
- Mix code from different sources in one program
- Reuse classical routines from LLVM IR with quantum from QASM
- Import Guppy libraries into HUGR graphs

### 2. **Preservation of Semantics**
- Source-specific optimizations possible
- Better error messages with original syntax
- Debugging shows source-level constructs

### 3. **Progressive Optimization**
- Language-specific passes first (QASM gate fusion)
- Then generic quantum passes
- Finally low-level optimization

### 4. **Tool Reuse**
- One set of analysis tools works on all languages
- Unified optimization infrastructure
- Common backend generation

## Implementation Strategy

```rust
// Frontend trait for each source language
pub trait QuantumLanguageFrontend {
    type SourceAST;
    
    fn parse(&self, source: &str) -> Result<Self::SourceAST, ParseError>;
    fn to_phir_ast(&self, ast: Self::SourceAST) -> Result<Module, ConversionError>;
    fn source_specific_passes(&self) -> Vec<Box<dyn Pass>>;
}

// Implementations for each language
impl QuantumLanguageFrontend for QASMFrontend {
    fn to_phir_ast(&self, ast: QASMProgram) -> Result<Module, ConversionError> {
        let mut module = Module::new();
        
        for item in ast.items {
            match item {
                QASMItem::GateDecl(gate) => {
                    module.add_op(self.convert_gate_decl(gate)?);
                }
                QASMItem::Reg(reg) => {
                    module.add_op(self.convert_register_decl(reg)?);
                }
                // ... etc
            }
        }
        
        Ok(module)
    }
}

// Unified pipeline
pub struct UnifiedCompiler {
    frontends: HashMap<SourceLanguage, Box<dyn QuantumLanguageFrontend>>,
}

impl UnifiedCompiler {
    pub fn compile(&self, source: Source) -> Result<Module, Error> {
        // 1. Parse to source-specific AST operations
        let frontend = &self.frontends[&source.language];
        let source_ast = frontend.parse(&source.code)?;
        let mut phir_ast = frontend.to_phir_ast(source_ast)?;
        
        // 2. Run source-specific optimizations
        for pass in frontend.source_specific_passes() {
            pass.run(&mut phir_ast)?;
        }
        
        // 3. Normalize to unified PHIR
        NormalizationPass::new().run(&mut phir_ast)?;
        
        // 4. Run unified optimization pipeline
        self.run_unified_passes(&mut phir_ast)?;
        
        Ok(phir_ast)
    }
}
```

## Conclusion

PHIR/MLIR's operation-based structure can capture **any** source language's constructs - both quantum and classical - in their original form, then progressively lower them to a unified representation. This gives us:

1. **True multi-language support** with faithful source representation
2. **Interoperability** between different quantum programming languages
3. **Reuse** of classical code from existing compilers
4. **Progressive optimization** from source-specific to generic
5. **Unified tooling** that works across all source languages

The same infrastructure that makes MLIR great for classical compilation makes it perfect for quantum-classical hybrid programs from any source!