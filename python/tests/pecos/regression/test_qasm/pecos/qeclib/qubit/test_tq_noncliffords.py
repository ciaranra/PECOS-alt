"""QASM regression tests for two-qubit non-Clifford gates."""

from collections.abc import Callable

from pecos.qeclib import qubit
from pecos.slr import QReg


def test_CH(compare_qasm: Callable[..., None]) -> None:
    """Test CH controlled Hadamard gate QASM regression."""
    q = QReg("q_test", 4)
    prog = qubit.CH(q[1], q[3])
    compare_qasm(prog)
