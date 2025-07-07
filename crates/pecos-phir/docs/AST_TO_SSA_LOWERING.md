# Using PHIR/MLIR as an AST with Progressive Lowering to SSA

## Overview

This document explains how PHIR leverages MLIR's flexibility to implement an AST-like frontend representation that progressively lowers to SSA form suitable for transformations. This approach gives us the best of both worlds: intuitive source-level representation and powerful SSA-based optimizations.

## The Progressive Lowering Pipeline

```
┌─────────────────┐     ┌─────────────────┐     ┌─────────────────┐     ┌─────────────────┐
│   Source Code   │────▶│  AST-like PHIR  │────▶│ Resolved PHIR   │────▶│   SSA PHIR      │
│ (QASM, Guppy)   │     │ (parse dialect) │     │ (quantum/arith) │     │  (cf/scf/llvm)  │
└─────────────────┘     └─────────────────┘     └─────────────────┘     └─────────────────┘
                              │                        │                        │
                              │                        │                        │
                        ┌─────▼─────┐            ┌────▼────┐            ┌──────▼──────┐
                        │ AST-like: │            │ Mostly  │            │ Pure SSA:   │
                        │ • Names   │            │ Resolved│            │ • CFG       │
                        │ • Scopes  │            │ • Types │            │ • Phi nodes │
                        │ • Nested  │            │ • Calls │            │ • Dominance │
                        └───────────┘            └─────────┘            └─────────────┘
```

## Phase 1: AST-like PHIR

### Design Principles

1. **Direct Source Mapping**: Operations map 1:1 to source constructs
2. **Hierarchical Scoping**: Nested regions represent lexical scopes
3. **Name-based References**: Use strings for unresolved references
4. **Structural Control Flow**: For/while/if as nested operations
5. **Implicit Operations**: Type coercions, default values

### AST-like Operations to Implement

```rust
// parsing_dialect.rs - High-level AST-like operations

pub enum ASTOp {
    // Variable and naming
    VarDecl { name: String, type_expr: Option<TypeExpr>, init: Option<Value> },
    UnresolvedRef { name: String, scope_hint: Option<String> },
    
    // Control flow (structured, not CFG)
    ForLoop { 
        var: String, 
        start: Value, 
        end: Value, 
        step: Option<Value>,
        body: Region 
    },
    WhileLoop { condition: Region, body: Region },
    IfElse { condition: Value, then_body: Region, else_body: Option<Region> },
    
    // Function operations
    FunctionDef { 
        name: String, 
        params: Vec<(String, TypeExpr)>, 
        return_type: Option<TypeExpr>,
        body: Region 
    },
    UnresolvedCall { name: String, args: Vec<Value> },
    Return { value: Option<Value> },
    
    // Type operations
    TypeExpr { repr: String }, // "qubit[5]", "int", etc.
    InferType { constraint_id: u32 },
    ImplicitCast { from: Value, to_type: TypeExpr },
    
    // Quantum-specific AST nodes
    QuantumReg { name: String, size: Value },
    QuantumGate { gate: String, qubits: Vec<String>, params: Vec<Value> },
    Measurement { qubit: String, classical: String },
    
    // Scope management
    ScopeBegin { name: Option<String> },
    ScopeEnd,
}
```

### Example: AST-like PHIR

```mlir
// Direct representation of source code structure
module @quantum_program {
  // Global quantum register declaration
  "parse.quantum_reg"() {name = "q", size = 5} : () -> ()
  "parse.var_decl"() {name = "results", type = "bit[5]"} : () -> ()
  
  "parse.function_def"() {
    name = "bell_pairs",
    params = [("n", "int")]
  } ({
    // Function body as a region - preserves lexical scope
    "parse.scope_begin"() : () -> ()
    
    // For loop with body as nested region
    "parse.for_loop"() {
      var = "i",
      start = 0,
      end = "parse.unresolved_ref"() {name = "n"} : () -> !parse.unknown
    } ({
      // Loop body
      %i = "parse.unresolved_ref"() {name = "i"} : () -> !parse.unknown
      %i2 = "parse.implicit_cast"(%i) {to_type = "index"} : (!parse.unknown) -> !parse.unknown
      
      "parse.quantum_gate"() {
        gate = "H",
        qubits = ["q[2*i]"]
      } : () -> ()
      
      "parse.quantum_gate"() {
        gate = "CNOT", 
        qubits = ["q[2*i]", "q[2*i+1]"]
      } : () -> ()
    }) : () -> ()
    
    "parse.scope_end"() : () -> ()
  }) : () -> ()
}
```

