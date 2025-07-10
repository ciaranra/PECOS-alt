#!/usr/bin/env python3
"""Example showing how to get in-memory HUGR from Guppy compilation."""

import guppylang
from guppylang import guppy
from guppylang.std.quantum import qubit

# Import quantum operations at module level
from guppylang.std.quantum import h

# Define a simple Guppy function
@guppy
def quantum_example(q: qubit) -> qubit:
    """Example quantum function in Guppy."""
    # Apply Hadamard gate
    q = h(q)
    return q

@guppy
def simple_function(x: int, y: int) -> int:
    """Simple classical function."""
    return x + y

def get_hugr_from_guppy(guppy_func):
    """
    Compile a Guppy function and extract the in-memory HUGR.
    
    Returns:
        tuple: (hugr_object, hugr_json_str, hugr_bytes)
    """
    # Compile the Guppy function to get a ModulePointer
    module_ptr = guppy.compile(guppy_func)
    
    # Extract the HUGR from the package
    hugr = module_ptr.package.modules[0]
    
    # Get different representations
    hugr_json = hugr.to_str()  # JSON string representation
    hugr_bytes = module_ptr.package.to_bytes()  # Binary representation
    
    return hugr, hugr_json, hugr_bytes

def main():
    # Example 1: Simple classical function
    print("=== Compiling simple classical function ===")
    hugr, hugr_json, hugr_bytes = get_hugr_from_guppy(simple_function)
    
    print(f"HUGR type: {type(hugr)}")
    print(f"Number of nodes: {hugr.num_nodes}")
    print(f"JSON size: {len(hugr_json)} characters")
    print(f"Binary size: {len(hugr_bytes)} bytes")
    
    # Show that HUGR is in-memory and accessible
    print(f"\nHUGR nodes:")
    for node_id, node_data in list(hugr.items())[:5]:  # First 5 nodes
        print(f"  {node_id}: {type(node_data.op).__name__}")
    
    # Example 2: Quantum function
    print("\n=== Compiling quantum function ===")
    hugr_q, hugr_json_q, hugr_bytes_q = get_hugr_from_guppy(quantum_example)
    
    print(f"HUGR type: {type(hugr_q)}")
    print(f"Number of nodes: {hugr_q.num_nodes}")
    print(f"JSON size: {len(hugr_json_q)} characters")
    
    # Show how this HUGR could be passed to other tools
    print("\n=== In-memory HUGR can be used directly ===")
    print(f"- Pass to HUGR optimization passes")
    print(f"- Convert to other formats")
    print(f"- Analyze the quantum circuit structure")
    print(f"- Feed into compilation pipeline")
    
    return hugr, hugr_q

if __name__ == "__main__":
    hugr_classical, hugr_quantum = main()
    
    # The HUGR objects are now available in memory for further processing
    print(f"\nHUGR objects are available in memory:")
    print(f"- Classical function HUGR: {hugr_classical}")
    print(f"- Quantum function HUGR: {hugr_quantum}")