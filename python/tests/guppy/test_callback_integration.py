"""Test the callback-based integration between Selene and PECOS.

This test demonstrates the complete flow:
1. Guppy program → HUGR
2. Selene build → Executable with Bridge simulator
3. Callback-based ByteMessage exchange
4. TCP stream for final results
"""


def test_callback_flow_documentation() -> None:
    """Document how the callback-based communication works."""
    print("=" * 60)
    print("CALLBACK-BASED SELENE-PECOS INTEGRATION")
    print("=" * 60)

    print(
        """
    ARCHITECTURE:

    1. Python Side (PySimBuilder):
       - Compile Guppy → HUGR
       - Use selene_sim.build() to create executable
       - Pass executable path to SeleneCallbackEngine

    2. Selene Executable (Separate Process):
       - Loads PecosSeleneBridgeSimulator plugin
       - Bridge simulator converts quantum ops to ByteMessages
       - Calls pecos_bridge_send_operations() to queue ops
       - Calls pecos_bridge_receive_measurements() for results

    3. SeleneCallbackEngine (PECOS Process):
       - Implements ControlEngine with EngineStage flow
       - start(): Launches Selene, gets first operations
       - continue_processing(): Provides measurements, gets next ops
       - Uses pecos_get_pending_operations() to retrieve ops
       - Uses pecos_provide_measurements() to send results

    4. HybridEngine Orchestration:
       - Calls engine.start() → gets ByteMessage ops
       - Sends to quantum engine → gets measurements
       - Calls engine.continue_processing(measurements)
       - Repeats until Complete

    5. Final Results:
       - Bridge simulator outputs to TCP stream
       - Engine captures results when execution complete
       - Formats as Shot for PECOS

    CALLBACK FUNCTIONS (FFI):

    Bridge → PECOS:
    - pecos_bridge_send_operations(data, len)
    - pecos_bridge_wait_for_measurements()
    - pecos_bridge_signal_complete()

    PECOS → Bridge:
    - pecos_get_pending_operations() → Option<ByteMessage>
    - pecos_provide_measurements(ByteMessage)
    - pecos_is_bridge_waiting() → bool
    - pecos_is_execution_complete() → bool

    SYNCHRONIZATION:

    The back-and-forth is managed by:
    - Bridge waits after measurements (blocks on receive)
    - Engine provides measurements via callback
    - Bridge continues, generates more ops
    - Cycle repeats until complete

    This maintains the proper EngineStage flow while using
    Selene naturally as a separate process with plugins.
    """,
    )


def test_example_guppy_program() -> None:
    """Show a Guppy program that would use this integration."""
    print("\n" + "=" * 60)
    print("EXAMPLE GUPPY PROGRAM")
    print("=" * 60)

    code = '''
from guppylang import guppy
from guppylang.std.quantum import qubit, h, cx, measure
from guppylang.std.builtins import result

@guppy
def teleportation() -> None:
    """Quantum teleportation with mid-circuit measurements."""
    # Create entangled pair
    q1, q2 = qubit(), qubit()
    h(q1)
    cx(q1, q2)

    # Prepare state to teleport
    q0 = qubit()
    h(q0)  # Teleport |+⟩ state

    # Bell measurement on q0, q1
    cx(q0, q1)
    h(q0)
    m0 = measure(q0)  # Mid-circuit measurement
    m1 = measure(q1)  # Mid-circuit measurement

    # Classical control based on measurements
    if m1:
        x(q2)  # Apply X correction
    if m0:
        z(q2)  # Apply Z correction

    # Measure final state
    final = measure(q2)

    # Output results
    result("bell_m0", m0)
    result("bell_m1", m1)
    result("teleported", final)
'''

    print(code)

    print(
        """
    EXECUTION FLOW:

    1. Initial operations (H, CNOT) → ByteMessage #1
       HybridEngine sends to quantum engine

    2. Measurements (m0, m1) → ByteMessage #2
       Quantum engine returns measurement results

    3. Engine.continue_processing(measurements)
       Bridge receives results, evaluates conditionals

    4. Conditional operations (X, Z) → ByteMessage #3
       Based on measurement outcomes

    5. Final measurement → ByteMessage #4
       Get teleported qubit state

    6. Results via TCP stream:
       ("bell_m0", True/False)
       ("bell_m1", True/False)
       ("teleported", True/False)
    """,
    )


def test_integration_steps() -> None:
    """Document the integration implementation steps."""
    print("\n" + "=" * 60)
    print("IMPLEMENTATION STEPS")
    print("=" * 60)

    print(
        """
    1. ✓ Design callback interface (callback_interface.rs)
    2. ✓ Update Bridge simulator to use callbacks
    3. ✓ Create SeleneCallbackEngine with EngineStage flow
    4. TODO: Update Python PySimBuilder to:
       - Detect Guppy programs
       - Compile to HUGR via compile_guppy_to_hugr()
       - Call selene_sim.build() with HUGR
       - Pass executable path to engine
    5. TODO: Implement TCP result stream capture
    6. TODO: Test with real Guppy programs
    7. TODO: Handle edge cases:
       - Timeout handling
       - Error propagation
       - Process cleanup
       - Multiple shots
    """,
    )


if __name__ == "__main__":
    test_callback_flow_documentation()
    test_example_guppy_program()
    test_integration_steps()
