#!/usr/bin/env python3
"""Generate real HUGR test files from guppy functions for Rust integration tests."""

import sys
from pathlib import Path

# Add Python package to path
python_path = Path(__file__).parent.parent / "python" / "quantum-pecos" / "src"
sys.path.insert(0, str(python_path))

try:
    import guppylang
    from guppylang import guppy
    from guppylang.std.quantum import qubit, h, cx, measure
    print("✓ Guppylang imported successfully")
except ImportError as e:
    print(f"✗ Failed to import guppylang: {e}")
    print("Please ensure guppylang is installed: pip install guppylang")
    sys.exit(1)


# Define simple quantum circuits as guppy functions
@guppy
def bell_state() -> tuple[bool, bool]:
    """Create a Bell state and measure both qubits."""
    q1 = qubit()
    q2 = qubit()
    h(q1)
    cx(q1, q2)
    return measure(q1), measure(q2)


@guppy  
def single_hadamard() -> bool:
    """Apply Hadamard to a qubit and measure."""
    q = qubit()
    h(q)
    return measure(q)


@guppy
def ghz_state() -> tuple[bool, bool, bool]:
    """Create a 3-qubit GHZ state."""
    q1 = qubit()
    q2 = qubit()
    q3 = qubit()
    
    h(q1)
    cx(q1, q2)
    cx(q2, q3)
    
    return measure(q1), measure(q2), measure(q3)


def compile_and_save_hugr(func, name: str, output_dir: Path):
    """Compile a guppy function to HUGR and save it."""
    try:
        print(f"\nCompiling {name}...")
        
        # Compile the function to get HUGR
        compiled = guppy.compile_function(func)
        print(f"  ✓ Compilation successful")
        
        # Get HUGR bytes
        hugr_bytes = compiled.package.to_bytes()
        print(f"  ✓ Generated {len(hugr_bytes)} bytes of HUGR data")
        
        # Save HUGR file
        # Note: The current HUGR "binary" format is actually a header + JSON,
        # which is git-friendly. If HUGR moves to a true binary format in the
        # future, we may need to reconsider storing these in git.
        hugr_file = output_dir / f"{name}.hugr"
        with open(hugr_file, 'wb') as f:
            f.write(hugr_bytes)
        print(f"  ✓ Saved to {hugr_file}")
        
        # Check if the format is still text-based (for git-friendliness)
        try:
            # Try to decode as UTF-8 after the header
            text_check = hugr_bytes[10:100].decode('utf-8')
            if not text_check.startswith('{'):
                print(f"  ⚠️  Warning: HUGR format may no longer be text-based")
        except UnicodeDecodeError:
            print(f"  ⚠️  Warning: HUGR format appears to be binary - consider using git-lfs")
        
        # We previously generated .hugr.json files but removed them since:
        # 1. The .hugr format already contains JSON data (after the header)
        # 2. It avoids duplication and potential sync issues
        # 3. The .hugr format is what the toolchain expects
        
        return hugr_file
        
    except Exception as e:
        print(f"  ✗ Failed to compile {name}: {e}")
        import traceback
        traceback.print_exc()
        return None


def main():
    """Generate HUGR test files for Rust integration tests."""
    # Output directory for test data
    output_dir = Path(__file__).parent.parent / "crates" / "pecos" / "tests" / "test_data" / "hugr"
    output_dir.mkdir(parents=True, exist_ok=True)
    
    print(f"Generating HUGR test files in: {output_dir}")
    print("=" * 60)
    
    # Test functions to compile
    test_cases = [
        (bell_state, "bell_state"),
        (single_hadamard, "single_hadamard"),
        (ghz_state, "ghz_state"),
    ]
    
    generated_files = []
    
    for func, name in test_cases:
        hugr_file = compile_and_save_hugr(func, name, output_dir)
        if hugr_file:
            generated_files.append(hugr_file)
    
    print("\n" + "=" * 60)
    print(f"Summary: Generated {len(generated_files)}/{len(test_cases)} HUGR files")
    
    # Generate README
    readme_path = output_dir / "README.md"
    with open(readme_path, 'w') as f:
        f.write("# HUGR Test Files\n\n")
        f.write("This directory contains HUGR test files generated from guppy quantum circuits.\n\n")
        f.write("## Files\n\n")
        for hugr_file in generated_files:
            name = hugr_file.stem
            f.write(f"- `{hugr_file.name}` - {name.replace('_', ' ').title()}\n")
        f.write("\n## Regenerating Files\n\n")
        f.write("To regenerate these files, run:\n")
        f.write("```bash\n")
        f.write("uv run python scripts/generate_hugr_test_files.py\n")
        f.write("```\n\n")
        f.write("Note: This requires guppylang to be installed.\n")
    
    print(f"\n✓ Generated README at {readme_path}")
    
    # Also copy the bell_state.hugr to the expected location for tests
    if any(f.name == "bell_state.hugr" for f in generated_files):
        src = output_dir / "bell_state.hugr"
        dst = output_dir.parent / "bell_state_sample.hugr"
        import shutil
        shutil.copy2(src, dst)
        print(f"✓ Copied bell_state.hugr to {dst} for integration tests")


if __name__ == "__main__":
    main()