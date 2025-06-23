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

"""Testing SLR->QASM permute cases."""

from pecos.qeclib.steane.steane_class import Steane
from pecos.slr import Block, CReg, Main, Permute, SlrConverter


def test_permute1() -> None:
    """Test basic permutation functionality with Steane codes."""
    prog = Main(
        a := Steane("a"),
        b := Steane("b"),
        meas := CReg("meas", 2),
        Permute(a.d, b.d),
        Permute(a.a, b.a),
        a.mx(meas[0]),
        b.my(meas[1]),
    )

    qasm = SlrConverter(prog).qasm()

    print(qasm)

    # Check that permutation was applied correctly
    assert "ry(-pi/2) b_d[0];" in qasm.lower()
    assert "measure b_d[0] -> a_raw_meas[0];" in qasm.lower()
    assert "rx(-pi/2) a_d[0];" in qasm.lower()
    assert "measure a_d[0] -> b_raw_meas[0];" in qasm.lower()


def test_permute2() -> None:
    """Test permutation functionality using block structure."""

    def my_permute(a: Steane, b: Steane) -> Block:
        return Block(
            Permute(a.d, b.d),
            Permute(a.a, b.a),
        )

    prog = Main(
        a := Steane("a"),
        b := Steane("b"),
        meas := CReg("meas", 2),
        my_permute(a, b),
        a.mx(meas[0]),
        b.my(meas[1]),
    )

    qasm = SlrConverter(prog).qasm()

    print(qasm)

    # Check that permutation was applied correctly
    assert "ry(-pi/2) b_d[0];" in qasm.lower()
    assert "measure b_d[0] -> a_raw_meas[0];" in qasm.lower()
    assert "rx(-pi/2) a_d[0];" in qasm.lower()
    assert "measure a_d[0] -> b_raw_meas[0];" in qasm.lower()
