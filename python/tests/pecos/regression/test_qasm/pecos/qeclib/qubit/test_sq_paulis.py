"""QASM regression tests for single-qubit Pauli gates."""
from collections.abc import Callable

from pecos.qeclib import qubit
from pecos.slr import QReg


def test_X(compare_qasm: Callable[..., None]) -> None:
    """Test X Pauli gate QASM regression."""
    q = QReg("q_test", 2)

    prog = qubit.X(q[1])
    compare_qasm(prog)


def test_Y(compare_qasm: Callable[..., None]) -> None:
    """Test Y Pauli gate QASM regression."""
    q = QReg("q_test", 2)
    prog = qubit.Y(q[1])
    compare_qasm(prog)


def test_Z(compare_qasm: Callable[..., None]) -> None:
    """Test Z Pauli gate QASM regression."""
    q = QReg("q_test", 2)
    prog = qubit.Z(q[1])
    compare_qasm(prog)
