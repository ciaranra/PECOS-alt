"""PECOS Bridge Plugin for Selene.

This module provides a bridge between Selene quantum circuit execution 
and PECOS quantum simulation infrastructure via ByteMessage communication.

The bridge plugin acts as a Selene simulator that translates quantum operations
into PECOS ByteMessages for processing by PECOS quantum engines.
"""

from .plugin import PecosBridgePlugin

__all__ = ["PecosBridgePlugin"]