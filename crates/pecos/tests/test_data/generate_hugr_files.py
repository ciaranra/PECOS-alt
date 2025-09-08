#!/usr/bin/env python3
"""Generate HUGR test files from Python using the GuppyFrontend."""

import sys
from pathlib import Path

# Add Python package to path
sys.path.insert(0, str(Path(__file__).parents[4] / "python" / "quantum-pecos" / "src"))

import shutil

from guppylang import guppy
from guppylang.std.quantum import cx, h, measure, qubit
from pecos.frontends.guppy_frontend import GuppyFrontend


# Define test functions
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


def main() -> None:
    """Generate HUGR test files."""
    output_dir = Path(__file__).parent / "hugr"
    output_dir.mkdir(exist_ok=True)

    print(f"Generating HUGR test files in {output_dir}")

    # Use GuppyFrontend to compile to HUGR
    frontend = GuppyFrontend(use_rust_backend=True)

    test_functions = [
        (bell_state, "bell_state"),
        (single_hadamard, "single_hadamard"),
        (ghz_state, "ghz_state"),
    ]

    generated_files = []

    try:
        for func, name in test_functions:
            try:
                # Compile function to LLVM IR (this generates HUGR internally)
                frontend.compile_function(func)

                # The frontend saves intermediate HUGR files in its temp directory
                # Let's find and copy them
                temp_dir = frontend._temp_dir
                if temp_dir:
                    hugr_files = list(Path(temp_dir).glob("*.hugr"))
                    if hugr_files:
                        # Copy the HUGR file
                        src_hugr = hugr_files[0]  # Take the first one
                        dst_hugr = output_dir / f"{name}.hugr"
                        shutil.copy2(src_hugr, dst_hugr)
                        print(f"✓ Generated {dst_hugr}")
                        generated_files.append(dst_hugr)
                    else:
                        print(f"✗ No HUGR file found for {name}")
                else:
                    print(f"✗ No temp directory for {name}")

            except Exception as e:
                print(f"✗ Failed to generate {name}: {e}")

    finally:
        # Clean up
        frontend.cleanup()

    # Generate README
    readme_path = output_dir / "README.md"
    with open(readme_path, "w") as f:
        f.write("# HUGR Test Files\n\n")
        f.write(
            "This directory contains HUGR test files generated from guppy functions.\n\n",
        )
        f.write("## Files\n\n")
        for file_path in generated_files:
            name = file_path.stem
            f.write(f"- `{file_path.name}` - {name.replace('_', ' ').title()}\n")
        f.write("\n## Regenerating Files\n\n")
        f.write("To regenerate these files, run:\n")
        f.write("```bash\n")
        f.write("uv run python crates/pecos/tests/test_data/generate_hugr_files.py\n")
        f.write("```\n")

    print(f"\n✓ Generated {len(generated_files)} HUGR test files")
    print(f"✓ Generated {readme_path}")


if __name__ == "__main__":
    main()
