from pecos.slr import Block, QReg


def transversal_tq(tq_gate, q1: QReg, q2: QReg,) -> Block:

    assert len(q1) == len(q2)

    block = Block()

    for i in range(len(q1)):
        block.extend(tq_gate(q1[i], q2[i]))

    return block
