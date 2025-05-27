"""QASM regression tests for Steane single-qubit face rotation gates."""
from collections.abc import Callable

from pecos.qeclib.steane.gates_sq.face_rots import F, Fdg
from pecos.slr import QReg


def test_F(compare_qasm: Callable[..., None]) -> None:
    """Test Steane F face rotation gate QASM regression."""
    q = QReg("q_test", 7)

    block = F(q)
    compare_qasm(block)


def test_Fdg(compare_qasm: Callable[..., None]) -> None:
    """Test Steane Fdg face rotation gate QASM regression."""
    q = QReg("q_test", 7)

    block = Fdg(q)
    compare_qasm(block)
