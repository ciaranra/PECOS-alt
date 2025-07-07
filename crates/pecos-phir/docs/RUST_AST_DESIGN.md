# Rust AST Design for PHIR (Inspired by pliron)

## Core Design Principles

Taking inspiration from pliron's clean Rust implementation of MLIR concepts:

1. **Operations as Data**: Operations are just structs with fields, not complex class hierarchies
2. **Traits for Behavior**: Use traits to define common interfaces (Operation, Verify, etc.)
3. **Macros for Boilerplate**: Define operations declaratively with macros
4. **Builder Pattern**: Ergonomic API for constructing AST structures
5. **Type Safety**: Leverage Rust's type system while allowing dynamic operation types

## Architecture Overview

```rust
// Core trait - all operations implement this
pub trait Operation: Debug + Clone {
    fn name(&self) -> &'static str;
    fn regions(&self) -> &[Region];
    fn regions_mut(&mut self) -> &mut Vec<Region>;
    fn operands(&self) -> &[Value];
    fn results(&self) -> &[Type];
    fn attributes(&self) -> &Attributes;
    
    // Dynamic casting (like pliron)
    fn as_any(&self) -> &dyn Any;
}

// Region contains blocks which contain operations
pub struct Region {
    blocks: Vec<Block>,
}

pub struct Block {
    operations: Vec<Box<dyn Operation>>,
    terminator: Option<Box<dyn Operation>>,
}
```

## Defining AST Operations

### Using Macros (pliron-style)

```rust
// Define operations declaratively
def_ast_op! {
    /// For loop with init, condition, update, and body
    ForLoop {
        // Fields specific to this operation
        loop_var: Option<String>,
    }
    // Specify counts
    regions: 4  // init, condition, update, body
    operands: 0
    results: 0
}

// The macro generates:
// - Struct definition
// - Constructor
// - Operation trait implementation
// - Builder methods
```

### Manual Definition (for complex cases)

```rust
#[derive(Debug, Clone)]
pub struct IfElseOp {
    /// Regions: condition, then_branch, else_branch
    regions: [Region; 3],
    attributes: Attributes,
}

impl Operation for IfElseOp {
    fn name(&self) -> &'static str { "ast.if_else" }
    
    fn regions(&self) -> &[Region] { &self.regions }
    
    // ... other trait methods
}

impl IfElseOp {
    /// Builder-style construction
    pub fn build(
        condition: impl FnOnce(&mut RegionBuilder) -> Value,
        then_branch: impl FnOnce(&mut RegionBuilder),
        else_branch: impl FnOnce(&mut RegionBuilder),
    ) -> Self {
        let mut op = Self {
            regions: Default::default(),
            attributes: Attributes::new(),
        };
        
        // Build each region with the provided closures
        op.regions[0] = RegionBuilder::build(condition);
        op.regions[1] = RegionBuilder::build(|_| then_branch);
        op.regions[2] = RegionBuilder::build(|_| else_branch);
        
        op
    }
}
```

## Source-Specific Dialects

Organize operations by source language:

```rust
pub mod dialects {
    pub mod qasm3 {
        use super::*;
        
        def_ast_op! {
            GateDecl {
                name: String,
                params: Vec<String>,
                qubits: Vec<String>,
            }
            regions: 1  // gate body
        }
        
        def_ast_op! {
            Include {
                file: String,
            }
        }
    }
    
    pub mod guppy {
        use super::*;
        
        def_ast_op! {
            ListComprehension {
                target_var: String,
            }
            regions: 3  // element, iterator, filter
        }
        
        def_ast_op! {
            Decorator {
                name: String,
                args: Vec<Attribute>,
            }
            regions: 1  // decorated item
        }
    }
    
    pub mod hugr {
        use super::*;
        
        def_ast_op! {
            Node {
                node_id: NodeId,
                op_type: String,
            }
            operands: variable  // connected inputs
            results: variable   // outputs
        }
    }
}
```

## Builder API

Ergonomic construction inspired by pliron:

```rust
pub struct AstBuilder {
    current_block: Block,
    symbol_table: SymbolTable,
}

impl AstBuilder {
    /// Build a complete function
    pub fn function(
        &mut self,
        name: &str,
        params: Vec<(&str, Type)>,
        body: impl FnOnce(&mut Self) -> Option<Value>,
    ) -> FunctionOp {
        let mut func = FunctionOp::new(name);
        
        // Create parameter list
        self.with_region(|b| {
            for (name, ty) in params {
                b.param(name, ty);
            }
        });
        func.set_params(self.take_region());
        
        // Build body
        self.with_region(|b| {
            if let Some(ret_val) = body(b) {
                b.return_value(ret_val);
            }
        });
        func.set_body(self.take_region());
        
        func
    }
    
    /// QASM-style operations
    pub fn qasm_gate(&mut self, gate: &str, qubits: Vec<&str>) -> &mut Self {
        self.push(qasm3::GateCall {
            gate: gate.to_string(),
            qubit_names: qubits.into_iter().map(String::from).collect(),
            ..Default::default()
        });
        self
    }
    
    /// Guppy-style operations
    pub fn list_comp<T>(
        &mut self,
        elem: impl FnOnce(&mut Self) -> Value,
        iter: impl FnOnce(&mut Self) -> Value,
    ) -> Value {
        let mut op = guppy::ListComprehension::default();
        
        self.with_region(elem);
        op.set_element(self.take_region());
        
        self.with_region(iter);
        op.set_iterator(self.take_region());
        
        let result = self.fresh_value();
        op.set_results(vec![result.clone()]);
        self.push(op);
        
        result
    }
}
```

