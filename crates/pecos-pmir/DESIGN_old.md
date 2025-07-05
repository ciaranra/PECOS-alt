# PMIR Design Document

## Overview

PMIR (PECOS Middle-level Intermediate Representation) is a MLIR-like intermediate representation written in Rust that serves as a universal bridge between various quantum circuit descriptions and multiple execution targets.

## MLIR Concepts in PMIR

PMIR adopts MLIR's hierarchical structure and concepts:

### Hierarchical Organization
```
Module
└── Functions
    └── Regions
        └── Blocks
            └── Operations
```

### Core Components

1. **Operations**: The basic unit of computation
   - Results (SSA values produced)
   - Operands (SSA values consumed)
   - Attributes (compile-time constants)
   - Regions (nested control flow)

2. **Blocks**: Linear sequences of operations
   - Block arguments (phi nodes)
   - Terminator operation
   - Predecessors and successors

3. **Regions**: Control flow scopes
   - Entry block
   - Multiple blocks with branches
   - SSA dominance rules

4. **Types**: First-class type system
   - Verification at construction
   - Type inference support

5. **Attributes**: Compile-time metadata
   - Constants
   - Configuration
   - Debug information

## Vision

PMIR should be a complete, self-contained IR that can:
1. **Accept multiple input formats** via different frontends
2. **Execute directly** via a pure Rust interpreter/simulator
3. **Compile to native Rust** by inlining simulation functions
4. **Export to MLIR text** for compilation to LLVM IR or quantum hardware
5. **Represent all operations** - both quantum and classical
6. **Support optimizations** at the IR level
7. **Feel natural** for quantum-classical hybrid programming
8. **Lower effortlessly** to MLIR with appropriate dialects
9. **Enable large-scale QEC** with fault-tolerant compilation
10. **Support resource estimation** and QEC analysis tools

## Design Philosophy: Flexibility First

PMIR prioritizes flexibility and extensibility by closely following MLIR's proven architecture:

### Core Principles

1. **Minimal Core, Rich Ecosystem**: Like MLIR, keep the core simple and extend through dialects
2. **Avoid Over-Specification**: Define mechanisms, not policies
3. **Extensible by Design**: New operations, types, and analyses should be easy to add
4. **Learn from MLIR**: Adopt patterns that have proven successful in MLIR's evolution

### MLIR Concepts We Adopt

1. **Hierarchical Structure**: Module → Function → Region → Block → Operation
2. **SSA Form**: All values are defined exactly once
3. **Dialect System**: Operations organized by domain - easy to add new dialects
4. **Type System**: Extensible type system with custom types
5. **Attributes**: Arbitrary compile-time metadata
6. **Regions**: Nested control flow scopes
7. **Pass Infrastructure**: Composable transformation passes
8. **Pattern Matching**: Declarative optimizations
9. **Interfaces**: Define common behavior across operations
10. **Progressive Lowering**: Transform between abstraction levels

### What We DON'T Prescribe

- Specific quantum operations (add them via dialects as needed)
- Fixed QEC schemes (implement via dialect extensions)
- Particular execution strategies (add backends as needed)
- Rigid type hierarchies (extend the type system as required)

## Core PMIR Design

PMIR provides a minimal, extensible core following MLIR's architecture:

### Minimal Core Structure

```rust
// Core operation type - everything is an operation
pub struct Operation {
    pub name: String,              // Dialect.operation format
    pub operands: Vec<Value>,      // Input values
    pub results: Vec<Value>,       // Output values  
    pub attributes: AttributeMap,  // Extensible metadata
    pub regions: Vec<Region>,      // Nested regions
    pub location: Location,        // Source tracking
}

// Values are just typed references
pub struct Value {
    pub id: ValueId,
    pub ty: Type,
}

// Types are extensible
pub enum Type {
    // Builtin types
    Builtin(BuiltinType),
    // Dialect-specific types
    Dialect { dialect: String, data: String },
}

// Attributes store arbitrary metadata
pub type AttributeMap = HashMap<String, Attribute>;

pub enum Attribute {
    // Basic attributes
    Int(i64),
    Float(f64),
    String(String),
    Type(Type),
    // Composite attributes
    Array(Vec<Attribute>),
    Dict(AttributeMap),
    // Custom attributes
    Custom { dialect: String, data: Box<dyn Any> },
}
```

### Extensibility Through Dialects

```rust
// Dialects group related operations, types, and attributes
pub trait Dialect: Send + Sync {
    fn name(&self) -> &str;
    
    // Register operations
    fn register_ops(&self, registry: &mut OpRegistry) {}
    
    // Register types
    fn register_types(&self, registry: &mut TypeRegistry) {}
    
    // Verify operations
    fn verify_op(&self, op: &Operation) -> Result<(), Error> {
        Ok(()) // Default: no verification
    }
}

// Example: Quantum dialect (not built-in, just an example)
pub struct QuantumDialect;

impl Dialect for QuantumDialect {
    fn name(&self) -> &str { "quantum" }
    
    fn register_ops(&self, registry: &mut OpRegistry) {
        registry.register("quantum.h", |builder| {
            builder.operands(1).results(0).build()
        });
        registry.register("quantum.cx", |builder| {
            builder.operands(2).results(0).build()
        });
        // ... more ops as needed
    }
}

```

### Insights from PHIR

PHIR demonstrates several design principles we should adopt:

1. **Clear Type System**: PHIR distinguishes quantum variables (qubits) from classical variables (integers)
2. **Explicit Data Flow**: Variables must be defined before use, results must be explicitly exported
3. **Structured Operations**: Operations are grouped by type (data, cop, qop, mop, block)
4. **Machine Operations**: First-class support for physical device operations (idle, transport)
5. **Metadata Support**: Operations can carry metadata for error modeling, timing, etc.

### PHIR-Inspired Enhancements

```rust
// PHIR-style variable definition in PMIR
let qvar_define = Operation {
    name: "data.qvar_define".to_string(),
    attributes: hashmap!{
        "variable" => Attribute::String("q".to_string()),
        "size" => Attribute::Int(10),
    },
    results: vec![quantum_register], // Returns the allocated register
    ..Default::default()
};

// PHIR-style measurement with explicit returns
let measure = Operation {
    name: "quantum.measure".to_string(),
    operands: vec![qubit0, qubit1],
    results: vec![bit0, bit1], // Explicit result mapping
    ..Default::default()
};

// PHIR-style classical operations
let assign = Operation {
    name: "cop.assign".to_string(),
    operands: vec![int_expression],
    results: vec![classical_var],
    ..Default::default()
};
```

### Machine Operations Dialect

Following PHIR's lead on machine operations:

```rust
pub struct MachineDialect;

impl Dialect for MachineDialect {
    fn name(&self) -> &str { "mop" }
    
    fn register_ops(&self, registry: &mut OpRegistry) {
        // Idle operation with duration
        registry.register("mop.idle", |builder| {
            builder
                .operands_variadic() // Qubits to idle
                .attribute("duration", AttrType::Float)
                .attribute("unit", AttrType::String)
                .build()
        });
        
        // Transport operation
        registry.register("mop.transport", |builder| {
            builder
                .operands(2) // From and to locations
                .attribute("duration", AttrType::Float)
                .build()
        });
    }
}
```

### Core Infrastructure Only

PMIR provides just the infrastructure needed for IR manipulation:

```rust
// Core components we provide
pub struct Context {
    dialects: HashMap<String, Box<dyn Dialect>>,
    passes: PassManager,
}

// Simple lowering to MLIR text
impl Operation {
    pub fn to_mlir_text(&self) -> String {
        // Just format the operation as MLIR text
        let mut text = String::new();
        
        // Results
        if !self.results.is_empty() {
            text.push_str(&format!("{} = ", 
                self.results.iter()
                    .map(|v| format!("%{}", v.id))
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        }
        
        // Operation name (already in dialect.op format)
        text.push_str(&self.name);
        
        // Operands
        if !self.operands.is_empty() {
            text.push_str(&format!(" {}", 
                self.operands.iter()
                    .map(|v| format!("%{}", v.id))
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        }
        
        // Attributes
        if !self.attributes.is_empty() {
            text.push_str(" {");
            for (k, v) in &self.attributes {
                text.push_str(&format!(" {} = {}", k, v));
            }
            text.push_str(" }");
        }
        
        text
    }
}
```

### Builder API for Natural Construction

```rust
// Natural builder API for quantum programs
pub mod builders {
    pub struct QuantumCircuitBuilder {
        ops: Vec<Operation>,
        current_block: BlockBuilder,
    }
    
    impl QuantumCircuitBuilder {
        // Natural quantum operations
        pub fn h(&mut self, qubit: impl Into<Value>) -> &mut Self {
            self.add_op(quantum::h(qubit.into()));
            self
        }
        
        pub fn cx(&mut self, control: impl Into<Value>, target: impl Into<Value>) -> &mut Self {
            self.add_op(quantum::cx(control.into(), target.into()));
            self
        }
        
        // Natural measurement with result binding
        pub fn measure(&mut self, qubit: impl Into<Value>) -> Value {
            let result = self.next_value(Type::MeasurementResult);
            self.add_op(quantum::measure_to(qubit.into(), result.clone()));
            result
        }
        
        // Natural control flow
        pub fn if_measured(&mut self, result: Value) -> IfBuilder {
            IfBuilder::new(self, result)
        }
        
        // Natural parallel regions
        pub fn parallel_over(&mut self, qubits: Vec<Value>) -> ParallelBuilder {
            ParallelBuilder::new(self, qubits)
        }
    }
}

// Example: Natural quantum algorithm expression
let mut circuit = QuantumCircuitBuilder::new();
let qubits = circuit.allocate_qubits(3);

circuit
    .h(qubits[0])
    .cx(qubits[0], qubits[1])
    .cx(qubits[0], qubits[2]);

let results = circuit.measure_all(&qubits);

circuit.if_measured(results[0])
    .then(|b| b.x(qubits[1]))
    .otherwise(|b| b.z(qubits[1]));
```

### PHIR Block Structures in PMIR

PHIR's block structures map naturally to PMIR's region-based design:

```rust
// PHIR blocks as PMIR regions
pub enum PHIRBlockType {
    // Basic sequence block
    Sequence,
    // Quantum parallel execution
    QParallel,
    // Conditional execution
    IfElse { condition: Value },
}

// Example: PHIR qparallel block becomes PMIR parallel region
// PHIR:
// {
//   "block": "qparallel",
//   "ops": [
//     {"qop": "H", "args": [["q", 0], ["q", 1]]},
//     {"qop": "X", "args": [["q", 2], ["q", 3]]}
//   ]
// }

// PMIR representation:
let qparallel_region = Region {
    blocks: vec![Block {
        operations: vec![
            Operation {
                name: "quantum.parallel".to_string(),
                regions: vec![Region {
                    blocks: vec![Block {
                        operations: vec![
                            quantum::h(q0),
                            quantum::h(q1),
                            quantum::x(q2),
                            quantum::x(q3),
                        ],
                        ..Default::default()
                    }],
                }],
                attributes: hashmap!{
                    "parallel_safe" => Attribute::Bool(true),
                },
                ..Default::default()
            }
        ],
        ..Default::default()
    }],
};
```

### PHIR Result Command Pattern

PHIR's explicit result declaration inspires a clear output specification pattern:

```rust
// PHIR-style Result command for explicit output declaration
pub struct ResultOp {
    // What to export
    sources: Vec<Value>,
    // Export names
    export_names: Vec<String>,
}

impl ResultOp {
    pub fn to_operation(&self) -> Operation {
        Operation {
            name: "data.result".to_string(),
            operands: self.sources.clone(),
            attributes: hashmap!{
                "export_names" => Attribute::StringArray(self.export_names.clone()),
            },
            ..Default::default()
        }
    }
}

// Example: Explicit result export like PHIR
// PHIR: {"cop": "Result", "args": ["m"], "returns": ["output"]}
// PMIR:
let result_op = Operation {
    name: "data.result".to_string(),
    operands: vec![measurement_register],
    attributes: hashmap!{
        "export_names" => Attribute::StringArray(vec!["output".to_string()]),
    },
    ..Default::default()
};
```

### PHIR Foreign Function Calls

PHIR's foreign function call pattern enables external classical computation:

```rust
// PHIR-style foreign function calls
pub struct ForeignCallOp {
    function_name: String,
    arguments: Vec<Value>,
    results: Vec<Value>,
    // Optional hints about the external object
    metadata: HashMap<String, Attribute>,
}

// FFCall dialect for external functions
pub struct FFCallDialect;

impl Dialect for FFCallDialect {
    fn name(&self) -> &str { "ffcall" }
    
    fn register_ops(&self, registry: &mut OpRegistry) {
        // Register foreign function call
        registry.register("ffcall.call", |builder| {
            builder
                .operands_variadic()  // Any number of arguments
                .results_variadic()   // Any number of results
                .attribute("function", AttrType::String)
                .attribute("module", AttrType::String)  // Optional: which module
                .attribute("async", AttrType::Bool)     // Optional: async call
                .build()
        });
        
        // Stateful object interaction
        registry.register("ffcall.stateful", |builder| {
            builder
                .operands_variadic()
                .results_variadic()
                .attribute("object", AttrType::String)
                .attribute("method", AttrType::String)
                .attribute("state_id", AttrType::Int)  // Track state
                .build()
        });
    }
}

// Example: PHIR ffcall in PMIR
// PHIR: {"cop": "ffcall", "function": "add", "args": ["b", "c"], "returns": [["a", 0]]}
let ffcall = Operation {
    name: "ffcall.call".to_string(),
    operands: vec![b_value, c_value],
    results: vec![OpResult::new(Type::I64)],
    attributes: hashmap!{
        "function" => Attribute::String("add".to_string()),
    },
    ..Default::default()
};

// Async FFCall with state
let async_call = Operation {
    name: "ffcall.stateful".to_string(),
    operands: vec![input_data],
    results: vec![future_handle],
    attributes: hashmap!{
        "object" => Attribute::String("decoder".to_string()),
        "method" => Attribute::String("decode_syndrome".to_string()),
        "async" => Attribute::Bool(true),
        "state_id" => Attribute::Int(decoder_id),
    },
    ..Default::default()
};
```

### Seamless Lowering with Dialect Inference

```rust
pub struct SmartLowering {
    dialect_registry: DialectRegistry,
    type_converter: TypeConverter,
}

impl SmartLowering {
    pub fn lower_operation(&self, op: &Operation) -> MLIROperation {
        // Automatically determine the right dialect and format
        let dialect = self.infer_dialect(op);
        let mlir_name = dialect.operation_name(op);
        
        // Convert types appropriately for the target dialect
        let operand_types = op.operands.iter()
            .map(|v| self.type_converter.convert(&v.ty, &dialect))
            .collect();
            
        // Handle special cases transparently
        match (&op.name, &dialect) {
            (OpName::Quantum(QuantumOp::Measure { .. }), QuantumDialect) => {
                // Natural measure becomes quantum.measure
                self.lower_quantum_measure(op)
            }
            (OpName::Quantum(_), QIRDialect) => {
                // Quantum ops become QIR calls when targeting QIR
                self.lower_to_qir_calls(op)
            }
            (OpName::Parallel(ParallelOp::QuantumParallel), _) => {
                // Parallel quantum ops get special handling
                self.lower_parallel_quantum(op)
            }
            _ => {
                // Standard lowering for everything else
                self.standard_lowering(op)
            }
        }
    }
}

// Example: Automatic dialect assignment during lowering
// PMIR:
let op = quantum::h(q0);

// Automatically becomes when lowering to quantum dialect:
"quantum.h %q0 : !quantum.qubit"

// Or when lowering to QIR:
"call @__quantum__qis__h__body(%q0) : (i64) -> ()"

// Or when lowering to matrix ops:
"linalg.matmul %q0_state, %hadamard_matrix : tensor<2x1xcomplex<f64>>, tensor<2x2xcomplex<f64>> -> tensor<2x1xcomplex<f64>>"
```

### Natural Hybrid Constructs

```rust
// PMIR makes quantum-classical interaction natural
pub mod hybrid {
    // Classical control of quantum operations
    pub struct ClassicallyControlled {
        condition: Value,  // Classical boolean
        quantum_op: QuantumOp,
        qubits: Vec<Value>,
    }
    
    // Quantum-influenced classical computation
    pub struct QuantumSampling {
        circuit: Region,
        shots: usize,
        aggregation: AggregationOp,
    }
    
    // Natural expression of VQE-like algorithms
    pub struct VariationalLoop {
        quantum_circuit: Region,
        classical_optimizer: OptimizerOp,
        parameters: Vec<Value>,
        cost_function: Region,
    }
}

// Lowers naturally to appropriate MLIR constructs
impl ClassicallyControlled {
    fn to_mlir(&self) -> String {
        format!(
            "scf.if {} {{\n  {}\n}}",
            self.condition,
            self.quantum_op.to_mlir()
        )
    }
}
```

## Architecture

