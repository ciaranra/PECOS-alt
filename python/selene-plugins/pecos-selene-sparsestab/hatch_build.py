# Copyright 2025 The PECOS Developers
#
# Licensed under the Apache License, Version 2.0 (the "License"); you may not use this file except
# in compliance with the License. You may obtain a copy of the License at
#
#     https://www.apache.org/licenses/LICENSE-2.0
#
# Unless required by applicable law or agreed to in writing, software distributed under the License
# is distributed on an "AS IS" BASIS, WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express
# or implied. See the License for the specific language governing permissions and limitations under
# the License.

"""Custom hatch build hook to compile and include the Rust shared library."""

import platform
import shutil
import subprocess
from pathlib import Path

from hatchling.builders.hooks.plugin.interface import BuildHookInterface


class PecosSeleneSparsestabBuildHook(BuildHookInterface):
    """Build hook that compiles the Rust plugin and copies it to the Python package."""

    def initialize(self, version: str, build_data: dict) -> None:
        """Build the Rust library and include it as an artifact."""
        # Determine library extension based on platform
        system = platform.system()
        if system == "Linux":
            lib_prefix = "lib"
            lib_suffix = ".so"
        elif system == "Darwin":
            lib_prefix = "lib"
            lib_suffix = ".dylib"
        elif system == "Windows":
            lib_prefix = ""
            lib_suffix = ".dll"
        else:
            msg = f"Unsupported platform: {system}"
            raise RuntimeError(msg)

        # Get the root directory (where pyproject.toml is)
        root = Path(self.root)
        lib_name = "pecos_selene_sparsestab"
        cargo_package = "pecos-selene-sparsestab"

        self.app.display_info(f"Building {cargo_package}...")

        # Run cargo build from the PECOS workspace root
        workspace_root = root.parent.parent  # Go up to PECOS root
        result = subprocess.run(
            [
                "cargo",
                "build",
                "--release",
                "--package",
                cargo_package,
            ],
            cwd=workspace_root,
            capture_output=True,
            text=True,
        )

        if result.returncode != 0:
            self.app.display_error(f"Failed to build {cargo_package}:")
            self.app.display_error(result.stderr)
            msg = f"Cargo build failed for {cargo_package}"
            raise RuntimeError(msg)

        # Find the compiled library
        lib_filename = f"{lib_prefix}{lib_name}{lib_suffix}"
        source_lib = workspace_root / "target" / "release" / lib_filename

        if not source_lib.exists():
            msg = f"Built library not found: {source_lib}"
            raise RuntimeError(msg)

        # Copy to the _dist/lib directory in the Python package
        dest_dir = root / "python" / "pecos_selene_sparsestab" / "_dist" / "lib"
        dest_dir.mkdir(parents=True, exist_ok=True)
        dest_lib = dest_dir / lib_filename

        self.app.display_info(f"Copying {source_lib} -> {dest_lib}")
        shutil.copy2(source_lib, dest_lib)

        # Collect artifacts
        artifacts = []
        dist_dir = root / "python" / "pecos_selene_sparsestab" / "_dist"
        for artifact in dist_dir.rglob("*"):
            if artifact.is_file():
                rel_path = artifact.relative_to(root)
                artifacts.append(str(rel_path.as_posix()))

        self.app.display_info("Found artifacts:")
        for a in artifacts:
            self.app.display_info(f"    {a}")

        build_data["artifacts"] += artifacts