## Pattern Matching and Lowering

Inspired by pliron's rewrite infrastructure:

```rust
/// Trait for lowering patterns
pub trait RewritePattern: Send + Sync {
    fn matches(&self, op: &dyn Operation) -> bool;
    fn rewrite(&self, op: &dyn Operation, rewriter: &mut PatternRewriter) -> Result<()>;
}

/// Example: Lower unresolved references
pub struct ResolveReferencesPattern {
    symbol_table: Arc<SymbolTable>,
}

impl RewritePattern for ResolveReferencesPattern {
    fn matches(&self, op: &dyn Operation) -> bool {
        op.name() == "ast.unresolved_ref"
    }
    
    fn rewrite(&self, op: &dyn Operation, rewriter: &mut PatternRewriter) -> Result<()> {
        let unresolved = op.downcast_ref::<UnresolvedRef>()?;
        let symbol = self.symbol_table.lookup(&unresolved.name)?;
        
        // Replace with resolved operation
        rewriter.replace_op(op, ValueRef {
            value: symbol.value,
            type: symbol.type,
        });
        
        Ok(())
    }
}

/// Pattern set for progressive lowering
pub fn ast_lowering_patterns() -> PatternSet {
    let mut patterns = PatternSet::new();
    
    // Name resolution
    patterns.add(ResolveReferencesPattern::new());
    patterns.add(ResolveFunctionCallsPattern::new());
    
    // Type inference
    patterns.add(InferTypesPattern::new());
    patterns.add(InsertImplicitCastsPattern::new());
    
    // Control flow lowering
    patterns.add(LowerForLoopsPattern::new());
    patterns.add(LowerIfElsePattern::new());
    
    patterns
}
```

## Dynamic Dispatch with Type Safety

Following pliron's approach:

```rust
/// Extension trait for downcasting
pub trait OperationExt {
    fn downcast_ref<T: Operation + 'static>(&self) -> Option<&T>;
    fn is<T: Operation + 'static>(&self) -> bool;
}

impl OperationExt for dyn Operation {
    fn downcast_ref<T: Operation + 'static>(&self) -> Option<&T> {
        self.as_any().downcast_ref::<T>()
    }
    
    fn is<T: Operation + 'static>(&self) -> bool {
        self.as_any().is::<T>()
    }
}

// Usage
fn process_operation(op: &dyn Operation) {
    if let Some(for_loop) = op.downcast_ref::<ForLoopOp>() {
        // Handle for loop specifically
    } else if op.is::<qasm3::GateCall>() {
        // Handle QASM gate
    }
}
```

## Verification Infrastructure

Type checking and verification:

```rust
pub trait Verify {
    fn verify(&self) -> Result<(), VerificationError>;
}

impl Verify for ForLoopOp {
    fn verify(&self) -> Result<(), VerificationError> {
        // Check init region declares loop variable
        let init = &self.regions[0];
        if !init.declares_variable(&self.loop_var) {
            return Err(VerificationError::MissingLoopVariable);
        }
        
        // Check condition produces boolean
        let cond = &self.regions[1];
        if cond.result_type() != Some(Type::Bool) {
            return Err(VerificationError::InvalidConditionType);
        }
        
        Ok(())
    }
}
```

## Integration with Existing PHIR

```rust
// Extend existing Operation enum
pub enum Operation {
    // Existing
    Builtin(BuiltinOp),
    Quantum(QuantumOp),
    Classical(ClassicalOp),
    
    // New AST variants
    Parse(ParseOp),
    Qasm3(qasm3::Op),
    Guppy(guppy::Op),
    Hugr(hugr::Op),
}

// Or use dynamic dispatch throughout
pub type Operation = Box<dyn OperationTrait>;
```

## Example: Building Quantum Teleportation

```rust
let mut builder = AstBuilder::new();

let module = builder.module("teleportation", |b| {
    // QASM-style
    b.qasm_qreg("q", 3);
    b.qasm_creg("c", 2);
    
    b.function("teleport", vec![], |b| {
        // Create Bell pair
        b.qasm_gate("h", vec!["q[1]"]);
        b.qasm_gate("cx", vec!["q[1]", "q[2]"]);
        
        // Bell measurement
        b.qasm_gate("cx", vec!["q[0]", "q[1]"]);
        b.qasm_gate("h", vec!["q[0]"]);
        
        // Measure
        let m1 = b.qasm_measure("q[0]", "c[0]");
        let m2 = b.qasm_measure("q[1]", "c[1]");
        
        // Conditional operations
        b.if_then(m2, |b| {
            b.qasm_gate("x", vec!["q[2]"]);
        });
        
        b.if_then(m1, |b| {
            b.qasm_gate("z", vec!["q[2]"]);
        });
        
        None // No return
    });
});

// Lower progressively
let module = module
    .lower_with(qasm_lowering_patterns())
    .lower_with(quantum_lowering_patterns())
    .lower_with(ssa_lowering_patterns());
```

## Benefits of This Approach

1. **Type Safety**: Rust's type system catches errors at compile time
2. **Performance**: Zero-cost abstractions, no virtual dispatch overhead unless needed
3. **Extensibility**: Easy to add new operations via macros
4. **Clarity**: Operations are just data, transformations are just functions
5. **Debugging**: Can inspect and print AST structures easily
6. **Memory Safety**: Rust prevents common IR manipulation bugs

This design combines MLIR's flexible operation system with Rust's safety and pliron's clean patterns!