### Multi-Frontend, Multi-Backend Design
```
    Input Formats                    PAST                     PMIR                    Output Targets
    
    HUGR JSON    ─────┐                                                         ┌──→ Interpreter
                      │                                                          │
    Guppy       ─────┤            ┌─────────┐            ┌──────────┐         │
                      ├──Parser──→│   PAST  │──Lower──→  │   PMIR   │─────────┼──→ Rust Codegen
    OpenQASM 2.0 ────┤            │  (AST)  │            │   (IR)   │         │
                      │            └─────────┘            └──────────┘         │
    LLVM IR     ─────┤                                                          ├──→ MLIR Text → LLVM IR
                      │                                                          │
    Quipper     ─────┘                                                          └──→ Other Backends
```

### Component Roles

#### PAST (PECOS AST)
- **Purpose**: Common AST representation for all input formats
- **Structure**: Tree-like version of PMIR operations
- **Scope**: Direct structural mapping to PMIR but in AST form

#### PMIR (PECOS MLIR)
- **Purpose**: Linear, SSA-based IR for optimization and execution
- **Structure**: Full MLIR structure with modules, functions, regions, blocks, and operations
- **Scope**: Complete IR that can be interpreted or compiled
- **Design**: Follows MLIR's principles but implemented in pure Rust

### Region and Operation Classification

PMIR uses MLIR-style traits and interfaces to classify regions and operations, enabling targeted optimizations:

```rust
// MLIR-style traits for operations
pub trait OpTrait {}

// Traits that classify operations
pub struct QuantumOp;
pub struct ClassicalOp;
pub struct ControlFlowOp;
pub struct DataFlowOp;
pub struct SideEffecting;
pub struct Pure;
pub struct Terminator;
pub struct BranchOp;       // Operations that branch control flow
pub struct RegionOp;       // Operations that contain regions
pub struct MemoryOp;       // Operations that access memory

impl OpTrait for QuantumOp {}
impl OpTrait for ClassicalOp {}
// ... etc

// Operations can have multiple traits
impl Operation {
    pub fn has_trait<T: OpTrait>(&self) -> bool {
        // Check if operation has the trait
    }
    
    pub fn is_quantum(&self) -> bool {
        self.has_trait::<QuantumOp>()
    }
    
    pub fn is_pure(&self) -> bool {
        self.has_trait::<Pure>() && !self.has_trait::<SideEffecting>()
    }
}

// Region traits for analysis and optimization
pub trait RegionTrait {
    fn analyze(&self, region: &Region) -> RegionKind;
}

pub enum RegionKind {
    // Pure regions (no side effects)
    PureQuantum,      // Only quantum operations, no measurements
    PureClassical,    // Only pure classical operations
    
    // Effectful regions
    QuantumMeasurement, // Contains measurements
    ClassicalIO,        // Contains I/O operations
    
    // Control flow regions
    Loop { 
        kind: Box<RegionKind>,  // What kind of computation in the loop
        parallel_safe: bool,     // Can iterations run in parallel
    },
    Conditional {
        then_kind: Box<RegionKind>,
        else_kind: Box<RegionKind>,
    },
    
    // Data flow regions
    Pipeline {           // Operations that can be pipelined
        stages: Vec<RegionKind>,
    },
    
    // Mixed regions
    Hybrid {
        quantum_ops: usize,
        classical_ops: usize,
        has_measurements: bool,
    },
}

// MLIR-style analysis pass
pub struct RegionClassificationPass;

impl Pass for RegionClassificationPass {
    fn run(&mut self, module: &mut Module) -> Result<(), Error> {
        for function in &mut module.functions {
            for region in function.walk_regions() {
                let kind = self.classify_region(region);
                region.set_property("region_kind", kind);
            }
        }
        Ok(())
    }
}

impl RegionClassificationPass {
    fn classify_region(&self, region: &Region) -> RegionKind {
        let mut quantum_ops = 0;
        let mut classical_ops = 0;
        let mut has_measurements = false;
        let mut has_io = false;
        
        for block in &region.blocks {
            for op in &block.operations {
                if op.is_quantum() {
                    quantum_ops += 1;
                    if op.name == OpName::Quantum(QuantumOp::Measure) {
                        has_measurements = true;
                    }
                } else if op.has_trait::<ClassicalOp>() {
                    classical_ops += 1;
                    if op.has_trait::<SideEffecting>() {
                        has_io = true;
                    }
                }
            }
        }
        
        match (quantum_ops, classical_ops, has_measurements, has_io) {
            (q, 0, false, false) if q > 0 => RegionKind::PureQuantum,
            (0, c, false, false) if c > 0 => RegionKind::PureClassical,
            (q, 0, true, false) if q > 0 => RegionKind::QuantumMeasurement,
            (0, c, _, true) if c > 0 => RegionKind::ClassicalIO,
            (q, c, m, _) if q > 0 && c > 0 => RegionKind::Hybrid {
                quantum_ops: q,
                classical_ops: c,
                has_measurements: m,
            },
            _ => RegionKind::PureClassical, // Empty region
        }
    }
}
```

This classification enables:
- **Targeted Optimizations**: Apply quantum gate fusion only to PureQuantum regions
- **Parallelization**: Identify regions that can run in parallel
- **Resource Estimation**: Calculate quantum resources needed per region
- **Execution Strategy**: Choose interpreter vs compilation per region
- **Verification**: Different verification strategies for different region types

### Control Flow vs Data Flow in MLIR Style

PMIR distinguishes between control flow and data flow following MLIR's design:

```rust
// Control flow operations determine execution order
impl ControlFlowOp {
    pub fn successors(&self) -> Vec<BlockId> {
        match self {
            ControlFlowOp::Branch(target) => vec![target],
            ControlFlowOp::CondBranch { then_block, else_block, .. } => {
                vec![then_block, else_block]
            }
            ControlFlowOp::Return => vec![], // No successors
        }
    }
}

// Data flow is explicit through SSA values
pub struct DataFlowAnalysis {
    pub fn trace_value_flow(&self, value: &Value) -> DataFlowGraph {
        let mut graph = DataFlowGraph::new();
        
        // Find definition
        let def_op = value.defining_op();
        graph.add_node(def_op);
        
        // Find all uses
        for use_op in value.uses() {
            graph.add_edge(def_op, use_op);
            
            // If the use is in a different region, mark region boundary
            if def_op.parent_region() != use_op.parent_region() {
                graph.mark_region_crossing(def_op, use_op);
            }
        }
        
        graph
    }
}

// Example: Quantum teleportation with explicit control and data flow
func @teleport(%msg: !quantum.qubit) -> !quantum.qubit {
    // Data flow: Create entangled pair
    %bell0 = quantum.alloc() : !quantum.qubit
    %bell1 = quantum.alloc() : !quantum.qubit
    quantum.h %bell0 : !quantum.qubit
    quantum.cx %bell0, %bell1 : !quantum.qubit, !quantum.qubit
    
    // Data flow: Bell measurement
    quantum.cx %msg, %bell0 : !quantum.qubit, !quantum.qubit
    quantum.h %msg : !quantum.qubit
    %m1 = quantum.measure %msg : !quantum.qubit -> i1
    %m2 = quantum.measure %bell0 : !quantum.qubit -> i1
    
    // Control flow: Classical communication and correction
    scf.if %m2 {
        quantum.x %bell1 : !quantum.qubit
    }
    scf.if %m1 {
        quantum.z %bell1 : !quantum.qubit
    }
    
    // Data flow: Return teleported qubit
    return %bell1 : !quantum.qubit
}
```

This separation enables:
- **SSA Construction**: Data flow is explicit through SSA values
- **Control Flow Analysis**: CFG construction for optimization
- **Region Isolation**: Regions can be analyzed independently
- **Quantum-Classical Boundaries**: Clear data flow across domains

## Parallelism, Async, and Concurrency

PMIR provides first-class support for parallel and asynchronous execution of quantum-classical programs:

### Parallel Execution Model

```rust
// MLIR-style parallel dialect operations
pub mod parallel {
    // Parallel execution of independent operations
    pub struct ParallelOp {
        pub regions: Vec<Region>, // Regions that can execute in parallel
        pub sync_points: Vec<SyncPoint>,
    }
    
    // Async execution with futures
    pub struct AsyncOp {
        pub region: Region,
        pub result_type: Type,
    }
    
    // Await async results
    pub struct AwaitOp {
        pub async_token: Value,
    }
    
    // Thread-local quantum resources
    pub struct QuantumContextOp {
        pub num_qubits: usize,
        pub thread_id: Option<ThreadId>,
    }
}

// Attributes for parallel execution
pub enum ParallelAttribute {
    // Execution strategy
    ExecutionMode(ExecutionMode),
    // Resource requirements
    ResourceRequirements {
        quantum_threads: usize,
        classical_threads: usize,
        memory_per_thread: usize,
    },
    // Synchronization requirements
    SyncMode(SyncMode),
}

pub enum ExecutionMode {
    // Single quantum context, parallel classical
    SharedQuantum,
    // Multiple independent quantum contexts
    DistributedQuantum,
    // GPU-accelerated simulation
    GPUAccelerated,
    // Distributed across multiple nodes
    Distributed { nodes: Vec<NodeId> },
}

pub enum SyncMode {
    // Barrier synchronization
    Barrier,
    // Message passing
    MessagePassing,
    // Lock-free algorithms
    LockFree,
}
```

### Quantum Resource Management

```rust
// Quantum resources are managed per execution context
pub struct QuantumResourceManager {
    // Thread-local quantum simulators
    thread_local_simulators: ThreadLocal<QuantumSimulator>,
    // Shared quantum state (for certain algorithms)
    shared_state: Arc<RwLock<SharedQuantumState>>,
    // Resource allocation tracking
    allocations: DashMap<ThreadId, ResourceAllocation>,
}

// MLIR operations for resource management
pub mod quantum_resource {
    // Allocate quantum resources for a thread/task
    pub struct AllocateContextOp {
        pub num_qubits: Value,
        pub properties: ContextProperties,
    }
    
    // Transfer quantum state between contexts
    pub struct TransferStateOp {
        pub from_context: Value,
        pub to_context: Value,
        pub qubit_range: Range<usize>,
    }
    
    // Synchronize quantum operations across threads
    pub struct QuantumBarrierOp {
        pub contexts: Vec<Value>,
    }
}
```

### Parallel Pattern Recognition

```rust
// Analysis pass to identify parallelization opportunities
pub struct ParallelizationAnalysisPass;

impl Pass for ParallelizationAnalysisPass {
    fn run(&mut self, module: &mut Module) -> Result<(), Error> {
        for function in &mut module.functions {
            // Identify parallel patterns
            let patterns = self.identify_patterns(function);
            
            for pattern in patterns {
                match pattern {
                    ParallelPattern::IndependentCircuits(circuits) => {
                        self.parallelize_independent_circuits(function, circuits)?;
                    }
                    ParallelPattern::DataParallelLoop(loop_op) => {
                        self.parallelize_data_parallel_loop(function, loop_op)?;
                    }
                    ParallelPattern::QuantumClassicalPipeline(stages) => {
                        self.create_pipeline(function, stages)?;
                    }
                    ParallelPattern::MonteCarloSimulation(config) => {
                        self.parallelize_monte_carlo(function, config)?;
                    }
                }
            }
        }
        Ok(())
    }
}

pub enum ParallelPattern {
    // Multiple quantum circuits that don't share qubits
    IndependentCircuits(Vec<Region>),
    // Loops where iterations are independent
    DataParallelLoop(LoopOp),
    // Pipeline of quantum and classical stages
    QuantumClassicalPipeline(Vec<Stage>),
    // Monte Carlo simulations (e.g., VQE)
    MonteCarloSimulation(MonteCarloConfig),
}
```

### Example: Parallel VQE Implementation

```mlir
module {
  // Parallel VQE with async execution
  func @parallel_vqe(%params: tensor<16xf64>, %num_shots: i32) -> f64 {
    %num_threads = arith.constant 4 : i32
    
    // Create parallel quantum contexts
    %contexts = parallel.create_contexts %num_threads : (i32) -> !parallel.context_array
    
    // Parallel execution of quantum circuits
    %energies = parallel.map %contexts, %params {
      ^bb0(%ctx: !parallel.context, %param_slice: tensor<4xf64>):
        // Each thread gets its own quantum simulator
        %qubits = quantum.alloc_in_context %ctx, 4 : (!parallel.context, i32) -> !quantum.reg<4>
        
        // Prepare quantum state
        quantum.h %qubits[0] : !quantum.reg<4>
        quantum.h %qubits[1] : !quantum.reg<4>
        
        // Apply parameterized gates (async)
        %future = async.execute {
          affine.for %i = 0 to 4 {
            %angle = tensor.extract %param_slice[%i] : tensor<4xf64>
            quantum.rz %angle, %qubits[%i] : f64, !quantum.reg<4>
          }
          async.yield
        }
        
        // Measure expectation value
        async.await %future : !async.token
        %energy = quantum.measure_expectation %qubits, @hamiltonian : !quantum.reg<4> -> f64
        
        parallel.yield %energy : f64
    } : tensor<4xf64>
    
    // Reduce results
    %total = parallel.reduce %energies, "sum" : tensor<4xf64> -> f64
    %avg = arith.divf %total, %num_threads : f64
    
    return %avg : f64
  }
}
```

### Async Quantum-Classical Protocols

```rust
// Async execution of quantum protocols
pub struct AsyncQuantumProtocol {
    // Async quantum operations return futures
    pub async fn execute_quantum_subroutine(&self) -> Result<QuantumResult> {
        // Quantum operations can be async
        let future = self.quantum_executor.submit(|sim| {
            sim.apply_circuit(&self.circuit)?;
            sim.measure_all()
        });
        
        // Can do classical work while waiting
        let classical_result = self.classical_preprocessing().await?;
        
        // Await quantum result
        let quantum_result = future.await?;
        
        Ok(self.combine_results(classical_result, quantum_result))
    }
}

// MLIR representation
func @async_protocol() -> i32 {
    // Start async quantum execution
    %quantum_future = async.execute {
        %q = quantum.alloc 10 : !quantum.reg<10>
        quantum.h %q[0] : !quantum.reg<10>
        %result = quantum.measure %q : !quantum.reg<10> -> i32
        async.yield %result : i32
    } : !async.future<i32>
    
    // Parallel classical computation
    %classical_future = async.execute {
        %data = arith.constant dense<[1,2,3,4]> : tensor<4xi32>
        %sum = linalg.reduce %data : tensor<4xi32> -> i32
        async.yield %sum : i32
    } : !async.future<i32>
    
    // Wait for both results
    %q_result = async.await %quantum_future : !async.future<i32>
    %c_result = async.await %classical_future : !async.future<i32>
    
    %final = arith.addi %q_result, %c_result : i32
    return %final : i32
}
```

### Thread-Safe Quantum Operations

```rust
// Thread-safe quantum operation execution
pub struct ThreadSafeQuantumExecutor {
    // Per-thread quantum simulators
    simulators: ThreadLocal<RefCell<QuantumSimulator>>,
    // Shared state for entangled systems
    shared_states: DashMap<StateId, Arc<RwLock<QuantumState>>>,
    // Synchronization primitives
    barriers: DashMap<BarrierId, Arc<Barrier>>,
}

impl ThreadSafeQuantumExecutor {
    pub fn execute_parallel_circuits(
        &self,
        circuits: Vec<QuantumCircuit>,
    ) -> Vec<Result<MeasurementResult>> {
        circuits
            .into_par_iter()
            .map(|circuit| {
                self.simulators.get_or(|| {
                    RefCell::new(QuantumSimulator::new())
                }).borrow_mut().execute_circuit(&circuit)
            })
            .collect()
    }
}
```

### Distributed Quantum Simulation

```rust
// Distributed execution across multiple nodes
pub mod distributed {
    pub struct DistributedQuantumOp {
        pub partition_strategy: PartitionStrategy,
        pub nodes: Vec<NodeId>,
    }
    
    pub enum PartitionStrategy {
        // Split quantum state across nodes
        StateVectorPartitioning { 
            qubits_per_node: usize 
        },
        // Each node simulates different parameters
        ParameterSweep {
            parameter_ranges: Vec<Range<f64>>
        },
        // Shot-based parallelism
        ShotDistribution {
            shots_per_node: usize
        },
    }
}

// Example: Distributed quantum simulation
func @distributed_simulation(%num_nodes: i32) -> tensor<1000xf64> {
    %nodes = distributed.get_nodes %num_nodes : (i32) -> !distributed.node_array
    
    %results = distributed.map %nodes {
        ^bb0(%node: !distributed.node):
            // Each node simulates part of the quantum system
            %local_qubits = distributed.get_local_qubits %node : (!distributed.node) -> !quantum.reg<?>
            
            // Local quantum operations
            quantum.h %local_qubits : !quantum.reg<?>
            
            // Distributed CNOT requires communication
            distributed.quantum_comm %node {
                quantum.cx %local_qubits[0], %remote_qubits[0] : !quantum.reg<?>, !quantum.reg<?>
            }
            
            %local_result = quantum.measure %local_qubits : !quantum.reg<?> -> tensor<250xf64>
            distributed.yield %local_result : tensor<250xf64>
    } : tensor<1000xf64>
    
    return %results : tensor<1000xf64>
}
```

This comprehensive parallelism support enables:
- **Parallel quantum circuit execution** across multiple simulators
- **Async quantum-classical protocols** for better resource utilization  
- **Thread-safe quantum operations** with proper synchronization
- **Distributed quantum simulation** across multiple nodes
- **GPU acceleration** for large-scale simulations
- **Automatic parallelization** of suitable patterns
- **Resource-aware scheduling** of quantum and classical tasks

