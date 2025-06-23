# Copyright 2019 The PECOS Developers
# Copyright 2018 National Technology & Engineering Solutions of Sandia, LLC (NTESS). Under the terms of Contract
# DE-NA0003525 with NTESS, the U.S. Government retains certain rights in this software.
#
# Licensed under the Apache License, Version 2.0 (the "License"); you may not use this file except in compliance with
# the License.You may obtain a copy of the License at
#
#     https://www.apache.org/licenses/LICENSE-2.0
#
# Unless required by applicable law or agreed to in writing, software distributed under the License is distributed on an
# "AS IS" BASIS, WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied. See the License for the
# specific language governing permissions and limitations under the License.

"""Single-qubit gate operations for ProjectQ simulator.

This module provides single-qubit quantum gate operations for the ProjectQ simulator, including Pauli gates,
rotation gates, Hadamard gates, and other fundamental single-qubit operations using the ProjectQ framework.
"""

from __future__ import annotations

from typing import TYPE_CHECKING

if TYPE_CHECKING:
    from pecos.simulators.projectq.state import ProjectQSim
    from pecos.typing import SimulatorGateParams

import numpy as np
from projectq import ops

from pecos.simulators.projectq.helper import MakeFunc


def Identity(state: ProjectQSim, qubit: int, **_params: SimulatorGateParams) -> None:
    """Identity does nothing.

    X -> X

    Z -> Z

    Y -> Y

    Args:
        state: The ProjectQ state instance.
        qubit (int): The qubit index to apply the gate to.

    Returns: None

    """


def X(state: ProjectQSim, qubit: int, **_params: SimulatorGateParams) -> None:
    """Pauli X.

    X -> X

    Z -> -Z

    Y -> -Y

    Args:
        state: The ProjectQ state instance.
        qubit (int): The qubit index to apply the gate to.

    Returns: None

    """
    ops.X | state.qids[qubit]


def Y(state: ProjectQSim, qubit: int, **_params: SimulatorGateParams) -> None:
    """X -> -X.

    Z -> -Z

    Y -> Y

    Args:
        state: The ProjectQ state instance.
        qubit (int): The qubit index to apply the gate to.

    Returns: None

    """
    ops.Y | state.qids[qubit]


def Z(state: ProjectQSim, qubit: int, **_params: SimulatorGateParams) -> None:
    """X -> -X.

    Z -> Z

    Y -> -Y

    Args:
        state: The ProjectQ state instance.
        qubit (int): The qubit index to apply the gate to.

    Returns: None

    """
    ops.Z | state.qids[qubit]


RX = MakeFunc(ops.Rx, angle=True).func  # Rotation about X (takes angle arg)
RY = MakeFunc(ops.Ry, angle=True).func  # Rotation about Y (takes angle arg)
RZ = MakeFunc(ops.Rz, angle=True).func  # Rotation about Z (takes angle arg)


def R1XY(
    state: ProjectQSim,
    qubit: int,
    angles: tuple[float, float],
    **_params: SimulatorGateParams,
) -> None:
    """Apply a single-qubit rotation gate composed of Y and Z rotations.

    R1XY(theta, phi) = U1q(theta, phi) = RZ(phi-pi/2)*RY(theta)*RZ(-phi+pi/2).

    Args:
        state: The ProjectQ state instance.
        qubit (int): The qubit index to apply the gate to.
        angles (tuple[float, float]): A tuple of (theta, phi) rotation angles.
        **_params: Unused additional parameters (kept for interface compatibility).
    """
    theta = angles[0]
    phi = angles[1]

    RZ(state, qubit, angle=-phi + np.pi / 2)
    RY(state, qubit, angle=theta)
    RZ(state, qubit, angle=phi - np.pi / 2)


def SX(state: ProjectQSim, qubit: int, **_params: SimulatorGateParams) -> None:
    """Square-root of X gate class."""
    RX(state, qubit, angle=np.pi / 2)


def SXdg(state: ProjectQSim, qubit: int, **_params: SimulatorGateParams) -> None:
    """Adjoint of the square-root of X gate class."""
    RX(state, qubit, angle=-np.pi / 2)


def SY(state: ProjectQSim, qubit: int, **_params: SimulatorGateParams) -> None:
    """Square-root of Y gate class."""
    RY(state, qubit, angle=np.pi / 2)


def SYdg(state: ProjectQSim, qubit: int, **_params: SimulatorGateParams) -> None:
    """Adjoint of the square-root of Y gate class."""
    RY(state, qubit, angle=-np.pi / 2)


def SZ(state: ProjectQSim, qubit: int, **_params: SimulatorGateParams) -> None:
    """Square-root of Z gate class."""
    ops.S | state.qids[qubit]


def SZdg(state: ProjectQSim, qubit: int, **_params: SimulatorGateParams) -> None:
    """Adjoint of the square-root of Z gate class."""
    ops.Sdag | state.qids[qubit]


