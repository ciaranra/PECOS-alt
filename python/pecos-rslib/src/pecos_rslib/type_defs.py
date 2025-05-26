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

"""Common type definitions used throughout PECOS."""

from __future__ import annotations

from typing import TypedDict

# JSON-like types for gate parameters and metadata
JSONValue = str | int | float | bool | None | dict[str, "JSONValue"] | list["JSONValue"]
JSONDict = dict[str, JSONValue]

# Simulator gate parameters - these are passed to simulator gate functions
SimulatorGateParams = JSONDict

# Simulator initialization parameters
SimulatorInitParams = (
    JSONDict  # Parameters for simulator initialization (e.g., MPS config)
)

