from pecos.slr import QReg, CReg, Bit, Block
from pecos.qeclib import qubit as qb

class MeasureZ(Block):
    """Measure in the logical Z basis."""

    def __init__(
        self,
        data: QReg,
        meas: CReg,
            # syn_idxes
            # log_idxes
            # syn
            # log
            # meas = None # optional
    ) -> None:
        """Initialize MeasureZ block for logical Z basis measurement.

        Args:
            data: Register containing the data qubits to be measured.
            meas: Classical register to store the measurement results.
        """
        super().__init__()

        assert len(data) == len(meas)

        self.extend(
            qb.Measure(data) > meas,
        )

        # TODO: Extract the syndromes and logical outcome

class SynMeasProcessing(Block):
    """Basic measuring process."""
    def __init__(
        self,
        meas: CReg,
        syn_indices: list[list[int]],
        syn: CReg,
    ):
        super().__init__()

        assert len(syn_indices) == len(syn)

        for i, s in enumerate(syn_indices):
            for j in s:
                self.extend(
                    syn.set(syn[i] ^ meas[j])
                )


class RawLogMeasProcessing(Block):
    """Basic measuring process for raw logical outcome."""

    def __init__(
            self,
            meas: CReg,
            log_indices: list[int],
            log: Bit,
    ):
        super().__init__()

        for i in log_indices:
            self.extend(log.set(log[i] ^ meas[i]))