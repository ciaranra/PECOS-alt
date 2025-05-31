# Copyright 2019 The PECOS Developers
#
# Licensed under the Apache License, Version 2.0 (the "License"); you may not use this file except in compliance with
# the License.You may obtain a copy of the License at
#
#     https://www.apache.org/licenses/LICENSE-2.0
#
# Unless required by applicable law or agreed to in writing, software distributed under the License is distributed on an
# "AS IS" BASIS, WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied. See the License for the
# specific language governing permissions and limitations under the License.

"""Simulator class type definitions for PECOS quantum error correction.

This module defines specialized simulator class types that extend the default
simulator interface for specific quantum error correction simulation approaches
like Pauli propagation and stabilizer simulations.
"""

from pecos.simulators.default_simulator import DefaultSimulator


class PauliPropagation(DefaultSimulator):
    """Base class for Pauli-propagation simulators."""

    def __init__(self) -> None:
        """Initialize the PauliPropagation simulator.

        Initializes the base DefaultSimulator and sets up bindings for
        Pauli propagation simulation.
        """
        super().__init__()


class Stabilizer(DefaultSimulator):
    """Base class for stabilizer simulators."""

    def __init__(self) -> None:
        """Initialize the Stabilizer simulator.

        Initializes the base DefaultSimulator and sets up bindings for
        stabilizer state simulation.
        """
        super().__init__()


class StateVector(DefaultSimulator):
    """Base class for state-vector simulators."""

    def __init__(self) -> None:
        """Initialize the StateVector simulator.

        Initializes the base DefaultSimulator and sets up bindings for
        state vector simulation.
        """
        super().__init__()


class StateTN(DefaultSimulator):
    """Base class for simulators whose state is represented as a tensor network."""

    def __init__(self) -> None:
        """Initialize the StateTN simulator.

        Initializes the base DefaultSimulator and sets up bindings for
        tensor network state simulation.
        """
        super().__init__()


class DensityMatrix(DefaultSimulator):
    """Base class for density-matrix simulators."""

    def __init__(self) -> None:
        """Initialize the DensityMatrix simulator.

        Initializes the base DefaultSimulator and sets up bindings for
        density matrix simulation.
        """
        super().__init__()


class ProcessMatrix(DefaultSimulator):
    """Base class for process-matrix simulators."""

    def __init__(self) -> None:
        """Initialize the ProcessMatrix simulator.

        Initializes the base DefaultSimulator and sets up bindings for
        process matrix simulation.
        """
        super().__init__()
