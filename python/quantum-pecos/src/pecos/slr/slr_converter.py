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

from __future__ import annotations

from pecos.slr.gen_codes.gen_qasm import QASMGenerator
from pecos.slr.gen_codes.language import Language
from pecos.slr.transforms.parallel_optimizer import ParallelOptimizer

try:
    from pecos.slr.gen_codes.gen_qir import QIRGenerator
except ImportError:
    QIRGenerator = None

try:
    from pecos.slr.gen_codes.guppy.ir_generator import (
        IRGuppyGenerator as GuppyGenerator,
    )
except ImportError:
    GuppyGenerator = None


class SlrConverter:

    def __init__(self, block, *, optimize_parallel: bool = True):
        """Initialize the SLR converter.

        Args:
            block: The SLR block to convert
            optimize_parallel: Whether to apply ParallelOptimizer transformation (default: True).
                             Only affects blocks containing Parallel() statements.
        """
        self._block = block

        # Apply transformations if requested
        if optimize_parallel:
            optimizer = ParallelOptimizer()
            self._block = optimizer.transform(self._block)

    def generate(
        self,
        target: Language,
        *,
        skip_headers: bool = False,
        add_versions: bool = False,
    ) -> str:
        if target == Language.QASM:
            generator = QASMGenerator(
                skip_headers=skip_headers,
                add_versions=add_versions,
            )
        elif target in [Language.QIR, Language.QIRBC]:
            self._check_qir_imported()
            generator = QIRGenerator()
        elif target == Language.GUPPY:
            self._check_guppy_imported()
            generator = GuppyGenerator()
        elif target == Language.HUGR:
            # HUGR is handled specially in the hugr() method
            msg = "Use the hugr() method directly to compile to HUGR"
            raise ValueError(msg)
        else:
            msg = f"Code gen target '{target}' is not supported."
            raise NotImplementedError(msg)

        generator.generate_block(self._block)
        if target == Language.QIRBC:

            return generator.get_bc()
        return generator.get_output()

    @staticmethod
    def _check_qir_imported():
        if QIRGenerator is None:
            msg = (
                "Trying to compile QIR without the appropriate optional dependencies install. "
                "Use optional dependency group `qir` or `all`"
            )
            raise Exception(
                msg,
            )

    def qasm(self, *, skip_headers: bool = False, add_versions: bool = False):
        return self.generate(
            Language.QASM,
            skip_headers=skip_headers,
            add_versions=add_versions,
        )

    def qir(self):
        self._check_qir_imported()
        return self.generate(Language.QIR)

    def qir_bc(self):
        self._check_qir_imported()
        return self.generate(Language.QIRBC)

    @staticmethod
    def _check_guppy_imported():
        if GuppyGenerator is None:
            msg = (
                "Trying to compile to Guppy without the GuppyGenerator. "
                "Make sure gen_guppy.py is available."
            )
            raise Exception(msg)

    def guppy(self):
        self._check_guppy_imported()
        return self.generate(Language.GUPPY)

    def hugr(self):
        """Compile the SLR block to HUGR via Guppy.

        Returns:
            The compiled HUGR module

        Raises:
            ImportError: If guppylang is not available
            RuntimeError: If compilation fails
        """
        self._check_guppy_imported()

        # First generate Guppy code
        generator = GuppyGenerator()
        generator.generate_block(self._block)

        # Then compile to HUGR
        try:
            from pecos.slr.gen_codes.guppy.hugr_compiler import HugrCompiler
        except ImportError as e:
            msg = "Failed to import HugrCompiler. Make sure guppylang is installed."
            raise ImportError(msg) from e

        compiler = HugrCompiler(generator)
        return compiler.compile_to_hugr()
