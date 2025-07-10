# Getting In-Memory HUGR from Guppy

Yes, Guppy provides in-memory HUGR objects when you compile Guppy code. Here's how it works:

## Key Findings

1. **`guppy.compile()` returns a `ModulePointer`** containing the compiled HUGR
2. **The HUGR is accessible as an in-memory Python object**, not just as files
3. **The HUGR can be serialized to various formats** (JSON, bytes) or used directly

## Example Code

```python
from guppylang import guppy

@guppy
def my_function(x: int, y: int) -> int:
    return x + y

# Compile to get ModulePointer
module_ptr = guppy.compile(my_function)

# Extract the in-memory HUGR object
hugr = module_ptr.package.modules[0]  # This is a hugr.hugr.base.Hugr object

# The HUGR is now available in memory for:
# - Direct manipulation
# - Passing to optimization passes
# - Converting to LLVM IR
# - Analysis and transformation
```

## HUGR Object Properties

The in-memory HUGR object (`hugr.hugr.base.Hugr`) provides:

- **Direct access to nodes**: `hugr.nodes`, `hugr.num_nodes`
- **Graph traversal**: `hugr.children`, `hugr.descendants`
- **Serialization**: `hugr.to_str()`, `hugr.to_bytes()`
- **Modification**: `hugr.add_node()`, `hugr.delete_node()`
- **Analysis**: `hugr.links`, `hugr.port_type()`

## Integration with `hugr_sim()`

Given that we get in-memory HUGR from Guppy, a `hugr_sim()` function would:

1. Accept a HUGR object (from Guppy or other sources)
2. Compile it to LLVM IR using HUGR → LLVM compilation
3. Pass the LLVM IR to `llvm_sim()`

```python
def hugr_sim(hugr: Union[Hugr, ModulePointer]) -> HugrSimBuilder:
    """Create a simulation from a HUGR object."""
    if isinstance(hugr, ModulePointer):
        hugr = hugr.package.modules[0]
    
    # Compile HUGR to LLVM IR
    llvm_ir = compile_hugr_to_llvm(hugr)
    
    # Use llvm_sim for the actual simulation
    return llvm_sim(llvm_ir)
```

## Benefits

1. **No file I/O required** - everything stays in memory
2. **Direct integration** - Guppy → HUGR → LLVM → Execution
3. **Tool interoperability** - HUGR from any source can be simulated
4. **Performance** - avoids serialization/deserialization overhead

## Next Steps

1. Implement HUGR → LLVM compilation in the PECOS codebase
2. Create `hugr_sim()` that leverages `llvm_sim()`
3. Update `guppy_sim()` to use this pipeline: Guppy → HUGR → `hugr_sim()`