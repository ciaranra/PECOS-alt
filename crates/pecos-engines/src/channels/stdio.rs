use super::{CommandChannel, Message, MessageChannel};
use crate::channels::CommandBatch;
use crate::errors::QueueError;
use log::debug;
use pecos_core::types::QuantumCommand;
use std::any::Any;
use std::io::{self, BufRead, BufReader, BufWriter, Write};
use std::sync::{Arc, Mutex};

#[derive(Clone)]
pub struct StdioChannel {
    reader: Arc<Mutex<Box<dyn BufRead + Send + Sync>>>,
    writer: Arc<Mutex<Box<dyn Write + Send + Sync>>>,
}

impl StdioChannel {
    #[must_use]
    pub fn new(
        reader: Box<dyn BufRead + Send + Sync>,
        writer: Box<dyn Write + Send + Sync>,
    ) -> Self {
        Self {
            reader: Arc::new(Mutex::new(reader)),
            writer: Arc::new(Mutex::new(writer)),
        }
    }

    pub fn send_measurement(&mut self, measurement: Message) -> Result<(), QueueError> {
        let mut writer = self
            .writer
            .lock()
            .map_err(|e| QueueError::LockError(format!("Failed to lock writer: {e}")))?;

        writeln!(*writer, "MEAS {measurement}")?;
        writer.flush()?;
        Ok(())
    }

    /// Creates a new `StdioChannel` that uses the standard input (`stdin`) and output (`stdout`) as its reader and writer.
    ///
    /// This method constructs the channel by wrapping `stdin` in a `BufReader` and `stdout` in a `BufWriter`,
    /// ensuring efficient reading and writing.
    ///
    /// # Returns
    /// - On success, returns an instance of `StdioChannel`.
    /// - On failure, returns a `std::io::Error`, which might occur while initializing the I/O streams.
    ///
    /// # Errors
    /// This function returns a `std::io::Error` if:
    /// - An error occurs while accessing the standard input or output streams.
    /// - System-level I/O errors prevent the construction of the channel.
    ///
    /// # Examples
    /// ```
    /// use pecos_engines::channels::stdio::StdioChannel;
    /// let channel = StdioChannel::from_stdio().expect("Failed to create StdioChannel from stdio");
    /// ```
    pub fn from_stdio() -> io::Result<Self> {
        Ok(Self {
            reader: Arc::new(Mutex::new(Box::new(BufReader::new(io::stdin())))),
            writer: Arc::new(Mutex::new(Box::new(BufWriter::new(io::stdout())))),
        })
    }

    /// Creates a `StdioChannel` instance with an anonymous pipe for testing or short-lived communication.
    ///
    /// This method sets up a pair of connected reader and writer pipes using `os_pipe`,
    /// wrapping the reader in a `BufReader` and the writer in a `BufWriter` for buffered I/O operations.
    ///
    /// # Returns
    /// - On success, returns a fully initialized `StdioChannel`.
    /// - On failure, returns an `std::io::Error` if the pipe creation fails.
    ///
    /// # Errors
    /// This function returns a `std::io::Error` if:
    /// - The operating system fails to create the anonymous pipe.
    /// - There is an error during initialization of the reader or writer.
    ///
    /// # Examples
    /// ```
    /// use pecos_engines::channels::stdio::StdioChannel;
    /// let channel = StdioChannel::create_for_shot().expect("Failed to create channel for shot");
    /// ```
    pub fn create_for_shot() -> io::Result<Self> {
        use os_pipe::pipe;
        let (reader, writer) = pipe()?;

        Ok(Self {
            reader: Arc::new(Mutex::new(Box::new(BufReader::new(reader)))),
            writer: Arc::new(Mutex::new(Box::new(BufWriter::new(writer)))),
        })
    }
}

impl CommandChannel for StdioChannel {
    fn send_batch(&mut self, batch: &CommandBatch) -> Result<(), QueueError> {
        let mut writer = self
            .writer
            .lock()
            .map_err(|e| QueueError::LockError(format!("Failed to lock writer: {e}")))?;

        debug!(
            "Command channel beginning batch of {} commands",
            batch.len()
        );
        writeln!(*writer, "FLUSH_BEGIN")?;

        for cmd in batch.commands() {
            let cmd_str = format_command(cmd);
            debug!("Command channel writing: 'CMD {}'", cmd_str);
            writeln!(*writer, "CMD {cmd_str}")?;
        }

        writeln!(*writer, "FLUSH_END")?;
        writer.flush()?;
        Ok(())
    }

