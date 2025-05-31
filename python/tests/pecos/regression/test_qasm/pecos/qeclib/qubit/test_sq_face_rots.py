"""QASM regression tests for single-qubit face rotation gates."""

from collections.abc import Callable

from pecos.qeclib import qubit
from pecos.slr import QReg


def test_F(compare_qasm: Callable[..., None]) -> None:
    """Test F face rotation gate QASM regression."""
    q = QReg("q_test", 2)

    prog = qubit.F(q[1])
    compare_qasm(prog)


def test_Fdg(compare_qasm: Callable[..., None]) -> None:
    """Test Fdg face rotation gate QASM regression."""
    q = QReg("q_test", 2)

    prog = qubit.Fdg(q[1])
    compare_qasm(prog)


def test_F4(compare_qasm: Callable[..., None]) -> None:
    """Test F4 face rotation gate QASM regression."""
    q = QReg("q_test", 2)

    prog = qubit.F4(q[1])
    compare_qasm(prog)


def test_F4dg(compare_qasm: Callable[..., None]) -> None:
    """Test F4dg face rotation gate QASM regression."""
    q = QReg("q_test", 2)

    prog = qubit.F4dg(q[1])
    compare_qasm(prog)
