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

"""A collection of useful functions."""

from typing import Any


def make_hashable_params(params: dict[str, Any]) -> tuple:
    """Checks to make sure that the parameters submitted is hashable.

    Args:
    ----
        params(dict): A dictionary of parameters to be made hashable. Values can be dicts, lists, sets, or other types.

    """
    tuple_params = []

    for key, value in params.items():
        if isinstance(value, dict):
            dict_tuple = tuple(value.items())
            tuple_params.append(dict_tuple)
        elif isinstance(value, list | set):
            tuple_params.append((key, tuple(value)))
        else:
            tuple_params.append((key, value))

    tuple_params = tuple(tuple_params)

    try:
        hash(tuple_params)
    except TypeError:
        msg = "The values of keywords given to this class must be hashable."
        raise TypeError(msg) from TypeError

    return tuple_params


def pos2qudit(layout: dict[Any, tuple]) -> dict[tuple, Any]:
    """Reverses the layout dictionary. Makes a new dictionary with (x, y, ...) => qudit_id.

    Args:
    ----
        layout: A dictionary mapping qudit IDs to their positions (coordinates).

    """
    return {p: qid for qid, p in layout.items()}


def expected_params(params: dict[str, Any], expected_set: set[str]) -> None:
    """Check that all parameters are in the expected set.

    Args:
    ----
        params(dict): Dictionary of parameters to validate
        expected_set(set): Set of expected parameter keys

    """
    keys = set(params.keys())

    unexpected = keys - expected_set
    if unexpected:
        msg = f"Received unexpected keys ({unexpected}). Expected keys include: {expected_set}"
        raise Exception(msg)
