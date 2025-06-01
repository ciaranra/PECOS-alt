use crate::byte_message::builder::ByteMessageBuilder;
use crate::byte_message::protocol::{
    BatchHeader, GateCommandHeader, MeasurementHeader, MeasurementResultHeader, MessageHeader,
    MessageType, calc_padding,
};
use log::trace;
use pecos_core::QubitId;
use pecos_core::errors::PecosError;
use pecos_core::gate_type::GateType;
use pecos_core::gates::Gate;
use std::mem::size_of;

/// A message encoded using the PECOS byte protocol
///
/// Uses Vec<u32> for guaranteed 4-byte alignment matching our protocol design
#[derive(Clone)]
pub struct ByteMessage {
    data: Vec<u32>,
    byte_len: usize,
}

impl ByteMessage {
    /// Create a new `ByteMessage` from raw bytes
    #[must_use]
    pub fn new(bytes: &[u8]) -> Self {
        let byte_len = bytes.len();

        if byte_len == 0 {
            return Self {
                data: Vec::new(),
                byte_len: 0,
            };
        }

        // Calculate word count (round up to 4-byte boundary)
        let word_count = byte_len.div_ceil(4);

        // Create aligned storage
        let mut data = vec![0u32; word_count];

        // Copy bytes into aligned storage
        let data_bytes = bytemuck::cast_slice_mut::<u32, u8>(&mut data);
        data_bytes[..byte_len].copy_from_slice(bytes);

        Self { data, byte_len }
    }

    /// Create a new `ByteMessage` from already-aligned u32 data
    ///
    /// This method is used when receiving data from FFI boundaries where
    /// the data is already guaranteed to be 4-byte aligned.
    #[must_use]
    pub fn from_aligned_u32_data(data: Vec<u32>, byte_len: usize) -> Self {
        Self { data, byte_len }
    }

    /// Get a reference to the raw bytes
    #[must_use]
    pub fn as_bytes(&self) -> &[u8] {
        if self.byte_len == 0 {
            return &[];
        }

        let all_bytes = bytemuck::cast_slice::<u32, u8>(&self.data);
        &all_bytes[..self.byte_len]
    }

    /// Consume the message and return the raw bytes
    #[must_use]
    pub fn into_bytes(self) -> Vec<u8> {
        if self.byte_len == 0 {
            return Vec::new();
        }

        let all_bytes = bytemuck::cast_slice::<u32, u8>(&self.data);
        all_bytes[..self.byte_len].to_vec()
    }

    /// Create a new message builder
    #[must_use]
    pub fn builder() -> ByteMessageBuilder {
        ByteMessageBuilder::new()
    }

    /// Create a new message builder pre-configured for quantum operations
    ///
    /// This is a convenience method that creates a new builder and configures it
    /// for quantum operations.
    ///
    /// # Returns
    ///
    /// A `MessageBuilder` configured for quantum operations.
    #[must_use]
    pub fn quantum_operations_builder() -> ByteMessageBuilder {
        let mut builder = Self::builder();
        let _ = builder.for_quantum_operations();
        builder
    }

    /// Create a new message builder pre-configured for measurement results
    ///
    /// This is a convenience method that creates a new builder and configures it
    /// for measurement results.
    ///
    /// # Returns
    ///
    /// A `MessageBuilder` configured for measurement results.
    #[must_use]
    pub fn measurement_results_builder() -> ByteMessageBuilder {
        let mut builder = Self::builder();
        let _ = builder.for_measurement_results();
        builder
    }

    /// Create a new flush message
    ///
    /// This is a convenience method that creates a new message with a flush command.
    /// Flush messages are used to signal the end of a batch of commands.
    ///
    /// # Returns
    ///
    /// A `ByteMessage` containing a flush command.
    #[must_use]
    pub fn create_flush() -> Self {
        let mut builder = ByteMessageBuilder::new();
        builder.add_flush(true);
        builder.build()
    }

    /// Create a new message with a circuit of quantum gates
    ///
    /// This is a convenience method that creates a new message with multiple quantum gates
    /// representing a quantum circuit.
    ///
    /// # Arguments
    ///
    /// * `gates` - A slice of `GateCommand` objects
    ///
    /// # Returns
    ///
    /// A Result containing a `ByteMessage` with the circuit if successful, or a `PecosError` if there was an error.
    ///
    /// # Errors
    ///
    /// This function may return a `PecosError` if:
    /// - There is an error adding the gates to the builder
    /// - There is an error building the message
    ///
    /// # Example
    ///
    /// ```
    /// use pecos_engines::byte_message::ByteMessage;
    /// use pecos_engines::byte_message::Gate;
    ///
    /// // Create a circuit with H and CX gates
    /// let gates = vec![
    ///     Gate::h(&[0]),
    ///     Gate::cx(&[(0, 1)])
    /// ];
    ///
    /// let message = ByteMessage::create_circuit_from_quantum_gates(&gates).unwrap();
    /// ```
    pub fn create_circuit_from_quantum_gates(gates: &[Gate]) -> Result<Self, PecosError> {
        let mut builder = Self::quantum_operations_builder();
        builder.add_gate_commands(gates);
        Ok(builder.build())
    }

