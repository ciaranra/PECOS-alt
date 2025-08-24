# Copyright 2018 The PECOS Developers
# Copyright 2018 National Technology & Engineering Solutions of Sandia, LLC (NTESS). Under the terms of Contract
# DE-NA0003525 with NTESS, the U.S. Government retains certain rights in this software.
#
# Licensed under the Apache License, Version 2.0 (the "License"); you may not use this file except in compliance with
# the License.You may obtain a copy of the License at
#
#     https://www.apache.org/licenses/LICENSE-2.0
#
# Unless required by applicable law or agreed to in writing, software distributed under the License is distributed on an
# "AS IS" BASIS, WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied. See the License for the
# specific language governing permissions and limitations under the License.
"""Performance Estimator of Codes On Surfaces (PECOS).

==================================================
A framework for developing, studying, and evaluating quantum error-correcting codes.
"""

# Allow for other namespace packages
__path__ = __import__("pkgutil").extend_path(__path__, __name__)

from importlib.metadata import PackageNotFoundError, version

try:
    __version__ = version("quantum-pecos")
except PackageNotFoundError:
    __version__ = "0.0.0"

# PECOS namespaces
from typing import NoReturn

from pecos import (
    circuit_converters,
    circuits,
    decoders,
    engines,
    error_models,
    frontends,
    misc,
    protocols,
    qeccs,
    rslib,
    simulators,
    tools,
)
from pecos.circuits.quantum_circuit import QuantumCircuit
from pecos.engines import circuit_runners
from pecos.engines.cvm.binarray import BinArray
from pecos.engines.hybrid_engine_old import HybridEngine

# Import Guppy functionality (with graceful fallback)
try:
    from pecos.frontends import (
        get_guppy_backends,
        guppy_sim,
        run_guppy,
        run_guppy_batch,
    )

    GUPPY_INTEGRATION_AVAILABLE = True
except ImportError:
    GUPPY_INTEGRATION_AVAILABLE = False

    def run_guppy(*args: object, **kwargs: object) -> NoReturn:
        """Stub for run_guppy when Guppy integration is not available."""
        del args, kwargs  # Unused
        msg = "Guppy integration not available. Install with: pip install quantum-pecos[guppy]"
        raise ImportError(
            msg,
        )

    def guppy_sim(*args: object, **kwargs: object) -> NoReturn:
        """Stub for guppy_sim when Guppy integration is not available."""
        del args, kwargs  # Unused
        msg = "Guppy integration not available. Install with: pip install quantum-pecos[guppy]"
        raise ImportError(
            msg,
        )

# Import Selene Bridge Plugin (with graceful fallback)
try:
    from pecos.selene_plugins.simulators import PecosBridgePlugin
    SELENE_BRIDGE_AVAILABLE = True
except ImportError:
    SELENE_BRIDGE_AVAILABLE = False
    PecosBridgePlugin = None

    def run_guppy_batch(*args: object, **kwargs: object) -> NoReturn:
        """Stub for run_guppy_batch when Guppy integration is not available."""
        del args, kwargs  # Unused
        msg = "Guppy integration not available. Install with: pip install quantum-pecos[guppy]"
        raise ImportError(
            msg,
        )

    def get_guppy_backends() -> dict[str, object]:
        """Stub for get_guppy_backends when Guppy integration is not available."""
        return {"guppy_available": False, "error": "Guppy integration not available"}


__all__ = [
    "GUPPY_INTEGRATION_AVAILABLE",
    "BinArray",
    "HybridEngine",
    "QuantumCircuit",
    "__version__",
    "circuit_converters",
    "circuit_runners",
    "circuits",
    "decoders",
    "engines",
    "error_models",
    "frontends",
    "get_guppy_backends",
    "guppy_sim",
    "misc",
    "protocols",
    "qeccs",
    "rslib",
    # Guppy integration
    "run_guppy",
    "run_guppy_batch",
    "simulators",
    "tools",
]
