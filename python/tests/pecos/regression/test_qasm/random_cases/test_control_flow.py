# Copyright 2025 The PECOS Developers
#
# Licensed under the Apache License, Version 2.0 (the "License"); you may not use this file except in compliance with
# the License.You may obtain a copy of the License at
#
#     https://www.apache.org/licenses/LICENSE-2.0
#
# Unless required by applicable law or agreed to in writing, software distributed under the License is distributed on an
# "AS IS" BASIS, WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied. See the License for the
# specific language governing permissions and limitations under the License.

"""QASM regression tests for control flow structures."""

from collections.abc import Callable

from pecos.qeclib import qubit as qb
from pecos.slr import Block, CReg, If, Main, QReg, Repeat


def test_phys_teleport(compare_qasm: Callable[..., None]) -> None:
    """Test basic physical teleportation circuit QASM regression."""
    prog = Main(
        q := QReg("q", 2),
        c := CReg("m", 2),
        qb.H(q[0]),
        qb.CX(q[0], q[1]),
        qb.Measure(q) > c,
    )

    compare_qasm(prog, filename="phys.teleport")


def test_phys_tele_block_block(compare_qasm: Callable[..., None]) -> None:
    """Test teleportation with nested block structure QASM regression."""
    prog = Main(
        q := QReg("q", 2),
        c := CReg("m", 2),
        qb.H(q[0]),
        qb.CX(q[0], q[1]),
        qb.Measure(q) > c,
        Block(
            qb.H(q[0]),
            Block(
                qb.H(q[1]),
            ),
        ),
    )

    compare_qasm(prog, filename="phys.tele_block_block")


def test_phys_tele_if(compare_qasm: Callable[..., None]) -> None:
    """Test teleportation with conditional statements QASM regression."""
    prog = Main(
        q := QReg("q", 2),
        c := CReg("m", 2),
        qb.H(q[0]),
        qb.CX(q[0], q[1]),
        qb.Measure(q) > c,
        If(c == 0).Then(
            qb.H(q[0]),
        ),
    )

    compare_qasm(prog, filename="phys.tele_if")


def test_phys_tele_if_block_block(compare_qasm: Callable[..., None]) -> None:
    """Test teleportation with conditional and nested blocks QASM regression."""
    prog = Main(
        q := QReg("q", 2),
        c := CReg("m", 2),
        qb.H(q[0]),
        qb.CX(q[0], q[1]),
        qb.Measure(q) > c,
        If(c == 0).Then(
            qb.H(q[0]),
            Block(
                qb.H(q[1]),
            ),
        ),
    )

    compare_qasm(prog, filename="phys.tele_if_block_block")


def test_phys_tele_block_telep_block(compare_qasm: Callable[..., None]) -> None:
    """Test complex teleportation with multiple nested blocks QASM regression."""
    prog = Main(
        q := QReg("q", 2),
        c := CReg("m", 2),
        c2 := CReg("m2", 2),
        qb.H(q[0]),
        qb.CX(q[0], q[1]),
        qb.Measure(q) > c,
        Block(
            qb.Prep(q),
            qb.H(q[0]),
            qb.CX(q[0], q[1]),
            qb.Measure(q) > c2,
            Block(
                qb.H(q[0]),
            ),
        ),
    )

    compare_qasm(prog, filename="phys.tele_block_telep_block")


def test_phys_repeat(compare_qasm: Callable[..., None]) -> None:
    """Test teleportation with repeat blocks QASM regression."""
    prog = Main(
        q := QReg("q", 2),
        c := CReg("m", 2),
        Repeat(3).block(
            qb.H(q[0]),
            qb.CX(q[0], q[1]),
            qb.Measure(q) > c,
        ),
    )

    compare_qasm(prog, filename="phys.tele_repeat")
