# ByteMessage Python API Examples

This directory contains examples of using the PECOS ByteMessage API from Python. The ByteMessage API allows you to build quantum circuit messages that can be processed by PECOS engines.

## Bell State Example

`bell_state_example.py` demonstrates how to create a Bell state circuit using the ByteMessage API. It shows:

1. Creating a `ByteMessageBuilder` and configuring it for quantum operations
2. Adding Hadamard and CNOT gates to create a Bell state
3. Adding measurement operations
4. Building the message
5. Dumping and parsing the message contents

## Bell State Simulator Example

`bell_state_simulator.py` demonstrates running a Bell state circuit using the Python API. It shows:

1. Creating a Bell state circuit using `ByteMessageBuilder`
2. Creating a `StateVectorSimulator` with 2 qubits
3. Processing the Bell state circuit on the simulator
4. Running multiple shots to analyze measurement correlations
5. Running a GHZ state on three qubits

## Running the Examples

To run the examples:

```bash
# Make sure you're in the PECOS root directory
cd python/pecos-rslib
python examples/bell_state_example.py
python examples/bell_state_simulator.py
```

## API Overview

The ByteMessage Python API provides the following classes:

### `ByteMessage`

The main class for working with byte-encoded quantum messages.

#### Class Methods

- `ByteMessage.builder()`: Create a new `ByteMessageBuilder`
- `ByteMessage.quantum_operations_builder()`: Create a builder pre-configured for quantum operations
- `ByteMessage.measurement_results_builder()`: Create a builder pre-configured for measurement results
- `ByteMessage.create_bell_state()`: Create a pre-built Bell state circuit
- `ByteMessage.create_flush()`: Create a flush message
- `ByteMessage.create_empty()`: Create an empty message

#### Instance Methods

- `as_bytes()`: Get the message as bytes
- `is_empty()`: Check if the message is empty
- `parse_quantum_operations()`: Parse quantum operations from the message
- `dump_batch()`: Dump the batch contents for debugging
- `measurement_results()`: Get measurement results as a list of (result_id, outcome) tuples

### `ByteMessageBuilder`

Builder class for creating `ByteMessage` instances.

#### Constructor

- `ByteMessageBuilder()`: Create a new message builder

#### Configuration Methods

- `for_quantum_operations()`: Configure the builder for quantum operations
- `for_measurement_results()`: Configure the builder for measurement results

#### Gate Methods

- `add_x(qubit)`: Add an X gate
- `add_y(qubit)`: Add a Y gate
- `add_z(qubit)`: Add a Z gate
- `add_h(qubit)`: Add an H gate
- `add_cx(control, target)`: Add a CX (CNOT) gate
- `add_rz(theta, qubit)`: Add an RZ gate
- `add_rzz(theta, qubit1, qubit2)`: Add an RZZ gate
- `add_szz(qubit1, qubit2)`: Add an SZZ gate
- `add_r1xy(theta, phi, qubit)`: Add an R1XY gate
- `add_measurement(qubit, result_id)`: Add a measurement operation
- `add_prep(qubit)`: Add a qubit preparation operation
- `add_flush(is_last=False)`: Add a flush command

#### Builder Methods

- `build()`: Build the message and return a `ByteMessage`
- `clear()`: Clear the builder and reset to initial state
- `reset()`: Reset the builder while preserving capacity

### `StateVectorSimulator`

Simulator class for executing quantum circuits encoded as `ByteMessage` objects.

#### Constructor

- `StateVectorSimulator(num_qubits)`: Create a new simulator with the specified number of qubits

#### Simulator Methods

- `reset()`: Reset the simulator state
- `process(message)`: Process a `ByteMessage` circuit and return the measurement results
- `run_bell_state_experiment()`: Execute a Bell state experiment and return the measurement results
- `run_circuit_with_shots(message, shots=1000)`: Execute a circuit multiple times and return the measurement results for each shot
