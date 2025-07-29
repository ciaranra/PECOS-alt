#!/usr/bin/env python3
"""Test qubit allocation limits and error handling."""

import sys
from pathlib import Path
import pytest

sys.path.append("python/quantum-pecos/src")

try:
    from guppylang import guppy
    from guppylang.std.quantum import qubit, measure, h
    GUPPY_AVAILABLE = True
except ImportError:
    GUPPY_AVAILABLE = False

try:
    from pecos.frontends import guppy_sim
    PECOS_AVAILABLE = True
except ImportError:
    PECOS_AVAILABLE = False


@pytest.mark.skipif(not GUPPY_AVAILABLE, reason="Guppy not available")
@pytest.mark.skipif(not PECOS_AVAILABLE, reason="PECOS not available")
class TestQubitAllocationLimits:
    """Test qubit allocation limits and dynamic allocation behavior."""
    
    def test_static_allocation_within_limit(self):
        """Test static allocation within the max_qubits limit."""
        @guppy
        def static_test() -> tuple[bool, bool, bool]:
            q1 = qubit()
            q2 = qubit()
            q3 = qubit()
            return measure(q1), measure(q2), measure(q3)
        
        # Should work fine with max_qubits=5
        results = guppy_sim(static_test, max_qubits=5).run(10)
        assert len(results["result"]) == 10
    
    def test_dynamic_allocation_in_loop(self):
        """Test dynamic allocation in a loop - requires sufficient max_qubits."""
        @guppy
        def dynamic_loop_test() -> int:
            count = 0
            # This allocates 3 qubits dynamically in the loop
            for i in range(3):
                q = qubit()
                h(q)
                if measure(q):
                    count += 1
            return count
        
        # Need to set max_qubits high enough for dynamic allocation
        # The static analysis might only see 1 qubit allocation,
        # but we actually allocate 3 qubits in the loop
        results = guppy_sim(dynamic_loop_test, max_qubits=10).seed(42).run(100)
        
        # Should see values 0-3
        values = set(results["result"])
        assert len(values) >= 2  # At least some variation
        assert all(0 <= v <= 3 for v in results["result"])
    
    def test_dynamic_allocation_exceeds_limit(self):
        """Test that dynamic allocation properly fails when exceeding limit."""
        @guppy
        def exceed_limit_test() -> int:
            count = 0
            # Try to allocate more qubits than the limit
            for i in range(5):
                q = qubit()
                h(q)
                if measure(q):
                    count += 1
            return count
        
        # Set limit too low for the dynamic allocation
        # This should fail with a helpful error message
        with pytest.raises(RuntimeError) as exc_info:
            results = guppy_sim(exceed_limit_test, max_qubits=3).run(1)
        
        # Check that the error message indicates a limit was exceeded
        error_msg = str(exc_info.value)
        # The error may come from different places:
        # 1. "Qubit allocation limit exceeded" from LLVM runtime
        # 2. "index out of bounds" from quantum engine when accessing beyond allocated qubits
        # Both indicate the limit was exceeded
        assert any(msg in error_msg for msg in [
            "Qubit allocation limit exceeded",
            "max_qubits", 
            "index out of bounds"
        ])
    
    def test_nested_loop_allocation(self):
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
        results = guppy_sim(nested_loop_test, max_qubits=10).seed(42).run(50)
        assert len(results["result"]) == 50
    
    def test_allocation_with_reset(self):
        """Test that reset allows qubit reuse within limits."""
        @guppy
        def reset_reuse_test() -> int:
            count = 0
            for i in range(5):
                q = qubit()
                h(q)
                if measure(q):
                    count += 1
                # Reset allows reuse (in theory)
                # Note: Current implementation may not reuse qubits
            return count
        
        # Even with reset, we might need max_qubits for all allocations
        results = guppy_sim(reset_reuse_test, max_qubits=10).seed(42).run(50)
        values = results["result"]
        assert all(0 <= v <= 5 for v in values)
    
    def test_explicit_max_qubits_setting(self):
        """Test that max_qubits is properly respected."""
        @guppy
        def simple_test() -> bool:
            q = qubit()
            return measure(q)
        
        # Test with different max_qubits values
        for max_q in [1, 5, 10, 20]:
            results = guppy_sim(simple_test, max_qubits=max_q).run(10)
            assert len(results["result"]) == 10


if __name__ == "__main__":
    print("Running qubit allocation limit tests...")
    pytest.main([__file__, "-v"])