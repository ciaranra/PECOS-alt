# Copyright 2025 The PECOS Developers
#
# Licensed under the Apache License, Version 2.0 (the "License"); you may not use this file except in compliance with
# the License.You may obtain a copy of the License at
#
#     https://www.apache.org/licenses/LICENSE-2.0
#
# Unless required by applicable law or agreed to in writing, software distributed under the License is distributed on an
# "AS IS" BASIS, WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied. See the License for the
# specific language governing permissions and limitations under the License.

"""Verification tests showing exact transformations performed by ParallelOptimizer."""

from pecos.qeclib import qubit as qb
from pecos.slr import Block, CReg, Main, Parallel, QReg
from pecos.slr.transforms import ParallelOptimizer


def test_exact_bell_state_transformation():
    """Test the exact transformation described in the documentation."""
    optimizer = ParallelOptimizer()
    
    # Before optimization:
    # Parallel(
    #     Block(H(q[0]), CX(q[0], q[1])),
    #     Block(H(q[2]), CX(q[2], q[3])),
    #     Block(H(q[4]), CX(q[4], q[5]))
    # )
    prog = Main(
        q := QReg("q", 6),
        Parallel(
            Block(
                qb.H(q[0]),
                qb.CX(q[0], q[1]),
            ),
            Block(
                qb.H(q[2]),
                qb.CX(q[2], q[3]),
            ),
            Block(
                qb.H(q[4]),
                qb.CX(q[4], q[5]),
            ),
        ),
    )
    
    optimized = optimizer.transform(prog)
    
    # After optimization:
    # Block(
    #     Parallel(H(q[0]), H(q[2]), H(q[4])),        # All H gates
    #     Parallel(CX(q[0],q[1]), CX(q[2],q[3]), CX(q[4],q[5]))  # All CX gates
    # )
    
    # Verify structure
    assert len(optimized.ops) == 1
    outer_block = optimized.ops[0]
    assert isinstance(outer_block, Block)
    
    # Should have exactly 2 groups
    assert len(outer_block.ops) == 2
    
    # First group: All H gates in parallel
    h_group = outer_block.ops[0]
    assert isinstance(h_group, Parallel)
    assert len(h_group.ops) == 3
    assert all(isinstance(op, qb.H) for op in h_group.ops)
    
    # Check specific qubits for H gates
    h_qubits = [op.qargs[0].index for op in h_group.ops]
    assert sorted(h_qubits) == [0, 2, 4]
    
    # Second group: All CX gates in parallel
    cx_group = outer_block.ops[1]
    assert isinstance(cx_group, Parallel)
    assert len(cx_group.ops) == 3
    assert all(isinstance(op, qb.CX) for op in cx_group.ops)
    
    # Check specific qubits for CX gates
    cx_pairs = [(op.qargs[0].index, op.qargs[1].index) for op in cx_group.ops]
    assert sorted(cx_pairs) == [(0, 1), (2, 3), (4, 5)]


def test_visual_transformation_output():
    """Test that shows the transformation visually."""
    optimizer = ParallelOptimizer()
    
    prog = Main(
        q := QReg("q", 6),
        Parallel(
            Block(qb.H(q[0]), qb.CX(q[0], q[1])),
            Block(qb.H(q[2]), qb.CX(q[2], q[3])),
            Block(qb.H(q[4]), qb.CX(q[4], q[5])),
        ),
    )
    
    def print_structure(block, indent=0):
        """Helper to visualize block structure."""
        prefix = "  " * indent
        if isinstance(block, Main):
            print(f"{prefix}Main(")
            for op in block.ops:
                print_structure(op, indent + 1)
            print(f"{prefix})")
        elif isinstance(block, Parallel):
            print(f"{prefix}Parallel(")
            for op in block.ops:
                print_structure(op, indent + 1)
            print(f"{prefix})")
        elif isinstance(block, Block):
            print(f"{prefix}Block(")
            for op in block.ops:
                print_structure(op, indent + 1)
            print(f"{prefix})")
        elif hasattr(block, 'qargs'):
            # Gate operation
            gate_name = type(block).__name__
            if len(block.qargs) == 1:
                print(f"{prefix}{gate_name}(q[{block.qargs[0].index}])")
            elif len(block.qargs) == 2:
                print(f"{prefix}{gate_name}(q[{block.qargs[0].index}], q[{block.qargs[1].index}])")
            else:
                print(f"{prefix}{gate_name}({block.qargs})")
        else:
            print(f"{prefix}{type(block).__name__}")
    
    print("=== Before optimization ===")
    print_structure(prog)
    
    optimized = optimizer.transform(prog)
    
    print("\n=== After optimization ===")
    print_structure(optimized)
    
    # The output should show the transformation from nested blocks to grouped operations


def test_mixed_gates_transformation():
    """Test transformation with different gate types to show grouping."""
    optimizer = ParallelOptimizer()
    
    prog = Main(
        q := QReg("q", 8),
        Parallel(
            qb.H(q[0]),
            qb.X(q[1]),
            qb.H(q[2]),
            qb.Y(q[3]),
            qb.H(q[4]),
            qb.Z(q[5]),
            qb.X(q[6]),
            qb.Y(q[7]),
        ),
    )
    
    optimized = optimizer.transform(prog)
    
    # Should group by gate type
    assert len(optimized.ops) == 1
    outer_block = optimized.ops[0]
    assert isinstance(outer_block, Block)
    
    # Find groups and verify ordering
    groups = outer_block.ops
    gate_types = []
    gate_counts = []
    
    for group in groups:
        if isinstance(group, Parallel):
            # Multiple gates of same type
            types_in_group = {type(op).__name__ for op in group.ops}
            assert len(types_in_group) == 1
            gate_types.append(list(types_in_group)[0])
            gate_counts.append(len(group.ops))
        else:
            # Single gate (not wrapped in Parallel)
            gate_types.append(type(group).__name__)
            gate_counts.append(1)
    
    # H gates should come first, then X, then Y, then Z (based on the ordering in _group_operations)
    assert gate_types == ['H', 'X', 'Y', 'Z']
    
    # Verify gate counts
    assert gate_counts[0] == 3  # 3 H gates
    assert gate_counts[1] == 2  # 2 X gates  
    assert gate_counts[2] == 2  # 2 Y gates
    assert gate_counts[3] == 1  # 1 Z gate


def test_dependent_operations_not_reordered():
    """Test that dependent operations maintain their order."""
    optimizer = ParallelOptimizer()
    
    prog = Main(
        q := QReg("q", 3),
        Parallel(
            qb.H(q[0]),
            qb.CX(q[0], q[1]),  # Depends on H(q[0])
            qb.CX(q[1], q[2]),  # Depends on CX(q[0], q[1])
        ),
    )
    
    optimized = optimizer.transform(prog)
    
    # Due to dependencies, operations should maintain order
    assert len(optimized.ops) == 1
    outer_block = optimized.ops[0]
    assert isinstance(outer_block, Block)
    
    # Should have 3 separate operations (no parallelization possible due to dependencies)
    assert len(outer_block.ops) == 3
    
    # First should be H
    assert isinstance(outer_block.ops[0], qb.H)
    
    # Then CX operations in order
    assert isinstance(outer_block.ops[1], qb.CX)
    assert outer_block.ops[1].qargs[0].index == 0
    assert outer_block.ops[1].qargs[1].index == 1
    
    assert isinstance(outer_block.ops[2], qb.CX)
    assert outer_block.ops[2].qargs[0].index == 1
    assert outer_block.ops[2].qargs[1].index == 2


if __name__ == "__main__":
    # Run the visual test to see the transformation
    test_visual_transformation_output()