"""Test the step by step debug in pytest"""

import sys
sys.path.append("python/quantum-pecos/src")

def test_step_by_step():
    from guppylang import guppy
    from guppylang.std.quantum import h, measure, qubit

    @guppy
    def simple_test() -> bool:
        q = qubit()
        return measure(q)

    print("Step 1: Guppy function defined")

    from pecos.frontends.guppy_frontend import GuppyFrontend

    print("Step 2: Creating frontend...")
    frontend = GuppyFrontend(use_rust_backend=True, llvm_convention="hugr")
    print("Step 2: Frontend created")

    print("Step 3: Compiling function...")
    qir_file = frontend.compile_function(simple_test)
    print(f"Step 3: QIR file created: {qir_file}")

    print("Step 4: Reading QIR content...")
    with open(qir_file, 'r') as f:
        qir_content = f.read()
    print(f"Step 4: QIR content length: {len(qir_content)} chars")

    print("Step 5: Testing execute_qir...")
    from pecos_rslib import execute_qir

    print("Step 5a: Calling execute_qir with 1 shot...")
    result = execute_qir(str(qir_file), 1, 42, None, None, llvm_convention="hugr")
    print(f"Step 5a: Success - result type: {type(result)}")
    print("Step 5a: execute_qir completed normally")

    print("Step 6: Test completed normally")
    assert result.get("execution_successful", False)