    /// Create a new message with a circuit of gate commands
    ///
    /// This is a convenience method that creates a new message with multiple gate commands
    /// representing a quantum circuit using the new flat `GateCommand` structure.
    ///
    /// # Arguments
    ///
    /// * `gates` - A slice of `GateCommand` objects
    ///
    /// # Returns
    ///
    /// A Result containing a `ByteMessage` with the circuit if successful, or a `PecosError` if there was an error.
    ///
    /// # Errors
    ///
    /// This function may return a `PecosError` if:
    /// - There is an error adding the gates to the builder
    /// - There is an error building the message
    ///
    /// # Example
    ///
    /// ```
    /// use pecos_engines::byte_message::ByteMessage;
    /// use pecos_engines::byte_message::Gate;
    ///
    /// // Create a circuit with H and CX gates
    /// let gates = vec![
    ///     Gate::h(&[0]),
    ///     Gate::cx(&[(0, 1)])
    /// ];
    ///
    /// let message = ByteMessage::create_circuit_from_gate_commands(&gates).unwrap();
    /// ```
    pub fn create_circuit_from_gate_commands(gates: &[Gate]) -> Result<Self, PecosError> {
        let mut builder = Self::quantum_operations_builder();
        builder.add_gate_commands(gates);
        Ok(builder.build())
    }

    /// Create a new message from a sequence of command strings
    ///
    /// This is a convenience method that creates a new message from a sequence of command strings
    /// in the format "`GATE_TYPE` [params...] qubit1 qubit2 ...".
    ///
    /// # Arguments
    ///
    /// * `commands` - A slice of command strings to parse
    ///
    /// # Returns
    ///
    /// A Result containing a `ByteMessage` with the commands if successful, or a `PecosError` if there was an error.
    ///
    /// # Errors
    ///
    /// This function may return a `PecosError` if:
    /// - A command string has an invalid format
    /// - A command string contains an unknown gate type
    /// - A command string contains invalid parameters (e.g., non-numeric values for angles)
    /// - A command string contains invalid qubit indices
    pub fn create_from_commands(commands: &[&str]) -> Result<Self, PecosError> {
        let mut builder = Self::quantum_operations_builder();
        for cmd in commands {
            Self::parse_command_to_builder(&mut builder, cmd)?;
        }
        Ok(builder.build())
    }

    /// Record measurement results
    ///
    /// This is a convenience method that creates a new message with measurement results.
    /// It's used to report measurement outcomes back to the classical controller.
    ///
    /// # Arguments
    ///
    /// * `result_pairs` - A slice of tuples containing (`result_id`, outcome)
    ///   where `result_id` corresponds to the ID used when requesting the measurement
    ///   and outcome is the measurement result (typically 0 or 1)
    ///
    /// # Returns
    ///
    /// A `ByteMessage` containing the measurement results.
    #[must_use]
    pub fn record_measurement_results(result_pairs: &[(usize, u32)]) -> Self {
        let mut builder = Self::measurement_results_builder();

        // Collect result_ids and outcomes into separate vectors
        let mut outcomes = Vec::with_capacity(result_pairs.len());

        for (_index, outcome) in result_pairs {
            outcomes.push(*outcome as usize); // Convert u32 to usize
        }

        builder.add_measurement_results(&outcomes);
        builder.build()
    }

    /// Create a message with a single quantum gate
    ///
    /// This is a convenience method that creates a new message with a single quantum gate.
    ///
    /// # Arguments
    ///
    /// * `gate` - The quantum gate to add
    ///
    /// # Returns
    ///
    /// A `ByteMessage` with the gate.
    ///
    /// # Example
    ///
    /// ```
    /// use pecos_engines::byte_message::ByteMessage;
    /// use pecos_engines::byte_message::Gate;
    ///
    /// // Create a message with an H gate on qubit 0
    /// let gate = Gate::h(&[0]);
    /// let message = ByteMessage::create_with_quantum_gate(&gate);
    /// ```
    #[must_use]
    pub fn create_with_quantum_gate(gate: &Gate) -> Self {
        let mut builder = Self::quantum_operations_builder();
        builder.add_gate_command(gate);
        builder.build()
    }