## Phase 2: Resolution and Type Checking

### Lowering AST-like → Resolved PHIR

```rust
// Resolution passes
pub struct NameResolutionPass {
    symbol_table: SymbolTable,
}

impl Pass for NameResolutionPass {
    fn run_on_operation(&mut self, op: &mut Operation) -> Result<()> {
        match op {
            Operation::Parse(ParseOp::UnresolvedRef { name, .. }) => {
                // Look up name in symbol table
                let symbol = self.symbol_table.lookup(name)?;
                // Replace with resolved reference
                *op = Operation::Core(CoreOp::ValueRef(symbol.value_id));
            }
            Operation::Parse(ParseOp::UnresolvedCall { name, args }) => {
                let func = self.symbol_table.lookup_function(name)?;
                *op = Operation::Core(CoreOp::Call {
                    callee: func.id,
                    args: args.clone(),
                });
            }
            _ => {}
        }
        Ok(())
    }
}
```

### After Resolution

```mlir
module @quantum_program {
  %q = quantum.alloc_register : !quantum.register<5>
  %results = arith.alloc : !array<5xi1>
  
  func @bell_pairs(%n: i32) {
    // For loop still structured but with resolved types
    scf.for %i = %c0 to %n step %c1 {
      %idx = arith.muli %i, %c2 : i32
      %q0 = quantum.extract %q[%idx] : !quantum.register<5> -> !quantum.qubit
      %idx_plus_1 = arith.addi %idx, %c1 : i32
      %q1 = quantum.extract %q[%idx_plus_1] : !quantum.register<5> -> !quantum.qubit
      
      quantum.h %q0 : !quantum.qubit
      quantum.cnot %q0, %q1 : !quantum.qubit, !quantum.qubit
    }
  }
}
```

## Phase 3: SSA Lowering

### Lowering Structured → SSA Form

```rust
pub struct ControlFlowLoweringPass;

impl Pass for ControlFlowLoweringPass {
    fn run_on_operation(&mut self, op: &mut Operation) -> Result<()> {
        match op {
            Operation::SCF(SCFOp::For { start, end, step, body, .. }) => {
                // Convert to CFG with explicit branches
                let cfg = self.build_loop_cfg(start, end, step, body)?;
                *op = Operation::CF(CFOp::Branch(cfg.entry_block));
            }
            _ => {}
        }
        Ok(())
    }
}
```

### Final SSA Form

```mlir
module @quantum_program {
  %q = quantum.alloc_register : !quantum.register<5>
  %results = llvm.alloca %c5 x i1 : !llvm.ptr<array<5 x i1>>
  
  func @bell_pairs(%n: i32) {
    %c0 = arith.constant 0 : i32
    %c1 = arith.constant 1 : i32
    %c2 = arith.constant 2 : i32
    cf.br ^loop_header(%c0 : i32)
    
  ^loop_header(%i: i32):
    %cond = arith.cmpi slt, %i, %n : i32
    cf.cond_br %cond, ^loop_body, ^exit
    
  ^loop_body:
    %idx = arith.muli %i, %c2 : i32
    %q0_ptr = quantum.get_qubit_ptr %q, %idx : !llvm.ptr
    %idx_plus_1 = arith.addi %idx, %c1 : i32
    %q1_ptr = quantum.get_qubit_ptr %q, %idx_plus_1 : !llvm.ptr
    
    call @__quantum__qis__h__body(%q0_ptr) : (!llvm.ptr) -> ()
    call @__quantum__qis__cnot__body(%q0_ptr, %q1_ptr) : (!llvm.ptr, !llvm.ptr) -> ()
    
    %next_i = arith.addi %i, %c1 : i32
    cf.br ^loop_header(%next_i : i32)
    
  ^exit:
    return
  }
}
```

## Implementation Strategy

### 1. Define Parse Dialect

