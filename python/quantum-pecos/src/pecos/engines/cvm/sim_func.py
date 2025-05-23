# Copyright 2022 The PECOS Developers
#
# Licensed under the Apache License, Version 2.0 (the "License"); you may not use this file except in compliance with
# the License.You may obtain a copy of the License at
#
#     https://www.apache.org/licenses/LICENSE-2.0
#
# Unless required by applicable law or agreed to in writing, software distributed under the License is distributed on an
# "AS IS" BASIS, WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied. See the License for the
# specific language governing permissions and limitations under the License.

from __future__ import annotations

from typing import Any


def sim_print(_runner, *args) -> None:
    syms = [s for s, _ in args]
    syms = ", ".join(syms)
    print(f"sim_print({syms}):")
    for sym, b in args:
        print(f"    {sym}: {b!s} ({int(b)})")
    print()


def sim_test(_runner, *_args) -> None:
    print("SIM TEST!")


def sim_get_amp(runner, key_state) -> dict[str, Any]:
    st = str(key_state[0][1])
    return runner.state.get_amps(st)


def sim_get_amps(runner, *_args) -> dict[str, Any]:
    return runner.state.get_amps()


def sim_noise(runner, *_args) -> int:
    return int(runner.generate_errors)


def sim_noise_off(runner, *_args) -> int:
    runner.generate_errors = False
    return sim_noise(runner)


def sim_noise_on(runner, *_args) -> int:
    runner.generate_errors = True
    return sim_noise(runner)


sim_funcs = {
    "sim_test": sim_test,
    "sim_print": sim_print,
    "sim_get_amp": sim_get_amp,
    "sim_get_amps": sim_get_amps,
    "sim_noise": sim_noise,
    "sim_noise_off": sim_noise_off,
    "sim_noise_on": sim_noise_on,
}


def sim_exec(func, runner, *args) -> None | int | dict[str, Any]:
    return sim_funcs[func](runner, *args)