### Quantum-Specific Parallelism Patterns

```rust
// Common quantum parallel patterns
pub mod quantum_parallel_patterns {
    
    // Parameter sweep parallelism (VQE, QAOA)
    pub struct ParameterSweepPattern {
        pub circuit_template: QuantumCircuit,
        pub parameter_sets: Vec<ParameterSet>,
        pub aggregation: AggregationMethod,
    }
    
    // Shot-based parallelism for sampling
    pub struct ShotParallelismPattern {
        pub circuit: QuantumCircuit,
        pub total_shots: usize,
        pub shots_per_thread: usize,
    }
    
    // Quantum circuit cutting for parallel execution
    pub struct CircuitCuttingPattern {
        pub original_circuit: QuantumCircuit,
        pub cut_points: Vec<CutPoint>,
        pub reconstruction_method: ReconstructionMethod,
    }
    
    // Parallel quantum error mitigation
    pub struct ErrorMitigationPattern {
        pub base_circuit: QuantumCircuit,
        pub mitigation_circuits: Vec<QuantumCircuit>,
        pub combination_strategy: CombinationStrategy,
    }
}

// Example: Parallel quantum phase estimation
func @parallel_qpe(%unitary: !quantum.unitary, %precision: i32) -> f64 {
    %num_parallel = arith.constant 8 : i32
    
    // Split precision bits across parallel executions
    %bits_per_thread = arith.divui %precision, %num_parallel : i32
    
    %phase_estimates = parallel.for %i = 0 to %num_parallel {
        // Each thread estimates different phase bits
        %offset = arith.muli %i, %bits_per_thread : i32
        
        // Allocate quantum resources per thread
        parallel.quantum_context {
            %ancilla = quantum.alloc %bits_per_thread : !quantum.reg<?>
            %target = quantum.alloc 1 : !quantum.qubit
            
            // Prepare eigenstate (parallel safe)
            quantum.prepare_eigenstate %target : !quantum.qubit
            
            // Parallel phase kickback
            affine.for %j = 0 to %bits_per_thread {
                %power = arith.shli %c1, %j : i32
                quantum.controlled_unitary %ancilla[%j], %target, %unitary, %power
            }
            
            // QFT on ancilla qubits
            quantum.qft %ancilla : !quantum.reg<?>
            
            // Measure phase bits
            %bits = quantum.measure %ancilla : !quantum.reg<?> -> i32
            %shifted = arith.shli %bits, %offset : i32
            
            parallel.yield %shifted : i32
        }
    } : tensor<?xi32>
    
    // Combine phase estimates
    %combined = parallel.reduce %phase_estimates, "or" : tensor<?xi32> -> i32
    %phase = arith.uitofp %combined : i32 to f64
    %normalized = arith.divf %phase, %c2pi : f64
    
    return %normalized : f64
}
```

### Synchronization Primitives

```rust
// Quantum-aware synchronization
pub mod quantum_sync {
    // Quantum barrier - ensures all quantum operations complete
    pub struct QuantumBarrier {
        pub participating_threads: Vec<ThreadId>,
        pub quantum_state_sync: bool,
    }
    
    // Classical-quantum synchronization point
    pub struct ClassicalQuantumSync {
        pub sync_type: SyncType,
        pub timeout: Option<Duration>,
    }
    
    pub enum SyncType {
        // Wait for all measurements to complete
        MeasurementBarrier,
        // Wait for quantum state preparation
        StatePreparationBarrier,
        // Synchronize before entangling operations
        EntanglementSync,
        // Wait for classical post-processing
        ClassicalProcessingSync,
    }
}

// Example: Synchronized distributed quantum algorithm
func @synchronized_shor(%n: i64, %num_nodes: i32) -> i64 {
    %nodes = distributed.init_nodes %num_nodes : (i32) -> !distributed.node_array
    
    // Phase 1: Parallel period finding
    %periods = distributed.parallel_map %nodes {
        ^bb0(%node: !distributed.node):
            %a = distributed.get_random_coprime %n, %node : (i64, !distributed.node) -> i64
            
            // Local quantum computation
            %local_period = quantum.order_finding %a, %n : (i64, i64) -> i64
            
            distributed.yield %local_period : i64
    } : tensor<?xi64>
    
    // Synchronization point - all nodes must complete
    distributed.barrier %nodes : !distributed.node_array
    
    // Phase 2: Classical GCD computation (single node)
    %factors = distributed.on_master %nodes {
        %gcd_result = classical.gcd_of_periods %periods, %n : (tensor<?xi64>, i64) -> i64
        distributed.broadcast %gcd_result : i64
    } : i64
    
    return %factors : i64
}
```

### Resource Scheduling

```rust
// Quantum resource scheduler
pub struct QuantumResourceScheduler {
    // Available quantum processing units
    qpus: Vec<QPU>,
    // Task queue with priorities
    task_queue: PriorityQueue<QuantumTask>,
    // Resource allocation strategy
    strategy: SchedulingStrategy,
}

pub enum SchedulingStrategy {
    // Minimize total execution time
    MinimizeLatency,
    // Maximize throughput
    MaximizeThroughput,
    // Balance load across QPUs
    LoadBalancing,
    // Energy-aware scheduling
    EnergyEfficient,
}

impl QuantumResourceScheduler {
    pub fn schedule_tasks(&mut self) -> Schedule {
        match self.strategy {
            SchedulingStrategy::MinimizeLatency => {
                // Schedule critical path first
                self.schedule_critical_path()
            }
            SchedulingStrategy::MaximizeThroughput => {
                // Pack tasks to maximize QPU utilization
                self.bin_packing_schedule()
            }
            // ... other strategies
        }
    }
}

// MLIR scheduling directives
module attributes {
    quantum.scheduling = #quantum.schedule<{
        strategy = "minimize_latency",
        max_parallel_qpus = 4,
        enable_circuit_cutting = true,
        memory_limit_per_qpu = "16GB"
    }>
} {
    func @scheduled_execution() {
        // Scheduler will optimize this execution
    }
}
```

## PAST: HUGR-Inspired Hierarchical AST

PAST should adopt HUGR's hierarchical design for a natural tree representation:

```rust
// HUGR-inspired hierarchical AST structure
pub struct PAST {
    // The root node of the AST
    root: NodeId,
    // All nodes indexed by ID
    nodes: HashMap<NodeId, PastNode>,
    // Parent-child relationships (like HUGR's hierarchy edges)
    hierarchy: HierarchyMap,
    // Dataflow edges for reference resolution
    dataflow: DataflowMap,
}

pub struct HierarchyMap {
    // Every node knows its parent (except root)
    parent: HashMap<NodeId, NodeId>,
    // Container nodes know their ordered children
    children: HashMap<NodeId, Vec<NodeId>>,
}

// PAST nodes are more AST-like than PMIR operations
pub struct PastNode {
    pub id: NodeId,
    pub kind: NodeKind,
    pub weight: NodeWeight,
}

pub enum NodeKind {
    // Container nodes (can have children)
    Module { name: String },
    Function { name: String, signature: Signature },
    Block { kind: BlockKind },
    Loop { condition: Option<NodeId> },
    Conditional { condition: NodeId },
    
    // Leaf nodes (operations)
    QuantumOp { op: QuantumOp, operands: Vec<PortRef> },
    ClassicalOp { op: ClassicalOp, operands: Vec<PortRef> },
    Constant { value: Value },
    Variable { name: String, ty: Type },
    
    // Port nodes (for complex wiring)
    Input { types: TypeRow },
    Output { types: TypeRow },
}

// Like HUGR, use ports for precise connections
pub struct PortRef {
    node: NodeId,
    port: PortIndex,
}

// Node weights carry metadata (like HUGR)
pub struct NodeWeight {
    // Source location for error reporting
    source_location: SourceLocation,
    // Type information
    signature: Option<Signature>,
    // Parser metadata
    parse_info: ParseInfo,
}

// HUGR-style sibling graphs for each scope
impl PAST {
    pub fn sibling_graph(&self, parent: NodeId) -> SiblingGraph {
        let children = &self.hierarchy.children[&parent];
        SiblingGraph {
            parent,
            nodes: children.clone(),
            // Only edges between siblings are included
            edges: self.dataflow.edges_between(children),
        }
    }
    
    // Natural tree traversal
    pub fn walk_preorder(&self) -> impl Iterator<Item = &PastNode> {
        PreorderWalk::new(self, self.root)
    }
    
    // Convert to linear PMIR
    pub fn lower_to_pmir(&self) -> Result<PMIRModule, Error> {
        let mut builder = PMIRBuilder::new();
        self.lower_node(&mut builder, self.root)?;
        builder.finish()
    }
}

// Builder pattern for constructing PAST (like HUGR's builders)
pub struct PASTBuilder {
    ast: PAST,
    current_parent: NodeId,
}

impl PASTBuilder {
    pub fn new() -> Self {
        let mut ast = PAST::new();
        let root = ast.add_node(NodeKind::Module { 
            name: "main".to_string() 
        });
        Self { ast, current_parent: root }
    }
    
    // HUGR-style nested building
    pub fn with_function<F>(&mut self, name: &str, sig: Signature, f: F) -> NodeId
    where F: FnOnce(&mut Self)
    {
        let func_id = self.add_child(NodeKind::Function { name: name.to_string(), signature: sig });
        let saved_parent = self.current_parent;
        self.current_parent = func_id;
        f(self);
        self.current_parent = saved_parent;
        func_id
    }
    
    // Add a child to current parent
    fn add_child(&mut self, kind: NodeKind) -> NodeId {
        let id = self.ast.add_node(kind);
        self.ast.hierarchy.add_child(self.current_parent, id);
        id
    }
}
```

### Why HUGR's Design Fits PAST Better

1. **Natural Tree Structure**: ASTs are inherently trees, and HUGR's hierarchy edges explicitly represent this
2. **Container Nodes**: HUGR's distinction between container and leaf nodes maps perfectly to AST structure
3. **Sibling Graphs**: Each scope in an AST has its own namespace - HUGR's sibling graphs capture this
4. **Preserved Source Structure**: HUGR maintains the hierarchical structure from source, which PAST needs
5. **Flexible References**: HUGR's combination of hierarchy and dataflow edges allows both tree structure and cross-references

### PAST to PMIR Lowering

The hierarchical PAST structure makes lowering to linear PMIR straightforward:

```rust
impl PAST {
    fn lower_node(&self, builder: &mut PMIRBuilder, node_id: NodeId) -> Result<Vec<Value>, Error> {
        let node = &self.nodes[&node_id];
        match &node.kind {
            NodeKind::Function { name, signature } => {
                builder.define_function(name, signature.clone(), |fb| {
                    // Lower function body children in order
                    for child in self.hierarchy.children[&node_id].iter() {
                        self.lower_node(fb, *child)?;
                    }
                    Ok(())
                })
            }
            NodeKind::QuantumOp { op, operands } => {
                // Resolve operands through dataflow edges
                let values = self.resolve_operands(operands)?;
                builder.add_operation(op.clone(), values)
            }
            NodeKind::Block { kind } => {
                builder.create_block(kind, |bb| {
                    // Lower block contents
                    for child in self.hierarchy.children[&node_id].iter() {
                        self.lower_node(bb, *child)?;
                    }
                    Ok(())
                })
            }
            // ... other node types
        }
    }
}
```

## Frontend Parsers

Each input format has its own parser that produces PAST:

### 1. HUGR Parser (existing)
```rust
pub trait QuantumParser {
    fn parse(&self, input: &str) -> Result<PastModule, ParseError>;
}

pub struct HugrParser;
impl QuantumParser for HugrParser {
    fn parse(&self, input: &str) -> Result<PastModule, ParseError> {
        // Current implementation
    }
}
```

### 2. OpenQASM 2.0 Parser
```rust
pub struct QasmParser;
impl QuantumParser for QasmParser {
    fn parse(&self, input: &str) -> Result<PastModule, ParseError> {
        // Parse OpenQASM 2.0 syntax
        // qreg q[2];
        // creg c[2];
        // h q[0];
        // cx q[0], q[1];
        // measure q -> c;
    }
}
```

### 3. LLVM IR Parser
```rust
pub struct LlvmIrParser;
impl QuantumParser for LlvmIrParser {
    fn parse(&self, input: &str) -> Result<PastModule, ParseError> {
        // Parse LLVM IR with quantum intrinsics
        // %q0 = call i64 @__quantum__rt__qubit_allocate()
        // call void @__quantum__qis__h__body(i64 %q0)
    }
}
```

### 4. Guppy Parser
```rust
pub struct GuppyParser;
impl QuantumParser for GuppyParser {
    fn parse(&self, input: &str) -> Result<PastModule, ParseError> {
        // Parse Guppy Python-like syntax
        // Either parse Guppy source or Guppy's HUGR output
    }
}
```

### Unified Parser Interface
```rust
pub struct UniversalParser {
    parsers: HashMap<String, Box<dyn QuantumParser>>,
}

impl UniversalParser {
    pub fn parse(&self, format: &str, input: &str) -> Result<PastModule, ParseError> {
        self.parsers
            .get(format)
            .ok_or(ParseError::UnsupportedFormat(format.to_string()))?
            .parse(input)
    }
    
    pub fn detect_format(&self, input: &str) -> Option<String> {
        // Auto-detect format based on content
        if input.starts_with("{") && input.contains("\"modules\"") {
            Some("hugr".to_string())
        } else if input.contains("OPENQASM 2.0") {
            Some("qasm2".to_string())
        } else if input.contains("@__quantum__") {
            Some("llvm".to_string())
        } else {
            None
        }
    }
}
```

## Design Goals

### 1. Complete Classical Support
PMIR should support all classical operations needed for quantum algorithms:
- Arithmetic operations (integer, floating-point)
- Bitwise operations
- Comparison and logical operations
- Control flow (branches, loops)
- Memory operations (allocation, load, store)
- Function calls

### 2. Rich Type System
```rust
pub enum MlirType {
    // Quantum types
    Qubit,
    QubitArray(usize),
    
    // Classical types
    I1,  // Boolean
    I8, I16, I32, I64,
    F32, F64,
    
    // Composite types
    Array(Box<MlirType>, usize),
    Tuple(Vec<MlirType>),
    Pointer(Box<MlirType>),
    
    // Function types
    Function(Vec<MlirType>, Vec<MlirType>),
}
```

### 3. MLIR-Style Operations
```rust
// MLIR-style operation definition
pub struct Operation {
    // Operation name (e.g., "arith.addi", "quantum.h")
    pub name: OpName,
    // Results produced by this operation
    pub results: Vec<OpResult>,
    // Operands consumed by this operation
    pub operands: Vec<Value>,
    // Compile-time attributes
    pub attributes: Attributes,
    // Nested regions (for control flow)
    pub regions: Vec<Region>,
    // Source location for debugging
    pub location: Location,
}

// Dialect-based operation names
pub enum OpName {
    // Standard dialect
    Std(StdOp),
    // Arithmetic dialect
    Arith(ArithOp),
    // Quantum dialect
    Quantum(QuantumOp),
    // Control flow dialect
    Cf(ControlFlowOp),
    // SCF (Structured Control Flow) dialect
    Scf(ScfOp),
    // Memory dialect
    Memref(MemrefOp),
    // LLVM dialect (for lowering)
    LLVM(LLVMOp),
    // Custom dialect
    Custom(String),
}

pub enum QuantumOp {
    H,
    X, Y, Z,
    CX, CY, CZ,
    RX(f64), RY(f64), RZ(f64),
    Measure,
    Reset,
}

pub enum NoiseOp {
    Depolarizing { target: QubitRef, probability: f64 },
    AmplitudeDamping { target: QubitRef, gamma: f64 },
    PhaseDamping { target: QubitRef, gamma: f64 },
    TwoQubitDepolarizing { targets: [QubitRef; 2], probability: f64 },
    KrausChannel { targets: Vec<QubitRef>, operators: Vec<KrausOperator> },
}
```

### 4. MLIR-Style Type and Value System

```rust
// Values in SSA form
pub struct Value {
    // Unique identifier in the function
    pub id: ValueId,
    // Type of this value
    pub ty: Type,
    // Defining operation (or block argument)
    pub def: ValueDef,
}

pub enum ValueDef {
    // Result of an operation
    OpResult { op: OperationId, index: usize },
    // Block argument
    BlockArg { block: BlockId, index: usize },
    // Constant
    Constant(Attribute),
}

// Attributes for compile-time values
pub enum Attribute {
    // Primitive attributes
    Bool(bool),
    Integer { value: i64, ty: IntegerType },
    Float { value: f64, ty: FloatType },
    String(String),
    
    // Composite attributes
    Array(Vec<Attribute>),
    Dictionary(HashMap<String, Attribute>),
    
    // Type attribute
    Type(Type),
    
    // Symbol reference
    SymbolRef(String),
}

// MLIR-style locations for debugging
pub enum Location {
    Unknown,
    FileLineCol { file: String, line: u32, col: u32 },
    Name(String),
    Fused(Vec<Location>),
}
```

## Execution Models