def H(state: ProjectQSim, qubit: int, **_params: SimulatorGateParams) -> None:
    """Square root of Z.

    X -> Z

    Z -> X

    Y -> -Y

    Args:
        state: The ProjectQ state instance.
        qubit (int): The qubit index to apply the gate to.

    Returns: None

    """
    ops.H | state.qids[qubit]


def H2(state: ProjectQSim, qubit: int, **_params: SimulatorGateParams) -> None:
    """Apply H2 Hadamard variant gate (Ry(π/2) followed by Z).

    Args:
        state: ProjectQ simulator state.
        qubit: Target qubit index.
    """
    # @property
    # def matrix(self):

    ops.Ry(np.pi / 2) | state.qids[qubit]
    ops.Z | state.qids[qubit]


def H3(state: ProjectQSim, qubit: int, **_params: SimulatorGateParams) -> None:
    """Apply H3 Hadamard variant gate (S followed by Y).

    Args:
        state: ProjectQ simulator state.
        qubit: Target qubit index.
    """
    # @property
    # def matrix(self):

    ops.S | state.qids[qubit]
    ops.Y | state.qids[qubit]


def H4(state: ProjectQSim, qubit: int, **_params: SimulatorGateParams) -> None:
    """Apply H4 Hadamard variant gate (S followed by X).

    Args:
        state: ProjectQ simulator state.
        qubit: Target qubit index.
    """
    # @property
    # def matrix(self):

    ops.S | state.qids[qubit]
    ops.X | state.qids[qubit]


def H5(state: ProjectQSim, qubit: int, **_params: SimulatorGateParams) -> None:
    """Apply H5 Hadamard variant gate (Rx(π/2) followed by Z).

    Args:
        state: ProjectQ simulator state.
        qubit: Target qubit index.
    """
    # @property
    # def matrix(self):

    ops.Rx(np.pi / 2) | state.qids[qubit]
    ops.Z | state.qids[qubit]


def H6(state: ProjectQSim, qubit: int, **_params: SimulatorGateParams) -> None:
    """Apply H6 Hadamard variant gate (Rx(π/2) followed by Y).

    Args:
        state: ProjectQ simulator state.
        qubit: Target qubit index.
    """
    # @property
    # def matrix(self):

    ops.Rx(np.pi / 2) | state.qids[qubit]
    ops.Y | state.qids[qubit]


def F(state: ProjectQSim, qubit: int, **_params: SimulatorGateParams) -> None:
    """Face rotations of an octahedron #1."""
    # @property
    # def matrix(self):

    ops.Rx(np.pi / 2) | state.qids[qubit]
    ops.Rz(np.pi / 2) | state.qids[qubit]


def Fdg(state: ProjectQSim, qubit: int, **_params: SimulatorGateParams) -> None:
    """Adjoint of face rotations of an octahedron #1."""
    ops.Rz(-np.pi / 2) | state.qids[qubit]
    ops.Rx(-np.pi / 2) | state.qids[qubit]


def F2(state: ProjectQSim, qubit: int, **_params: SimulatorGateParams) -> None:
    """Face rotations of an octahedron #2."""
    # @property
    # def matrix(self):

    ops.Rz(np.pi / 2) | state.qids[qubit]
    ops.Rx(-np.pi / 2) | state.qids[qubit]


def F2dg(state: ProjectQSim, qubit: int, **_params: SimulatorGateParams) -> None:
    """Adjoint of face rotations of an octahedron #2."""
    ops.Rx(np.pi / 2) | state.qids[qubit]
    ops.Rz(-np.pi / 2) | state.qids[qubit]


def F3(state: ProjectQSim, qubit: int, **_params: SimulatorGateParams) -> None:
    """Face rotations of an octahedron #3."""
    # @property
    # def matrix(self):

    ops.Rx(-np.pi / 2) | state.qids[qubit]
    ops.Rz(np.pi / 2) | state.qids[qubit]


def F3dg(state: ProjectQSim, qubit: int, **_params: SimulatorGateParams) -> None:
    """Adjoint of face rotations of an octahedron #3."""
    ops.Rz(-np.pi / 2) | state.qids[qubit]
    ops.Rx(np.pi / 2) | state.qids[qubit]


def F4(state: ProjectQSim, qubit: int, **_params: SimulatorGateParams) -> None:
    """Face rotations of an octahedron #4."""
    # @property
    # def matrix(self):

    ops.Rz(np.pi / 2) | state.qids[qubit]
    ops.Rx(np.pi / 2) | state.qids[qubit]


def F4dg(state: ProjectQSim, qubit: int, **_params: SimulatorGateParams) -> None:
    """Adjoint of face rotations of an octahedron #4."""
    ops.Rx(-np.pi / 2) | state.qids[qubit]
    ops.Rz(-np.pi / 2) | state.qids[qubit]
