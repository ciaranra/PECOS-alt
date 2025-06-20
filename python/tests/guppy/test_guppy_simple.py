#!/usr/bin/env python3
"""Simple Guppy test."""

try:
    from guppylang.decorator import guppy
    from guppylang.std.quantum import h, measure, qubit

    print("✅ Imports successful")

    @guppy
    def random_bit() -> bool:
        """Generate a random bit using quantum superposition."""
        q = qubit()
        h(q)
        return measure(q)

    print("✅ Function defined")

    from guppylang import guppy as guppy_compiler

    compiled = guppy_compiler.compile(random_bit)
    print(f"✅ Compiled: {type(compiled)}")

except (ImportError, RuntimeError) as e:
    print(f"❌ Error: {e}")
    import traceback

    traceback.print_exc()
