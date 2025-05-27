"""QASM regression tests for Steane single-qubit Hadamard gates."""
from collections.abc import Callable

from pecos.qeclib.steane.gates_sq.hadamards import H
from pecos.slr import QReg


def test_H(compare_qasm: Callable[..., None]) -> None:
    """Test Steane H Hadamard gate QASM regression."""
    q = QReg("q_test", 7)

    block = H(q)
    compare_qasm(block)
