/// Global lock for QIR tests to ensure they run sequentially
///
/// This module provides a file-based lock that works across different test binaries
/// to ensure QIR tests don't run concurrently and cause race conditions.
use std::fs::{File, OpenOptions};
use std::io::ErrorKind;
use std::path::PathBuf;
use std::time::Duration;

const MAX_RETRIES: u32 = 600; // 60 seconds total to handle test load
const RETRY_DELAY_MS: u64 = 100;

pub struct QirTestLock {
    _file: File,
    path: PathBuf,
}

impl QirTestLock {
    /// Acquire the QIR test lock
    ///
    /// # Panics
    ///
    /// Panics if:
    /// - Failed to create the lock file due to an unexpected error
    /// - Failed to acquire the lock after maximum retries
    #[must_use]
    pub fn acquire() -> Self {
        // Use target directory for lock file to avoid /tmp issues
        let lock_dir = std::env::var("CARGO_TARGET_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|_| {
                // Find the workspace root by looking for Cargo.lock
                let mut current = std::env::current_dir().unwrap();
                loop {
                    if current.join("Cargo.lock").exists() {
                        break current.join("target");
                    }
                    if !current.pop() {
                        // Fallback to current directory
                        break PathBuf::from("target");
                    }
                }
            });
        
        // Ensure directory exists
        let _ = std::fs::create_dir_all(&lock_dir);
        let lock_path = lock_dir.join("pecos_qir_test.lock");

        // Try to acquire lock with retries

        for attempt in 0..MAX_RETRIES {
            match OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&lock_path)
            {
                Ok(file) => {
                    eprintln!("Acquired QIR test lock");
                    return Self {
                        _file: file,
                        path: lock_path,
                    };
                }
                Err(e) if e.kind() == ErrorKind::AlreadyExists => {
                    if attempt == 0 {
                        eprintln!("Waiting for QIR test lock...");
                    }
                    std::thread::sleep(Duration::from_millis(RETRY_DELAY_MS));
                }
                Err(e) => {
                    panic!("Failed to create QIR test lock file: {e}");
                }
            }
        }

        panic!(
            "Failed to acquire QIR test lock after {} seconds",
            u64::from(MAX_RETRIES) * RETRY_DELAY_MS / 1000
        );
    }
}

impl Drop for QirTestLock {
    fn drop(&mut self) {
        eprintln!("Releasing QIR test lock");
        let _ = std::fs::remove_file(&self.path);
        // Add a small delay to ensure file system updates
        std::thread::sleep(Duration::from_millis(50));
    }
}
