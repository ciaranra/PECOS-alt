"""QASM regression tests for two-qubit Clifford gates."""
from collections.abc import Callable

from pecos.qeclib import qubit
from pecos.slr import QReg


def test_CX(compare_qasm: Callable[..., None]) -> None:
    """Test CX controlled Pauli gate QASM regression."""
    q = QReg("q_test", 4)
    prog = qubit.CX(q[1], q[3])
    compare_qasm(prog)


def test_CY(compare_qasm: Callable[..., None]) -> None:
    """Test CY controlled Pauli gate QASM regression."""
    q = QReg("q_test", 4)
    prog = qubit.CY(q[1], q[3])
    compare_qasm(prog)


def test_CZ(compare_qasm: Callable[..., None]) -> None:
    """Test CZ controlled Pauli gate QASM regression."""
    q = QReg("q_test", 4)
    prog = qubit.CZ(q[1], q[3])
    compare_qasm(prog)


def test_SXX(compare_qasm: Callable[..., None]) -> None:
    """Test SXX two-qubit Clifford gate QASM regression."""
    q = QReg("q_test", 4)
    prog = qubit.SXX(q[1], q[3])
    compare_qasm(prog)


def test_SXXdg(compare_qasm: Callable[..., None]) -> None:
    """Test SXXdg two-qubit Clifford gate QASM regression."""
    q = QReg("q_test", 4)
    prog = qubit.SXXdg(q[1], q[3])
    compare_qasm(prog)


def test_SYY(compare_qasm: Callable[..., None]) -> None:
    """Test SYY two-qubit Clifford gate QASM regression."""
    q = QReg("q_test", 4)
    prog = qubit.SYY(q[1], q[3])
    compare_qasm(prog)


def test_SYYdg(compare_qasm: Callable[..., None]) -> None:
    """Test SYYdg two-qubit Clifford gate QASM regression."""
    q = QReg("q_test", 4)
    prog = qubit.SYYdg(q[1], q[3])
    compare_qasm(prog)


def test_SZZ(compare_qasm: Callable[..., None]) -> None:
    """Test SZZ two-qubit Clifford gate QASM regression."""
    q = QReg("q_test", 4)
    prog = qubit.SZZ(q[1], q[3])
    compare_qasm(prog)


def test_SZZdg(compare_qasm: Callable[..., None]) -> None:
    """Test SZZdg two-qubit Clifford gate QASM regression."""
    q = QReg("q_test", 4)
    prog = qubit.SZZdg(q[1], q[3])
    compare_qasm(prog)