### 1. Direct Interpreter
```rust
pub struct PmirInterpreter {
    // Quantum state simulator
    quantum_sim: Box<dyn QuantumSimulator>,
    
    // Classical memory
    memory: Memory,
    
    // SSA value storage
    values: HashMap<String, Value>,
    
    // Control flow state
    current_block: String,
    call_stack: Vec<CallFrame>,
}

impl PmirInterpreter {
    pub fn execute_module(&mut self, module: &MlirModule) -> Result<Value> {
        // Find entry point
        let main_func = module.get_function("main")?;
        self.execute_function(main_func, vec![])
    }
    
    pub fn execute_operation(&mut self, op: &MlirOperation) -> Result<()> {
        match &op.opcode {
            MlirOpcode::Quantum(q_op) => self.execute_quantum_op(q_op, &op.operands),
            MlirOpcode::Arithmetic(a_op) => self.execute_arithmetic_op(a_op, &op.operands),
            // ... other operation types
        }
    }
}
```

### 2. Rust Code Generation (Inline Compilation)

This approach generates Rust source code that directly calls simulation functions, then compiles it with `rustc` for native performance.

```rust
pub struct RustCodegen {
    output: String,
    indent_level: usize,
    value_counter: usize,
}

impl RustCodegen {
    pub fn generate_module(&mut self, module: &MlirModule) -> Result<String> {
        self.writeln("use pecos_qsim::prelude::*;");
        self.writeln("use pecos_core::prelude::*;");
        self.writeln("");
        
        for func in &module.functions {
            self.generate_function(func)?;
        }
        
        Ok(self.output.clone())
    }
    
    pub fn generate_operation(&mut self, op: &MlirOperation) -> Result<()> {
        match &op.opcode {
            MlirOpcode::Quantum(QuantumOp::H) => {
                let qubit = self.operand_to_rust(&op.operands[0]);
                let result = self.next_value();
                self.writeln(&format!("let {} = sim.hadamard({});", result, qubit));
                self.store_result(&op.results[0], result);
            }
            MlirOpcode::Quantum(QuantumOp::CX) => {
                let control = self.operand_to_rust(&op.operands[0]);
                let target = self.operand_to_rust(&op.operands[1]);
                self.writeln(&format!("sim.cnot({}, {});", control, target));
            }
            MlirOpcode::Arithmetic(ArithOp::AddI32) => {
                let lhs = self.operand_to_rust(&op.operands[0]);
                let rhs = self.operand_to_rust(&op.operands[1]);
                let result = self.next_value();
                self.writeln(&format!("let {} = {} + {};", result, lhs, rhs));
                self.store_result(&op.results[0], result);
            }
            // ... other operations
        }
        Ok(())
    }
}
```

#### Example Generated Rust Code:
```rust
use pecos_qsim::prelude::*;
use pecos_core::prelude::*;

fn main() -> Result<Vec<u32>, PecosError> {
    let mut sim = QuantumSimulator::new(2);
    
    // Quantum operations
    sim.hadamard(0)?;
    sim.cnot(0, 1)?;
    
    // Conditional noise (compiled out for hardware)
    #[cfg(feature = "noise-simulation")]
    {
        sim.apply_depolarizing_noise(0, 0.001)?;
        sim.apply_depolarizing_noise(1, 0.001)?;
    }
    
    // Measurements
    let m0 = sim.measure(0)?;
    let m1 = sim.measure(1)?;
    
    // Classical computation
    let result = m0 + m1;
    
    Ok(vec![result])
}
```

#### Compilation Pipeline:
```rust
pub struct RustCompiler {
    rustc_path: PathBuf,
    cargo_path: PathBuf,
    temp_dir: TempDir,
}

impl RustCompiler {
    pub fn compile_and_execute(
        &self, 
        pmir_module: &MlirModule,
        shots: usize
    ) -> Result<Vec<ExecutionResult>> {
        // 1. Generate Rust code
        let mut codegen = RustCodegen::new();
        let rust_code = codegen.generate_module(pmir_module)?;
        
        // 2. Create temporary Cargo project
        let project_dir = self.create_temp_project()?;
        std::fs::write(project_dir.join("src/main.rs"), rust_code)?;
        
        // 3. Compile with cargo
        let output = Command::new(&self.cargo_path)
            .current_dir(&project_dir)
            .args(&["build", "--release"])
            .output()?;
            
        // 4. Execute the binary
        let binary = project_dir.join("target/release/quantum_sim");
        let results = self.execute_binary(&binary, shots)?;
        
        Ok(results)
    }
}
```

### 3. Memory Model
```rust
pub struct Memory {
    // Stack frames
    stack: Vec<StackFrame>,
    
    // Heap allocations
    heap: HashMap<usize, HeapObject>,
    
    // Global variables
    globals: HashMap<String, Value>,
}
```

### 4. Data-Oriented State Representation

For high-performance simulation, PMIR uses cache-friendly data layouts:

```rust
// Cache-line aligned state vector
#[repr(align(64))]
pub struct StateVector {
    pub data: Vec<Complex64>,
    pub num_qubits: u32,
}

// Structure-of-Arrays for better SIMD utilization
pub struct StateVectorSOA {
    pub real_parts: AlignedVec<f64>,
    pub imag_parts: AlignedVec<f64>,
    pub num_qubits: u32,
}
```

This enables:
- Better cache utilization
- SIMD vectorization opportunities
- Reduced memory bandwidth requirements

## Natural Lowering Patterns

PMIR provides high-level patterns that lower naturally to MLIR:

### Quantum Algorithm Patterns

```rust
// High-level PMIR patterns for common quantum algorithms
pub mod patterns {
    // VQE pattern - natural in PMIR
    pub struct VQEPattern {
        pub ansatz: QuantumCircuit,
        pub hamiltonian: Hamiltonian,
        pub optimizer: ClassicalOptimizer,
    }
    
    // Lowers to structured MLIR
    impl VQEPattern {
        pub fn lower_to_mlir(&self) -> MLIRModule {
            // Automatically generates:
            // - Parameterized quantum circuit function
            // - Classical optimization loop
            // - Measurement and expectation value computation
            // - Parameter update logic
        }
    }
    
    // Quantum phase estimation - natural expression
    pub struct QPEPattern {
        pub unitary: QuantumOperator,
        pub precision_bits: usize,
        pub eigenstate_prep: Option<QuantumCircuit>,
    }
    
    // Grover search - domain-specific
    pub struct GroverPattern {
        pub oracle: QuantumCircuit,
        pub num_iterations: usize,
    }
}

// These patterns lower to efficient MLIR code
let vqe = VQEPattern {
    ansatz: circuit!{
        for (i, j) in coupling_map {
            ry(theta[i], qubits[i]);
            cx(qubits[i], qubits[j]);
        }
    },
    hamiltonian: H,
    optimizer: GradientDescent::new(),
};

// Automatically becomes optimized MLIR with:
// - Parallel parameter evaluation
// - Batched quantum execution  
// - Efficient classical optimization
```

### Natural Type Conversions

```rust
// PMIR types naturally map to appropriate MLIR types
impl Type {
    pub fn to_mlir_type(&self, context: &LoweringContext) -> String {
        match (self, context.target) {
            // Quantum types adapt to target
            (Type::Qubit, Target::QuantumDialect) => "!quantum.qubit",
            (Type::Qubit, Target::QIR) => "i64",  // QIR convention
            (Type::Qubit, Target::Simulation) => "!sim.qubit_ref",
            
            // High-level types lower appropriately
            (Type::QuantumState { dim }, _) => 
                format!("tensor<{}xcomplex<f64>>", dim),
            (Type::MeasurementResult, Target::QuantumDialect) => 
                "!quantum.result",
            (Type::MeasurementResult, _) => "i1",
            
            // Classical types are straightforward
            (Type::Tensor(elem, shape), _) => 
                format!("tensor<{}x{}>", shape.dims(), elem.to_mlir_type(context)),
        }
    }
}
```

### Automatic Optimization During Lowering

```rust
pub struct OptimizingLowerer {
    patterns: Vec<Box<dyn LoweringPattern>>,
}

impl OptimizingLowerer {
    pub fn lower(&self, op: &Operation) -> Vec<MLIROperation> {
        // Recognize high-level patterns and optimize
        for pattern in &self.patterns {
            if let Some(optimized) = pattern.try_match_and_lower(op) {
                return optimized;
            }
        }
        
        // Default lowering
        vec![self.default_lower(op)]
    }
}

// Example: Bell state preparation pattern
pub struct BellStatePattern;

impl LoweringPattern for BellStatePattern {
    fn try_match_and_lower(&self, op: &Operation) -> Option<Vec<MLIROperation>> {
        // Recognize: H(q0); CX(q0, q1)
        if matches!(op, QuantumCircuit { ops } if is_bell_prep(ops)) {
            // Generate optimized MLIR
            Some(vec![
                mlir_op!("quantum.bell_pair %q0, %q1 : !quantum.qubit, !quantum.qubit"),
            ])
        } else {
            None
        }
    }
}
```

## MLIR Dialect Design

PMIR organizes operations into dialects, following MLIR's approach:

### Core Dialects

1. **Standard Dialect** (`std`)
   - Module and function operations
   - Basic control flow

2. **Arithmetic Dialect** (`arith`)
   - Integer arithmetic: `addi`, `subi`, `muli`, `divi`
   - Floating-point: `addf`, `subf`, `mulf`, `divf`
   - Comparisons: `cmpi`, `cmpf`
   - Constants: `constant`

3. **Quantum Dialect** (`quantum`)
   ```mlir
   // Quantum types
   !quantum.qubit
   !quantum.reg<n>
   !quantum.result
   
   // Quantum operations
   %q = quantum.alloc() : !quantum.qubit
   quantum.h %q : !quantum.qubit
   quantum.cx %q0, %q1 : !quantum.qubit, !quantum.qubit
   %r = quantum.measure %q : !quantum.qubit -> !quantum.result
   ```

4. **Control Flow Dialect** (`cf`)
   - `cf.br`: Unconditional branch
   - `cf.cond_br`: Conditional branch
   - `cf.switch`: Multi-way branch

5. **SCF Dialect** (`scf`)
   - `scf.for`: For loops
   - `scf.while`: While loops
   - `scf.if`: If-then-else

### Custom Dialects

```rust
pub trait Dialect {
    fn name(&self) -> &str;
    fn initialize(&mut self, registry: &mut DialectRegistry);
    fn verify_operation(&self, op: &Operation) -> Result<(), Error>;
}

pub struct QuantumDialect;

impl Dialect for QuantumDialect {
    fn name(&self) -> &str { "quantum" }
    
    fn verify_operation(&self, op: &Operation) -> Result<(), Error> {
        match op.name {
            OpName::Quantum(QuantumOp::CX) => {
                // Verify two qubit operands
                if op.operands.len() != 2 {
                    return Err(Error::InvalidOperandCount);
                }
                // Verify qubit types
                for operand in &op.operands {
                    if !operand.ty.is_qubit() {
                        return Err(Error::TypeMismatch);
                    }
                }
                Ok(())
            }
            // ... other operations
        }
    }
}
```

## Pass Infrastructure

Following MLIR's pass infrastructure design:

```rust
// Base pass trait
pub trait Pass {
    fn name(&self) -> &str;
    fn run(&mut self, module: &mut Module) -> Result<(), Error>;
}

// Pass manager
pub struct PassManager {
    passes: Vec<Box<dyn Pass>>,
}

impl PassManager {
    pub fn add_pass(&mut self, pass: Box<dyn Pass>) {
        self.passes.push(pass);
    }
    
    pub fn run(&mut self, module: &mut Module) -> Result<(), Error> {
        for pass in &mut self.passes {
            pass.run(module)?;
        }
        Ok(())
    }
}

// Example passes
pub struct ConstantFoldingPass;
pub struct DeadCodeEliminationPass;
pub struct QuantumGateFusionPass;
pub struct LoopUnrollingPass;
```

### Analysis Infrastructure

```rust
// Dominance analysis
pub struct DominanceInfo {
    // Implementation
}

// Dataflow analysis framework
pub trait DataflowAnalysis {
    type State;
    fn transfer(&self, op: &Operation, state: &Self::State) -> Self::State;
    fn merge(&self, states: &[Self::State]) -> Self::State;
}
```

## Pattern Matching and Rewriting

MLIR's pattern-based transformations:

```rust
pub trait Pattern {
    fn match_and_rewrite(
        &self,
        op: &Operation,
        rewriter: &mut PatternRewriter
    ) -> Result<(), Error>;
}

pub struct PatternRewriter {
    // Methods for IR manipulation
    pub fn replace_op(&mut self, op: &Operation, new_op: Operation);
    pub fn erase_op(&mut self, op: &Operation);
    pub fn insert_before(&mut self, op: &Operation, new_op: Operation);
}

// Example: Gate fusion pattern
pub struct HadamardFusionPattern;

impl Pattern for HadamardFusionPattern {
    fn match_and_rewrite(
        &self,
        op: &Operation,
        rewriter: &mut PatternRewriter
    ) -> Result<(), Error> {
        // H(H(q)) -> I(q)
        if let OpName::Quantum(QuantumOp::H) = op.name {
            if let Some(prev_op) = op.operands[0].defining_op() {
                if let OpName::Quantum(QuantumOp::H) = prev_op.name {
                    // Two Hadamards cancel out
                    rewriter.replace_op(op, Identity);
                    return Ok(());
                }
            }
        }
        Err(Error::PatternNotMatched)
    }
}
```

## MLIR Text Generation and Lowering

### Lowering PMIR to MLIR Text

Lowering from our MLIR-like PMIR to actual MLIR text is straightforward because PMIR follows MLIR's structure:

```rust
// PMIR to MLIR text lowering
impl Module {
    pub fn to_mlir_text(&self) -> String {
        let mut output = String::new();
        
        // Module attributes
        if !self.attributes.is_empty() {
            output.push_str("module attributes {\n");
            for (key, attr) in &self.attributes {
                output.push_str(&format!("  {} = {}\n", key, attr.to_mlir()));
            }
            output.push_str("} {\n");
        } else {
            output.push_str("module {\n");
        }
        
        // Lower each function
        for function in &self.functions {
            output.push_str(&function.to_mlir_text());
            output.push('\n');
        }
        
        output.push_str("}\n");
        output
    }
}

impl Operation {
    pub fn to_mlir_text(&self) -> String {
        let mut output = String::new();
        
        // Results
        if !self.results.is_empty() {
            let results: Vec<String> = self.results.iter()
                .map(|r| format!("%{}", r.id))
                .collect();
            output.push_str(&results.join(", "));
            output.push_str(" = ");
        }
        
        // Operation name with dialect prefix
        output.push_str(&self.name.to_mlir_string());
        
        // Operands
        if !self.operands.is_empty() {
            output.push(' ');
            let operands: Vec<String> = self.operands.iter()
                .map(|v| v.to_mlir_string())
                .collect();
            output.push_str(&operands.join(", "));
        }
        
        // Attributes
        if !self.attributes.is_empty() {
            output.push_str(" {");
            let attrs: Vec<String> = self.attributes.iter()
                .map(|(k, v)| format!("{} = {}", k, v.to_mlir()))
                .collect();
            output.push_str(&attrs.join(", "));
            output.push('}');
        }
        
        // Type signature
        output.push_str(" : ");
        output.push_str(&self.type_signature());
        
        // Regions (for control flow ops)
        for (i, region) in self.regions.iter().enumerate() {
            if i == 0 {
                output.push_str(" {\n");
            } else {
                output.push_str("} {\n");
            }
            output.push_str(&region.to_mlir_text());
        }
        if !self.regions.is_empty() {
            output.push('}');
        }
        
        output
    }
}
```

### Challenges and Solutions

#### 1. Quantum Operations Lowering

Since standard MLIR doesn't have quantum operations, we need to choose a lowering strategy:

```rust
pub enum QuantumLoweringStrategy {
    // Lower to QIR function calls (current approach)
    QIRFunctionCalls,
    // Use custom quantum dialect (requires MLIR extension)
    QuantumDialect,
    // Lower to matrix operations for simulation
    MatrixOperations,
}

impl QuantumOp {
    pub fn lower_to_mlir(&self, strategy: QuantumLoweringStrategy) -> String {
        match strategy {
            QuantumLoweringStrategy::QIRFunctionCalls => {
                match self {
                    QuantumOp::H => "call @__quantum__qis__h__body",
                    QuantumOp::CX => "call @__quantum__qis__cx__body",
                    // ... etc
                }
            }
            QuantumLoweringStrategy::QuantumDialect => {
                match self {
                    QuantumOp::H => "quantum.h",
                    QuantumOp::CX => "quantum.cx",
                    // ... etc
                }
            }
            QuantumLoweringStrategy::MatrixOperations => {
                // Lower to linalg operations
                match self {
                    QuantumOp::H => "linalg.matmul %hadamard_matrix",
                    // ... etc
                }
            }
        }
    }
}
```

#### 2. Custom Types to MLIR Types

```rust
impl Type {
    pub fn to_mlir_type(&self) -> String {
        match self {
            // Standard types map directly
            Type::I32 => "i32".to_string(),
            Type::F64 => "f64".to_string(),
            
            // Quantum types need mapping
            Type::Qubit => match LOWERING_CONFIG.quantum_type_mapping {
                QuantumTypeMapping::QIR => "i64".to_string(), // QIR uses i64 for qubits
                QuantumTypeMapping::Custom => "!quantum.qubit".to_string(),
            },
            Type::QuantumRegister(n) => format!("!quantum.reg<{}>", n),
            
            // Complex types
            Type::Tensor(shape, elem) => {
                format!("tensor<{}x{}>", shape.join("x"), elem.to_mlir_type())
            }
        }
    }
}
```

