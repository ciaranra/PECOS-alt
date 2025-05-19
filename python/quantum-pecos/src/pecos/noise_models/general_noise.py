from __future__ import annotations

from typing import TYPE_CHECKING

import numpy as np

from pecos.error_models.error_model_abc import ErrorModel
from pecos.error_models.noise_impl_old.gate_groups import one_qubits, two_qubits
from pecos.reps.pypmir.block_types import IfBlock
from pecos.reps.pypmir.op_types import COp, QOp

if TYPE_CHECKING:
    from pecos.machines.generic_machine import GenericMachine
    from pecos.reps.pypmir.block_types import SeqBlock
    from pecos.reps.pypmir.op_types import MOp

# TODO: Encode as much as we can in a dictionary like form:
#   Overall noise rate, followed by relative errors...
#   |0><0| L -> m=1: p
#   XL: p
#   and modify the noise rates in dict and overall rate before hand
#   precalculate and simplify code as much as possible

two_qubit_paulis = {
    "IX",
    "IY",
    "IZ",
    "XI",
    "XX",
    "XY",
    "XZ",
    "YI",
    "YX",
    "YY",
    "YZ",
    "ZI",
    "ZX",
    "ZY",
    "ZZ",
}
SYMMETRIC_P2_PAULI_MODEL = {p: 1 / 15 for p in two_qubit_paulis}

one_qubit_paulis = {
    "X",
    "Y",
    "Z",
}
SYMMETRIC_P1_PAULI_MODEL = {p: 1 / 3 for p in one_qubit_paulis}


