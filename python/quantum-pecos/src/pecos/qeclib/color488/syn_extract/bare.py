from pecos.qeclib import qubit as qb
from pecos.slr import Block, Comment, CReg, QReg, Qubit, Bit, Parallel

from pecos.qeclib.generic.check import Check
from itertools import cycle, chain, repeat
from math import ceil

def poly2qubits(poly, data: QReg):
    qubits = []
    for q in poly[:-1]:
        qubits.append(data[q])
    return qubits

class SynExtractBare(Block):

    def __init__(self, data: QReg, ancillas: QReg, checks: list, syn: CReg):

        a = cycle(range(len(ancillas)))
        s = iter(range(len(syn)))

        super().__init__()

        pauli = "Z"
        for c in checks:
            data_ids = c[:-1]
            syn_id = next(s)
            anc_id = next(a)
            self.extend(Comment(f"Check['{pauli}', {data_ids}] -> {syn}[{syn_id}]"),
                Check(d=poly2qubits(c, data), paulis=pauli, a=ancillas[anc_id], out=syn[syn_id], with_barriers=False)
            )

        pauli = "X"
        for c in checks:
            data_ids = c[:-1]
            syn_id = next(s)
            anc_id = next(a)
            self.extend(Comment(f"Check['{pauli}', {data_ids}] -> {syn}[{syn_id}]"),
                Check(d=poly2qubits(c, data), paulis=pauli, a=ancillas[anc_id], out=syn[syn_id], with_barriers=False)
            )

class SynExtractBareParallel(Block):

    def __init__(self, data: QReg, ancillas: QReg, checks: list, syn: CReg):

        a = cycle(range(len(ancillas)))
        s = iter(range(len(syn)))

        super().__init__()

        annotations = Block()
        num_parallel_blocks = 2 * ceil(len(checks)/len(ancillas))
        par_blocks = [Parallel() for _ in range(num_parallel_blocks)]

        # iterator for parallelizing circuits for one round of ancilla use
        par_iter = chain.from_iterable(
            repeat(obj, len(ancillas)) for obj in par_blocks
        )

        pauli = "Z"
        for c in checks:
            data_ids = c[:-1]
            syn_id = next(s)
            anc_id = next(a)
            annotations.extend(Comment(f"Check['{pauli}', {data_ids}] -> {syn}[{syn_id}]"), )
            par = next(par_iter)
            par.extend(
                Check(d=poly2qubits(c, data), paulis=pauli, a=ancillas[anc_id], out=syn[syn_id], with_barriers=False)
            )

        pauli = "X"
        for c in checks:
            data_ids = c[:-1]
            syn_id = next(s)
            anc_id = next(a)
            annotations.extend(Comment(f"Check['{pauli}', {data_ids}] -> {syn}[{syn_id}]"), )
            par = next(par_iter)
            par.extend(
                Check(d=poly2qubits(c, data), paulis=pauli, a=ancillas[anc_id], out=syn[syn_id], with_barriers=False)
            )

        self.extend(
            annotations,
            Comment(),
        )

        for p in par_blocks:
            self.extend(
                Comment(),
                p,
            )
