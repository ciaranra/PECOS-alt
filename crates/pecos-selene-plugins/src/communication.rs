//! Communication channel between `ByteMessageSimulator` and PECOS
//!
//! This module provides a simple file-based communication mechanism
//! for passing `ByteMessages` between the Selene runtime plugin and PECOS.

use anyhow::{Result, anyhow};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

/// Communication channel using files for message passing
pub struct FileChannel {
    /// File for operations (runtime -> PECOS)
    operations: PathBuf,
    /// File for results (PECOS -> runtime)
    results: PathBuf,
    /// File for control messages
    control: PathBuf,
}

impl FileChannel {
    /// Create a new file channel in the specified directory
    ///
    /// # Errors
    ///
    /// Returns an error if the directory cannot be created
    pub fn new(comm_dir: impl AsRef<Path>) -> Result<Self> {
        let comm_dir = comm_dir.as_ref().to_path_buf();

        // Create directory if it doesn't exist
        fs::create_dir_all(&comm_dir)?;

        Ok(Self {
            operations: comm_dir.join("operations.bin"),
            results: comm_dir.join("results.bin"),
            control: comm_dir.join("control.txt"),
        })
    }

    /// Send operations data (from runtime to PECOS)
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be written
    pub fn send_operations(&self, data: &[u8]) -> Result<()> {
        let mut file = fs::File::create(&self.operations)?;
        file.write_all(data)?;
        file.sync_all()?;

        // Signal that operations are ready
        fs::write(&self.control, "OPERATIONS_READY")?;

        Ok(())
    }

    /// Receive operations data (PECOS side)
    ///
    /// # Errors
    ///
    /// Returns an error if the signal times out or file cannot be read
    pub fn receive_operations(&self) -> Result<Vec<u8>> {
        // Wait for signal
        self.wait_for_signal("OPERATIONS_READY")?;

        // Read operations
        let data = fs::read(&self.operations)?;

        // Clear signal
        fs::write(&self.control, "OPERATIONS_RECEIVED")?;

        Ok(data)
    }

    /// Send results data (from PECOS to runtime)
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be written
    pub fn send_results(&self, data: &[u8]) -> Result<()> {
        let mut file = fs::File::create(&self.results)?;
        file.write_all(data)?;
        file.sync_all()?;

        // Signal that results are ready
        fs::write(&self.control, "RESULTS_READY")?;

        Ok(())
    }

    /// Receive results data (runtime side)
    ///
    /// # Errors
    ///
    /// Returns an error if the signal times out or file cannot be read
    pub fn receive_results(&self) -> Result<Vec<u8>> {
        // Wait for signal
        self.wait_for_signal("RESULTS_READY")?;

        // Read results
        let data = fs::read(&self.results)?;

        // Clear signal
        fs::write(&self.control, "RESULTS_RECEIVED")?;

        Ok(data)
    }

    /// Wait for a specific signal in the control file
    fn wait_for_signal(&self, expected: &str) -> Result<()> {
        let timeout = std::time::Duration::from_secs(10);
        let start = std::time::Instant::now();

        loop {
            if let Ok(content) = fs::read_to_string(&self.control)
                && content.trim() == expected
            {
                return Ok(());
            }

            if start.elapsed() > timeout {
                return Err(anyhow!("Timeout waiting for signal: {}", expected));
            }

            std::thread::sleep(std::time::Duration::from_millis(10));
        }
    }

    /// Clean up communication files
    ///
    /// # Errors
    ///
    /// This function always returns Ok - file removal errors are ignored
    pub fn cleanup(&self) -> Result<()> {
        let _ = fs::remove_file(&self.operations);
        let _ = fs::remove_file(&self.results);
        let _ = fs::remove_file(&self.control);
        Ok(())
    }
}

/// Environment variable to specify communication directory
pub const COMM_DIR_ENV: &str = "PECOS_SELENE_COMM_DIR";