    /// Parse a command string and add it to the `ByteMessage` builder
    ///
    /// This function parses a command string in the format "`GATE_TYPE` [params...] qubit1 qubit2 ..."
    /// and adds the corresponding quantum gate to the provided builder.
    ///
    /// # Arguments
    ///
    /// * `builder` - The `ByteMessageBuilder` to add the command to
    /// * `cmd` - The command string to parse
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if the command was successfully parsed and added to the builder,
    /// or a `PecosError` if there was an error.
    ///
    /// # Errors
    ///
    /// This function may return a `PecosError::InvalidInput` if:
    /// - The command string has an invalid format
    /// - The command string contains an unknown gate type
    /// - The command string contains invalid parameters (e.g., non-numeric values for angles)
    /// - The command string contains invalid qubit indices
    #[allow(clippy::too_many_lines)]
    pub fn parse_command_to_builder(
        builder: &mut ByteMessageBuilder,
        cmd: &str,
    ) -> Result<(), PecosError> {
        let parts: Vec<&str> = cmd.split_whitespace().collect();
        if parts.is_empty() {
            return Ok(());
        }

        match parts.first() {
            Some(&"RZ") => {
                if parts.len() >= 3 {
                    let theta = parts[1].parse::<f64>().map_err(|_| {
                        PecosError::Input(format!("Invalid angle in RZ command: {}", parts[1]))
                    })?;
                    let qubit = parts[2].parse::<usize>().map_err(|_| {
                        PecosError::Input(format!("Invalid qubit in RZ command: {}", parts[2]))
                    })?;
                    builder.add_rz(theta, &[qubit]);
                }
            }
            Some(&"R1XY") => {
                if parts.len() >= 4 {
                    let theta = parts[1].parse::<f64>().map_err(|_| {
                        PecosError::Input(format!(
                            "Invalid theta angle in R1XY command: {}",
                            parts[1]
                        ))
                    })?;
                    let phi = parts[2].parse::<f64>().map_err(|_| {
                        PecosError::Input(format!(
                            "Invalid phi angle in R1XY command: {}",
                            parts[2]
                        ))
                    })?;
                    let qubit = parts[3].parse::<usize>().map_err(|_| {
                        PecosError::Input(format!("Invalid qubit in R1XY command: {}", parts[3]))
                    })?;
                    builder.add_r1xy(theta, phi, &[qubit]);
                }
            }
            Some(&"SZZ") => {
                if parts.len() >= 3 {
                    let qubit1 = parts[1].parse::<usize>().map_err(|_| {
                        PecosError::Input(format!("Invalid qubit1 in SZZ command: {}", parts[1]))
                    })?;
                    let qubit2 = parts[2].parse::<usize>().map_err(|_| {
                        PecosError::Input(format!("Invalid qubit2 in SZZ command: {}", parts[2]))
                    })?;
                    builder.add_szz(&[qubit1], &[qubit2]);
                }
            }
            Some(&"H") => {
                if parts.len() >= 2 {
                    let qubit = parts[1].parse::<usize>().map_err(|_| {
                        PecosError::Input(format!("Invalid qubit in H command: {}", parts[1]))
                    })?;
                    builder.add_h(&[qubit]);
                }
            }
            Some(&"CX") => {
                if parts.len() >= 3 {
                    let control = parts[1].parse::<usize>().map_err(|_| {
                        PecosError::Input(format!(
                            "Invalid control qubit in CX command: {}",
                            parts[1]
                        ))
                    })?;
                    let target = parts[2].parse::<usize>().map_err(|_| {
                        PecosError::Input(format!(
                            "Invalid target qubit in CX command: {}",
                            parts[2]
                        ))
                    })?;
                    builder.add_cx(&[control], &[target]);
                }
            }
            Some(&"M") => {
                if parts.len() >= 2 {
                    let qubit = parts[1].parse::<usize>().map_err(|_| {
                        PecosError::Input(format!("Invalid qubit in M command: {}", parts[1]))
                    })?;
                    builder.add_measurements(&[qubit]);
                }
            }
            Some(&"P") => {
                if parts.len() >= 2 {
                    let qubit = parts[1].parse::<usize>().map_err(|_| {
                        PecosError::Input(format!("Invalid qubit in P command: {}", parts[1]))
                    })?;
                    builder.add_prep(&[qubit]);
                }
            }
            _ => {
                return Err(PecosError::Input(format!(
                    "Unknown command type: {}",
                    parts[0]
                )));
            }
        }

        Ok(())
    }

    /// Determine the message type by parsing the header
    ///
    /// This function parses the message header to determine the type of the message.
    ///
    /// # Returns
    ///
    /// Returns a `Result` containing the `MessageType` if successful, or a `PecosError` if there was an error.
    ///
    /// # Errors
    ///
    /// This function may return a `PecosError::InvalidInput` if:
    /// - The message is too small to contain a batch header
    /// - The batch header is invalid
    /// - The batch contains no messages
    /// - The message is too small to contain a message header
    /// - The message header contains an invalid message type
    pub fn message_type(&self) -> Result<MessageType, PecosError> {
        if self.byte_len < size_of::<BatchHeader>() {
            return Err(PecosError::Input(
                "Message too small for batch header".to_string(),
            ));
        }

        // Parse batch header - guaranteed aligned at offset 0 due to Vec<u32> storage
        let batch_header =
            *bytemuck::from_bytes::<BatchHeader>(&self.as_bytes()[0..size_of::<BatchHeader>()]);
        if !batch_header.is_valid() {
            return Err(PecosError::Input("Invalid batch header".to_string()));
        }

        // Need at least one message to determine type
        if batch_header.msg_count == 0 {
            return Err(PecosError::Input("Batch contains no messages".to_string()));
        }

        // Skip to first message header (after batch header)
        let msg_offset = size_of::<BatchHeader>();
        if self.byte_len < msg_offset + size_of::<MessageHeader>() {
            return Err(PecosError::Input(
                "Message too small for message header".to_string(),
            ));
        }

        // Parse message header - guaranteed aligned due to builder padding
        let msg_header = *bytemuck::from_bytes::<MessageHeader>(
            &self.as_bytes()[msg_offset..msg_offset + size_of::<MessageHeader>()],
        );
        msg_header
            .get_type()
            .map_err(|e| PecosError::Input(format!("Failed to determine message type: {e}")))
    }

    /// Check if this message is empty (contains no operations)
    ///
    /// This function checks if the message is empty, meaning it either contains a flush command
    /// or a batch with no operations.
    ///
    /// # Returns
    ///
    /// Returns a `Result` containing a boolean indicating whether the message is empty if successful,
    /// or a `PecosError` if there was an error.
    ///
    /// # Errors
    ///
    /// This function may return a `PecosError` if:
    /// - There is an error determining the message type
    /// - There is an error parsing the quantum operations in the message
    pub fn is_empty(&self) -> Result<bool, PecosError> {
        match self.message_type()? {
            MessageType::Flush => Ok(true),
            MessageType::BeginBatch => {
                // Check if this is a batch with no operations
                let commands = self.parse_quantum_operations()?;
                Ok(commands.is_empty())
            }
            _ => Ok(false),
        }
    }

