"""QASM regression tests for single-qubit non-Clifford gates."""

from collections.abc import Callable

from pecos.qeclib import qubit
from pecos.slr import QReg


def test_T(compare_qasm: Callable[..., None]) -> None:
    """Test T non-Clifford gate QASM regression."""
    q = QReg("q_test", 2)

    prog = qubit.T(q[1])
    compare_qasm(prog)


def test_Tdg(compare_qasm: Callable[..., None]) -> None:
    """Test Tdg non-Clifford gate QASM regression."""
    q = QReg("q_test", 2)

    prog = qubit.Tdg(q[1])
    compare_qasm(prog)