    fn receive_batch(&mut self) -> Result<Option<CommandBatch>, QueueError> {
        let mut reader = self
            .reader
            .lock()
            .map_err(|e| QueueError::LockError(format!("Failed to lock reader: {e}")))?;

        let mut batch = CommandBatch::new();
        let mut line = String::new();

        // Wait for batch start
        while let Ok(len) = reader.read_line(&mut line) {
            if len == 0 || line.trim() == "FLUSH_BEGIN" {
                break;
            }
            line.clear();
        }

        // Read commands until batch end
        loop {
            line.clear();
            let len = reader.read_line(&mut line)?;
            if len == 0 {
                debug!("Command channel EOF");
                return if batch.is_empty() {
                    Ok(None)
                } else {
                    Ok(Some(batch))
                };
            }

            let trimmed = line.trim();
            match trimmed {
                "FLUSH_END" => {
                    debug!(
                        "Command channel batch complete with {} commands",
                        batch.len()
                    );
                    return Ok(Some(batch));
                }
                cmd_str if cmd_str.starts_with("CMD ") => {
                    if let Ok(cmd) = QuantumCommand::parse_from_str(&cmd_str[4..]) {
                        batch.add_command(cmd);
                    }
                }
                _ => debug!("Command channel skipping line: {}", trimmed),
            }
        }
    }

    /// Flushes any remaining data in the writer, ensuring it is written out.
    ///
    /// # Errors
    /// This function returns a `QueueError` if:
    /// - There is an error locking the writer.
    /// - The flush operation fails for any reason.
    fn flush(&mut self) -> Result<(), QueueError> {
        debug!("Command channel flushing");
        let mut writer = self
            .writer
            .lock()
            .map_err(|e| QueueError::LockError(format!("Failed to lock writer: {e}")))?;

        writeln!(*writer, "END_COMMANDS")?;
        writer.flush()?;
        debug!("Command channel flushed");
        Ok(())
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl MessageChannel for StdioChannel {
    fn send_measurement(&mut self, measurement: Message) -> Result<(), QueueError> {
        let result_id = measurement >> 16;
        let outcome = measurement & 0xFFFF;

        let mut writer = self
            .writer
            .lock()
            .map_err(|e| QueueError::LockError(format!("Failed to lock writer: {e}")))?;

        debug!(
            "Measurement channel writing measurement: encoded={}, result_id={}, outcome={}",
            measurement, result_id, outcome
        );
        writeln!(*writer, "MEAS {measurement}")?;
        writer.flush()?;
        Ok(())
    }

    /// Receives a message (measurement) from the channel.
    ///
    /// This method tries to read a line of input, parses it into a `Message` (u32),
    /// and returns the result.
    ///
    /// # Errors
    /// This function returns a `QueueError` if:
    /// - There is an error locking the reader.
    /// - The operation fails to read a line from the reader.
    /// - The parsed measurement is invalid (not a valid `u32`).
    fn receive_message(&mut self) -> Result<Option<Message>, QueueError> {
        debug!("Measurement channel reading");
        let mut reader = self
            .reader
            .lock()
            .map_err(|e| QueueError::LockError(format!("Failed to lock reader: {e}")))?;

        let mut line = String::new();
        loop {
            line.clear();
            let bytes_read = reader.read_line(&mut line)?;
            if bytes_read == 0 {
                debug!("Measurement channel EOF");
                return Ok(None);
            }

            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }

            if let Some(meas_str) = trimmed.strip_prefix("MEAS ") {
                debug!("Measurement channel parsing: '{}'", meas_str);
                match meas_str.parse::<Message>() {
                    Ok(measurement) => {
                        let result_id = measurement >> 16;
                        let outcome = measurement & 0xFFFF;
                        debug!(
                            "Measurement channel parsed: encoded={}, result_id={}, outcome={}",
                            measurement, result_id, outcome
                        );
                        return Ok(Some(measurement));
                    }
                    Err(e) => {
                        debug!("Measurement channel parse error: {} for '{}'", e, meas_str);
                        return Err(QueueError::OperationError(format!(
                            "Failed to parse measurement '{meas_str}': {e}"
                        )));
                    }
                }
            }
            debug!("Measurement channel skipping non-measurement line");
        }
    }

    fn flush(&mut self) -> Result<(), QueueError> {
        debug!("Measurement channel flushing");
        let mut writer = self
            .writer
            .lock()
            .map_err(|e| QueueError::LockError(format!("Failed to lock writer: {e}")))?;

        writeln!(*writer, "END_MEASUREMENTS")?;
        writer.flush()?;
        debug!("Measurement channel flushed");
        Ok(())
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

#[must_use]
pub fn format_command(cmd: &QuantumCommand) -> String {
    use pecos_core::types::GateType::{CX, H, Measure, R1XY, RZ, SZZ};

    match &cmd.gate {
        RZ { theta } => format!("RZ {} {}", theta, cmd.qubits[0]),
        R1XY { phi, theta } => format!("R1XY {} {} {}", phi, theta, cmd.qubits[0]),
        SZZ => format!("SZZ {} {}", cmd.qubits[0], cmd.qubits[1]),
        H => format!("H {}", cmd.qubits[0]),
        CX => format!("CX {} {}", cmd.qubits[0], cmd.qubits[1]),
        Measure { result_id } => format!("M {} {}", cmd.qubits[0], result_id),
    }
}