    /// Parse quantum operations from this message
    ///
    /// # Errors
    ///
    /// Returns an error if the message is malformed or contains invalid quantum operations.
    pub fn parse_quantum_operations(&self) -> Result<Vec<Gate>, PecosError> {
        if self.byte_len < size_of::<BatchHeader>() {
            return Err(PecosError::Input(
                "Message too small for batch header".to_string(),
            ));
        }

        // Parse batch header - guaranteed aligned at offset 0 due to Vec<u32> storage
        let batch_header =
            *bytemuck::from_bytes::<BatchHeader>(&self.as_bytes()[0..size_of::<BatchHeader>()]);
        if !batch_header.is_valid() {
            return Err(PecosError::Input("Invalid batch header".to_string()));
        }

        let mut commands = Vec::new();
        let mut offset = size_of::<BatchHeader>();
        let mut in_command_batch = false;

        // Process each message
        for _ in 0..batch_header.msg_count {
            if offset + size_of::<MessageHeader>() > self.byte_len {
                break;
            }

            // Parse message header - guaranteed aligned due to builder padding
            let msg_header = *bytemuck::from_bytes::<MessageHeader>(
                &self.as_bytes()[offset..offset + size_of::<MessageHeader>()],
            );
            offset += size_of::<MessageHeader>();

            // Check if this is a quantum operations message
            if msg_header.msg_type == MessageType::BeginBatch as u8 {
                in_command_batch = true;
            } else if msg_header.msg_type == MessageType::EndBatch as u8 {
                // End of batch
                break;
            }

            // Skip to next message if not in a command batch
            if !in_command_batch {
                offset += msg_header.payload_size as usize;
                let padding = calc_padding(msg_header.payload_size as usize, 4);
                if padding > 0 {
                    offset += padding;
                }
                continue;
            }

            // Process payload based on message type
            match msg_header.msg_type {
                x if x == MessageType::GateCommand as u8 => {
                    if offset + msg_header.payload_size as usize <= self.byte_len {
                        let payload =
                            &self.as_bytes()[offset..offset + msg_header.payload_size as usize];
                        match Self::parse_gate_command(payload) {
                            Ok(cmd) => commands.push(cmd),
                            Err(e) => {
                                trace!("Error parsing quantum gate: {}", e);
                            }
                        }
                    }
                }
                x if x == MessageType::Measurement as u8 => {
                    if offset + msg_header.payload_size as usize <= self.byte_len {
                        let payload =
                            &self.as_bytes()[offset..offset + msg_header.payload_size as usize];
                        match Self::parse_measurement_command(payload) {
                            Ok(cmd) => commands.push(cmd),
                            Err(e) => {
                                trace!("Error parsing measurement: {}", e);
                            }
                        }
                    }
                }
                _ => {}
            }

            // Move to next message
            offset += msg_header.payload_size as usize;
            let padding = calc_padding(msg_header.payload_size as usize, 4);
            if padding > 0 {
                offset += padding;
            }
        }

        Ok(commands)
    }

    /// Parse gate commands from this message using the new flat `GateCommand` structure
    ///
    /// # Errors
    ///
    /// Returns an error if the message is malformed or contains invalid quantum operations.
    pub fn parse_gate_commands(&self) -> Result<Vec<Gate>, PecosError> {
        if self.byte_len < size_of::<BatchHeader>() {
            return Err(PecosError::Input(
                "Message too small for batch header".to_string(),
            ));
        }

        // Parse batch header - guaranteed aligned at offset 0 due to Vec<u32> storage
        let batch_header =
            *bytemuck::from_bytes::<BatchHeader>(&self.as_bytes()[0..size_of::<BatchHeader>()]);
        if !batch_header.is_valid() {
            return Err(PecosError::Input("Invalid batch header".to_string()));
        }

        let mut commands = Vec::new();
        let mut offset = size_of::<BatchHeader>();

        for _ in 0..batch_header.msg_count {
            if offset + size_of::<MessageHeader>() > self.byte_len {
                break;
            }

            // Parse message header - guaranteed aligned due to padding
            let msg_header = *bytemuck::from_bytes::<MessageHeader>(
                &self.as_bytes()[offset..offset + size_of::<MessageHeader>()],
            );
            offset += size_of::<MessageHeader>();

            // Handle batch control messages
            if msg_header.msg_type == MessageType::BeginBatch as u8 {
                continue;
            }
            if msg_header.msg_type == MessageType::EndBatch as u8 {
                continue;
            }

            // Skip padding if needed
            if msg_header.payload_size == 0 {
                let padding = calc_padding(msg_header.payload_size as usize, 4);
                if padding > 0 {
                    offset += padding;
                }
                continue;
            }

            // Process payload based on message type
            match msg_header.msg_type {
                x if x == MessageType::GateCommand as u8 => {
                    if offset + msg_header.payload_size as usize <= self.byte_len {
                        let payload =
                            &self.as_bytes()[offset..offset + msg_header.payload_size as usize];
                        match Self::parse_gate_command(payload) {
                            Ok(cmd) => commands.push(cmd),
                            Err(e) => {
                                trace!("Error parsing gate command: {}", e);
                            }
                        }
                    }
                }
                x if x == MessageType::Measurement as u8 => {
                    if offset + msg_header.payload_size as usize <= self.byte_len {
                        let payload =
                            &self.as_bytes()[offset..offset + msg_header.payload_size as usize];
                        match Self::parse_measurement_command(payload) {
                            Ok(cmd) => commands.push(cmd),
                            Err(e) => {
                                trace!("Error parsing measurement command: {}", e);
                            }
                        }
                    }
                }
                _ => {
                    // Skip unknown message types
                }
            }

            // Move to next message
            offset += msg_header.payload_size as usize;
            let padding = calc_padding(msg_header.payload_size as usize, 4);
            if padding > 0 {
                offset += padding;
            }
        }

        Ok(commands)
    }

