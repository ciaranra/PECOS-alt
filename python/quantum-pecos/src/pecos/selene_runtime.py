"""PECOS ByteMessage Runtime for Selene integration."""

import os
from pathlib import Path

from selene_core import Runtime


class ByteMessageRuntime(Runtime):
    """A Selene runtime plugin that collects quantum operations as ByteMessages
    for communication with PECOS.

    This runtime is used when running Guppy programs through PECOS's sim() API.
    It collects quantum operations from the Interface Plugin and converts them
    to ByteMessages that can be processed by PECOS quantum engines.
    """

    def __init__(self) -> None:
        """Initialize the ByteMessage runtime."""
        super().__init__()

        # Find the ByteMessage simulator plugin library
        self.library_file = self._find_plugin_library()

        # No special arguments needed for basic operation
        self.args = []

    def _find_plugin_library(self) -> Path:
        """Find the ByteMessage simulator plugin library."""
        # Check environment variable first
        if "PECOS_BYTMESSAGE_RUNTIME_PATH" in os.environ:
            path = Path(os.environ["PECOS_BYTMESSAGE_RUNTIME_PATH"])
            if path.exists():
                return path

        # Determine library name based on OS
        import platform

        if platform.system() == "Windows":
            lib_name = "pecos_selene_plugins.dll"
        elif platform.system() == "Darwin":
            lib_name = "libpecos_selene_plugins.dylib"
        else:
            lib_name = "libpecos_selene_plugins.so"

        # Check in PECOS target directories
        # Start from this file's location
        pecos_root = Path(__file__).parent.parent.parent.parent.parent

        # Check debug build
        debug_path = pecos_root / "target" / "debug" / lib_name
        if debug_path.exists():
            return debug_path

        # Check release build
        release_path = pecos_root / "target" / "release" / lib_name
        if release_path.exists():
            return release_path

        # Check in deps directories
        debug_deps = pecos_root / "target" / "debug" / "deps" / lib_name
        if debug_deps.exists():
            return debug_deps

        release_deps = pecos_root / "target" / "release" / "deps" / lib_name
        if release_deps.exists():
            return release_deps

        msg = (
            f"Could not find ByteMessage runtime plugin library ({lib_name}). "
            "Please ensure pecos-selene-plugins is built. "
            "You can also set PECOS_BYTMESSAGE_RUNTIME_PATH environment variable."
        )
        raise RuntimeError(
            msg,
        )

    def get_init_args(self) -> list[str]:
        """Get initialization arguments for the runtime."""
        return self.args
