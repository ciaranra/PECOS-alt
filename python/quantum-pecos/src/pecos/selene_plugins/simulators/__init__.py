"""Selene simulator plugins for PECOS.

This package provides simulator plugins that bridge Selene's quantum circuit
execution with PECOS's quantum simulation infrastructure.
"""

# Bridge Plugin (optional - requires selene-core)
try:
    from pecos.selene_plugins.simulators.bridge import PecosBridgePlugin

    __all__ = ["PecosBridgePlugin"]
except ImportError:
    # selene-core not available, Bridge plugin unavailable
    __all__ = []
    PecosBridgePlugin = None
