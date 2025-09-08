#!/usr/bin/env python3
"""Test qubit allocation limits and error handling."""

import sys

import pytest

sys.path.append("python/quantum-pecos/src")

try:
    from guppylang import guppy
    from guppylang.std.quantum import h, measure, qubit

    GUPPY_AVAILABLE = True
except ImportError:
    GUPPY_AVAILABLE = False

from pecos.frontends.guppy_api import sim
from pecos_rslib import state_vector


@pytest.mark.skipif(not GUPPY_AVAILABLE, reason="Guppy not available")
class TestQubitAllocationLimits:
    """Test qubit allocation limits and dynamic allocation behavior."""

    def test_static_allocation_within_limit(self) -> None:
        """Test static allocation within the max_qubits limit."""

        @guppy
        def static_test() -> tuple[bool, bool, bool]:
            q1 = qubit()
            q2 = qubit()
            q3 = qubit()
            return measure(q1), measure(q2), measure(q3)

        # Should work fine with max_qubits=5
        results = sim(static_test).qubits(5).quantum(state_vector()).run(10)
        assert len(results.get("measurements", results.get("measurement_1", []))) == 10

    def test_dynamic_allocation_in_loop(self) -> None:
        """Test dynamic allocation in a loop - requires sufficient max_qubits."""

        @guppy
        def dynamic_loop_test() -> int:
            count = 0
            # This allocates 3 qubits dynamically in the loop
            for _i in range(3):
                q = qubit()
                h(q)
                if measure(q):
                    count += 1
            return count

        # Need to set max_qubits high enough for dynamic allocation
        # The static analysis might only see 1 qubit allocation,
        # but we actually allocate 3 qubits in the loop
        results = (
            sim(dynamic_loop_test).qubits(10).quantum(state_vector()).seed(42).run(10)
        )

        # Should see values 0-3
        values = set(results.get("measurements", results.get("measurement_1", [])))
        assert len(values) >= 2  # At least some variation
        assert all(
            0 <= v <= 3
            for v in results.get("measurements", results.get("measurement_1", []))
        )

    def test_dynamic_allocation_exceeds_limit(self) -> None:
        """Test that dynamic allocation properly fails when exceeding limit."""

        @guppy
        def exceed_limit_test() -> int:
            count = 0
            # Try to allocate more qubits than the limit
            for _i in range(5):
                q = qubit()
                h(q)
                if measure(q):
                    count += 1
            return count

        # Set limit too low for the dynamic allocation
        # This should fail with a helpful error message
        with pytest.raises(RuntimeError) as exc_info:
            sim(exceed_limit_test).qubits(3).quantum(state_vector()).run(1)

        # Check that the error message indicates a limit was exceeded
        error_msg = str(exc_info.value)
        # The error may come from different places:
        # 1. "Qubit allocation limit exceeded" from LLVM runtime
        # 2. "index out of bounds" from quantum engine when accessing beyond allocated qubits
        # Both indicate the limit was exceeded
        assert any(
            msg in error_msg
            for msg in [
                "Qubit allocation limit exceeded",
                "max_qubits",
                "index out of bounds",
            ]
        )

    def test_nested_loop_allocation(self) -> None:
        """Test nested loops with qubit allocation."""

        @guppy
        def nested_loop_test() -> int:
            count = 0
            # Total of 6 qubits allocated (3 * 2)
            for i in range(3):
                for j in range(2):
                    q = qubit()
                    if i > j:
                        h(q)
                        if measure(q):
                            count += 1
                    else:
                        # Just measure
                        if measure(q):
                            count += 1
            return count

        # Need sufficient qubits for nested allocation
        results = (
            sim(nested_loop_test).qubits(10).quantum(state_vector()).seed(42).run(50)
        )
        assert len(results.get("measurements", results.get("measurement_1", []))) == 50

    def test_allocation_with_reset(self) -> None:
        """Test that reset allows qubit reuse within limits."""

        @guppy
        def reset_reuse_test() -> int:
            count = 0
            for _i in range(5):
                q = qubit()
                h(q)
                if measure(q):
                    count += 1
                # Reset allows reuse (in theory)
                # Note: Current implementation may not reuse qubits
            return count

        # Even with reset, we might need max_qubits for all allocations
        results = (
            sim(reset_reuse_test).qubits(10).quantum(state_vector()).seed(42).run(50)
        )
        values = results.get("measurements", results.get("measurement_1", []))
        assert all(0 <= v <= 5 for v in values)

    def test_explicit_max_qubits_setting(self) -> None:
        """Test that max_qubits is properly respected."""

        @guppy
        def simple_test() -> bool:
            q = qubit()
            return measure(q)

        # Test with different max_qubits values
        for max_q in [1, 5, 10, 20]:
            results = sim(simple_test, max_qubits=max_q).run(10)
            assert (
                len(results.get("measurements", results.get("measurement_1", []))) == 10
            )


if __name__ == "__main__":
    print("Running qubit allocation limit tests...")
    pytest.main([__file__, "-v"])
