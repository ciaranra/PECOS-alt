from pecos.slr import Vars, QReg, CReg, Bit, Block
from pecos.qeclib.color488.color488 import Color488
from pecos.qeclib.color488.syn_extract.bare_syn_extract import BareSynExtractParallel
from pecos.qeclib.color488.gates_tq import transversal_tq
from pecos.qeclib.color488.gates_sq import hadamards, sqrt_paulis
from pecos.qeclib.color488.meas.destructive_meas import MeasureZ, SynMeasProcessing, RawLogMeasProcessing
from pecos.qeclib.color488.plot_layout import plot_layout
from pecos.qeclib import qubit as qb


class Color488Patch(Vars):

    def __init__(self, name: str, distance: int, num_ancillas: int):
        self.name = name
        self.distance = distance
        self.num_ancillas = num_ancillas
        self.layout = Color488(distance)
        self.num_data = self.layout.num_data_qubits()

        self.d = QReg(f"_{name}_d", self.num_data)
        # self.syn = QReg(f"{name}_syn", self.num_data-1)
        self.a = QReg(f"_{name}_a", self.num_ancillas)
        self.meas = QReg(f"_{name}", self.num_data)

        self.vars = [
            self.d,
            # self.syn,
            self.a,
            self.meas,
        ]

    def plot_layout(self, *, numbered_qubits: bool = False, numbered_poly=False):
        return plot_layout(self.layout, numbered_qubits=numbered_qubits, numbered_poly=numbered_poly)

    def bare_syn_extract(self, syn: CReg) -> Block:
        syn_input_len = len(syn)
        syn_len = self.num_data - 1
        assert syn_len == syn_input_len, f"{syn_len} != {syn_input_len}"

        _, poly = self.layout.get_layout()
        return BareSynExtractParallel(self.d, self.a, poly, syn)

    def bare_prep_z(self, syndromes: list[CReg]) -> Block:
        block = Block()
        block.extend(
            qb.Prep(self.d),
        )
        for s in syndromes:
            block.extend(
                self.bare_syn_extract(s),
            )
        return block

    def meas_z(self, *, meas: CReg = None, syn: CReg = None, log: Bit = None) -> Block:
        """Destructive Z basis measurement."""
        block = Block(
            MeasureZ(self.d, self.meas),
        )

        if meas is not None:
            block.extend(
                meas.set(self.meas)
            )

        if syn is not None:
            _, poly = self.layout.get_layout()
            syn_indices = [check[:-1] for check in poly]
            block.extend(
                SynMeasProcessing(
                    self.meas,
                    syn_indices,
                    syn,
                )
            )

        if log is not None:
            block.extend(
                SynMeasProcessing(
                    self.meas,
                    ...,
                    log,
                )
            )


            # block.extend(
            #     SynMeasProcessing(self.meas,
            #     syn_indices: list[list[int]],
            #     syn: CReg,
            #     )
            # )

    def cx(self, target: "Color488Patch") -> Block:
        """Logical CX."""
        return transversal_tq.CX(self.d, target.d)

    def cy(self, target: "Color488Patch") -> Block:
        """Logical CX."""
        return transversal_tq.CY(self.d, target.d)

    def cz(self, target: "Color488Patch") -> Block:
        """Logical CX."""
        return transversal_tq.CZ(self.d, target.d)

    def szz(self, target: "Color488Patch") -> Block:
        """Logical CX."""
        return transversal_tq.SZZ(self.d, target.d)

    def h(self) -> Block:
        """Logical H."""
        return hadamards.H(self.d)

    def sz(self) -> Block:
        """Logical SZ."""
        return sqrt_paulis.SZ(self.d)