    /// Parse measurements from this message
    ///
    /// # Errors
    ///
    /// Returns an error if the message is malformed or contains invalid measurement data.
    pub fn parse_measurements(&self) -> Result<Vec<u32>, PecosError> {
        if self.byte_len < size_of::<BatchHeader>() {
            return Err(PecosError::Input(
                "Message too small for batch header".to_string(),
            ));
        }

        // Parse batch header - guaranteed aligned at offset 0 due to Vec<u32> storage
        let batch_header =
            *bytemuck::from_bytes::<BatchHeader>(&self.as_bytes()[0..size_of::<BatchHeader>()]);
        if !batch_header.is_valid() {
            return Err(PecosError::Input("Invalid batch header".to_string()));
        }

        let mut measurements = Vec::new();
        let mut offset = size_of::<BatchHeader>();

        // Process each message
        for _ in 0..batch_header.msg_count {
            if offset + size_of::<MessageHeader>() > self.byte_len {
                break;
            }

            // Parse message header - guaranteed aligned due to builder padding
            let msg_header = *bytemuck::from_bytes::<MessageHeader>(
                &self.as_bytes()[offset..offset + size_of::<MessageHeader>()],
            );
            offset += size_of::<MessageHeader>();

            let msg_type = msg_header
                .get_type()
                .map_err(|e| PecosError::Input(e.to_string()))?;

            let payload_size = msg_header.payload_size as usize;
            let payload_end = offset + payload_size;

            if payload_end > self.byte_len {
                return Err(PecosError::Input(format!(
                    "Message payload extends beyond message bounds: offset={}, size={}, total_len={}",
                    offset, payload_size, self.byte_len
                )));
            }

            if msg_type == MessageType::MeasurementResult {
                // Process measurement result
                let payload = &self.as_bytes()[offset..payload_end];
                if payload.len() >= size_of::<MeasurementResultHeader>() {
                    // MeasurementResultHeader at aligned payload start
                    let result_header = *bytemuck::from_bytes::<MeasurementResultHeader>(
                        &payload[0..size_of::<MeasurementResultHeader>()],
                    );

                    // Return outcome
                    measurements.push(result_header.outcome);
                }
            }

            // Move offset to next message, accounting for padding
            offset = payload_end;
            let padding = calc_padding(payload_size, 4);
            if padding > 0 {
                offset += padding;
            }
        }

        Ok(measurements)
    }

    /// Get measurement results as a vector of outcomes
    ///
    /// This is a convenience method that parses the measurement results from the message
    /// and returns them as a vector of measurement outcomes in order.
    ///
    /// # Returns
    ///
    /// A Result containing a vector of measurement outcomes if successful,
    /// or a `PecosError` if there was an error parsing the message.
    ///
    /// # Errors
    ///
    /// Returns an error if the message is malformed or contains invalid measurement data.
    pub fn measurement_results_as_vec(&self) -> Result<Vec<(usize, u32)>, PecosError> {
        let outcomes = self.parse_measurements()?;

        // Convert to indexed results (index, outcome) for compatibility
        let converted = outcomes.into_iter().enumerate().collect();

        Ok(converted)
    }

    /// Validate if the payload has enough bytes for the gate header
    fn validate_gate_payload_size(payload: &[u8]) -> Result<(), PecosError> {
        if payload.len() < size_of::<GateCommandHeader>() {
            return Err(PecosError::Input(
                "Quantum gate message payload too small".to_string(),
            ));
        }
        Ok(())
    }

    /// Validate if the payload has enough bytes for qubit indices
    fn validate_qubit_indices_size(
        payload: &[u8],
        qubits_offset: usize,
        qubits_size: usize,
    ) -> Result<(), PecosError> {
        let minimum_size = qubits_offset + qubits_size;
        if payload.len() < minimum_size {
            return Err(PecosError::Input(
                "Quantum gate message payload too small for qubit indices".to_string(),
            ));
        }
        Ok(())
    }

    /// Parse qubit indices from the payload
    fn parse_qubit_indices(payload: &[u8], qubits_offset: usize, num_qubits: usize) -> Vec<usize> {
        let mut qubits = Vec::with_capacity(num_qubits);
        for i in 0..num_qubits {
            let qubit_offset = qubits_offset + i * size_of::<u32>();
            let qubit = u32::from_le_bytes([
                payload[qubit_offset],
                payload[qubit_offset + 1],
                payload[qubit_offset + 2],
                payload[qubit_offset + 3],
            ]) as usize;
            qubits.push(qubit);
        }
        qubits
    }

    /// Parse gate parameters based on gate type
    fn parse_gate_parameters(
        payload: &[u8],
        params_offset: usize,
        gate_type: GateType,
    ) -> Result<Vec<f64>, PecosError> {
        let mut params = Vec::new();

        match gate_type {
            GateType::RZ => {
                Self::validate_params_size(
                    payload,
                    params_offset,
                    size_of::<f64>(),
                    "RZ parameters",
                )?;

                let theta = Self::parse_f64_param(payload, params_offset);
                params.push(theta);
            }
            GateType::R1XY => {
                Self::validate_params_size(
                    payload,
                    params_offset,
                    2 * size_of::<f64>(),
                    "R1XY parameters",
                )?;

                let theta = Self::parse_f64_param(payload, params_offset);
                params.push(theta);

                let phi = Self::parse_f64_param(payload, params_offset + size_of::<f64>());
                params.push(phi);
            }
            GateType::RZZ => {
                Self::validate_params_size(
                    payload,
                    params_offset,
                    size_of::<f64>(),
                    "RZZ parameters",
                )?;

                let theta = Self::parse_f64_param(payload, params_offset);
                params.push(theta);
            }
            GateType::Measure
            | GateType::I
            | GateType::X
            | GateType::Y
            | GateType::Z
            | GateType::H
            | GateType::CX
            | GateType::SZZ
            | GateType::SZZdg
            | GateType::Prep
            | GateType::Idle
            | GateType::U => {
                // These gates have no parameters in the message format
            }
        }

        Ok(params)
    }

    /// Validate if the payload has enough bytes for parameters
    fn validate_params_size(
        payload: &[u8],
        params_offset: usize,
        required_size: usize,
        gate_name: &str,
    ) -> Result<(), PecosError> {
        if payload.len() < params_offset + required_size {
            return Err(PecosError::Input(format!(
                "Quantum gate message payload too small for {gate_name}"
            )));
        }
        Ok(())
    }

