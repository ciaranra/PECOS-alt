# PMIR Implementation Plan

This document provides a detailed, phased implementation plan for PMIR, organized to enable incremental development with useful functionality at each stage.

## Table of Contents

1. [Overview](#1-overview)
2. [Phase 1: Minimal Core](#phase-1-minimal-core-weeks-1-2)
3. [Phase 2: Dialect Infrastructure](#phase-2-dialect-infrastructure-weeks-3-4)
4. [Phase 3: Essential Passes](#phase-3-essential-passes-weeks-5-6)
5. [Phase 4: Builder API](#phase-4-builder-api-weeks-7-8)
6. [Phase 5: Interfaces and Traits](#phase-5-interfaces-and-traits-weeks-9-10)
7. [Phase 6: Pattern Matching Infrastructure](#phase-6-pattern-matching-infrastructure-weeks-11-12)
8. [Phase 7: Basic Interpreter](#phase-7-basic-interpreter-weeks-13-14)
9. [Phase 8: Transformations and Lowering](#phase-8-transformations-and-lowering-weeks-15-16)
10. [Phase 9: Rust Code Generation](#phase-9-rust-code-generation-weeks-17-18)
11. [Phase 10: Full Classical Support](#phase-10-full-classical-support-weeks-19-20)
12. [Phase 11: Advanced Optimizations](#phase-11-advanced-optimizations-weeks-21-22)
13. [Phase 12: Parallelism and Concurrency](#phase-12-parallelism-and-concurrency-weeks-23-24)
14. [Phase 13: QEC and Fault Tolerance](#phase-13-qec-and-fault-tolerance-weeks-25-28)
15. [Testing Strategy](#testing-strategy)
16. [Integration Points](#integration-points)

## 1. Overview

The implementation follows a bottom-up approach, starting with core data structures and gradually adding functionality. Each phase produces working code that can be tested and used.

### Guiding Principles

1. **Incremental Functionality**: Each phase adds usable features
2. **Test-Driven**: Comprehensive tests at each stage
3. **Documentation**: Keep docs in sync with implementation
4. **Performance Awareness**: Profile and optimize as we go
5. **Early Integration**: Connect with existing PECOS components early

## Phase 1: Minimal Core (Weeks 1-2)

### Goals
Establish the fundamental data structures and type system.

### Implementation Tasks

```rust
// 1. Core types (src/types.rs)
pub struct NodeId(u32);
pub struct ValueId(u32);
pub struct BlockId(u32);
pub struct RegionId(u32);

#[derive(Clone, Debug, PartialEq)]
pub enum Type {
    Qubit,
    Bit,
    Int(Width),
    Float(Width),
    Array(Box<Type>, usize),
    Custom(String, Vec<TypeArg>),
}

// 2. Basic structures (src/core.rs)
pub struct Operation {
    pub id: NodeId,
    pub name: OpName,
    pub operands: Vec<Value>,
    pub results: Vec<Value>,
    pub attributes: HashMap<String, Attribute>,
}

pub struct Block {
    pub id: BlockId,
    pub arguments: Vec<BlockArgument>,
    pub operations: Vec<Operation>,
    pub terminator: Terminator,
}

pub struct Region {
    pub id: RegionId,
    pub blocks: Vec<Block>,
    pub entry: BlockId,
}

pub struct Module {
    pub functions: Vec<Function>,
    pub globals: Vec<Global>,
}

// 3. SSA values (src/value.rs)
pub struct Value {
    pub id: ValueId,
    pub ty: Type,
    pub defining_op: Option<NodeId>,
}

// 4. Attributes (src/attribute.rs)
#[derive(Clone, Debug)]
pub enum Attribute {
    Int(i64),
    Float(f64),
    String(String),
    Type(Type),
    Array(Vec<Attribute>),
}
```

### Deliverables
- Core type definitions
- Basic IR structures
- Value and attribute system
- Unit tests for all types

## Phase 2: Dialect Infrastructure (Weeks 3-4)

### Goals
Implement the extensible dialect system.

### Implementation Tasks

```rust
// 1. Dialect trait (src/dialect.rs)
pub trait Dialect: Send + Sync {
    fn name(&self) -> &str;
    fn initialize(&self, registry: &mut DialectRegistry);
}

pub struct DialectRegistry {
    dialects: HashMap<String, Box<dyn Dialect>>,
    operations: HashMap<OpName, OpDefinition>,
    types: HashMap<String, TypeDefinition>,
}

// 2. Operation definition (src/op_def.rs)
pub struct OpDefinition {
    pub name: OpName,
    pub verify: Box<dyn Fn(&Operation) -> Result<(), Error>>,
    pub traits: Vec<OpTrait>,
}

// 3. Core dialects (src/dialects/)
pub struct QuantumDialect;
impl Dialect for QuantumDialect {
    fn initialize(&self, registry: &mut DialectRegistry) {
        registry.register_op("quantum.h", h_gate_def());
        registry.register_op("quantum.cx", cx_gate_def());
        registry.register_op("quantum.measure", measure_def());
        // ...
    }
}

pub struct ArithDialect;
pub struct ControlFlowDialect;
pub struct MemoryDialect;

// 4. Dynamic loading
impl Module {
    pub fn load_dialect(&mut self, dialect: Box<dyn Dialect>) {
        self.registry.add_dialect(dialect);
    }
}
```

### Deliverables
- Dialect trait and registry
- Basic quantum dialect
- Arithmetic dialect
- Control flow dialect
- Tests for dialect loading

## Phase 3: Essential Passes (Weeks 5-6)

### Goals
Create the pass infrastructure for IR transformation.

### Implementation Tasks

```rust
// 1. Pass trait (src/pass.rs)
pub trait Pass {
    fn name(&self) -> &str;
    fn run(&mut self, module: &mut Module) -> Result<(), Error>;
}

pub struct PassManager {
    passes: Vec<Box<dyn Pass>>,
}

// 2. Basic passes (src/passes/)
pub struct VerificationPass;
impl Pass for VerificationPass {
    fn run(&mut self, module: &mut Module) -> Result<(), Error> {
        // Type checking
        // SSA verification
        // Dominance checking
    }
}

pub struct MLIRExportPass;
impl Pass for MLIRExportPass {
    fn run(&mut self, module: &mut Module) -> Result<(), Error> {
        // Generate MLIR text
    }
}

// 3. Analysis framework (src/analysis.rs)
pub trait Analysis {
    type Result;
    fn analyze(&self, module: &Module) -> Self::Result;
}

pub struct DominanceAnalysis;
pub struct UseDefAnalysis;
```

### Deliverables
- Pass infrastructure
- Verification pass
- MLIR export pass
- Basic analyses
- Pass pipeline tests

## Phase 4: Builder API (Weeks 7-8)

### Goals
Create ergonomic APIs for constructing PMIR programs.

### Implementation Tasks

```rust
// 1. Module builder (src/builder/module.rs)
pub struct ModuleBuilder {
    module: Module,
    insertion_point: InsertionPoint,
}

impl ModuleBuilder {
    pub fn new() -> Self { ... }
    
    pub fn define_function<F>(&mut self, name: &str, sig: Signature, f: F) -> Result<FuncId, Error>
    where F: FnOnce(&mut FunctionBuilder) -> Result<(), Error>
    { ... }
}

// 2. Function builder (src/builder/function.rs)
pub struct FunctionBuilder {
    function: Function,
    current_block: BlockId,
    value_map: HashMap<String, ValueId>,
}

impl FunctionBuilder {
    pub fn add_operation(&mut self, op: impl Into<Operation>) -> Result<Vec<Value>, Error> { ... }
    pub fn create_block(&mut self) -> BlockId { ... }
    pub fn set_insertion_point(&mut self, block: BlockId) { ... }
}

// 3. Circuit builder (src/builder/circuit.rs)
pub struct CircuitBuilder {
    builder: FunctionBuilder,
    qubit_map: Vec<Value>,
}

impl CircuitBuilder {
    pub fn h(&mut self, qubit: usize) -> Result<&mut Self, Error> { ... }
    pub fn cx(&mut self, control: usize, target: usize) -> Result<&mut Self, Error> { ... }
    pub fn measure(&mut self, qubit: usize) -> Result<Value, Error> { ... }
}

// 4. Pattern builder (src/builder/pattern.rs)
impl ModuleBuilder {
    pub fn with_qec<F>(&mut self, config: QECConfig, f: F) -> Result<(), Error>
    where F: FnOnce(&mut QECBuilder) -> Result<(), Error>
    { ... }
}
```

### Deliverables
- Module builder
- Function builder
- Circuit builder
- Specialized builders
- Builder API tests

## Phase 5: Interfaces and Traits (Weeks 9-10)

### Goals
Define common behaviors across operations.

### Implementation Tasks

```rust
// 1. Core interfaces (src/interfaces.rs)
pub trait ConstantFoldable {
    fn fold(&self, operands: &[Attribute]) -> Option<Attribute>;
}

pub trait SideEffectFree {
    fn has_side_effects(&self) -> bool { false }
    fn is_speculatable(&self) -> bool { true }
}

pub trait Commutative {
    fn is_commutative(&self) -> bool;
}

// 2. Memory effects (src/interfaces/memory.rs)
pub trait MemoryEffects {
    fn memory_effects(&self) -> Effects;
}

pub struct Effects {
    pub reads: Vec<MemorySlot>,
    pub writes: Vec<MemorySlot>,
    pub may_alias: bool,
}

// 3. Quantum interfaces (src/interfaces/quantum.rs)
pub trait QuantumOp {
    fn qubit_count(&self) -> usize;
    fn is_unitary(&self) -> bool;
    fn is_measurement(&self) -> bool;
}

// 4. Interface registration
impl OpDefinition {
    pub fn with_interface<I: Interface>(mut self, interface: I) -> Self {
        self.interfaces.push(Box::new(interface));
        self
    }
}
```

### Deliverables
- Core interface definitions
- Memory effect tracking
- Quantum-specific interfaces
- Interface-based optimizations

## Phase 6: Pattern Matching Infrastructure (Weeks 11-12)

### Goals
Enable declarative optimizations through pattern matching.

### Implementation Tasks

```rust
// 1. Pattern definition (src/pattern.rs)
pub trait Pattern {
    fn matches(&self, op: &Operation) -> Option<PatternMatch>;
    fn rewrite(&self, match_: PatternMatch, rewriter: &mut Rewriter) -> Result<(), Error>;
}

pub struct PatternMatch {
    pub root: NodeId,
    pub bindings: HashMap<String, Value>,
}

// 2. Rewriter (src/rewrite.rs)
pub struct Rewriter {
    module: &mut Module,
    worklist: Vec<NodeId>,
}

impl Rewriter {
    pub fn replace_op(&mut self, old: NodeId, new: Operation) { ... }
    pub fn erase_op(&mut self, op: NodeId) { ... }
    pub fn create_op(&mut self, op: Operation) -> NodeId { ... }
}

// 3. Pattern sets (src/patterns/)
pub struct QuantumPatterns;
impl QuantumPatterns {
    pub fn cancellation_patterns() -> Vec<Box<dyn Pattern>> {
        vec![
            Box::new(HadamardCancellation),
            Box::new(CXCancellation),
            Box::new(PhaseGateFusion),
        ]
    }
}

// 4. Greedy rewriter (src/passes/greedy_rewrite.rs)
pub struct GreedyPatternRewriter {
    patterns: Vec<Box<dyn Pattern>>,
}
```

### Deliverables
- Pattern matching framework
- Rewriter infrastructure
- Basic quantum patterns
- Pattern application pass

## Phase 7: Basic Interpreter (Weeks 13-14)

### Goals
Implement direct execution of PMIR programs.

### Implementation Tasks

```rust
// 1. Interpreter core (src/interpreter/mod.rs)
pub struct Interpreter {
    quantum_state: Box<dyn QuantumSimulator>,
    memory: Memory,
    call_stack: CallStack,
}

impl Interpreter {
    pub fn execute_module(&mut self, module: &Module) -> Result<ExecutionResult, Error> { ... }
    pub fn execute_function(&mut self, func: &Function, args: Vec<Value>) -> Result<Vec<Value>, Error> { ... }
}

// 2. Operation dispatch (src/interpreter/dispatch.rs)
impl Interpreter {
    fn execute_op(&mut self, op: &Operation) -> Result<Vec<Value>, Error> {
        match &op.name {
            OpName::Quantum(q) => self.execute_quantum_op(q, &op.operands),
            OpName::Arith(a) => self.execute_arith_op(a, &op.operands),
            OpName::Control(c) => self.execute_control_op(c, &op.operands),
            OpName::Custom(name) => self.execute_custom_op(name, op),
        }
    }
}

// 3. Quantum backend integration (src/interpreter/quantum.rs)
pub trait QuantumSimulator {
    fn apply_gate(&mut self, gate: Gate, qubits: &[usize]) -> Result<(), Error>;
    fn measure(&mut self, qubit: usize) -> Result<bool, Error>;
    fn get_state(&self) -> &QuantumState;
}

// 4. Memory management (src/interpreter/memory.rs)
pub struct Memory {
    allocations: HashMap<AllocId, Allocation>,
    stack: Vec<StackFrame>,
}
```

### Deliverables
- Basic interpreter
- Quantum operation execution
- Classical operation execution
- Memory management
- Integration with PECOS simulators

## Phase 8: Transformations and Lowering (Weeks 15-16)

### Goals
Implement progressive lowering between abstraction levels.

### Implementation Tasks

```rust
// 1. Lowering framework (src/lowering/mod.rs)
pub trait LoweringPass {
    fn lower(&self, module: &Module) -> Result<Module, Error>;
}

// 2. Quantum lowering (src/lowering/quantum.rs)
pub struct QuantumToQIRLowering;
impl LoweringPass for QuantumToQIRLowering {
    fn lower(&self, module: &Module) -> Result<Module, Error> {
        // Convert quantum ops to QIR function calls
    }
}

// 3. Control flow lowering (src/lowering/control.rs)
pub struct StructuredToSSACFG;
impl LoweringPass for StructuredToSSACFG {
    fn lower(&self, module: &Module) -> Result<Module, Error> {
        // Convert structured control flow to CFG
    }
}

// 4. Dialect conversion (src/lowering/conversion.rs)
pub struct DialectConverter {
    conversions: HashMap<OpName, Box<dyn ConversionPattern>>,
}
```

### Deliverables
- Lowering framework
- Quantum to QIR lowering
- Control flow lowering
- Type conversions
- Lowering tests

## Phase 9: Rust Code Generation (Weeks 17-18)

### Goals
Generate executable Rust code from PMIR.

### Implementation Tasks

```rust
// 1. Code generator (src/codegen/rust.rs)
pub struct RustCodegen {
    config: CodegenConfig,
    namer: Namer,
}

impl RustCodegen {
    pub fn generate(&self, module: &Module) -> Result<String, Error> {
        let mut code = String::new();
        self.write_imports(&mut code)?;
        self.write_module(&mut code, module)?;
        Ok(code)
    }
}

// 2. Operation mapping (src/codegen/rust/ops.rs)
impl RustCodegen {
    fn generate_quantum_op(&self, op: &QuantumOp, args: &[Value]) -> String {
        match op {
            QuantumOp::H => format!("sim.h({})?", self.value_to_rust(args[0])),
            QuantumOp::CX => format!("sim.cx({}, {})?", 
                self.value_to_rust(args[0]), 
                self.value_to_rust(args[1])),
            // ...
        }
    }
}

// 3. Optimization (src/codegen/rust/optimize.rs)
pub struct RustOptimizer {
    pub fn optimize_for_batch(&self, ops: &[Operation]) -> Vec<Operation> { ... }
    pub fn optimize_for_parallel(&self, ops: &[Operation]) -> Vec<Operation> { ... }
}

// 4. Compilation pipeline (src/codegen/rust/compile.rs)
pub struct RustCompiler {
    pub fn compile_to_binary(&self, rust_code: &str) -> Result<PathBuf, Error> {
        // Use rustc or cargo
    }
}
```

### Deliverables
- Rust code generator
- Operation mappings
- Code optimization
- Compilation pipeline
- Generated code tests

## Phase 10: Full Classical Support (Weeks 19-20)

### Goals
Complete support for classical computation.

### Implementation Tasks

```rust
// 1. Memory operations (src/dialects/memory.rs)
pub enum MemoryOp {
    Alloc { size: usize, align: usize },
    Load { ptr: Value },
    Store { ptr: Value, value: Value },
    Free { ptr: Value },
}

// 2. Advanced arithmetic (src/dialects/arith_ext.rs)
pub enum ArithExtOp {
    DivMod { dividend: Value, divisor: Value },
    Pow { base: Value, exp: Value },
    BitCount { value: Value },
    // Vector operations
    VectorAdd { lhs: Value, rhs: Value },
}

// 3. Function calls (src/ops/call.rs)
pub struct CallOp {
    pub callee: CalleeType,
    pub args: Vec<Value>,
    pub results: Vec<Type>,
}

pub enum CalleeType {
    Direct(FunctionRef),
    Indirect(Value),
    Foreign(String),
}

// 4. Complex control flow (src/dialects/scf.rs)
pub enum SCFOp {
    For { start: Value, end: Value, step: Value, body: Region },
    While { condition: Region, body: Region },
    Switch { value: Value, cases: Vec<(Value, Region)>, default: Region },
}
```

### Deliverables
- Memory management operations
- Extended arithmetic
- Function call support
- Complex control flow
- Classical computation tests

## Phase 11: Advanced Optimizations (Weeks 21-22)

### Goals
Implement sophisticated optimization passes.

### Implementation Tasks

```rust
// 1. Quantum optimizations (src/passes/quantum_opt.rs)
pub struct QuantumCircuitOptimizer {
    passes: Vec<Box<dyn QuantumPass>>,
}

pub trait QuantumPass {
    fn optimize(&self, circuit: &mut Circuit) -> OptimizationResult;
}

// Gate fusion
pub struct GateFusionPass;
// Commutation analysis
pub struct CommutationPass;
// Peephole optimization
pub struct PeepholeOptimizer;

// 2. Classical optimizations (src/passes/classical_opt.rs)
pub struct CommonSubexpressionElimination;
pub struct DeadCodeElimination;
pub struct LoopInvariantCodeMotion;
pub struct ConstantPropagation;

// 3. Cross-domain optimizations (src/passes/hybrid_opt.rs)
pub struct MeasurementDelayPass;
pub struct ClassicalControlOptimization;
pub struct QuantumClassicalScheduling;

// 4. Profile-guided optimization (src/passes/pgo.rs)
pub struct ProfileGuidedOptimizer {
    profile_data: ProfileData,
}
```

### Deliverables
- Quantum circuit optimizations
- Classical optimizations
- Hybrid optimizations
- Optimization pipeline
- Benchmark suite

## Phase 12: Parallelism and Concurrency (Weeks 23-24)

### Goals
Add support for parallel execution.

### Implementation Tasks

```rust
// 1. Parallel dialect (src/dialects/parallel.rs)
pub enum ParallelOp {
    ParallelFor { bounds: ParallelBounds, body: Region },
    Task { body: Region, depends_on: Vec<Value> },
    Sync { tasks: Vec<Value> },
    Atomic { op: AtomicOp, operands: Vec<Value> },
}

// 2. Quantum parallelism (src/parallel/quantum.rs)
pub struct QuantumParallelizer {
    pub fn find_parallel_groups(&self, ops: &[QuantumOp]) -> Vec<ParallelGroup> { ... }
    pub fn verify_commutativity(&self, group: &ParallelGroup) -> bool { ... }
}

// 3. Thread management (src/parallel/threads.rs)
pub struct ThreadPool {
    quantum_threads: Vec<QuantumThread>,
    classical_threads: Vec<ClassicalThread>,
}

// 4. Synchronization (src/parallel/sync.rs)
pub struct SyncPrimitives {
    barriers: HashMap<BarrierId, Barrier>,
    channels: HashMap<ChannelId, Channel>,
}
```

### Deliverables
- Parallel dialect
- Quantum parallelization
- Thread management
- Synchronization primitives
- Parallel execution tests

## Phase 13: QEC and Fault Tolerance (Weeks 25-28)

### Goals
Implement comprehensive QEC support.

### Implementation Tasks

```rust
// 1. QEC dialect (src/dialects/qec.rs)
pub struct QECDialect;
impl Dialect for QECDialect {
    fn initialize(&self, registry: &mut DialectRegistry) {
        // Logical qubit operations
        registry.register_type("qec.logical_qubit", logical_qubit_type());
        registry.register_op("qec.init_logical", init_logical_op());
        
        // Syndrome operations
        registry.register_op("qec.extract_syndrome", syndrome_extract_op());
        registry.register_op("qec.decode", decode_op());
        
        // Magic states
        registry.register_op("qec.distill_magic", magic_distill_op());
    }
}

// 2. QEC types (src/types/qec.rs)
pub enum QECType {
    LogicalQubit { code: CodeType, distance: usize },
    Syndrome { code: CodeType },
    MagicState { fidelity: f64 },
}

// 3. Fault-tolerant compilation (src/qec/compiler.rs)
pub struct FaultTolerantCompiler {
    pub fn compile(&self, module: &Module, config: QECConfig) -> Result<FTModule, Error> {
        let analyzer = ErrorAnalyzer::new(config.noise_model);
        let code_selector = CodeSelector::new(config.available_codes);
        let synthesizer = GateSynthesizer::new();
        
        // Full compilation pipeline
    }
}

// 4. Resource estimation (src/qec/resources.rs)
pub struct ResourceEstimator {
    pub fn estimate(&self, module: &FTModule) -> ResourceEstimate { ... }
}
```

### Deliverables
- QEC dialect
- QEC type system
- Fault-tolerant compiler
- Resource estimator
- QEC integration tests

## Testing Strategy

### Unit Tests
- Test each component in isolation
- Property-based testing for core structures
- Fuzzing for parser robustness

### Integration Tests
- End-to-end compilation tests
- Cross-dialect interaction tests
- Performance benchmarks

### Test Infrastructure

```rust
// Test utilities (tests/common/mod.rs)
pub fn test_module() -> Module { ... }
pub fn assert_valid(module: &Module) { ... }
pub fn assert_equivalent(m1: &Module, m2: &Module) { ... }

// Golden tests (tests/golden/)
#[test]
fn test_quantum_circuit() {
    let module = parse_file("tests/golden/bell.pmir");
    let result = interpret(module);
    assert_eq!(result, expected_output());
}
```

## Integration Points

### With Existing PECOS Components

1. **Quantum Simulators**
   - Week 13: Connect interpreter to `pecos-qsim`
   - Week 17: Generate code using `pecos-qsim` API

2. **PHIR Parser**
   - Week 4: Define PHIR dialect operations
   - Week 8: Implement PHIR to PAST parser

3. **Error Models**
   - Week 25: Integrate PECOS noise models
   - Week 27: Connect to syndrome decoders

### External Dependencies

1. **MLIR/LLVM**
   - Use `llvm-sys` for LLVM integration
   - Consider `melior` for MLIR bindings

2. **Optimization Libraries**
   - Graph algorithms for circuit optimization
   - Linear algebra for state representation

## Risk Mitigation

### Technical Risks

1. **Performance**: Profile early and often
2. **Complexity**: Keep phases small and focused
3. **Integration**: Test with real PECOS code early
4. **Scalability**: Design for large circuits from start

### Process Risks

1. **Scope Creep**: Stick to phase goals
2. **Dependencies**: Minimize external dependencies
3. **Testing Debt**: Maintain test coverage >80%
4. **Documentation**: Update docs with code

## Success Metrics

### Phase Completion
- All tests passing
- Documentation updated
- Performance benchmarks met
- Integration tests working

### Overall Success
- Can compile real quantum algorithms
- Performance competitive with alternatives
- Easy to extend with new features
- Well-documented and maintainable