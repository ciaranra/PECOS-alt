# Copyright 2023 The PECOS Developers
#
# Licensed under the Apache License, Version 2.0 (the "License"); you may not use this file except in compliance with
# the License.You may obtain a copy of the License at
#
#     https://www.apache.org/licenses/LICENSE-2.0
#
# Unless required by applicable law or agreed to in writing, software distributed under the License is distributed on an
# "AS IS" BASIS, WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied. See the License for the
# specific language governing permissions and limitations under the License.

"""Common type aliases and imports for PECOS.

This module provides centralized imports and type aliases to ensure consistent
naming conventions throughout the PECOS codebase while maintaining compatibility
with external packages.
"""

# Import external PHIR model with consistent naming
from phir.model import PHIRModel as PhirModel

__all__ = ["PhirModel"]
