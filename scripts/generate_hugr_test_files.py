"""Generate real HUGR test files from guppy functions for Rust integration tests."""

import json
import shutil
import sys
import traceback
from collections.abc import Callable
from pathlib import Path

# Add Python package to path
python_path = Path(__file__).parent.parent / "python" / "quantum-pecos" / "src"
sys.path.insert(0, str(python_path))

try:
    from guppylang import guppy
    from guppylang.std.quantum import cx, h, measure, qubit

    print("Guppylang imported successfully")
except ImportError as e:
    print(f"Failed to import guppylang: {e}")
    print("Please ensure guppylang is installed: pip install guppylang")
    sys.exit(1)


# Define quantum circuits using guppylang
@guppy
def simple_hadamard() -> bool:
    """Apply Hadamard gate and measure."""
    q = qubit()
    h(q)
    return measure(q)


@guppy
def bell_state() -> tuple[bool, bool]:
    """Create Bell state and measure both qubits."""
    q1 = qubit()
    q2 = qubit()
    h(q1)
    cx(q1, q2)
    return (measure(q1), measure(q2))


@guppy
def ghz_state() -> tuple[bool, bool, bool]:
    """Create 3-qubit GHZ state."""
    q1 = qubit()
    q2 = qubit()
    q3 = qubit()
    h(q1)
    cx(q1, q2)
    cx(q2, q3)
    return (measure(q1), measure(q2), measure(q3))


def compile_and_save_hugr(func: Callable, name: str, output_dir: Path) -> Path | None:
    """Compile a guppy function to HUGR and save it."""
    try:
        print(f"\nCompiling {name}...")

        # Compile to HUGR
        hugr = func.compile_hugr()
        print("  Successfully compiled to HUGR")

        # Serialize to bytes
        hugr_bytes = hugr.to_raw().to_bytes()
        print(f"  Serialized to {len(hugr_bytes)} bytes")

        # Save as raw HUGR binary format (.hugr)
        # This is the raw binary HUGR format that can be parsed by Rust
        # NOTE: These files can be large (hundreds of KB for simple circuits)
        # and may not be human-readable. If they become too large in the
        # future, we may need to reconsider storing these in git.
        hugr_file = output_dir / f"{name}.hugr"
        with hugr_file.open("wb") as f:
            f.write(hugr_bytes)
        print(f"  Saved to {hugr_file}")

        # Also save as JSON for readability
        try:
            hugr_json = hugr.to_raw().to_json()
            json_file = output_dir / f"{name}.json"
            with json_file.open("w") as f:
                json.dump(json.loads(hugr_json), f, indent=2)
            print(f"  Also saved as JSON to {json_file}")
        except (json.JSONDecodeError, OSError, ValueError) as e:
            print(f"  Warning: Could not save JSON version: {e}")

        # Notes for future reference:
        # 1. The binary .hugr format is the canonical format for HUGR
        # 2. JSON is provided for debugging/readability but may not preserve all data
        # 3. The .hugr format is what the toolchain expects

    except (ImportError, RuntimeError, ValueError, AttributeError) as e:
        print(f"  Failed to compile {name}: {e}")
        traceback.print_exc()
        return None
    else:
        return hugr_file


def main() -> None:
    """Generate HUGR test files."""
    # Define output directory
    output_dir = Path(__file__).parent.parent / "test_data" / "hugr"
    output_dir.mkdir(parents=True, exist_ok=True)

    print(f"Generating HUGR test files in {output_dir}")
    print("=" * 60)

    # Compile all test circuits
    test_circuits = [
        (simple_hadamard, "simple_hadamard"),
        (bell_state, "bell_state"),
        (ghz_state, "ghz_state"),
    ]

    generated_files = []
    for func, name in test_circuits:
        hugr_file = compile_and_save_hugr(func, name, output_dir)
        if hugr_file:
            generated_files.append(hugr_file)

    print("\n" + "=" * 60)
    print(f"\nGenerated {len(generated_files)} HUGR files:")
    for file in generated_files:
        size_kb = file.stat().st_size / 1024
        print(f"  - {file.name} ({size_kb:.1f} KB)")

    # Check file sizes
    total_size = sum(f.stat().st_size for f in generated_files)
    if total_size > 1024 * 1024:  # 1 MB
        print(
            f"\nWARNING: Total size is {total_size / 1024 / 1024:.1f} MB. "
            "Consider using git-lfs for these files.",
        )

    # Generate README
    readme_path = output_dir / "README.md"
    with readme_path.open("w") as f:
        f.write("# HUGR Test Files\n\n")
        f.write(
            "This directory contains HUGR (Hierarchical Unified Graph Representation) "
            "test files generated from Guppy quantum circuits.\n\n",
        )
        f.write("## Files\n\n")
        for func, name in test_circuits:
            f.write(f"- `{name}.hugr`: {func.__doc__}\n")
        f.write("\n## Format\n\n")
        f.write(
            "- `.hugr` files are the raw binary HUGR format (canonical)\n"
            "- `.json` files are human-readable JSON representations (for debugging)\n\n",
        )
        f.write("## Regenerating\n\n")
        f.write("To regenerate these files, run:\n")
        f.write("```bash\npython scripts/generate_hugr_test_files.py\n```\n")

    print(f"\nGenerated README at {readme_path}")

    # Copy a sample file to the parent directory for quick access
    if generated_files:
        src = output_dir / "bell_state.hugr"
        dst = output_dir.parent / "bell_state_sample.hugr"
        shutil.copy2(src, dst)
        print(f"\nCopied sample file to {dst}")


if __name__ == "__main__":
    main()