    /// Parse an f64 parameter from the payload
    fn parse_f64_param(payload: &[u8], offset: usize) -> f64 {
        let param_bytes = &payload[offset..offset + size_of::<f64>()];
        // Performance critical path during simulation - slice to array conversion should never fail
        // when we already verified the buffer size (8 bytes for f64)
        f64::from_le_bytes(
            param_bytes[..8]
                .try_into()
                .expect("Byte buffer has incorrect length for f64 conversion"),
        )
    }

    /// Parse a quantum gate message payload to `GateCommand`
    fn parse_gate_command(payload: &[u8]) -> Result<Gate, PecosError> {
        Self::validate_gate_payload_size(payload)?;

        // Parse gate header - guaranteed aligned since payload starts at aligned boundary
        let header =
            *bytemuck::from_bytes::<GateCommandHeader>(&payload[0..size_of::<GateCommandHeader>()]);
        let num_qubits = header.num_qubits as usize;
        let has_params = header.has_params != 0;
        let gate_type = GateType::from(header.gate_type);

        // Calculate sizes
        let qubits_byte_size = num_qubits * size_of::<u32>();
        let qubits_offset = size_of::<GateCommandHeader>();

        Self::validate_qubit_indices_size(payload, qubits_offset, qubits_byte_size)?;

        // Parse qubit indices and convert to QubitId
        let qubits_usize = Self::parse_qubit_indices(payload, qubits_offset, num_qubits);
        let qubits: Vec<QubitId> = qubits_usize.into_iter().map(QubitId::from).collect();

        // Parse parameters if present
        let params = if has_params {
            let params_offset = qubits_offset + qubits_byte_size;
            Self::parse_gate_parameters(payload, params_offset, gate_type)?
        } else {
            Vec::new()
        };

        Ok(Gate::new(gate_type, params, qubits))
    }

    /// Parse a measurement message payload to `GateCommand`
    fn parse_measurement_command(payload: &[u8]) -> Result<Gate, PecosError> {
        if payload.len() < size_of::<MeasurementHeader>() {
            return Err(PecosError::Input(
                "Measurement message payload too small".to_string(),
            ));
        }

        // Parse measurement header - guaranteed aligned since payload starts at aligned boundary
        let header =
            *bytemuck::from_bytes::<MeasurementHeader>(&payload[0..size_of::<MeasurementHeader>()]);
        let qubit = header.qubit as usize;

        Ok(Gate::measure(&[qubit]))
    }

    /// Creates an empty `ByteMessage`
    ///
    /// This method creates a minimal valid `ByteMessage` with no content.
    /// It's useful as a fallback when processing operations fails.
    ///
    /// # Returns
    /// A new empty `ByteMessage`
    #[must_use]
    pub fn create_empty() -> Self {
        Self {
            data: Vec::new(),
            byte_len: 0,
        }
    }

    /// Create a record data message with key-value pair
    #[must_use]
    pub fn create_record_data(key: &str, value: f64) -> Self {
        let mut builder = ByteMessageBuilder::new();
        builder.add_record_data(key, value);
        builder.build()
    }

    /// Create a result record message
    #[must_use]
    pub fn create_result_record(result_id: usize, label: Option<&str>) -> Self {
        let mut builder = ByteMessageBuilder::new();
        builder.add_result_record(result_id, label);
        builder.build()
    }

    /// Create an info message
    #[must_use]
    pub fn create_info(message: &str) -> Self {
        let mut builder = ByteMessageBuilder::new();
        builder.add_info_message(message);
        builder.build()
    }

    /// Create a warning message
    #[must_use]
    pub fn create_warning(message: &str) -> Self {
        let mut builder = ByteMessageBuilder::new();
        builder.add_warning_message(message);
        builder.build()
    }

    /// Create an error message
    #[must_use]
    pub fn create_error(message: &str) -> Self {
        let mut builder = ByteMessageBuilder::new();
        builder.add_error_message(message);
        builder.build()
    }

    /// Create a debug message
    #[must_use]
    pub fn create_debug(message: &str) -> Self {
        let mut builder = ByteMessageBuilder::new();
        builder.add_debug_message(message);
        builder.build()
    }