#### 3. Parallel/Async Operations

These need to be lowered to appropriate MLIR dialects:

```rust
impl ParallelOp {
    pub fn lower_to_mlir(&self) -> Result<String, Error> {
        match self {
            ParallelOp::ParallelFor { .. } => {
                // Lower to scf.parallel or async dialect
                Ok("scf.parallel".to_string())
            }
            ParallelOp::Async { .. } => {
                // Lower to async dialect
                Ok("async.execute".to_string())
            }
            ParallelOp::QuantumContext { .. } => {
                // Custom lowering for quantum contexts
                // Might need runtime library calls
                Ok("call @__quantum__rt__context_create".to_string())
            }
        }
    }
}
```

### Progressive Lowering Pipeline

```rust
pub struct LoweringPipeline {
    passes: Vec<Box<dyn LoweringPass>>,
}

impl LoweringPipeline {
    pub fn standard_pipeline() -> Self {
        Self {
            passes: vec![
                Box::new(LowerQuantumOpsPass),
                Box::new(LowerParallelOpsPass),
                Box::new(LowerCustomTypesPass),
                Box::new(CanonicalizePass),
                Box::new(VerifyLoweringPass),
            ],
        }
    }
    
    pub fn run(&mut self, module: &mut Module) -> Result<String, Error> {
        // Apply each lowering pass
        for pass in &mut self.passes {
            pass.run(module)?;
        }
        
        // Generate final MLIR text
        Ok(module.to_mlir_text())
    }
}
```

### Example: Complete Lowering

```rust
// Input PMIR
let pmir = Module {
    functions: vec![Function {
        name: "bell_state",
        body: Region {
            blocks: vec![Block {
                operations: vec![
                    Operation {
                        name: OpName::Quantum(QuantumOp::Alloc),
                        results: vec![Value::new("%q0", Type::Qubit)],
                        ..
                    },
                    Operation {
                        name: OpName::Quantum(QuantumOp::H),
                        operands: vec![Value::ref("%q0")],
                        ..
                    },
                    // ... more ops
                ],
            }],
        },
    }],
};

// Lower to MLIR text
let mlir_text = LoweringPipeline::standard_pipeline()
    .run(&mut pmir)?;

// Output MLIR text
/*
module {
  func @bell_state() -> i32 {
    %0 = call @__quantum__rt__qubit_allocate() : () -> i64
    %1 = call @__quantum__rt__qubit_allocate() : () -> i64
    call @__quantum__qis__h__body(%0) : (i64) -> ()
    call @__quantum__qis__cx__body(%0, %1) : (i64, i64) -> ()
    %2 = call @__quantum__qis__m__body(%0) : (i64) -> i1
    %3 = call @__quantum__qis__m__body(%1) : (i64) -> i1
    %4 = arith.extui %2 : i1 to i32
    %5 = arith.extui %3 : i1 to i32
    %6 = arith.addi %4, %5 : i32
    return %6 : i32
  }
}
*/
```

### Verification After Lowering

```rust
pub struct MLIRVerifier {
    pub fn verify_lowered_mlir(&self, mlir_text: &str) -> Result<(), Error> {
        // Check that all operations are from standard MLIR dialects
        // or properly declared external functions
        self.verify_dialects(mlir_text)?;
        self.verify_types(mlir_text)?;
        self.verify_ssa_form(mlir_text)?;
        Ok(())
    }
}
```

The key advantages of our approach:
1. **Structure Preservation**: PMIR follows MLIR structure, so lowering is mostly formatting
2. **Flexible Lowering**: Can target different dialects/representations
3. **Progressive Lowering**: Can lower in stages for debugging
4. **Verification**: Can verify the output is valid MLIR

### Domain-Natural to MLIR Philosophy

PMIR's design philosophy ensures natural quantum-classical expression while maintaining trivial MLIR lowering:

1. **Natural Expression**: Quantum operations, measurements, and classical control feel native
2. **Automatic Dialect Mapping**: Operations know which MLIR dialect they belong to
3. **Smart Type Conversion**: Types adapt based on lowering target (QIR, quantum dialect, simulation)
4. **Pattern Recognition**: High-level patterns (VQE, QPE, Grover) lower to optimized MLIR
5. **Preserving Intent**: The lowering preserves algorithmic intent, not just operations

### PMIR to MLIR Correspondence

Here's how PMIR structures map to MLIR text:

```rust
// PMIR Structure
Module {
    name: "example",
    functions: vec![
        Function {
            name: "quantum_teleport",
            signature: FunctionType {
                inputs: vec![Type::Qubit],
                outputs: vec![Type::Qubit],
            },
            body: Region {
                blocks: vec![
                    Block {
                        label: "entry",
                        operations: vec![
                            Operation {
                                name: OpName::Quantum(QuantumOp::Alloc),
                                results: vec![Value::new("0", Type::Qubit)],
                                operands: vec![],
                                attributes: HashMap::new(),
                                regions: vec![],
                                location: Location::Unknown,
                            },
                            // ... more operations
                        ],
                        terminator: Operation {
                            name: OpName::Cf(ControlFlowOp::Return),
                            operands: vec![Value::ref("5")],
                            ..
                        },
                    },
                ],
            },
        },
    ],
}

// Generates MLIR Text:
module {
  func @quantum_teleport(%arg0: i64) -> i64 {
    ^entry:
      %0 = call @__quantum__rt__qubit_allocate() : () -> i64
      %1 = call @__quantum__rt__qubit_allocate() : () -> i64
      call @__quantum__qis__h__body(%0) : (i64) -> ()
      call @__quantum__qis__cx__body(%0, %1) : (i64, i64) -> ()
      call @__quantum__qis__cx__body(%arg0, %0) : (i64, i64) -> ()
      call @__quantum__qis__h__body(%arg0) : (i64) -> ()
      %2 = call @__quantum__qis__m__body(%arg0) : (i64) -> i1
      %3 = call @__quantum__qis__m__body(%0) : (i64) -> i1
      cf.cond_br %3, ^bb1, ^bb2
    ^bb1:
      call @__quantum__qis__x__body(%1) : (i64) -> ()
      cf.br ^bb3
    ^bb2:
      cf.br ^bb3
    ^bb3:
      cf.cond_br %2, ^bb4, ^bb5
    ^bb4:
      call @__quantum__qis__z__body(%1) : (i64) -> ()
      cf.br ^bb5
    ^bb5:
      return %1 : i64
  }
}
```

### Lowering Configuration

```rust
pub struct LoweringConfig {
    // Which strategy to use for quantum ops
    pub quantum_strategy: QuantumLoweringStrategy,
    // Whether to preserve debug information
    pub preserve_debug_info: bool,
    // Target-specific options
    pub target: LoweringTarget,
    // Optimization level
    pub optimization_level: OptLevel,
}

pub enum LoweringTarget {
    // Standard MLIR for use with mlir-opt
    StandardMLIR,
    // MLIR with custom quantum dialect
    QuantumMLIR,
    // QIR-compatible MLIR
    QIR,
    // Direct to LLVM dialect
    LLVMIR,
}

impl Module {
    pub fn lower_with_config(&self, config: &LoweringConfig) -> Result<String, Error> {
        let mut pipeline = match config.target {
            LoweringTarget::QIR => LoweringPipeline::qir_pipeline(),
            LoweringTarget::QuantumMLIR => LoweringPipeline::quantum_mlir_pipeline(),
            LoweringTarget::StandardMLIR => LoweringPipeline::standard_pipeline(),
            LoweringTarget::LLVMIR => LoweringPipeline::llvm_pipeline(),
        };
        
        pipeline.run(self)
    }
}
```

### Example PMIR with Nested Regions

MLIR's power comes from its ability to represent complex control flow with nested regions:

```mlir
module {
  // Function with multiple region types
  func @quantum_algorithm(%n: i32) -> i32 {
    // Allocate quantum resources
    %qubits = quantum.alloc_register(%n) : (i32) -> !quantum.reg<?>
    
    // Pure quantum region - can be optimized independently
    quantum.circuit %qubits {
      ^bb0:
        quantum.h %qubits[0] : !quantum.reg<?>
        quantum.h %qubits[1] : !quantum.reg<?>
    }
    
    // Classical preprocessing region
    %angle = scf.execute_region -> f64 {
      %pi = arith.constant 3.14159 : f64
      %divisor = arith.uitofp %n : i32 to f64
      %result = arith.divf %pi, %divisor : f64
      scf.yield %result : f64
    }
    
    // Hybrid loop region - contains both quantum and classical
    %final = scf.for %i = %c0 to %n step %c1 iter_args(%acc = %c0) -> i32 {
      // Quantum operations inside loop
      quantum.rz %angle, %qubits[%i] : f64, !quantum.reg<?>
      
      // Conditional quantum operation
      %cond = arith.cmpi eq, %i, %c0 : i32
      scf.if %cond {
        quantum.cx %qubits[%i], %qubits[%i+1] : !quantum.reg<?>, !quantum.reg<?>
      }
      
      // Measurement (quantum → classical boundary)
      %measured = quantum.measure %qubits[%i] : !quantum.reg<?> -> i1
      %m_int = arith.extui %measured : i1 to i32
      
      // Classical accumulation
      %new_acc = arith.addi %acc, %m_int : i32
      scf.yield %new_acc : i32
    }
    
    return %final : i32
  }
}
```

This example shows:
- **Nested regions**: `quantum.circuit`, `scf.execute_region`, `scf.for`, `scf.if`
- **Region types**: Pure quantum, pure classical, hybrid loops
- **Cross-region dataflow**: Values flow between regions
- **Type safety**: Quantum and classical types are distinct

## Execution Strategy Comparison

### Performance Characteristics

| Strategy | Startup Time | Execution Speed | Memory Usage | Use Case |
|----------|-------------|-----------------|--------------|----------|
| **Interpreter** | Fast (ms) | Slow | Low | Development, debugging, small circuits |
| **Rust Codegen** | Slow (seconds) | Fast | Medium | Medium circuits, repeated execution |
| **MLIR/LLVM** | Slow (seconds) | Fast | Low | Production, hardware deployment |

### Feature Support

| Feature | Interpreter | Rust Codegen | MLIR/LLVM |
|---------|------------|--------------|-----------|
| Dynamic circuits | ✓ | ✓ | ✓ |
| Debugging | Excellent | Limited | Limited |
| Optimization | Basic | Rust optimizer | LLVM optimizer |
| Hardware support | ✗ | ✗ | ✓ |
| Distribution | Single binary | Requires rustc | Requires LLVM |

### Choosing the Right Strategy

```rust
pub enum ExecutionStrategy {
    /// Direct interpretation - best for development and debugging
    Interpret,
    
    /// Compile to Rust - best for performance with pure simulation
    RustCodegen {
        optimization_level: OptLevel,
        parallel: bool,
    },
    
    /// Compile to LLVM - best for hardware or production use
    LLVM {
        target: Target,
        optimization_level: u8,
    },
}

impl PmirModule {
    pub fn execute(&self, strategy: ExecutionStrategy) -> Result<ExecutionResult> {
        match strategy {
            ExecutionStrategy::Interpret => {
                let mut interpreter = PmirInterpreter::new();
                interpreter.execute_module(self)
            }
            ExecutionStrategy::RustCodegen { .. } => {
                let compiler = RustCompiler::new();
                compiler.compile_and_execute(self)
            }
            ExecutionStrategy::LLVM { .. } => {
                let llvm_ir = self.to_mlir_text()
                    .and_then(|mlir| compile_mlir_to_llvm(mlir))?;
                execute_llvm(llvm_ir)
            }
        }
    }
}
```

## Shared Operation Set

PAST and PMIR share the same operation set, with PAST providing the tree-like AST representation and PMIR providing the linear SSA form:

```rust
// Shared operation definitions used by both PAST and PMIR
pub mod ops {
    pub enum Operation {
        // Quantum operations
        Quantum(QuantumOp),
        // Classical operations  
        Classical(ClassicalOp),
        // Control flow
        ControlFlow(ControlFlowOp),
        // Memory operations
        Memory(MemoryOp),
        // Noise operations
        Noise(NoiseOp),
    }
}

// PAST uses tree structure
pub struct PastNode {
    pub op: ops::Operation,
    pub children: Vec<NodeId>,
}

// PMIR uses linear SSA structure
pub struct PmirInstruction {
    pub op: ops::Operation,
    pub operands: Vec<SSAValue>,
    pub results: Vec<SSAValue>,
}
```

## MLIR Builder Pattern

PMIR provides a builder API for constructing IR programmatically:

```rust
pub struct OpBuilder {
    insertion_point: InsertionPoint,
    context: &mut Context,
}

impl OpBuilder {
    // Create operations with automatic type inference
    pub fn create_addi(&mut self, lhs: Value, rhs: Value) -> Value {
        let result_type = self.infer_integer_type(&lhs, &rhs);
        let op = Operation {
            name: OpName::Arith(ArithOp::AddI),
            operands: vec![lhs, rhs],
            results: vec![OpResult::new(result_type)],
            ..Default::default()
        };
        self.insert(op);
        op.results[0].as_value()
    }
    
    // Build control flow structures
    pub fn create_if_then_else<F, G>(
        &mut self,
        condition: Value,
        then_builder: F,
        else_builder: G,
    ) -> Vec<Value>
    where
        F: FnOnce(&mut OpBuilder),
        G: FnOnce(&mut OpBuilder),
    {
        let if_op = self.create_scf_if(condition);
        
        // Build then region
        self.set_insertion_point_to_start(&if_op.then_region());
        then_builder(self);
        
        // Build else region
        self.set_insertion_point_to_start(&if_op.else_region());
        else_builder(self);
        
        if_op.results()
    }
}
```

## MLIR Interfaces

Interfaces provide a way to define common behavior across operations:

```rust
// Interface for operations that can be constant-folded
pub trait ConstantFoldable {
    fn fold(&self, operands: &[Attribute]) -> Option<Attribute>;
}

// Interface for side-effect free operations
pub trait Pure {
    fn has_side_effects(&self) -> bool { false }
    fn is_speculatable(&self) -> bool { true }
}

// Interface for operations with memory effects
pub trait MemoryEffects {
    fn memory_effects(&self) -> Effects;
}

pub struct Effects {
    pub reads: Vec<MemorySlot>,
    pub writes: Vec<MemorySlot>,
    pub allocates: Vec<MemorySlot>,
}

// Register interfaces with operations
impl ConstantFoldable for ArithOp {
    fn fold(&self, operands: &[Attribute]) -> Option<Attribute> {
        match (self, &operands[..]) {
            (ArithOp::AddI, [Attribute::Integer { value: a, .. }, 
                            Attribute::Integer { value: b, .. }]) => {
                Some(Attribute::Integer { value: a + b, ty: self.result_type() })
            }
            // ... other cases
            _ => None,
        }
    }
}
```

## Simplified Implementation Approach

### Phase 1: Minimal Core (Start Here)
- [ ] Basic Operation, Block, Region, Module types
- [ ] Simple Value and Type representations
- [ ] Attribute system (just HashMap)
- [ ] Basic MLIR text generation

### Phase 2: Dialect Infrastructure
- [ ] Dialect trait and registry
- [ ] Dynamic operation registration
- [ ] Type extension mechanism
- [ ] Basic verification framework

### Phase 3: Essential Passes
- [ ] Pass manager interface
- [ ] Simple lowering passes
- [ ] Basic verification pass
- [ ] MLIR text output pass

### Phase 4: Builder API
- [ ] MLIR-style builder pattern for constructing IR
- [ ] Type inference in builder
- [ ] Automatic dominance ordering
- [ ] Verification during construction

### Phase 5: Interfaces and Traits
- [ ] Define core interfaces (ConstantFoldable, Pure, etc.)
- [ ] Implement interface registration
- [ ] Add interface-based optimizations
- [ ] Create quantum-specific interfaces

### Phase 6: Pattern Matching Infrastructure
- [ ] Pattern matching framework
- [ ] Rewrite patterns for optimizations
- [ ] Pattern applicator with cost model
- [ ] Greedy pattern rewriter

### Phase 7: Basic Interpreter
- [ ] Implement value storage and SSA management
- [ ] Add quantum operation execution (using existing simulators)
- [ ] Support basic arithmetic and control flow
- [ ] Region and block execution

### Phase 8: Transformations and Lowering
- [ ] Progressive lowering (quantum → QIR calls)
- [ ] Dialect conversion framework
- [ ] Canonicalization patterns
- [ ] Verification after transformations

### Phase 9: Rust Code Generation
- [ ] Basic Rust code generation for quantum operations
- [ ] SSA to Rust variable mapping
- [ ] Integration with existing `pecos-qsim` simulators
- [ ] Compilation and execution pipeline

### Phase 10: Full Classical Support
- [ ] Memory allocation and management
- [ ] Function calls and returns
- [ ] Complex control flow (loops, switches)
- [ ] Support in all three execution strategies

