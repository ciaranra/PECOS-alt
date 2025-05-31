"""QASM regression tests for Steane encoding circuits."""

from collections.abc import Callable

from pecos.qeclib.steane.preps.encoding_circ import EncodingCircuit
from pecos.slr import QReg


def test_EncodingCircuit(compare_qasm: Callable[..., None]) -> None:
    """Test Steane encoding circuit QASM regression."""
    q = QReg("q_test", 7)

    block = EncodingCircuit(q)
    compare_qasm(block)
