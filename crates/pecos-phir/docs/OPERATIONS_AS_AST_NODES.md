# Operations as AST Nodes: MLIR's Recursive Structure

## The Key Insight

In MLIR, **everything is an operation**, and operations can contain regions, which contain blocks, which contain more operations. This recursive structure allows us to build exactly the same tree structures as traditional ASTs.

## Traditional AST vs MLIR Operations

### Traditional AST Structure
```
ProgramNode
├── FunctionNode("main")
│   ├── ParamListNode
│   │   └── ParamNode("x", IntType)
│   └── BlockNode
│       ├── IfNode
│       │   ├── ConditionNode(BinaryOp(">", Var("x"), Literal(0)))
│       │   ├── ThenNode
│       │   │   └── ReturnNode(Var("x"))
│       │   └── ElseNode
│       │       └── ReturnNode(UnaryOp("-", Var("x")))
│       └── ...
└── FunctionNode("helper")
    └── ...
```

### Same Structure with MLIR Operations
```mlir
"ast.program"() ({
  "ast.function"() {name = "main"} ({
    "ast.param_list"() ({
      "ast.param"() {name = "x", type = "int"} : () -> ()
    }) : () -> ()
    
    "ast.block"() ({
      "ast.if"() ({
        // Condition region
        %cond = "ast.binary_op"() {op = ">"} ({
          %x = "ast.var_ref"() {name = "x"} : () -> !ast.value
          %zero = "ast.literal"() {value = 0 : i32} : () -> !ast.value
        }) : () -> !ast.value
      }, {
        // Then region
        %x_then = "ast.var_ref"() {name = "x"} : () -> !ast.value
        "ast.return"(%x_then) : (!ast.value) -> ()
      }, {
        // Else region
        %x_else = "ast.var_ref"() {name = "x"} : () -> !ast.value
        %neg = "ast.unary_op"(%x_else) {op = "-"} : (!ast.value) -> !ast.value
        "ast.return"(%neg) : (!ast.value) -> ()
      }) : () -> ()
    }) : () -> ()
  }) : () -> ()
  
  "ast.function"() {name = "helper"} ({
    // ...
  }) : () -> ()
}) : () -> ()
```

## How Operations Enable AST Structure

### 1. **Hierarchical Nesting**

Operations can contain regions, creating parent-child relationships:

```rust
// Define an operation that represents an AST node
pub struct IfExprOp {
    // Can contain multiple regions (condition, then, else)
    regions: [Region; 3],
}

impl Operation for IfExprOp {
    fn regions(&self) -> &[Region] {
        &self.regions
    }
}
```

### 2. **Arbitrary Children**

Unlike fixed IR structures, you can design operations to hold any number of child operations:

```rust
// A function call AST node with variable arguments
pub struct CallExprOp {
    callee: Region,        // Expression that evaluates to function
    arguments: Region,     // Contains list of argument expressions
}

// Usage in MLIR:
"ast.call_expr"() ({
    // Callee can be complex expression
    "ast.member_access"() {field = "method"} ({
      "ast.var_ref"() {name = "obj"} : () -> !ast.value
    }) : () -> !ast.value
  }, {
    // Arguments as a list
    "ast.literal"() {value = 42} : () -> !ast.value
    "ast.var_ref"() {name = "x"} : () -> !ast.value
    "ast.binary_op"() {op = "+"} ({...}) : () -> !ast.value
}) : () -> !ast.value
```

### 3. **Preserving Source Structure**

Operations can maintain the exact structure of the source code:

```rust
// Original source
for (int i = 0; i < n; i++) {
    if (arr[i] > max) {
        max = arr[i];
    }
}

// AST-like MLIR preserving structure
"ast.for_loop"() ({
  // Init region
  "ast.var_decl"() {name = "i", type = "int"} ({
    "ast.literal"() {value = 0} : () -> !ast.value
  }) : () -> ()
}, {
  // Condition region  
  "ast.binary_op"() {op = "<"} ({
    "ast.var_ref"() {name = "i"} : () -> !ast.value
    "ast.var_ref"() {name = "n"} : () -> !ast.value
  }) : () -> !ast.bool
}, {
  // Update region
  "ast.assign"() ({
    "ast.var_ref"() {name = "i"} : () -> !ast.lvalue
    "ast.binary_op"() {op = "+"} ({
      "ast.var_ref"() {name = "i"} : () -> !ast.value
      "ast.literal"() {value = 1} : () -> !ast.value
    }) : () -> !ast.value
  }) : () -> ()
}, {
  // Body region
  "ast.if"() ({
    // Condition: arr[i] > max
    "ast.binary_op"() {op = ">"} ({
      "ast.array_access"() ({
        "ast.var_ref"() {name = "arr"} : () -> !ast.value
        "ast.var_ref"() {name = "i"} : () -> !ast.value
      }) : () -> !ast.value
      "ast.var_ref"() {name = "max"} : () -> !ast.value
    }) : () -> !ast.bool
  }, {
    // Then: max = arr[i]
    "ast.assign"() ({
      "ast.var_ref"() {name = "max"} : () -> !ast.lvalue
      "ast.array_access"() ({
        "ast.var_ref"() {name = "arr"} : () -> !ast.value
        "ast.var_ref"() {name = "i"} : () -> !ast.value
      }) : () -> !ast.value
    }) : () -> ()
  }) : () -> ()
}) : () -> ()
```