```rust
// dialects/parse_dialect.rs
pub struct ParseDialect;

impl Dialect for ParseDialect {
    fn name(&self) -> &'static str { "parse" }
    
    fn register_operations(&self, registry: &mut OpRegistry) {
        registry.register::<VarDeclOp>();
        registry.register::<UnresolvedRefOp>();
        registry.register::<ForLoopOp>();
        registry.register::<IfElseOp>();
        registry.register::<FunctionDefOp>();
        registry.register::<UnresolvedCallOp>();
        registry.register::<QuantumGateOp>();
        // ... etc
    }
}
```

### 2. Parser Implementation

```rust
// parser/ast_builder.rs
pub struct ASTBuilder {
    current_scope: ScopeId,
    pending_refs: Vec<UnresolvedRef>,
}

impl ASTBuilder {
    pub fn build_for_loop(&mut self, 
        var: &str, 
        start: Expr, 
        end: Expr, 
        body: Vec<Statement>
    ) -> Operation {
        let body_region = self.with_new_scope(|builder| {
            builder.declare_variable(var, Type::Infer);
            builder.build_statements(body)
        });
        
        Operation::Parse(ParseOp::ForLoop {
            var: var.to_string(),
            start: self.build_expr(start),
            end: self.build_expr(end),
            step: None,
            body: body_region,
        })
    }
    
    pub fn build_var_ref(&mut self, name: &str) -> Operation {
        // Create unresolved reference - will be resolved in later pass
        let ref_op = Operation::Parse(ParseOp::UnresolvedRef {
            name: name.to_string(),
            scope_hint: Some(self.current_scope.to_string()),
        });
        self.pending_refs.push(ref_op.clone());
        ref_op
    }
}
```

### 3. Progressive Lowering Passes

```rust
// passes/progressive_lowering.rs
pub struct ProgressiveLoweringPipeline {
    passes: Vec<Box<dyn Pass>>,
}

impl ProgressiveLoweringPipeline {
    pub fn new() -> Self {
        Self {
            passes: vec![
                Box::new(NameResolutionPass::new()),
                Box::new(TypeInferencePass::new()),
                Box::new(ImplicitCastInsertionPass::new()),
                Box::new(ControlFlowLoweringPass::new()),
                Box::new(QuantumGateLoweringPass::new()),
                Box::new(SSAConstructionPass::new()),
            ]
        }
    }
    
    pub fn run(&mut self, module: &mut Module) -> Result<()> {
        for pass in &mut self.passes {
            pass.run(module)?;
            // Optionally verify after each pass
            verify_module(module)?;
        }
        Ok(())
    }
}
```

## Benefits of This Approach

### 1. **Intuitive Frontend**
- Source code maps directly to IR structure
- Easy to understand and debug
- Natural for parser implementation

### 2. **Preservation of Intent**
- High-level constructs preserved until needed
- Better error messages with source context
- Easier to implement source-level optimizations

### 3. **Flexible Lowering**
- Can stop at any level for analysis
- Mix abstraction levels during compilation
- Add domain-specific optimizations at each level

### 4. **Unified Infrastructure**
- Same visitor/builder patterns throughout
- Single IR to learn and work with
- Reuse verification and printing infrastructure

## Example Use Cases

### 1. **Quantum Circuit Synthesis**
Keep quantum operations high-level until you know the target:
```mlir
// High-level
"parse.quantum_algo"() {algo = "QFT", size = 4} : () -> ()

// After target-specific lowering
quantum.h %q0
quantum.cphase(%q0, %q1) {angle = 1.57}
// ... expanded QFT circuit
```

### 2. **Error Message Generation**
Unresolved references contain scope hints for better errors:
```
Error: Undefined variable 'x' in function 'foo' at line 42
  Hint: Did you mean 'y' from outer scope?
```

### 3. **Optimization at Multiple Levels**
- AST level: Constant folding, dead code elimination
- Resolved level: Inlining, quantum gate fusion
- SSA level: Traditional compiler optimizations

## Next Steps

1. **Implement Parse Dialect**: Create the AST-like operations
2. **Update Parser**: Generate parse dialect operations
3. **Write Lowering Passes**: Implement progressive lowering
4. **Add Verification**: Ensure well-formed IR at each level
5. **Create Tests**: Test each lowering phase independently

This approach gives us a true "AST in MLIR" that progressively lowers to efficient SSA form, combining the benefits of both representations in a single, unified IR.