### Phase 11: Advanced Optimizations
- [ ] Quantum circuit optimization (gate fusion, commutation)
- [ ] Classical optimizations (CSE, loop optimizations)
- [ ] Cross-dialect optimizations
- [ ] Profile-guided optimizations
- [ ] Rust codegen optimizations (e.g., batching operations)

### Phase 12: Parallelism and Concurrency
- [ ] Implement parallel dialect operations
- [ ] Thread-safe quantum resource management
- [ ] Async quantum operation support
- [ ] Distributed simulation infrastructure
- [ ] Quantum-classical synchronization primitives
- [ ] Parallel pattern recognition and optimization
- [ ] Resource scheduling algorithms

### Phase 13: QEC and Fault Tolerance
- [ ] QEC-aware type system (logical qubits, syndromes)
- [ ] Fault-tolerant operation synthesis
- [ ] Syndrome extraction scheduling
- [ ] Classical decoder integration
- [ ] Resource estimation backend
- [ ] Multiple QEC code support
- [ ] Lattice surgery operations
- [ ] Magic state distillation

## Quantum Error Correction and Fault Tolerance

PMIR provides first-class support for QEC and large-scale fault-tolerant quantum computing:

### QEC-Aware Type System

```rust
// QEC-specific types in PMIR
pub enum QECType {
    // Logical qubits with error correction
    LogicalQubit {
        code: ErrorCorrectionCode,
        distance: usize,
    },
    
    // Physical qubits (for syndrome extraction)
    PhysicalQubit,
    
    // Syndrome measurement results
    Syndrome {
        code: ErrorCorrectionCode,
        syndrome_type: SyndromeType,
    },
    
    // Ancilla qubits for QEC
    AncillaQubit {
        purpose: AncillaPurpose,
        reset_protocol: ResetProtocol,
    },
    
    // Fault-tolerant regions
    FaultTolerantBlock {
        error_threshold: f64,
        verification_level: usize,
    },
}

pub enum ErrorCorrectionCode {
    Surface { distance: usize },
    Color { distance: usize },
    RepetitionCode { length: usize },
    Custom { name: String, params: CodeParams },
}

pub enum SyndromeType {
    XStabilizer,
    ZStabilizer,
    FlagQubits,
}
```

### QEC Operations

```rust
// Natural QEC operations in PMIR
pub enum QECOp {
    // Logical qubit operations
    LogicalInit {
        code: ErrorCorrectionCode,
        state: LogicalState,
    },
    LogicalGate {
        gate: LogicalGateType,
        qubits: Vec<LogicalQubitRef>,
        transversal: bool,
    },
    
    // Syndrome extraction
    SyndromeExtraction {
        logical_qubits: Vec<LogicalQubitRef>,
        syndrome_qubits: Vec<AncillaRef>,
        extraction_circuit: Region,
    },
    
    // Error correction
    ErrorCorrection {
        syndrome: SyndromeRef,
        correction_table: CorrectionTable,
        parallel_decode: bool,
    },
    
    // State injection for non-Clifford gates
    MagicStateInjection {
        state_type: MagicStateType,
        distillation_level: usize,
        verification_rounds: usize,
    },
    
    // Lattice surgery operations
    LatticeSurgery {
        operation: SurgeryOp,
        patches: Vec<CodePatch>,
        merge_protocol: MergeProtocol,
    },
}

// Example: Natural surface code operation
let surface_code_cnot = QECOp::LogicalGate {
    gate: LogicalGateType::CNOT,
    qubits: vec![logical_control, logical_target],
    transversal: false, // Use lattice surgery
};
```

### Resource-Aware QEC Compilation

```rust
// QEC resource management
pub struct QECResourceManager {
    // Physical qubit allocation
    physical_layout: PhysicalQubitLayout,
    
    // Syndrome extraction scheduling
    syndrome_schedule: SyndromeSchedule,
    
    // Classical decoding resources
    decoder_allocation: DecoderResources,
    
    // Magic state factories
    distillation_factories: Vec<DistillationFactory>,
}

// Attributes for QEC compilation
pub struct QECAttributes {
    // Target error rate
    pub target_logical_error_rate: f64,
    
    // Hardware constraints
    pub connectivity: QubitConnectivity,
    pub measurement_error_rate: f64,
    pub gate_error_rates: HashMap<GateType, f64>,
    
    // Optimization preferences
    pub optimization_target: QECOptimizationTarget,
}

pub enum QECOptimizationTarget {
    MinimizePhysicalQubits,
    MinimizeCircuitDepth,
    MinimizeDecoding Latency,
    MaximizeThroughput,
    BalancedResourceUsage,
}
```

### Fault-Tolerant Compilation Pipeline

```rust
// Specialized lowering for fault-tolerant execution
pub struct FaultTolerantLowering {
    // Code choice based on hardware
    code_selector: CodeSelector,
    
    // Fault-tolerant gate synthesis
    gate_synthesizer: FTGateSynthesizer,
    
    // Resource estimator
    resource_estimator: QECResourceEstimator,
}

impl FaultTolerantLowering {
    pub fn lower_to_fault_tolerant(&self, module: &Module) -> Result<FTModule, Error> {
        // 1. Analyze error requirements
        let error_budget = self.analyze_error_requirements(module)?;
        
        // 2. Choose appropriate QEC codes
        let code_assignment = self.code_selector.assign_codes(module, error_budget)?;
        
        // 3. Synthesize fault-tolerant operations
        let ft_ops = self.synthesize_operations(module, &code_assignment)?;
        
        // 4. Insert syndrome extraction
        let with_syndromes = self.insert_syndrome_extraction(ft_ops)?;
        
        // 5. Schedule operations
        let scheduled = self.schedule_ft_operations(with_syndromes)?;
        
        // 6. Estimate resources
        let resources = self.resource_estimator.estimate(&scheduled)?;
        
        Ok(FTModule {
            operations: scheduled,
            resource_requirements: resources,
            code_assignment,
        })
    }
}
```

### Resource Estimation Backend

```rust
// Lower to resource estimation instead of execution
pub enum EstimationTarget {
    // Physical resource counts
    PhysicalResources {
        include_routing: bool,
        include_distillation: bool,
    },
    
    // Time/space tradeoffs
    SpaceTimeVolume,
    
    // Error analysis
    LogicalErrorRate {
        num_samples: usize,
        include_correlated_errors: bool,
    },
    
    // Classical resources for decoding
    DecodingResources {
        decoder_type: DecoderType,
        parallel_windows: usize,
    },
}

impl Module {
    pub fn estimate_resources(&self, target: EstimationTarget) -> ResourceEstimate {
        match target {
            EstimationTarget::PhysicalResources { .. } => {
                // Count physical qubits, gates, measurements
                self.estimate_physical_resources()
            }
            EstimationTarget::SpaceTimeVolume => {
                // Calculate space-time trade-offs
                self.calculate_space_time_volume()
            }
            EstimationTarget::LogicalErrorRate { .. } => {
                // Run error propagation analysis
                self.analyze_logical_errors()
            }
            EstimationTarget::DecodingResources { .. } => {
                // Estimate classical compute needs
                self.estimate_decoding_requirements()
            }
        }
    }
}
```

### Syndrome Decoding Integration

```rust
// Classical syndrome decoding as first-class operations
pub enum DecodingOp {
    // Minimum weight perfect matching
    MWPM {
        syndrome: SyndromeData,
        graph: DecodingGraph,
        parallel_regions: usize,
    },
    
    // Union-Find decoder
    UnionFind {
        syndrome: SyndromeData,
        growth_rate: f64,
    },
    
    // Machine learning decoder
    NeuralDecoder {
        model: DecoderModel,
        batch_size: usize,
        accelerator: ComputeDevice,
    },
    
    // Correlated error decoding
    CorrelatedDecoding {
        syndrome_history: Vec<SyndromeData>,
        correlation_model: ErrorModel,
    },
}

// Example: Parallel syndrome processing
func @parallel_syndrome_decode(%syndrome: tensor<1000x1000xi1>) -> tensor<1000x1000xi1> {
    // Split syndrome into regions for parallel decoding
    %regions = tensor.split %syndrome, %num_decoders : tensor<1000x1000xi1> -> tensor<?x?x?xi1>
    
    %corrections = parallel.map %regions {
        ^bb0(%region: tensor<?x?xi1>):
            // Run MWPM on each region
            %local_correction = qec.mwpm %region : tensor<?x?xi1> -> tensor<?x?xi1>
            parallel.yield %local_correction
    }
    
    // Merge corrections with boundary resolution
    %merged = qec.merge_corrections %corrections : tensor<?x?x?xi1> -> tensor<1000x1000xi1>
    
    return %merged : tensor<1000x1000xi1>
}
```

### QEC-Aware Scheduling

```rust
// Schedule operations considering QEC constraints
pub struct QECScheduler {
    // Syndrome extraction frequency
    syndrome_interval: usize,
    
    // Preserve fault-tolerance
    ft_constraints: FTConstraints,
    
    // Classical-quantum coordination
    decode_latency_budget: Duration,
}

impl QECScheduler {
    pub fn schedule_with_qec(&self, ops: Vec<Operation>) -> Schedule {
        let mut schedule = Schedule::new();
        
        // Group operations by QEC rounds
        let rounds = self.group_into_qec_rounds(ops);
        
        for round in rounds {
            // Schedule quantum operations
            schedule.add_quantum_ops(round.quantum_ops);
            
            // Insert syndrome extraction
            schedule.add_syndrome_extraction();
            
            // Schedule classical decoding (parallel)
            schedule.add_parallel_decoding();
            
            // Apply corrections before next round
            schedule.add_error_correction();
        }
        
        schedule
    }
}
```

### Example: Surface Code Circuit with Resource Estimation

```mlir
module attributes {qec.code = "surface", qec.distance = 21} {
    func @fault_tolerant_algorithm() -> !qec.logical_qubit {
        // Allocate logical qubits with surface code
        %logical = qec.alloc_logical @surface_code : () -> !qec.logical_qubit<d=21>
        
        // Magic state for T gate (with distillation)
        %magic = qec.distill_magic_state @15_to_1_distillation : () -> !qec.magic_state
        
        // Fault-tolerant T gate via state injection
        qec.inject_t_gate %logical, %magic : !qec.logical_qubit<d=21>, !qec.magic_state
        
        // Syndrome extraction (automatic scheduling)
        qec.extract_syndrome %logical attributes {parallel_decode = true}
        
        // Resource estimation metadata
        qec.estimate {
            physical_qubits = 10000,
            code_cycles = 1000,
            magic_states = 100,
            classical_ops = 1e9
        }
        
        return %logical : !qec.logical_qubit<d=21>
    }
}
```

This QEC support enables:
- **Natural expression** of logical operations and error correction
- **Automatic lowering** to physical implementations
- **Resource estimation** instead of simulation when needed
- **Parallel classical-quantum** coordination for syndrome decoding
- **Multiple QEC codes** with automatic selection
- **Fault-tolerant compilation** with error budget management

### QEC Compilation Analysis Tools

PMIR supports multiple analysis backends for QEC optimization:

```rust
// QEC analysis and optimization tools
pub enum QECAnalysisBackend {
    // Analyze logical error rates
    ErrorRateAnalyzer {
        error_model: NoiseModel,
        monte_carlo_samples: usize,
        include_correlations: bool,
    },
    
    // Optimize QEC code placement
    CodeOptimizer {
        hardware_graph: ConnectivityGraph,
        optimization_metric: OptimizationMetric,
        search_strategy: SearchStrategy,
    },
    
    // Analyze decoder performance
    DecoderProfiler {
        decoder_implementations: Vec<DecoderImpl>,
        benchmark_syndrome_sets: Vec<SyndromeSet>,
        parallel_configs: Vec<ParallelConfig>,
    },
    
    // Resource-performance tradeoff analysis
    TradeoffAnalyzer {
        parameter_space: ParameterSpace,
        pareto_objectives: Vec<Objective>,
    },
}

// Integration with PMIR compilation
impl Module {
    pub fn analyze_qec(&self, backend: QECAnalysisBackend) -> AnalysisReport {
        match backend {
            QECAnalysisBackend::ErrorRateAnalyzer { .. } => {
                // Analyze error propagation through the circuit
                self.analyze_error_rates()
            }
            QECAnalysisBackend::CodeOptimizer { .. } => {
                // Find optimal QEC code parameters
                self.optimize_code_selection()
            }
            QECAnalysisBackend::DecoderProfiler { .. } => {
                // Profile decoder performance
                self.profile_decoders()
            }
            QECAnalysisBackend::TradeoffAnalyzer { .. } => {
                // Analyze resource-performance tradeoffs
                self.analyze_tradeoffs()
            }
        }
    }
}
```

### Lowering to QEC Analysis Tools

```rust
// Lower PMIR to specialized QEC analysis formats
pub enum QECAnalysisTarget {
    // Stim circuit for fast Clifford simulation
    Stim {
        include_detectors: bool,
        include_observables: bool,
    },
    
    // PyMatching format for decoder analysis
    PyMatching {
        graph_format: GraphFormat,
        weight_calculation: WeightMethod,
    },
    
    // Qiskit QEC framework
    QiskitQEC {
        code_type: String,
        decoder_type: String,
    },
    
    // Custom analysis format
    Custom {
        serializer: Box<dyn QECSerializer>,
    },
}

impl Module {
    pub fn to_qec_analysis_format(&self, target: QECAnalysisTarget) -> String {
        match target {
            QECAnalysisTarget::Stim { .. } => {
                // Convert to Stim circuit format
                self.to_stim_circuit()
            }
            QECAnalysisTarget::PyMatching { .. } => {
                // Generate matching graph
                self.to_matching_graph()
            }
            // ... other targets
        }
    }
}
```

### Example: Complete QEC Compilation Pipeline

```rust
// End-to-end QEC compilation with analysis
let mut pipeline = QECCompilationPipeline::new();

// 1. Parse high-level algorithm
let algorithm = parse_quantum_algorithm(source)?;

// 2. Compile to logical operations
let logical_circuit = pipeline.compile_to_logical(algorithm)?;

// 3. Analyze resource requirements
let resources = logical_circuit.analyze_qec(
    QECAnalysisBackend::ErrorRateAnalyzer {
        error_model: hardware.noise_model(),
        monte_carlo_samples: 10_000,
        include_correlations: true,
    }
)?;

// 4. Optimize QEC encoding
let optimized = logical_circuit.analyze_qec(
    QECAnalysisBackend::CodeOptimizer {
        hardware_graph: hardware.connectivity(),
        optimization_metric: OptimizationMetric::MinimizeSpaceTime,
        search_strategy: SearchStrategy::SimulatedAnnealing,
    }
)?;

// 5. Lower to physical implementation
let physical = pipeline.lower_to_physical(optimized)?;

// 6. Generate execution plan with parallel decoding
let execution_plan = physical.schedule_with_parallel_decoding()?;

// 7. Export to various backends
match target {
    Target::Simulation => execution_plan.to_qec_simulator(),
    Target::ResourceEstimation => execution_plan.to_resource_estimator(),
    Target::Hardware => execution_plan.to_hardware_instructions(),
    Target::Analysis => execution_plan.to_analysis_tool(),
}
```

### Real-time QEC Monitoring

```rust
// Support for real-time QEC monitoring and adaptation
pub struct QECMonitor {
    // Real-time syndrome data
    syndrome_stream: SyndromeStream,
    
    // Adaptive decoder
    adaptive_decoder: AdaptiveDecoder,
    
    // Performance metrics
    metrics: QECMetrics,
}

// MLIR operations for monitoring
module {
    func @monitored_qec_execution() {
        // Start monitoring
        %monitor = qec.start_monitor {
            syndrome_batch_size = 1000,
            decoder_timeout_ms = 10,
            adaptation_interval = 100
        }
        
        // Execute with monitoring
        qec.monitored_region %monitor {
            // Quantum operations here
            %logical = qec.logical_h %q : !qec.logical_qubit
            
            // Monitoring points
            qec.checkpoint %monitor, "after_h_gate"
            
            %syndrome = qec.extract_syndrome %logical
            %correction = qec.decode_adaptive %syndrome, %monitor
            qec.apply_correction %logical, %correction
        }
        
        // Analyze monitoring results
        %metrics = qec.get_metrics %monitor
        qec.export_metrics %metrics, "qec_performance.json"
    }
}
```

## Handling Entanglement in Parallel Execution

Special care is needed when parallelizing quantum operations due to entanglement:

```rust
// Entanglement-aware parallelization
pub struct EntanglementTracker {
    // Track which qubits are entangled
    entanglement_graph: Graph<QubitId, EntanglementEdge>,
    // Partition qubits into independent groups
    partitions: Vec<QubitPartition>,
}

impl EntanglementTracker {
    pub fn analyze_parallelism(&self, ops: &[Operation]) -> ParallelSchedule {
        let mut schedule = ParallelSchedule::new();
        
        for op in ops {
            match self.get_entanglement_requirements(op) {
                EntanglementReq::Independent => {
                    // Can run in parallel with any other operation
                    schedule.add_to_parallel_group(op);
                }
                EntanglementReq::LocalEntanglement(partition) => {
                    // Can run in parallel with ops on other partitions
                    schedule.add_to_partition_group(partition, op);
                }
                EntanglementReq::CrossPartition(partitions) => {
                    // Requires synchronization between partitions
                    schedule.add_sync_point(partitions);
                    schedule.add_sequential(op);
                }
            }
        }
        
        schedule
    }
}

// Example: Parallel quantum algorithm with entanglement tracking
func @entanglement_aware_parallel(%n: i32) -> tensor<?xi1> {
    %qubits = quantum.alloc %n : (i32) -> !quantum.reg<?>
    
    // Track entanglement during execution
    %tracker = quantum.init_entanglement_tracker %qubits : !quantum.reg<?> -> !quantum.tracker
    
    // Phase 1: Create GHZ state (fully entangled)
    quantum.track %tracker {
        quantum.h %qubits[0] : !quantum.reg<?>
        affine.for %i = 1 to %n {
            quantum.cx %qubits[0], %qubits[%i] : !quantum.reg<?>, !quantum.reg<?>
        }
    }
    
    // Phase 2: Parallel local operations (entanglement preserved)
    %results = parallel.for %i = 0 to %n {
        // Each thread can operate on its qubit independently
        quantum.local_operation %qubits[%i] : !quantum.reg<?>
        %m = quantum.measure %qubits[%i] : !quantum.reg<?> -> i1
        parallel.yield %m : i1
    } : tensor<?xi1>
    
    // Phase 3: Verify entanglement was preserved
    quantum.verify_entanglement %tracker : !quantum.tracker
    
    return %results : tensor<?xi1>
}

// Entanglement-preserving transformations
pub struct EntanglementPreservingPass;

impl Pass for EntanglementPreservingPass {
    fn run(&mut self, module: &mut Module) -> Result<(), Error> {
        for function in &mut module.functions {
            let tracker = EntanglementTracker::new();
            
            for block in function.walk_blocks_mut() {
                // Analyze entanglement flow
                tracker.analyze_block(block);
                
                // Apply parallelization only where safe
                let parallel_groups = tracker.find_parallel_groups(block);
                for group in parallel_groups {
                    self.parallelize_group(block, group)?;
                }
            }
        }
        Ok(())
    }
}
```

## Testing Strategy

1. **Unit tests**: Each PMIR operation
2. **Integration tests**: Complete quantum algorithms
3. **Comparison tests**: PMIR interpreter vs LLVM execution
4. **Performance benchmarks**: Direct execution vs compiled

### Unified Development Workflow

```rust
pub struct HybridCompiler {
    debug_mode: bool,
    
    pub fn execute(&mut self, source: Source, options: CompileOptions) -> Result<(), Error> {
        if self.debug_mode {
            // Quick path for development
            self.execute_debug_path(source)
        } else {
            // Optimized production path
            self.execute_production_path(source, options.target)
        }
    }
    
    // Validation: run both paths and compare
    pub fn validate(&self, source: Source) -> Result<(), Error> {
        let debug_result = self.run_debug_path(source.clone())?;
        let prod_result = self.run_production_path(source)?;
        assert_eq!(debug_result.quantum_state, prod_result.quantum_state);
        Ok(())
    }
}
```

This dual approach enables:
- Fast iteration during development
- Correctness validation by comparing paths
- Performance analysis to verify optimization benefits
- Incremental migration from debug to production

## Rust Codegen Advantages

The Rust code generation strategy offers unique benefits:

1. **Type Safety**: Leverage Rust's type system for compile-time correctness
2. **Zero-Cost Abstractions**: Rust's optimizations apply to generated code
3. **Native Performance**: No interpreter overhead, direct CPU instructions
4. **Easy Integration**: Generated code can use any Rust crate/library
5. **Debugging**: Can use standard Rust debugging tools on generated code
6. **Incremental Compilation**: Rust's incremental compilation for faster rebuilds

### Advanced Rust Codegen Features

```rust
// Example: Generating optimized batch operations
impl RustCodegen {
    fn generate_batched_measurements(&mut self, measurements: &[MlirOperation]) {
        // Instead of individual measurements:
        // let m0 = sim.measure(0)?;
        // let m1 = sim.measure(1)?;
        
        // Generate batched version:
        self.writeln("let measurements = sim.measure_batch(&[");
        for m in measurements {
            let qubit = self.get_qubit_index(m);
            self.writeln(&format!("    {},", qubit));
        }
        self.writeln("])?;");
    }
    
    fn generate_parallel_circuit(&mut self, parallel_ops: &[MlirOperation]) {
        self.writeln("rayon::scope(|s| {");
        for op in parallel_ops {
            self.writeln("    s.spawn(|_| {");
            self.generate_operation(op)?;
            self.writeln("    });");
        }
        self.writeln("});");
    }
}
```

## Open Questions

1. **Memory management**: Should we use reference counting or garbage collection for the interpreter?
2. **Quantum simulator interface**: How to make it pluggable for different backends?
3. **Optimization passes**: Should we use MLIR's pass infrastructure or build our own?
4. **Debug information**: How to preserve source location info through transformations?
5. **Parallelism**: How to support parallel quantum operations in the interpreter?
6. **Rust codegen caching**: Should we cache compiled Rust binaries?
7. **Cross-compilation**: How to support different target architectures in Rust codegen?

## Backend Targets

PMIR can target multiple backends beyond the three main execution strategies:

### Quantum Hardware Backends
```rust
pub enum HardwareTarget {
    IBMQ { backend: String },
    IonQ { device: String },
    Rigetti { processor: String },
    // ... other quantum hardware
}

impl PmirModule {
    pub fn compile_for_hardware(&self, target: HardwareTarget) -> Result<HardwareProgram> {
        match target {
            HardwareTarget::IBMQ { .. } => self.to_qiskit_circuit(),
            HardwareTarget::IonQ { .. } => self.to_ionq_circuit(),
            HardwareTarget::Rigetti { .. } => self.to_pyquil(),
        }
    }
}
```

### Other Software Backends
- **WebAssembly**: Compile to WASM for browser execution
- **CUDA/GPU**: Generate GPU kernels for large-scale simulation
- **Cirq/Qiskit**: Export to other quantum frameworks
- **QIR**: Microsoft's Quantum Intermediate Representation

## MLIR Dialect Design

To fully leverage MLIR's ecosystem, PMIR operations should map to MLIR dialects:

```mlir
// Quantum dialect
quantum.h %q0 : !quantum.qubit
quantum.cx %q0, %q1 : !quantum.qubit, !quantum.qubit
%m0 = quantum.measure %q0 : !quantum.qubit -> i1

// Standard dialects for classical operations
%sum = arith.addi %a, %b : i32
%cond = arith.cmpi eq, %x, %y : i32
cf.cond_br %cond, ^bb1, ^bb2
```

## Debug vs Production Compilation Paths

### Debug Path (Development)
For rapid development and debugging:

```rust
pub struct DebugExecutor {
    // Direct interpretation with full debugging support
    interpreter: PmirInterpreter,
    // Breakpoints, step execution, state inspection
    debugger: PmirDebugger,
}
```

Features:
- Fast startup (no compilation)
- Full debugging capabilities
- State inspection at any point
- Step-by-step execution

### Production Path (Performance)
For maximum performance:

```rust
pub struct ProductionCompiler {
    pub fn compile(&self, source: Source, target: Target) -> Result<Executable, Error> {
        // 1. Parse to PMIR
        let pmir = self.parse_to_pmir(source)?;
        
        // 2. Optimize aggressively
        let optimized = self.optimize_for_production(pmir)?;
        
        // 3. Generate specialized code
        match target {
            Target::RustSimulation => {
                let rust_code = self.generate_optimized_rust(&optimized)?;
                compile_rust_code(rust_code)
            }
            Target::QuantumHardware => {
                // Strip noise operations
                let hardware_ready = optimized.strip_noise_ops();
                self.generate_hardware_code(&hardware_ready)
            }
        }
    }
}
```

## MLIR-Style Region-Based Execution

PMIR can use region classification to choose execution strategies:

```rust
pub struct AdaptiveExecutor {
    interpreter: PmirInterpreter,
    rust_compiler: RustCompiler,
    llvm_compiler: LLVMCompiler,
}

impl AdaptiveExecutor {
    pub fn execute_function(&mut self, func: &Function) -> Result<Value> {
        for region in func.walk_regions() {
            match region.get_property::<RegionKind>("region_kind") {
                Some(RegionKind::PureClassical) => {
                    // JIT compile classical regions for performance
                    self.rust_compiler.compile_and_execute(region)?
                }
                Some(RegionKind::PureQuantum) => {
                    // Use specialized quantum simulator
                    self.interpreter.execute_quantum_region(region)?
                }
                Some(RegionKind::Loop { kind, parallel_safe: true }) => {
                    // Parallelize the loop execution
                    self.execute_parallel_loop(region, kind)?
                }
                _ => {
                    // Fall back to interpretation
                    self.interpreter.execute_region(region)?
                }
            }
        }
        Ok(Value::Unit)
    }
}
```

## Performance Optimization Strategies

### 1. MLIR-Style Region-Based Optimization

Using region classification for targeted optimizations:

```rust
pub struct RegionOptimizationPass;

impl Pass for RegionOptimizationPass {
    fn run(&mut self, module: &mut Module) -> Result<(), Error> {
        // First classify regions
        let mut classifier = RegionClassificationPass;
        classifier.run(module)?;
        
        // Then apply targeted optimizations
        for function in &mut module.functions {
            for region in function.walk_regions_mut() {
                match region.get_property::<RegionKind>("region_kind") {
                    Some(RegionKind::PureQuantum) => {
                        // Quantum-specific optimizations
                        apply_gate_fusion(region)?;
                        apply_commutation_rules(region)?;
                        apply_cancellation_patterns(region)?;
                    }
                    Some(RegionKind::PureClassical) => {
                        // Classical optimizations
                        apply_constant_folding(region)?;
                        apply_common_subexpression_elimination(region)?;
                        apply_loop_invariant_code_motion(region)?;
                    }
                    Some(RegionKind::Hybrid { .. }) => {
                        // Minimize quantum-classical boundaries
                        batch_measurements(region)?;
                        hoist_classical_computation(region)?;
                    }
                    Some(RegionKind::Loop { parallel_safe: true, .. }) => {
                        // Parallelization transformations
                        unroll_and_vectorize(region)?;
                    }
                    _ => {}
                }
            }
        }
        Ok(())
    }
}
```

### MLIR-Style Dialect-Specific Optimizations

Operations from different dialects get different optimizations:

```rust
// Quantum dialect patterns
pub struct QuantumDialectPatterns;

impl QuantumDialectPatterns {
    pub fn get_patterns() -> Vec<Box<dyn Pattern>> {
        vec![
            Box::new(HadamardCancellationPattern),   // H·H → I
            Box::new(CNOTCancellationPattern),        // CX·CX → I
            Box::new(PhaseGateFusionPattern),         // RZ(a)·RZ(b) → RZ(a+b)
            Box::new(SingleQubitGateFusionPattern),   // Merge adjacent 1Q gates
        ]
    }
}

// Arithmetic dialect patterns  
pub struct ArithDialectPatterns;

impl ArithDialectPatterns {
    pub fn get_patterns() -> Vec<Box<dyn Pattern>> {
        vec![
            Box::new(ConstantFoldingPattern),         // 2+3 → 5
            Box::new(IdentityEliminationPattern),     // x+0 → x
            Box::new(StrengthReductionPattern),       // x*2 → x<<1
        ]
    }
}
```

### 2. Data-Oriented Design
- Cache-friendly memory layouts
- SIMD-friendly data structures
- Minimize pointer chasing

### 3. Compilation Strategies
- **Interpretation**: For development and small circuits
- **Rust codegen**: For medium circuits with repeated execution
- **MLIR/LLVM**: For production and hardware deployment

### 4. Noise Handling
- Conditional compilation for noise operations
- Zero overhead when targeting hardware
- Efficient noise models for simulation

## Summary: Flexibility Through MLIR-Like Design

PMIR achieves flexibility by closely following MLIR's architecture:

### What PMIR Provides (Core Only)

1. **MLIR-Compatible Structure**: Operations, Blocks, Regions, Modules
2. **Extensible Type System**: Add new types through dialects
3. **Dialect Mechanism**: Group related functionality
4. **Pass Infrastructure**: Transform and analyze IR
5. **MLIR Text Generation**: Easy interop with MLIR tools

### What You Add (Through Dialects)

1. **Quantum Operations**: Define the operations you need
2. **QEC Support**: Add logical qubits, syndromes, decoders as needed
3. **Execution Strategies**: Interpreter, compiler, estimator backends
4. **Analysis Tools**: Custom passes for your use cases
5. **Domain-Specific Types**: Whatever your application requires

### Example Extensions

```rust
// Quantum dialect - add only what you need
let quantum_dialect = QuantumDialect::new()
    .with_ops(vec!["h", "cx", "measure"])
    .with_types(vec!["qubit"]);

// QEC dialect - add when you need it
let qec_dialect = QECDialect::new()
    .with_ops(vec!["syndrome_extract", "decode"])
    .with_types(vec!["logical_qubit", "syndrome"]);

// Analysis passes - add as required
let resource_estimator = ResourceEstimationPass::new();
let qec_optimizer = QECOptimizationPass::new();
```

### Key Insight

By following MLIR's design patterns, PMIR gets:
- Proven architecture that scales
- Easy addition of new features
- Natural lowering to MLIR
- Flexibility to evolve with needs

We avoid over-design by providing mechanisms, not policies. The specific quantum operations, QEC schemes, and analysis tools are all added through the extension mechanisms as needed.

## Strategic Design Considerations

### Frontend Parser Architecture

Given the multiple input formats (HUGR, Guppy, OpenQASM, LLVM IR, PHIR), consider a unified parser framework:

```rust
// Trait for incremental parsing and error recovery
pub trait IncrementalParser {
    fn parse_partial(&mut self, input: &str) -> Result<PartialAST, ParseError>;
    fn recover_from_error(&mut self, error: ParseError) -> Option<PartialAST>;
    fn combine_partials(&self, parts: Vec<PartialAST>) -> PastModule;
}

// Parser pipeline with transformations
pub struct ParserPipeline {
    stages: Vec<Box<dyn ParserStage>>,
}

pub trait ParserStage {
    fn transform(&self, input: ParseTree) -> Result<ParseTree, Error>;
}

// Example: PHIR to PAST with preservation of metadata
impl PHIRParser {
    fn preserve_metadata(&self, phir_op: &PHIROperation) -> Attributes {
        // Preserve timing, error modeling, and other metadata
        let mut attrs = HashMap::new();
        if let Some(duration) = phir_op.duration {
            attrs.insert("duration", Attribute::Duration(duration));
        }
        if let Some(metadata) = &phir_op.metadata {
            attrs.insert("original_metadata", Attribute::Dict(metadata.clone()));
        }
        attrs
    }
}
```

### Execution Strategy Selection

Make execution strategy selection more intelligent and adaptive:

```rust
// Smart execution strategy selector
pub struct ExecutionPlanner {
    profiler: CircuitProfiler,
    hardware_info: HardwareCapabilities,
}

impl ExecutionPlanner {
    pub fn select_strategy(&self, module: &Module) -> ExecutionStrategy {
        let profile = self.profiler.analyze(module);
        
        match profile {
            CircuitProfile { 
                num_qubits, 
                circuit_depth, 
                classical_complexity,
                expected_iterations,
                ..
            } => {
                // Small circuits with heavy classical: interpret
                if num_qubits < 10 && classical_complexity > HighComplexity {
                    return ExecutionStrategy::Interpreter;
                }
                
                // Repeated execution: compile to Rust
                if expected_iterations > 1000 {
                    return ExecutionStrategy::RustCodegen {
                        optimization_level: OptLevel::Aggressive,
                        parallel: true,
                    };
                }
                
                // Large-scale with QEC: MLIR with custom lowering
                if num_qubits > 1000 && profile.uses_qec {
                    return ExecutionStrategy::MLIR {
                        target: MLIRTarget::QECOptimized,
                        parallel_decode: true,
                    };
                }
                
                // Default: adaptive hybrid
                ExecutionStrategy::Adaptive
            }
        }
    }
}
```

### Verification and Validation Infrastructure

Add built-in verification capabilities:

```rust
// Quantum program verification
pub trait QuantumVerifier {
    fn verify_unitarity(&self, ops: &[Operation]) -> Result<(), VerificationError>;
    fn verify_measurement_consistency(&self, module: &Module) -> Result<(), VerificationError>;
    fn verify_no_cloning(&self, module: &Module) -> Result<(), VerificationError>;
}

// QEC verification
pub struct QECVerifier {
    pub fn verify_fault_tolerance(&self, module: &Module) -> Result<FTReport, Error> {
        // Verify transversal gates are used correctly
        // Check syndrome extraction doesn't propagate errors
        // Validate error correction procedures
    }
    
    pub fn verify_threshold(&self, module: &Module, target_error: f64) -> Result<bool, Error> {
        // Verify the circuit meets error threshold requirements
    }
}

// Classical-quantum boundary verification
pub struct BoundaryVerifier {
    pub fn verify_no_quantum_branching(&self, module: &Module) -> Result<(), Error> {
        // Ensure quantum operations don't depend on quantum measurements
        // within the same coherent block
    }
}
```

### Resource Estimation as First-Class Feature

Make resource estimation more sophisticated:

```rust
// Hierarchical resource estimation
pub struct ResourceEstimator {
    estimators: HashMap<String, Box<dyn Estimator>>,
}

pub trait Estimator {
    fn estimate(&self, module: &Module) -> ResourceReport;
    fn confidence_interval(&self) -> (f64, f64);
}

// Different estimation strategies
pub enum EstimationStrategy {
    // Fast but approximate
    Analytical {
        include_routing: bool,
        include_magic_states: bool,
    },
    // Slower but accurate
    MonteCarlo {
        samples: usize,
        error_model: NoiseModel,
    },
    // Learn from previous executions
    MachineLearning {
        model: ResourceModel,
        features: FeatureExtractor,
    },
}

// Composable resource reports
pub struct ResourceReport {
    physical_qubits: Range<usize>,
    gate_count: HashMap<String, usize>,
    circuit_depth: usize,
    classical_operations: usize,
    estimated_runtime: Duration,
    confidence: f64,
    breakdown: Option<ResourceBreakdown>,
}
```

### Debugging and Profiling Integration

Enhanced debugging support across all execution strategies:

```rust
// Unified debugging interface
pub trait DebugContext {
    fn set_breakpoint(&mut self, location: Location) -> BreakpointId;
    fn inspect_quantum_state(&self, qubits: &[QubitId]) -> QuantumState;
    fn replay_from_checkpoint(&mut self, checkpoint: CheckpointId) -> Result<(), Error>;
    fn profile_region(&mut self, region: &Region) -> ProfileData;
}

// Works across interpreter, Rust codegen, and MLIR
pub struct UniversalDebugger {
    strategy: Box<dyn DebugStrategy>,
}

// Quantum-specific profiling
pub struct QuantumProfiler {
    pub fn measure_entanglement_depth(&self, ops: &[Operation]) -> usize;
    pub fn identify_critical_path(&self, module: &Module) -> Vec<Operation>;
    pub fn suggest_parallelization(&self, module: &Module) -> ParallelizationPlan;
}
```

### Modular Backend Architecture

Make backends truly pluggable:

```rust
// Backend trait for easy extension
pub trait ExecutionBackend {
    fn can_handle(&self, module: &Module) -> bool;
    fn compile(&self, module: &Module) -> Result<Executable, Error>;
    fn estimate_performance(&self, module: &Module) -> PerformanceEstimate;
}

// Backend registry for extensibility
pub struct BackendRegistry {
    backends: Vec<Box<dyn ExecutionBackend>>,
    
    pub fn register_backend(&mut self, backend: Box<dyn ExecutionBackend>) {
        self.backends.push(backend);
    }
    
    pub fn select_best_backend(&self, module: &Module) -> &dyn ExecutionBackend {
        self.backends.iter()
            .filter(|b| b.can_handle(module))
            .max_by_key(|b| b.estimate_performance(module).score())
            .expect("No suitable backend found")
    }
}

// Example: Distributed quantum simulation backend
pub struct DistributedSimulationBackend {
    cluster_config: ClusterConfig,
    partitioning_strategy: PartitionStrategy,
}

impl ExecutionBackend for DistributedSimulationBackend {
    fn can_handle(&self, module: &Module) -> bool {
        module.num_qubits() > 30 && self.cluster_config.is_available()
    }
    // ...
}
```

### Incremental Compilation Support

Support for incremental and hot-reload development:

```rust
// Incremental compilation for faster development
pub struct IncrementalCompiler {
    cache: CompilationCache,
    dependency_graph: DependencyGraph,
    
    pub fn compile_incremental(&mut self, changes: &[Change]) -> Result<Module, Error> {
        // Only recompile affected parts
        let affected = self.dependency_graph.find_affected(changes);
        for module in affected {
            let compiled = self.compile_module(module)?;
            self.cache.update(module.id, compiled);
        }
        self.link_modules()
    }
    
    pub fn hot_reload(&mut self, module: &Module) -> Result<(), Error> {
        // Replace running module without full restart
        self.compile_incremental(&module.changes())?;
        self.patch_running_code()?;
        Ok(())
    }
}
```

### Domain-Specific Optimizations

Add quantum-aware optimization infrastructure:

```rust
// Quantum circuit optimization framework
pub struct QuantumOptimizationPipeline {
    passes: Vec<Box<dyn QuantumPass>>,
}

pub trait QuantumPass {
    fn applicable(&self, circuit: &Circuit) -> bool;
    fn expected_improvement(&self, circuit: &Circuit) -> f64;
    fn apply(&self, circuit: &mut Circuit) -> Result<OptimizationStats, Error>;
}

// Example: Topology-aware optimization
pub struct TopologyAwareRouter {
    hardware_topology: ConnectivityGraph,
    
    impl QuantumPass for TopologyAwareRouter {
        fn apply(&self, circuit: &mut Circuit) -> Result<OptimizationStats, Error> {
            // Insert SWAPs to match hardware connectivity
            // Minimize SWAP overhead using advanced algorithms
        }
    }
}

// Noise-aware optimization
pub struct NoiseAwareOptimizer {
    noise_model: NoiseModel,
    
    pub fn optimize_for_noise(&self, circuit: &mut Circuit) -> Result<(), Error> {
        // Reorder operations to minimize decoherence
        // Use dynamical decoupling
        // Optimize gate decompositions for specific noise
    }
}
```

### Simplicity vs Sophistication Balance

While PMIR can support sophisticated features, the core should remain simple:

```rust
// Simple API for common cases
pub mod simple {
    use super::*;
    
    // Easy circuit construction
    pub fn bell_pair() -> Module {
        circuit()
            .h(0)
            .cx(0, 1)
            .measure_all()
            .build()
    }
    
    // But allow sophisticated usage when needed
    pub fn qec_circuit(code: SurfaceCode) -> Module {
        let mut builder = ModuleBuilder::new()
            .with_qec_dialect()
            .with_parallelism();
            
        builder.build_with(|b| {
            let logical = b.allocate_logical_qubits(code);
            // ... complex QEC operations
        })
    }
}

// Progressive disclosure of complexity
pub struct PMIRBuilder {
    // Start simple
    pub fn new() -> Self { 
        Self::with_basic_dialects() 
    }
    
    // Add complexity as needed
    pub fn with_all_dialects(self) -> Self { ... }
    pub fn with_custom_dialect(self, dialect: impl Dialect) -> Self { ... }
    pub fn with_optimization_level(self, level: OptLevel) -> Self { ... }
}
```

### Practical Considerations

Keep practical usage in mind:

```rust
// Fast path for development
#[cfg(debug_assertions)]
pub fn quick_run(source: &str) -> Result<ExecutionResult, Error> {
    parse(source)?
        .interpret() // Skip optimization in debug
}

// Production path with full optimization
#[cfg(not(debug_assertions))]
pub fn quick_run(source: &str) -> Result<ExecutionResult, Error> {
    let module = parse(source)?;
    let strategy = ExecutionPlanner::new().select_strategy(&module);
    
    match strategy {
        ExecutionStrategy::Interpreter => module.interpret(),
        ExecutionStrategy::RustCodegen { .. } => {
            let rust_code = module.to_rust()?;
            compile_and_run(rust_code)
        }
        ExecutionStrategy::MLIR { .. } => {
            let mlir_text = module.to_mlir_text()?;
            lower_and_execute(mlir_text)
        }
    }
}

// Clear error messages for quantum domain
pub enum QuantumError {
    NonUnitaryOperation { op: String, reason: String },
    MeasurementInSuperposition { qubit: QubitId },
    EntanglementViolation { qubits: Vec<QubitId> },
    QECThresholdExceeded { logical_error_rate: f64, threshold: f64 },
}

impl Display for QuantumError {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            Self::NonUnitaryOperation { op, reason } => {
                write!(f, "Operation '{}' is not unitary: {}", op, reason)
            }
            // ... helpful error messages
        }
    }
}
```

### PHIR to PMIR Direct Pipeline

Since PHIR will be a frontend, optimize this path:

```rust
// Optimized PHIR to PMIR lowering
pub struct PHIRToPMIR {
    preserve_semantics: bool,
    optimization_level: OptLevel,
    
    pub fn lower(&self, phir: PHIRProgram) -> Result<Module, Error> {
        let mut builder = PMIRBuilder::new();
        
        // Direct mapping for common patterns
        for op in phir.ops {
            match op {
                PHIROp::QParallel { ops } => {
                    // Direct to parallel region - no intermediate AST
                    builder.create_parallel_region(|region| {
                        for qop in ops {
                            region.add_quantum_op(self.lower_qop(qop)?);
                        }
                    });
                }
                PHIROp::Measurement { args, returns } => {
                    // Preserve PHIR's explicit return mapping
                    builder.create_measurement_with_mapping(args, returns);
                }
                // ... other direct mappings
            }
        }
        
        builder.build()
    }
}
```

## Insights from HUGR

HUGR's design offers several architectural patterns we should adopt:

### 1. Hierarchical Structure with Parent-Child Relationships

HUGR uses hierarchy edges to form a tree structure. PMIR should adopt this for clear nesting:

```rust
// HUGR-inspired hierarchical structure
pub struct HierarchicalModule {
    // Every node knows its parent
    parent_map: HashMap<NodeId, NodeId>,
    // Container nodes know their children
    children_map: HashMap<NodeId, Vec<NodeId>>,
    // Root node of the hierarchy
    root: NodeId,
}

// HUGR-style sibling graphs
pub struct SiblingGraph {
    parent: NodeId,
    nodes: Vec<NodeId>,
    // Only edges between siblings
    edges: Vec<Edge>,
}
```

### 2. Port-Based Connection System

HUGR's port system provides precise connectivity:

```rust
// HUGR-inspired port system
pub struct Port {
    node: NodeId,
    direction: Direction,
    index: usize,
    port_type: Type,
}

pub enum Direction {
    Incoming,
    Outgoing,
}

// Ports have types, edges connect compatible ports
pub struct TypedEdge {
    source: Port,
    target: Port,
    edge_kind: EdgeKind,
}

// HUGR's EdgeKind adapted for PMIR
pub enum EdgeKind {
    Value(Type),         // Runtime dataflow
    Const(Type),         // Compile-time constants
    Function(PolyType),  // Function references
    ControlFlow,         // CFG edges
    StateOrder,          // Ordering constraints
}
```

### 3. Linear Type Support

HUGR tracks linearity through its type system:

```rust
// Linear vs copyable types
pub enum TypeBound {
    Linear,    // Must be used exactly once (quantum data)
    Copyable,  // Can be copied/discarded (classical data)
}

impl Type {
    pub fn is_linear(&self) -> bool {
        match self.bound() {
            TypeBound::Linear => true,
            TypeBound::Copyable => false,
        }
    }
}

// Enforce linear type rules in PMIR
impl PMIRValidator {
    fn validate_linear_types(&self, module: &Module) -> Result<(), Error> {
        for value in module.all_values() {
            if value.ty.is_linear() {
                let uses = self.count_uses(value);
                if uses != 1 {
                    return Err(Error::LinearTypeViolation { value, uses });
                }
            }
        }
        Ok(())
    }
}
```

### 4. Builder Pattern Hierarchy

HUGR provides specialized builders for different node types:

```rust
// HUGR-style builder hierarchy
pub trait NodeBuilder {
    type Node;
    fn finish(self) -> Result<Self::Node, BuildError>;
}

pub struct ModuleBuilder { ... }
pub struct FunctionBuilder { ... }
pub struct DFGBuilder { ... }
pub struct CFGBuilder { ... }
pub struct ConditionalBuilder { ... }
pub struct CircuitBuilder { ... }  // Alternative to DFG for circuits

// Builders can be nested
impl ModuleBuilder {
    pub fn define_function<F>(&mut self, name: &str, sig: Signature, f: F) -> Result<FuncId, Error>
    where F: FnOnce(&mut FunctionBuilder) -> Result<(), Error>
    {
        let mut func_builder = FunctionBuilder::new(sig);
        f(&mut func_builder)?;
        let func = func_builder.finish()?;
        self.add_function(name, func)
    }
}
```

### 5. Extension Registry Pattern

HUGR's extension system is more sophisticated than simple dialects:

```rust
// HUGR-style extension registry
pub struct ExtensionRegistry {
    extensions: HashMap<ExtensionId, Extension>,
    // Resolution for weak references
    weak_registry: WeakExtensionRegistry,
}

pub struct Extension {
    name: ExtensionId,
    operations: HashMap<OpName, OpDef>,
    types: HashMap<TypeName, TypeDef>,
    // Extension-specific validation
    validation: Box<dyn ExtensionValidation>,
}

// Operations can reference extensions they require
impl Operation {
    pub fn required_extensions(&self) -> ExtensionSet {
        // Collect all extensions used by this operation
        self.collect_extensions()
    }
}
```

### 6. Node Weights and Metadata

HUGR associates data with nodes as "weights":

```rust
// Node weights for additional data
pub struct Node {
    id: NodeId,
    op: Operation,
    weight: NodeWeight,
}

pub struct NodeWeight {
    // Debugging information
    source_location: Option<SourceLocation>,
    // User metadata
    metadata: HashMap<String, MetadataValue>,
    // Cached information
    cached_signature: Option<Signature>,
}
```

### 7. Signature-Based Type Checking

HUGR uses signatures pervasively for type checking:

```rust
// Every operation has a signature
pub trait OpSignature {
    fn signature(&self) -> Signature;
    fn input_types(&self) -> &[Type];
    fn output_types(&self) -> &[Type];
}

// Signatures include both value and static edges
pub struct Signature {
    // Value inputs/outputs
    inputs: TypeRow,
    outputs: TypeRow,
    // Static inputs (constants, functions)
    static_inputs: Vec<Type>,
    // Extension requirements
    required_extensions: ExtensionSet,
}
```

### 8. Validation Architecture

HUGR has a comprehensive validation system:

```rust
// Multi-phase validation
pub struct Validator {
    phases: Vec<Box<dyn ValidationPhase>>,
}

pub trait ValidationPhase {
    fn validate(&self, hugr: &Hugr) -> Result<(), ValidationError>;
}

// Example phases from HUGR
struct StructureValidation;  // Check hierarchy is a tree
struct TypeValidation;       // Check all connections type-check
struct ExtensionValidation;  // Check all extensions are available
struct LinearityValidation;  // Check linear types used correctly
```

## Design Summary: Best of All Worlds

PMIR combines the best ideas from MLIR, PHIR, and HUGR:

### From MLIR
- **Hierarchical structure**: Operations → Blocks → Regions → Modules
- **SSA form**: Clean value semantics
- **Dialect system**: Extensible operations and types
- **Pass infrastructure**: Composable transformations
- **Progressive lowering**: High-level to low-level transformations

### From PHIR
- **Clear variable semantics**: Explicit definition and scoping
- **Machine operations**: First-class hardware control
- **Block structures**: Natural quantum parallelism
- **Result command**: Explicit output declaration
- **Foreign functions**: External classical computation

### From HUGR  
- **Port-based connections**: Precise type-safe wiring
- **Linear type tracking**: Quantum data correctness
- **Hierarchical builders**: Specialized construction APIs
- **Extension registry**: Sophisticated extension management
- **Comprehensive validation**: Multi-phase correctness checking

### PMIR's Unique Contributions
- **Three execution strategies**: Interpreter, Rust codegen, MLIR lowering
- **QEC as first-class**: Built-in support for fault tolerance
- **Adaptive execution**: Smart strategy selection
- **Domain-natural API**: Quantum programming feels natural
- **Progressive complexity**: Simple API with advanced features available

### Key Design Principles
1. **Mechanism, not policy**: Core provides structure, extensions add semantics
2. **Pay for what you use**: Start simple, add complexity as needed
3. **Domain-natural**: Quantum concepts map directly
4. **Performance-oriented**: Multiple backends for different needs
5. **Correctness by construction**: Type system prevents errors

### The Path Forward

Start with Phase 1 (minimal core) and build incrementally:
1. Basic MLIR-like structure
2. Quantum and classical dialects
3. PHIR parser for immediate utility
4. Simple interpreter for testing
5. Add features as needed

This design provides a solid foundation that can grow with PECOS's needs while maintaining compatibility with the broader quantum software ecosystem.

## Future Extensions

1. **JIT compilation**: Compile hot paths to native code
2. **Distributed execution**: Support for distributed quantum simulators
3. **Hardware backends**: Direct compilation to quantum hardware instructions
4. **Verification**: Formal verification of quantum programs at PMIR level
5. **Optimization framework**: MLIR-style pass infrastructure in Rust
6. **Hybrid classical-quantum**: Better integration of classical and quantum code
7. **Resource estimation**: Analyze quantum resource requirements at PMIR level
8. **ECS Architecture**: Entity Component System for flexible simulation
9. **GPU Acceleration**: Generate CUDA/GPU kernels for large-scale simulation
10. **Profile-Guided Optimization**: Use runtime profiling to guide optimizations
11. **Advanced QEC Codes**: Quantum LDPC, concatenated codes, topological codes
12. **Fault-Tolerant Synthesis**: Automatic synthesis of FT gadgets
13. **Dynamic QEC**: Runtime code switching based on error rates
14. **Distributed Syndrome Processing**: Scale decoding across clusters
15. **Hardware-Aware QEC**: Optimize for specific quantum hardware