## Advantages Over Traditional ASTs

### 1. **Unified Infrastructure**

```rust
// Traditional approach needs different types
enum ASTNode {
    Program(ProgramNode),
    Function(FunctionNode),
    If(IfNode),
    While(WhileNode),
    // ... dozens more
}

// MLIR approach - everything is an Operation
trait Operation {
    fn name(&self) -> &str;
    fn attributes(&self) -> &Attributes;
    fn regions(&self) -> &[Region];
    fn operands(&self) -> &[Value];
    fn results(&self) -> &[Type];
}
```

### 2. **Progressive Refinement**

The same operation can be refined in-place:

```mlir
// Stage 1: Unresolved AST
%result = "ast.call_expr"() {callee = "foo", args = ["x", "y"]} : () -> !ast.unknown

// Stage 2: After name resolution  
%result = "ast.call_expr"() {callee = @foo, args = [%x, %y]} : () -> !ast.unknown

// Stage 3: After type inference
%result = "ast.call_expr"() {callee = @foo, args = [%x, %y]} : () -> i32

// Stage 4: Lowered to standard call
%result = call @foo(%x, %y) : (i32, i32) -> i32
```

### 3. **Extensible Without Core Changes**

Adding new language constructs is just defining new operations:

```rust
// Add async/await support
pub struct AsyncExprOp {
    body: Region,
}

pub struct AwaitExprOp {
    future: Value,
}

// Add pattern matching
pub struct MatchExprOp {
    scrutinee: Region,
    arms: Vec<Region>,  // Each arm is a pattern + body
}
```

## Practical Example: Lambda Expressions

Here's how you might represent lambda expressions as operations:

```rust
// Original source
auto doubler = [](int x) { return x * 2; };
auto result = map(vec, [capture](auto x) { return x + capture; });

// AST-like MLIR
%doubler = "ast.lambda"() ({
  "ast.param_list"() ({
    "ast.param"() {name = "x", type = "int"} : () -> ()
  }) : () -> ()
}, {
  "ast.return"() ({
    "ast.binary_op"() {op = "*"} ({
      "ast.var_ref"() {name = "x"} : () -> !ast.value
      "ast.literal"() {value = 2} : () -> !ast.value
    }) : () -> !ast.value
  }) : () -> ()
}) : () -> !ast.function

%result = "ast.call_expr"() {callee = "map"} ({
  "ast.var_ref"() {name = "vec"} : () -> !ast.value
  "ast.lambda"() {captures = ["capture"]} ({
    "ast.param_list"() ({
      "ast.param"() {name = "x", type = "auto"} : () -> ()
    }) : () -> ()
  }, {
    "ast.return"() ({
      "ast.binary_op"() {op = "+"} ({
        "ast.var_ref"() {name = "x"} : () -> !ast.value
        "ast.var_ref"() {name = "capture"} : () -> !ast.value
      }) : () -> !ast.value
    }) : () -> ()
  }) : () -> !ast.function
}) : () -> !ast.value
```

## Implementation Pattern

```rust
// Base trait for all AST operations
pub trait ASTOperation: Operation {
    fn source_location(&self) -> SourceLocation;
    fn node_type(&self) -> ASTNodeType;
}

// Specific AST node types
pub struct ForLoopOp {
    init: Region,
    condition: Region,
    update: Region,
    body: Region,
    attributes: Attributes,
}

impl Operation for ForLoopOp {
    fn name(&self) -> &str { "ast.for_loop" }
    
    fn num_regions(&self) -> usize { 4 }
    
    fn verify(&self) -> Result<(), Error> {
        // Verify init produces declarations
        // Verify condition produces boolean
        // Verify body is well-formed
        Ok(())
    }
}

// Builder pattern for construction
impl ForLoopOp {
    pub fn build(
        init: impl FnOnce(&mut RegionBuilder),
        condition: impl FnOnce(&mut RegionBuilder) -> Value,
        update: impl FnOnce(&mut RegionBuilder),
        body: impl FnOnce(&mut RegionBuilder),
    ) -> Self {
        // Build regions maintaining AST structure
    }
}
```

## Traversal and Transformation

Since it's all operations, standard MLIR patterns work:

```rust
// Visit all AST nodes
module.walk(|op: &Operation| {
    if let Some(ast_op) = op.dyn_cast::<ASTOperation>() {
        match ast_op.node_type() {
            ASTNodeType::ForLoop => process_for_loop(ast_op),
            ASTNodeType::IfExpr => process_if_expr(ast_op),
            // ...
        }
    }
});

// Pattern-based rewriting
rewrite_pattern! {
    // Simplify double negation
    "ast.unary_op"(%x) {op = "-"} (
        "ast.unary_op"(%y) {op = "-"} (...) : _ -> %x
    ) : _ -> %y
}
```

## Conclusion

MLIR's operation system is powerful enough to represent any AST structure while providing:

1. **Exact source structure preservation**
2. **Type safety and verification**
3. **Unified traversal and transformation APIs**
4. **Progressive lowering capabilities**
5. **Extensibility without core changes**

You don't need a separate AST - MLIR operations ARE your AST nodes, with all the benefits of the MLIR infrastructure!