    /// Parse measured qubits from this message
    ///
    /// This method extracts the qubit indices of measurements from the message.
    /// It returns a list of qubits that were measured, in the same order as
    /// the measurement results returned by `parse_measurements()`.
    ///
    /// # Returns
    ///
    /// A Result containing a vector of qubit indices if successful,
    /// or a `PecosError` if there was an error parsing the message.
    ///
    /// # Errors
    ///
    /// Returns an error if the message is malformed or contains invalid measurement data.
    pub fn parse_measured_qubits(&self) -> Result<Vec<u32>, PecosError> {
        if self.byte_len == 0 {
            return Ok(Vec::new());
        }

        let qubits = Vec::new();
        let mut offset = 0;

        while offset + size_of::<MessageHeader>() <= self.byte_len {
            // Read message header - guaranteed aligned due to builder padding
            let msg_header = *bytemuck::from_bytes::<MessageHeader>(
                &self.as_bytes()[offset..offset + size_of::<MessageHeader>()],
            );
            offset += size_of::<MessageHeader>();

            // Skip if not enough bytes for payload
            let payload_size = msg_header.payload_size as usize;
            let payload_end = offset + payload_size;

            if payload_end > self.byte_len {
                break;
            }

            // Convert the msg_type to MessageType
            if let Ok(msg_type) = msg_header.get_type() {
                if msg_type == MessageType::MeasurementResult {
                    // Process measurement result
                    let payload = &self.as_bytes()[offset..payload_end];
                    if payload.len() >= size_of::<MeasurementResultHeader>() {
                        // MeasurementResultHeader at aligned payload start
                        let _result_header = *bytemuck::from_bytes::<MeasurementResultHeader>(
                            &payload[0..size_of::<MeasurementResultHeader>()],
                        );

                        // Since MeasurementResultHeader doesn't have a qubit field, we can't get it directly
                        // This information is no longer available in the result messages
                        // The calling code needs to track which qubits were measured based on the order of measurement commands
                    }
                }
            }

            // Move offset to next message, accounting for padding
            offset = payload_end;
            let padding = calc_padding(payload_size, 4);
            if padding > 0 {
                offset += padding;
            }
        }

        Ok(qubits)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Engine;
    use crate::quantum::StateVecEngine;
    use pecos_core::QubitId;

    #[test]
    fn test_bytemap_builder() {
        // Create a message with H and CX gates
        let mut builder = ByteMessage::quantum_operations_builder();
        builder.add_h(&[0]);
        builder.add_cx(&[0], &[1]);
        let message = builder.build();

        // Parse the message
        let parsed_commands = message.parse_quantum_operations().unwrap();
        assert_eq!(parsed_commands.len(), 2);
        assert_eq!(parsed_commands[0].gate_type, GateType::H);
        assert_eq!(parsed_commands[0].qubits, vec![QubitId(0)]);
        assert_eq!(parsed_commands[1].gate_type, GateType::CX);
        assert_eq!(parsed_commands[1].qubits, vec![QubitId(0), QubitId(1)]);
    }

    #[test]
    fn test_gate_command_parsing() {
        // Create a message with gate commands using the new interface
        let gates = vec![Gate::h(&[0]), Gate::rz(0.5, &[1]), Gate::cx(&[(0, 1)])];
        let message = ByteMessage::create_circuit_from_gate_commands(&gates).unwrap();

        // Parse the message using the new gate command interface
        let parsed_commands = message.parse_gate_commands().unwrap();
        assert_eq!(parsed_commands.len(), 3);

        // Verify H gate
        assert_eq!(parsed_commands[0].gate_type, GateType::H);
        assert_eq!(parsed_commands[0].qubits, vec![QubitId(0)]);
        assert!(parsed_commands[0].params.is_empty());

        // Verify RZ gate
        assert_eq!(parsed_commands[1].gate_type, GateType::RZ);
        assert_eq!(parsed_commands[1].qubits, vec![QubitId(1)]);
        assert_eq!(parsed_commands[1].params, vec![0.5]);

        // Verify CX gate
        assert_eq!(parsed_commands[2].gate_type, GateType::CX);
        assert_eq!(parsed_commands[2].qubits, vec![QubitId(0), QubitId(1)]);
        assert!(parsed_commands[2].params.is_empty());
    }

    #[test]
    fn test_message_type() {
        // Create a flush message
        let flush_message = ByteMessage::create_flush();
        assert_eq!(flush_message.message_type().unwrap(), MessageType::Flush);

        // Create a quantum operations message
        let mut builder = ByteMessage::quantum_operations_builder();
        builder.add_h(&[0]);
        let quantum_message = builder.build();
        assert_eq!(
            quantum_message.message_type().unwrap(),
            MessageType::BeginBatch
        );

        // Create a measurement results message
        let mut builder = ByteMessage::measurement_results_builder();
        builder.add_measurement_results(&[0]);
        let results_message = builder.build();
        assert_eq!(
            results_message.message_type().unwrap(),
            MessageType::BeginBatch
        );
    }

    #[test]
    fn test_parse_measurements() {
        // Create a message with measurement results
        let mut builder = ByteMessage::measurement_results_builder();
        builder.add_measurement_results(&[0, 1]);
        let message = builder.build();

        // Parse the measurements
        let measurements = message.parse_measurements().unwrap();
        assert_eq!(measurements.len(), 2);

        // The measurements now just return outcomes
        assert_eq!(measurements[0], 0);
        assert_eq!(measurements[1], 1);
    }

    #[test]
    fn test_measurement_results_as_vec() {
        // Create a message with measurement results
        let result_pairs = [(0, 0), (1, 1), (2, 0)];
        let message = ByteMessage::record_measurement_results(&result_pairs);

        // Get the results as a vector
        let results = message.measurement_results_as_vec().unwrap();

        // Verify the results match the input
        assert_eq!(results.len(), 3);
        assert_eq!(results[0], (0, 0));
        assert_eq!(results[1], (1, 1));
        assert_eq!(results[2], (2, 0));

        // Verify the types are correct (usize, u32) by checking if they can be assigned to variables of those types
        let (result_id, outcome) = results[0];
        let _: usize = result_id; // This will fail to compile if result_id is not usize
        let _: u32 = outcome; // This will fail to compile if outcome is not u32
    }

    #[test]
    fn test_bell_state_measurements() {
        // Create a Bell state circuit: H on qubit 0, CX from 0 to 1, measure both qubits
        let mut builder = ByteMessage::quantum_operations_builder();

        // Apply H to qubit 0
        builder.add_h(&[0]);

        // Apply CX with control=0, target=1
        builder.add_cx(&[0], &[1]);

        // Measure qubit 0 with result_id 0
        builder.add_measurements(&[0]);

        // Measure qubit 1 with result_id 1
        builder.add_measurements(&[1]);

        let bell_circuit = builder.build();

        // Run the circuit multiple times and check the results
        let mut engine = StateVecEngine::new(2); // Create a simulator with 2 qubits

        for _ in 0..10 {
            // Reset the engine for each run
            engine.reset().unwrap();

            // Process the circuit
            let result_message = engine.process(bell_circuit.clone()).unwrap();

            // Get the measurement results as a vector
            let results = result_message.measurement_results_as_vec().unwrap();

            // Convert to booleans (0 -> false, 1 -> true)
            let q0_result = results
                .iter()
                .find(|(id, _)| *id == 0)
                .map(|(_, val)| *val != 0)
                .unwrap();
            let q1_result = results
                .iter()
                .find(|(id, _)| *id == 1)
                .map(|(_, val)| *val != 0)
                .unwrap();

            // In a Bell state, the qubits should always have the same measurement outcome
            assert_eq!(
                q0_result, q1_result,
                "Qubits in Bell state should have correlated measurements"
            );
        }
    }

    #[test]
    fn test_is_empty() {
        // Create an empty message
        let empty_message = ByteMessage::builder().build();
        assert!(empty_message.is_empty().unwrap());

        // Create a non-empty message
        let non_empty_message = ByteMessage::quantum_operations_builder()
            .add_h(&[0])
            .build();
        assert!(!non_empty_message.is_empty().unwrap());
    }

    #[test]
    fn test_parse_command_to_builder() {
        // Test various commands including the new "P" command
        let commands = [
            "H 0", "CX 0 1", "RZ 0.5 2", "P 3", // Test the new P command
            "M 4 0",
        ];

        let message = ByteMessage::create_from_commands(&commands).unwrap();

        // Parse the quantum operations from the message
        let operations = message.parse_quantum_operations().unwrap();

        // We should have 5 operations
        assert_eq!(operations.len(), 5);

        // Check the P command was correctly parsed
        assert_eq!(operations[3].gate_type, GateType::Prep);
        assert_eq!(operations[3].qubits, vec![QubitId(3)]);
        assert!(operations[3].params.is_empty());
    }

    #[test]
    fn test_measurement_result_order_preservation() {
        // Test that measurement results maintain their order through ByteMessage
        let mut builder = ByteMessage::measurement_results_builder();

        // Add measurement results in a specific order
        builder.add_measurement_results(&[1]); // First result: 1
        builder.add_measurement_results(&[0]); // Second result: 0
        builder.add_measurement_results(&[1]); // Third result: 1
        builder.add_measurement_results(&[1]); // Fourth result: 1
        builder.add_measurement_results(&[0]); // Fifth result: 0

        let message = builder.build();

        // Parse the measurements back
        let results = message.parse_measurements().unwrap();

        // Verify order is preserved
        assert_eq!(results.len(), 5);
        assert_eq!(results[0], 1, "First result should be 1");
        assert_eq!(results[1], 0, "Second result should be 0");
        assert_eq!(results[2], 1, "Third result should be 1");
        assert_eq!(results[3], 1, "Fourth result should be 1");
        assert_eq!(results[4], 0, "Fifth result should be 0");

        // Also test measurement_results_as_vec which adds indices
        let indexed_results = message.measurement_results_as_vec().unwrap();
        assert_eq!(indexed_results.len(), 5);
        assert_eq!(indexed_results[0], (0, 1), "First indexed result");
        assert_eq!(indexed_results[1], (1, 0), "Second indexed result");
        assert_eq!(indexed_results[2], (2, 1), "Third indexed result");
        assert_eq!(indexed_results[3], (3, 1), "Fourth indexed result");
        assert_eq!(indexed_results[4], (4, 0), "Fifth indexed result");
    }

    #[test]
    fn test_alignment_guarantees() {
        // Test various buffer sizes to ensure alignment is guaranteed
        for size in [0, 1, 2, 3, 4, 5, 7, 8, 15, 16, 32, 1024] {
            let test_data: Vec<u8> = (0..size).map(|i| u8::try_from(i % 256).unwrap()).collect();
            let message = ByteMessage::new(&test_data);
            let bytes = message.as_bytes();

            // Verify data integrity
            assert_eq!(
                bytes,
                &test_data[..],
                "Data integrity check failed for size {size}"
            );

            // Verify alignment - the internal buffer should be 4-byte aligned
            // We can't directly test the internal alignment, but we can test that
            // our bytemuck calls work without fallback by creating structures
            if bytes.len() >= 4 {
                // Try to parse as u32 - guaranteed aligned at offset 0
                let _test_u32 = *bytemuck::from_bytes::<u32>(&bytes[0..4]);
                // If we reach here, parsing is working correctly
            }
        }
    }

    #[test]
    fn test_measurement_gate_order_preservation() {
        // Test that measurement gate order is preserved
        let mut builder = ByteMessage::quantum_operations_builder();

        // Add measurements of different qubits in specific order
        builder.add_measurements(&[3]); // First: measure qubit 3
        builder.add_measurements(&[1]); // Second: measure qubit 1
        builder.add_measurements(&[4]); // Third: measure qubit 4
        builder.add_measurements(&[0]); // Fourth: measure qubit 0
        builder.add_measurements(&[2]); // Fifth: measure qubit 2

        let message = builder.build();

        // Parse operations back
        let operations = message.parse_quantum_operations().unwrap();

        // Verify we have 5 measurement operations in the correct order
        assert_eq!(operations.len(), 5);

        assert_eq!(operations[0].gate_type, GateType::Measure);
        assert_eq!(operations[0].qubits, vec![QubitId(3)]);

        assert_eq!(operations[1].gate_type, GateType::Measure);
        assert_eq!(operations[1].qubits, vec![QubitId(1)]);

        assert_eq!(operations[2].gate_type, GateType::Measure);
        assert_eq!(operations[2].qubits, vec![QubitId(4)]);

        assert_eq!(operations[3].gate_type, GateType::Measure);
        assert_eq!(operations[3].qubits, vec![QubitId(0)]);

        assert_eq!(operations[4].gate_type, GateType::Measure);
        assert_eq!(operations[4].qubits, vec![QubitId(2)]);
    }
}
