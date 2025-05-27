"""QASM regression tests for single-qubit Hadamard gates."""
from collections.abc import Callable

from pecos.qeclib import qubit
from pecos.slr import QReg


def test_H(compare_qasm: Callable[..., None]) -> None:
    """Test H Hadamard gate QASM regression."""
    q = QReg("q_test", 2)

    prog = qubit.H(q[1])
    compare_qasm(prog)
