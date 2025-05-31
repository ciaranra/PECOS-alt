"""QASM regression tests for Steane non-flagged Z measurement."""

from collections.abc import Callable

from pecos.qeclib.steane.meas.measure_z import NoFlagMeasureZ
from pecos.slr import CReg, QReg


def test_MeasureX(compare_qasm: Callable[..., None]) -> None:
    """Test Steane non-flagged Z measurement QASM regression."""
    q = QReg("q_test", 7)
    a = QReg("a_test", 1)
    out = CReg("out_test", 1)

    block = NoFlagMeasureZ(q, a, out)
    compare_qasm(block)