class GeneralNoiseModel(ErrorModel):
    """Parameterized error mode."""

    def __init__(self, error_params: dict) -> None:
        super().__init__(error_params=error_params)
        self._eparams = None

        self.qubit_set = set()
        self.leaked_qubits = set()

    def reset(self):
        """Reset error generatootr for another round of syndrome extraction."""
        return GeneralNoiseModel(error_params=self.error_params)

    def init(self, num_qubits: int, machine: GenericMachine):

        self.qubit_set: set[int] = set(range(num_qubits))
        self.leakded_qubits: set[int] = set()

        if not self.error_params:
            msg = "Error params not set!"
            raise Exception(msg)

        self._eparams: dict = dict(self.error_params)
        self._set_eparams_default()
        self._scale()

        if "p1_error_model" not in self._eparams:
            self._eparams["p1_error_model"] = SYMMETRIC_P1_PAULI_MODEL

        if "p2_error_model" not in self._eparams:
            self._eparams["p2_error_model"] = SYMMETRIC_P2_PAULI_MODEL

        if "p2_mem" in self._eparams and "p2_mem_error_model" not in self._eparams:
            self._eparams["p2_mem_error_model"] = SYMMETRIC_P2_PAULI_MODEL

    def _set_eparams_default(self):
        for key in [
            "p1",
            "p2",
            "p_meas",
            "p_init",
            "quadratic_dephasing_rate",
            "linear_dephasing_rate",
            "p_crosstalk_meas",
            "p_crosstalk_init",
        ]:
            if key not in self._eparams:
                self._eparams[key] = 0.0

        for key in [
            "coherent_dephasing",
            "crosstalk_per_gate",
        ]:
            if key not in self._eparams:
                self._eparams[key] = False

        for key in [
            "coherent_to_incoherent_factor",
            "p1_emission_rescale",
            "emission_scale",
            "scale",
            "init_scale",
            "leakage_scale",
            "memory_scale",
            "p1_scale",
            "p2_scale",
            "crosstalk_scale",
            "meas_scale",
            "p_crosstalk_meas_rescale",
            "p_crosstalk_init_rescale",
        ]:
            if key not in self._eparams:
                self._eparams[key] = 1.0

        for key in ["p_init_leak_ratio", "p1_emission_ratio", "p2_emission_ratio"]:
            if key not in self._eparams:
                self._eparams[key] = 0.5

        if (
            "p1_pauli_model" not in self._eparams
            or self._eparams["p1_pauli_model"] == "symmetric"
        ):
            self._eparams["p1_pauli_model"] = SYMMETRIC_P1_PAULI_MODEL

        if (
            "p2_pauli_model" not in self._eparams
            or self._eparams["p2_pauli_model"] == "symmetric"
        ):
            self._eparams["p2_pauli_model"] = SYMMETRIC_P2_PAULI_MODEL

    def _scale(self):

        if not self._eparams["coherent_dephasing"]:
            self._eparams["quadratic_dephasing_rate"] *= self._eparams[
                "coherent_to_incoherent_factor"
            ]
            # to get rid of the 0.5 factor in (rate x duration x 0.5)^2 calcs
            self._eparams[
                "quadratic_dephasing_rate"
            ] *= 0.5  # << added only for the incoherent approximation

        self._eparams["quadratic_dephasing_rate"] *= 2 * np.pi

        if self._eparams.get("linear_dephasing_rate") is None:
            self._eparams["linear_dephasing_rate"] = 0.0

        # ==============================================================================================================
        # Begin scaling
        # ==============================================================================================================
        scale = self._eparams["scale"]

        self._eparams["quadratic_dephasing_rate"] *= np.sqrt(
            self._eparams["memory_scale"] * scale,
        )
        self._eparams["linear_dephasing_rate"] *= self._eparams["memory_scale"] * scale

        cxscale = self._eparams["crosstalk_scale"] * scale

        self._eparams["p_crosstalk_meas"] *= self._eparams["p_crosstalk_meas_rescale"]
        self._eparams["p_crosstalk_init"] *= self._eparams["p_crosstalk_init_rescale"]

        self._eparams["p_crosstalk_meas"] *= self._eparams["meas_scale"] * cxscale
        self._eparams["p_crosstalk_init"] *= self._eparams["init_scale"] * cxscale

        self._eparams["p1"] *= self._eparams["p1_scale"] * scale
        self._eparams["p2"] *= self._eparams["p2_scale"] * scale

        if isinstance(self._eparams["p_meas"], (tuple, list)):
            m1, m2 = self._eparams["p_meas"]

            m1 *= self._eparams["meas_scale"] * scale
            m2 *= self._eparams["meas_scale"] * scale

            self._eparams["p_meas"] = (m1, m2)
            # TODO: REMOVE THIS!
            self._eparams["p_meas"] = np.mean([m1, m2])
        else:
            self._eparams["p_meas"] *= self._eparams["meas_scale"] * scale

        self._eparams["p_init"] *= self._eparams["init_scale"] * scale

        self._eparams["p_init_leak_ratio"] *= self._eparams["leakage_scale"]

        self._eparams["p1_emission_ratio"] *= self._eparams["p1_emission_rescale"]
        self._eparams["p1_emission_ratio"] *= self._eparams["emission_scale"] * scale
        self._eparams["p1_emission_ratio"] = min(
            self._eparams["p1_emission_ratio"],
            1.0,
        )

        self._eparams["p2_emission_ratio"] *= self._eparams["emission_scale"] * scale
        self._eparams["p2_emission_ratio"] = min(
            self._eparams["p2_emission_ratio"],
            1.0,
        )
        # ==============================================================================================================
        # End scaling
        # ==============================================================================================================

        # Rescaling from average error to total error
        self._eparams["p1"] *= 3 / 2
        self._eparams["p2"] *= 5 / 4
        self._eparams["p_crosstalk_meas"] *= 18 / 5
        self._eparams["p_crosstalk_init"] *= 18 / 5

        # Experimentalists are reporting average error rate for dephasing, need to convert to total.
        self._eparams["quadratic_dephasing_rate"] *= np.sqrt(3 / 2)
        self._eparams["linear_dephasing_rate"] *= 3 / 2

        if self._eparams.get("biased_tq_pauli_noise"):
            self._eparams["p2_pauli_model"] = self._eparams["p2_pauli_model_exp"]

    def shot_reinit(self) -> None:
        """Run all code needed at the beginning of each shot, e.g., resetting state."""

    def process(self, qops: list[QOp]) -> list[QOp | SeqBlock]:
        noisy_ops = []

        for op in qops:
            qops_after = None
            qops_before = None
            erroneous_ops = None

            # TODO: Have qops_ideal that doesn't get changed

            match op.name:

                case x if op.metadata.get("noiseless"):
                    pass

                case x if (
                    "noiseless_gates" in self._eparams
                    and x in self._eparams["noiseless_gates"]
                ):
                    pass

                case "Idle" | "Sleep" | "Transport":
                    if (
                        op.name == "Idle"
                        and not self.error_params.get("idle_dephasing", True)
                    ) or (
                        op.name == "Transport"
                        and not self.error_params.get("transport_dephasing", True)
                    ):
                        erroneous_ops = []

                    else:
                        erroneous_ops = self.faults_dephasing(
                            op,
                            op.metadata["duration"],
                            rate=self._eparams["quadratic_dephasing_rate"],
                        )
                        if erroneous_ops is None:
                            erroneous_ops = []

                case "init |0>" | "Init" | "Init +Z":
                    qops_after = self.faults_init(op, flip="X")

                case x if x in one_qubits:
                    erroneous_ops = self.faults_one_qubit_gates(
                        op,
                        p1=self._eparams["p1"],
                        p1_emission_ratio=self._eparams["p1_emission_ratio"],
                        p1_pauli_model=self._eparams["p1_pauli_model"],
                        p1_emission_model=self._eparams["p1_emission_model"],
                    )

                case x if x in two_qubits:
                    erroneous_ops = self.faults_two_qubit_gates(
                        op,
                        p2=self._eparams["p2"],
                        p2_emission_ratio=self._eparams["p2_emission_ratio"],
                        p2_pauli_model=self._eparams["p2_pauli_model"],
                        p2_emission_model=self._eparams["p2_emission_model"],
                    )
                    # TODO: angle dependent noise

                    if self._eparams.get("p2_mem"):
                        qops_mem = self.noise_tq_depolarizing_leakage(
                            op,
                            p=self._eparams["p2_mem"],
                            noise_dict=self._eparams["p2_mem_error_model"],
                        )
                        if qops_after:
                            qops_after = qops_after.extend(qops_mem)
                        else:
                            qops_after = qops_mem

                case "measure Z" | "Measure" | "Measure +Z":
                    erroneous_ops = self.noise_meas_bitflip_leakage(
                        op,
                        p=self._eparams["p_meas"],
                    )
                    # TODO: Deal with biased measurement error rates (fix in scaling too)
                    # TODO: Measurement crosstalk

                case "Leak":
                    erroneous_ops = self.leak(set(op.args))

                case _:
                    msg = f"This error model doesn't handle gate: {op.name}!"
                    raise Exception(msg)

            if qops_before:
                noisy_ops.extend(qops_before)

            if erroneous_ops is None:
                noisy_ops.append(op)
            else:
                noisy_ops.extend(erroneous_ops)

            if qops_after:
                noisy_ops.extend(qops_after)

        return noisy_ops

    def faults_dephasing(
        self,
        op: QOp,
        duration: float,
        rate: float | None = None,
    ) -> list[QOp] | None:
        """Applies both coherent dephasing and linear incoherent dephasing."""

        if rate:
            if self._eparams["coherent_dephasing"]:
                return self.faults_dephase_coherent(op, duration, rate)
            else:  # quadratic incoherent dephasing
                return self.faults_dephase_incoherent(op, duration, rate, linear=False)

        linear_rate = self._eparams.get("linear_dephasing_rate")

        if linear_rate:
            return self.faults_dephase_incoherent(
                op,
                duration,
                rate=linear_rate,
                linear=True,
            )
        return None

    def faults_dephase_coherent(
        self,
        op: MOp,
        duration: float,
        rate: float | None = None,
    ) -> list[QOp]:
        """The dephasing noise model for idling qubits.

        Args:
            op: A machine operation, e.g., "Idle" or "Sleep".
            duration: The time spent dephasing.
            rate: custom linear dephasing rate
        """

        notleaked = set(op.args) - self.leaked_qubits

        return [
            QOp(
                name="RZ",
                args=list(notleaked),
                angles=(rate * duration,),
            ),
        ]

    def faults_dephase_incoherent(
        self,
        op: MOp,
        duration: float,
        rate: float | None = None,
        linear=False,
    ) -> list[QOp] | None:
        """The dephasing noise model for idling qubits.

        Args:
            op: A machine operation, e.g., "Idle" or "Sleep".
            duration: The time spent dephasing.
            rate: custom linear dephasing rate
            linear: Whether the scaling should be linear.
        """

        pdeph = rate * duration

        if pdeph:

            if not linear:
                pdeph = np.power(np.sin(pdeph), 2)

            notleaked = set(op.args) - self.leaked_qubits

            # ---------------------------
            # dephasing noise
            # ---------------------------
            rand_nums = np.random.random(len(notleaked)) <= pdeph

            err_qubits = []
            for r, loc in zip(rand_nums, notleaked, strict=False):

                if r:
                    err_qubits.append(loc)

            if err_qubits:
                return [QOp(name="Z", args=err_qubits)]
            return None
        return None

    def faults_init(self, op: QOp, flip: str) -> list[QOp]:
        """The noise model for qubit (re)initialization.

        Args:
            op: A quantum operator
            flip: The symbol for what Pauli operator should be applied if an initialization fault occurs.
        """

        locations: set[int] = set(op.args)

        self.simple_unleak(locations)

        rand_nums = np.random.random(len(locations)) <= self._eparams["p_init"]

        after = []
        toleak = set()
        for r, loc in zip(rand_nums, locations, strict=False):
            if r:

                if np.random.random() <= self._eparams["p_init_leak_ratio"]:
                    toleak.add(loc)
                else:
                    after.append(QOp(name=flip, args=[loc]))

        # Leakage noise
        # -------------
        self.leak(toleak, p_leak=self._eparams["leakage_scale"])

        # crosstalk
        # ---------
        if not op.metadata.get("start_init", False) and op.metadata.get("z2qs"):
            noise = self.init_crosstalk(op)
            after.extend(noise)

        return after

    def init_crosstalk(self, op) -> list[QOp]:

        var = ("__pecos_scratch", 0)
        p_cross = self._eparams["p_crosstalk_init"]

        qs = set()
        q_zone = {}
        for gz in self._eparams.get("crosstalk_zones", []):
            qsz = op.metadata.get("z2qs", {}).get(gz, [])
            for q in qsz:
                q_zone[q] = gz

            qs |= set(qsz)

        qs -= set(op.args)
        ls = (qs & self.leaked_qubits) & self.qubit_set
        qs -= self.leaked_qubits
        qs &= self.qubit_set

        rand_nums = np.random.random(len(qs))

        num_cross = len(op.args) if self._eparams["crosstalk_per_gate"] else 1

        after = []
        for _ in range(num_cross):
            for r, q in zip(rand_nums, qs, strict=False):

                if self._eparams.get("zones"):
                    gz = q_zone[q]
                    p = self._eparams["zones"][gz]["p_crosstalk_init"]
                else:
                    p = p_cross

                if r <= p:

                    after.append(
                        QOp(
                            name="measure Z",
                            args=[q],
                            returns=[var],
                        ),
                    )

                    if np.random.random() <= 1 / 3:
                        after.append(
                            IfBlock(
                                condition=COp(name="==", args=[var, 1]),
                                true_branch=[
                                    QOp(
                                        name="init |0>",
                                        args=[q],
                                    ),
                                ],
                            ),
                        )

                    elif np.random.random() <= 2 / 3 * self._eparams["leakage_scale"]:
                        if self._eparams.get("leak2depolar"):
                            if np.random.random() <= 0.75:
                                err = np.random.choice(one_qubit_paulis)
                                after.append(QOp(name=err, args=[q]))
                        else:
                            after.append(
                                IfBlock(
                                    condition=COp(name="==", args=[var, 1]),
                                    true_branch=[
                                        QOp(
                                            name="Leak",
                                            args=[q],
                                        ),
                                    ],
                                ),
                            )

            if ls and self._eparams.get("seepage", True):
                rand_nums = np.random.random(len(ls))
                for r, q in zip(rand_nums, ls, strict=False):

                    if r <= p_cross:

                        if np.random.random() <= 0.5:
                            # 1/3 still leaked
                            if np.random.random() <= 2 / 3:
                                noise = self.unleak(
                                    {q},
                                    pop0_prob=0.5,
                                )  # go to |0> or |1>
                                after.extend(noise)
                        else:
                            # 2/3 still leaked
                            if np.random.random() <= 1 / 3:
                                noise = self.unleak(
                                    {q},
                                    pop0_prob=0.0,
                                )  # go to |1>
                                after.extend(noise)

        return after

    def old_faults_meas(
        self,
        locations: set[int],
        metadata: dict,
        before,
        after,
        flip: str,
    ) -> None:
        """The noise model for measurements.

        TODO: Discuss error model

        Args:
            locations: Set of qubits the ideal gates act on.
            metadata: Extra information about the gate.
            before: QuantumCircuit collecting the noise that occurs before the ideal gates.
            after: QuantumCircuit collecting the noise that occurs after the ideal gates.
            flip: The symbol for what Pauli operator should be applied if an measurement fault occurs.
        """

        leaked = locations & self.leaked_qubits

        rand_nums1 = (
            np.random.random(len(locations)) <= self.error_params["meas1_leak_ratio"]
        )
        rand_nums2 = np.random.random(len(locations))

        for r1, r2, loc in zip(rand_nums1, rand_nums2, locations, strict=False):

            if metadata.get("var_output"):
                var = metadata["var_output"][loc]
            else:
                var = metadata["var"]

            if loc in leaked:
                self.unleak({loc}, before, pop0_prob=0.0, trigger="meas")

                # If leaked, force measurement to 1
                expr = {"t": var, "a": 1, "op": "="}
                after.append("cop", {loc}, cond=metadata.get("cond"), expr=expr)

            # Leakage noise (after the measurement)
            # -------------------------------------
            # if measured in 1, then 1/3 in 1 and 2/3 leaked
            # if measured in 0, then remains in 0
            # => select 2/3 qubits, then leak with condition measurement is 1

            if r1:
                after.append(
                    "leak",
                    {loc},
                    cond=metadata.get("cond"),
                    cond2={"a": var, "op": "==", "b": 1},
                    trigger="meas",
                )

            # Bit-flip noise
            # -----------------------------------------------
            p = self.error_params["p_meas"]

            if isinstance(p, (tuple, list)):
                p0, p1 = p
            else:
                p0, p1 = p, p

            if r2 <= p0 or r2 <= p1:

                expr = {"t": var, "a": var, "op": "^", "b": 1}  # flip output bit

                if r2 <= p0 and r2 <= p1:  # whether 0 or 1, we should flip
                    cond2 = None

                elif (
                    r2 <= p0
                ):  # either r2 > p0 or r2 > p1 but r2 smaller than one of them, check if p1 < r2 <= p0
                    cond2 = {"a": var, "op": "==", "b": 0}

                else:  # p0 < r2 <= p1:
                    cond2 = {"a": var, "op": "==", "b": 1}

                after.append(
                    "cop",
                    {loc},
                    cond=metadata.get("cond"),
                    cond2=cond2,
                    expr=expr,
                )

            # if r <= p:
            #     before.append(flip, {loc})

        if metadata.get("mid_circuit") and metadata.get("z2qs"):
            self.meas_crosstalk(locations, metadata, after)

    def old_meas_crosstalk(self, locations: set[int], metadata: dict, after):
        """
        The ion will get projected into either 0 or 1. If it is projected into 0, that's it, no leakage. If it gets
        projected into 1, it will then come back to 1 with probability 1/3, and go to the leaked state with probability
        2/3.

        Probability of going down 0 or 1 branch == probability of meas 0 or 1.

        |0><0|, |1><1| -> 2/3 leak
        ~= meas Z, if 1 -> 2/3 leak

        Args:
            locations:
            metadata:
            after:

        Returns:

        """

        var = ("__pecos_scratch", 0)
        p_cross = self.error_params["p_crosstalk_meas"]

        # Apply crosstalk equally to all qubits not being measured
        qs = set()
        q_zone = {}
        for gz in self.error_params["crosstalk_zones"]:
            qsz = metadata.get("z2qs", {}).get(gz, [])
            for q in qsz:
                q_zone[q] = gz

            qs |= set(qsz)

        qs -= locations
        ls = (qs & self.leaked_qubits) & self.qubit_set
        qs -= self.leaked_qubits
        qs &= self.qubit_set

        rand_nums = np.random.random(len(qs))

        num_cross = len(locations) if self.error_params["crosstalk_per_gate"] else 1

        for _ in range(num_cross):
            for r, q in zip(rand_nums, qs, strict=False):

                if self.error_params.get("zones"):
                    gz = q_zone[q]
                    p = self.error_params["zones"][gz]["p_crosstalk_meas"]
                else:
                    p = p_cross

                if r <= p:

                    # 1 - measure q -> 0 or 1
                    # if 0: nothing more
                    # if 1: -> 2/3 prob. to leak
                    after.append("measure Z", {q}, cond=metadata.get("cond"), var=var)

                    if np.random.random() <= 2 / 3 * self.error_params["leakage_scale"]:

                        if self.error_params.get("leak2depolar"):
                            if np.random.random() <= 0.75:
                                err = np.random.choice(one_qubit_paulis)
                                after.append(err, {q})
                        else:
                            # if meas -> 1: 2/3 leak
                            after.append(
                                "leak",
                                {q},
                                cond=metadata.get("cond"),
                                cond2={"a": var, "op": "==", "b": 1},
                                trigger="meas_crosstalk",
                            )

            if ls and self.error_params.get("seepage", True):
                # if a leaked qubit is touched by measurement crosstalk, it will remain leaked w/ 2/3 probability
                # but go back to |1> with 1/3 probability

                rand_nums = np.random.random(len(ls))

                # Leaked qubits: 1/3 of the time goes to |1>
                for r, q in zip(rand_nums, ls, strict=False):
                    if r <= p_cross:

                        if np.random.random() <= 1 / 3:
                            self.unleak(
                                {q},
                                after,
                                pop0_prob=0.0,
                                trigger="meas_crosstalk",
                            )  # goes to |1>

    def leak(self, locations: set[int], p_leak: float) -> list[QOp]:
        """The method that leaks qubits.

        Args:
            locations: Set of qubits the ideal gates act on.
            p_leak: Probability to leak.
        """

        error_circ = []
        if locations:

            if self._eparams.get(
                "leak2depolar",
            ):  # Whether to replace leakage with depolarizing noise
                for q in locations:
                    if np.random.random() <= 0.75 * p_leak:
                        err = np.random.choice(one_qubit_paulis)
                        error_circ.append(QOp(name=err, args=[q]))
            else:

                for loc in locations:
                    if np.random.random() <= p_leak:
                        self.leaked_qubits.add(loc)
                        error_circ.append(QOp(name="Init -Z", args=[loc]))

        return error_circ

    def simple_leak(self, qubits: set[int]) -> None:
        """Track qubits as leaked qubits and calls the quantum simulation appropriately to trigger leakage."""
        self.leaked_qubits += qubits

    def leak_to_zero(self, qubits: set[int]) -> list[QOp]:
        self.simple_leak(qubits)
        return [
            QOp(name="Init +Z", args=list(qubits)),
        ]

    def leak_to_one(self, qubits: set[int]) -> list[QOp]:
        self.simple_leak(qubits)
        return [
            QOp(name="Init -Z", args=list(qubits)),
        ]

    def simple_unleak(self, qubits: set[int]) -> None:
        """Untrack qubits as leaked qubits and calls the quantum simulation appropriately to trigger leakage."""
        self.leaked_qubits -= qubits

    def unleak_to_zero(self, qubits: set[int]) -> list[QOp]:
        self.simple_unleak(qubits)
        return [
            QOp(name="Init +Z", args=list(qubits)),
        ]

    def unleak_to_one(self, qubits: set[int]) -> list[QOp]:
        self.simple_unleak(qubits)
        return [
            QOp(name="Init -Z", args=list(qubits)),
        ]

    def unleak(
        self,
        locations: set[int],
        pop0_prob: float = 0.5,
    ) -> list[QOp]:
        """The method that returns leaked qubits to the computation space.

        Args:
            locations: Set of qubits the ideal gates act on.
            pop0_prob: The probability that a qubit returning to the computational space is re-prepared in |0> instead
                of |1>.
        """

        error_circ = []
        if locations:

            if pop0_prob == 0.0:
                error_circ.extend(self.unleak_to_one(locations))

            else:

                rand_nums = np.random.random(len(locations)) <= pop0_prob

                for r, loc in zip(rand_nums, locations, strict=False):

                    if r:
                        error_circ.extend(self.unleak_to_zero({loc}))

                    else:
                        error_circ.extend(self.unleak_to_one({loc}))

        return error_circ

    def noise_sq_depolarizing_leakage(self, op: QOp, p: float, noise_dict: dict):
        args: set[int] = set(op.args)
        leaked = self.leaked_qubits & args

        # Only apply gate if qubits are not leaked
        if leaked:
            not_leaked = args - leaked
            noisy_op = QOp(
                name=op.name,
                args=list(not_leaked),
                metadata=dict(op.metadata),
            )
        else:
            noisy_op = op

        rand_nums = np.random.random(len(noisy_op.args)) <= p

        # Determine what noise (if any) to apply
        noise = {}
        if np.any(rand_nums):
            for r, loc in zip(rand_nums, noisy_op.args):
                if r:
                    rand = np.random.random()
                    p_tot = 0.0
                    for fault1, prob in noise_dict.items():
                        p_tot += prob
                        if p_tot >= rand:
                            noise.setdefault(fault1, []).append(loc)
                            break

        # Modify operations if noise or leaked qubits are present
        if noise or leaked:
            buffered_ops = []

            if noise:
                # For noise, apply appropriate gate
                for sym, args in noise.items():
                    if sym == "L":
                        leak_ops = self.leak(set(noise["L"]))
                        buffered_ops.extend(leak_ops)
                    else:
                        buffered_ops.extend(
                            (noisy_op, QOp(name=sym, args=args, metadata={})),
                        )

            else:
                buffered_ops.append(noisy_op)

            return buffered_ops
        return None

    def noise_tq_depolarizing_leakage(self, op: QOp, p: float, noise_dict: dict):
        """Two-qubit gate depolarizing noise plus leakage."""
        args = set()
        for a in op.args:
            for q in a:
                args.add(q)

        leaked = self.leaked_qubits & args

        # Don't apply a gate if an input qubit has already leaked
        if leaked:
            not_leaked = args - leaked

            new_args = []
            for a, b in op.args:
                if a not in not_leaked and b not in leaked:
                    new_args.append([a, b])
            op = QOp(name=op.name, args=new_args, metadata=dict(op.metadata))

        rand_nums = np.random.random(len(op.args)) <= p

        if np.any(rand_nums):
            noise = {}
            for r, loc in zip(rand_nums, op.args):
                if r:
                    rand = np.random.random()
                    p_tot = 0.0
                    for (fault1, fault2), prob in noise_dict.items():
                        p_tot += prob

                        if p_tot >= rand:
                            loc1, loc2 = loc
                            if fault1 != "I":
                                noise.setdefault(fault1, []).append(loc1)
                            if fault2 != "I":
                                noise.setdefault(fault2, []).append(loc2)
                            break

            if noise:
                buffered_ops = []
                for sym, args in noise.items():
                    if sym != "L":
                        buffered_ops.append(QOp(name=sym, args=args, metadata={}))
                    else:
                        noisy_ops = self.leak_to_one(set(args))
                        buffered_ops.extend(noisy_ops)
                return buffered_ops

        return None

    def noise_meas_bitflip_leakage(self, op: QOp, p: float):
        """Bit-flip noise model for measurements.

        Args:
        ----
            op: Ideal quantum operation.
            p: measurement error rate.
        """
        # Bit flip noise
        # --------------
        rand_nums = np.random.random(len(op.args)) <= p

        noise = []

        leakded = self.leaked_qubits & set(op.args)
        if leakded:
            noisy_ops = self.unleak_to_one(leakded)
            noise.extend(noisy_ops)

        if np.any(rand_nums):
            bitflips = []
            for r, loc in zip(rand_nums, op.args):
                if r:
                    bitflips.append(loc)

            noisy_op = QOp(
                name="Measure",
                args=list(op.args),
                returns=list(op.returns),
                metadata=dict(op.metadata),
            )
            noisy_op.metadata["bitflips"] = bitflips
            noise.append(noisy_op)

            return noise

        else:
            if noise:
                return noise
            else:
                return None

    def old_leak(self, locations: set[int], error_circ, p_leak: float, **meta) -> None:
        """The method that leaks qubits.

        Args:
            locations: Set of qubits the ideal gates act on.
            p_leak: Probability to leak.
            error_circ: QuantumCircuit collecting the noise applied to the ideal circuit.
        """

        if locations:

            if self.error_params.get(
                "leak2depolar",
            ):  # Whether to replace leakage with depolarizing noise
                for q in locations:
                    if np.random.random() <= 0.75 * p_leak:
                        err = np.random.choice(one_qubit_paulis)
                        error_circ.append(err, {q}, **meta)
            else:

                for loc in locations:
                    if np.random.random() <= p_leak:
                        self.leaked_qubits.add(loc)
                        error_circ.append("init |1>", {loc}, leak=True, **meta)

    def probabilistic_unleak(
        self,
        locations: set[int],
        pop0_prob: float = 0.5,
    ) -> list[QOp]:
        """The method that returns leaked qubits to the computation space.

        Args:
            locations: Set of qubits the ideal gates act on.
            pop0_prob: The probability that a qubit returning to the computational space is re-prepared in |0> instead
                of |1>.
        """

        error_circ = []
        if locations:

            self.leaked_qubits -= locations

            if pop0_prob == 0.0:
                error_circ.extend(self.unleak_to_one(locations))

            else:

                rand_nums = np.random.random(len(locations)) <= pop0_prob

                for r, loc in zip(rand_nums, locations, strict=False):

                    if r:
                        error_circ.extend(self.unleak_to_zero({loc}))
                    else:
                        error_circ.extend(self.unleak_to_one({loc}))
        return error_circ

    def apply_model_noise(self, fault, loc) -> [QOp]:
        noise = []
        if fault == "I":
            pass
        elif fault in ["X", "Y", "Z"]:
            noise.append(QOp(name=fault, args=[loc]))
        elif fault == "L":
            noise.extend(self.leak({loc}, p_leak=self.error_params["leakage_scale"]))
        else:
            msg = f"Was not expecting noise model to have sym = {fault}"
            raise Exception(msg)
        return noise

    def faults_one_qubit_gates(
        self,
        op: QOp,
        p1: float,
        p1_emission_ratio: float,
        p1_pauli_model: dict,
        p1_emission_model: dict,
    ) -> list[QOp]:
        """Noise for single-qubit gates.

        1) Leak qubits with probability `self.pleak_1q`.
        2) Apply depolarizing noise.

        Args:
            op: The quantum operations potentially experiencing noise
        """

        locations: set[int] = set(op.args)

        previously_leaked = locations & self.leaked_qubits
        emission_qubits = set()

        apply_p1 = np.random.random(len(locations)) <= p1

        noise: list[QOp] = []
        for r, loc in zip(apply_p1, locations, strict=False):

            if r:

                if loc in previously_leaked:

                    if (
                        np.random.random() <= p1_emission_ratio
                        and self.error_params.get("seepage", True)
                    ):

                        if np.random.random() <= 1 / 3:
                            noise.extend(
                                self.probabilistic_unleak({loc}, pop0_prob=0.5),
                            )  # reset to |0> or |1>

                elif np.random.random() <= p1_emission_ratio:
                    emission_qubits = {loc}
                    rand = np.random.random()
                    p = 0.0
                    for fault, prob in p1_emission_model.items():

                        p += prob

                        if p >= rand:
                            noise_temp = self.apply_model_noise(fault, loc)
                            noise.extend(noise_temp)
                            break

                else:  # Depolarizing noise
                    rand = np.random.random()
                    p = 0.0
                    for fault, prob in p1_pauli_model.items():

                        p += prob

                        if p >= rand:
                            noise.extend(self.apply_model_noise(fault, loc))
                            break

        remove_locations = (
            previously_leaked | emission_qubits
        )  # remove ideal gates on leaked qubits
        new_ops = [
            QOp(
                name=op.name,
                args=list(locations - remove_locations),
                angles=tuple(op.angles) if op.angles else None,
                metadata=dict(op.metadata),
            ),
        ]
        new_ops.extend(noise)
        return new_ops

    def faults_two_qubit_gates(
        self,
        op: QOp,
        p2: float,
        p2_emission_ratio: float,
        p2_pauli_model: dict | None = None,
        p2_emission_model: dict | None = None,
    ) -> list[QOp]:
        """Noise for two-qubit gates."""

        # locations = set(op.args)
        locations = op.args
        final_locations = {tuple(qpair) for qpair in op.args}
        rand_nums = np.random.random(len(locations))
        apply_p2 = rand_nums <= p2

        after: list[QOp] = []
        for r, loc in zip(apply_p2, locations, strict=False):

            spnt_emiss_happened = False
            previous_leaked = set(loc) & self.leaked_qubits
            loc1, loc2 = loc

            if r:

                if previous_leaked:  # Seepage via spontaneous emission + residual gates

                    if self.error_params.get("seepage", True):
                        if np.random.random() <= p2_emission_ratio:
                            for q in previous_leaked:
                                if np.random.random() <= 1 / 3:
                                    after.extend(
                                        self.probabilistic_unleak({q}, pop0_prob=0.5),
                                    )

                elif np.random.random() <= p2_emission_ratio:  # spontaneous emission
                    spnt_emiss_happened = True
                    rand = np.random.random()
                    p = 0.0
                    for (fault1, fault2), prob in p2_emission_model.items():

                        p += prob

                        if rand <= p:

                            after.extend(self.apply_model_noise(fault1, loc1))
                            after.extend(self.apply_model_noise(fault2, loc2))
                            break

                else:  # Pauli noise

                    rand = np.random.random()
                    p = 0.0

                    for (fault1, fault2), prob in p2_pauli_model.items():

                        p += prob

                        if rand <= p:
                            after.extend(self.apply_model_noise(fault1, loc1))
                            after.extend(self.apply_model_noise(fault2, loc2))
                            break

            if spnt_emiss_happened or previous_leaked:
                final_locations.remove((loc1, loc2))

        noisy_ops = [
            QOp(
                name=op.name,
                args=list(final_locations),
                angles=op.angles,
                metadata=dict(op.metadata),
            ),
        ]
        noisy_ops.extend(after)
        return noisy_ops
