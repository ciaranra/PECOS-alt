"""QASM regression tests for Steane single-qubit Pauli gates."""
from collections.abc import Callable

from pecos.qeclib.steane.gates_sq.paulis import X, Y, Z
from pecos.slr import QReg


def test_X(compare_qasm: Callable[..., None]) -> None:
    """Test Steane X Pauli gate QASM regression."""
    q = QReg("q_test", 7)

    block = X(q)
    compare_qasm(block)


def test_Y(compare_qasm: Callable[..., None]) -> None:
    """Test Steane Y Pauli gate QASM regression."""
    q = QReg("q_test", 7)

    block = Y(q)
    compare_qasm(block)


def test_Z(compare_qasm: Callable[..., None]) -> None:
    """Test Steane Z Pauli gate QASM regression."""
    q = QReg("q_test", 7)

    block = Z(q)
    compare_qasm(block)
