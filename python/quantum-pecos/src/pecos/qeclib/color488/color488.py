# Copyright 2024 The PECOS Developers
#
# Licensed under the Apache License, Version 2.0 (the "License"); you may not use this file except in compliance with
# the License.You may obtain a copy of the License at
#
#     https://www.apache.org/licenses/LICENSE-2.0
#
# Unless required by applicable law or agreed to in writing, software distributed under the License is distributed on an
# "AS IS" BASIS, WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied. See the License for the
# specific language governing permissions and limitations under the License.

from functools import lru_cache

from pecos.qeclib.color488.abstract_layout import gen_layout, get_boundaries


class Color488:

    def __init__(self, distance: int) -> None:
        self.distance = distance

    @lru_cache(maxsize=None)
    def get_layout(self):
        return gen_layout(self.distance)

    def get_boundaries(self):
        nodeid2pos, _ = self.get_layout()
        return get_boundaries(nodeid2pos)

    def num_data_qubits(self):
        nodes, _ = self.get_layout()
        return len(nodes)
