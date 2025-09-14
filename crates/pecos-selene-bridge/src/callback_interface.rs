/// Callback interface for `ByteMessage` communication between PECOS and Selene
///
/// This module provides FFI-safe functions that allow the Bridge simulator
/// (running inside Selene executable) to communicate with PECOS via callbacks.
use pecos_engines::ByteMessage;
use std::collections::VecDeque;
use std::sync::Mutex;

/// Global state for managing the callback communication
pub(crate) static CALLBACK_STATE: Mutex<Option<CallbackState>> = Mutex::new(None);

/// State that manages the `ByteMessage` queues and synchronization
pub(crate) struct CallbackState {
    /// Queue of operations from Bridge to PECOS (H, CNOT, etc.)
    outgoing_operations: VecDeque<ByteMessage>,

    /// Queue of measurements from PECOS to Bridge
    incoming_measurements: VecDeque<ByteMessage>,

    /// Indicates if Bridge is waiting for measurements
    waiting_for_measurements: bool,

    /// Indicates if the program has completed
    execution_complete: bool,
}

/// Initialize the callback state
///
/// # Panics
///
/// Panics if the mutex is poisoned
pub fn pecos_bridge_init() {
    let mut state = CALLBACK_STATE.lock().unwrap();
    *state = Some(CallbackState {
        outgoing_operations: VecDeque::new(),
        incoming_measurements: VecDeque::new(),
        waiting_for_measurements: false,
        execution_complete: false,
    });
    log::debug!("[Bridge] Callback interface initialized");
}

/// Bridge simulator calls this to send quantum operations to PECOS
/// Returns 0 on success, -1 on error
///
/// # Safety
///
/// The caller must ensure that `data` points to a valid memory region of at least `len` bytes
///
/// # Panics
///
/// Panics if the mutex is poisoned
pub unsafe fn pecos_bridge_send_operations(data: *const u8, len: usize) -> i32 {
    if data.is_null() {
        return -1;
    }

    let bytes = unsafe { std::slice::from_raw_parts(data, len) };
    let message = ByteMessage::new(bytes);

    let mut state = CALLBACK_STATE.lock().unwrap();
    if let Some(ref mut state) = *state {
        state.outgoing_operations.push_back(message);
        log::trace!("[Bridge] Queued operations ByteMessage ({len} bytes)");
        0
    } else {
        log::error!("[Bridge] Callback state not initialized");
        -1
    }
}

/// Bridge simulator calls this to request measurement results
/// Returns number of bytes written, 0 if no measurements available, -1 on error
///
/// # Safety
///
/// The caller must ensure that `data_out` points to a valid writable memory region of at least `max_len` bytes
///
/// # Panics
///
/// Panics if the mutex is poisoned
pub unsafe fn pecos_bridge_receive_measurements(data_out: *mut u8, max_len: usize) -> i32 {
    let mut state = CALLBACK_STATE.lock().unwrap();
    if let Some(ref mut state) = *state {
        // Check if measurements are available
        if let Some(message) = state.incoming_measurements.pop_front() {
            let bytes = message.as_bytes();
            if bytes.len() > max_len {
                log::error!("[Bridge] Buffer too small for measurements");
                // Put it back
                state.incoming_measurements.push_front(message);
                return -1;
            }

            // Copy to output buffer
            unsafe { std::ptr::copy_nonoverlapping(bytes.as_ptr(), data_out, bytes.len()) };

            log::trace!("[Bridge] Returned measurements ({} bytes)", bytes.len());
            state.waiting_for_measurements = false;
            i32::try_from(bytes.len()).unwrap_or(i32::MAX)
        } else {
            // No measurements available yet - Bridge should wait
            state.waiting_for_measurements = true;
            log::debug!("[Bridge] No measurements available, waiting...");
            0
        }
    } else {
        log::error!("[Bridge] Callback state not initialized");
        -1
    }
}

/// Bridge simulator calls this to signal it's waiting for measurements
///
/// # Panics
///
/// Panics if the mutex is poisoned
pub fn pecos_bridge_wait_for_measurements() {
    let mut state = CALLBACK_STATE.lock().unwrap();
    if let Some(ref mut state) = *state {
        state.waiting_for_measurements = true;
        log::debug!("[Bridge] Signaled waiting for measurements");
    }
}

/// Bridge simulator calls this to signal execution is complete
///
/// # Panics
///
/// Panics if the mutex is poisoned
pub fn pecos_bridge_signal_complete() {
    let mut state = CALLBACK_STATE.lock().unwrap();
    if let Some(ref mut state) = *state {
        state.execution_complete = true;
        log::debug!("[Bridge] Signaled execution complete");
    }
}

// ============================================================================
// Functions for PECOS Engine to call
// ============================================================================

/// PECOS calls this to get pending operations from the Bridge
/// Returns None if no operations available
///
/// # Panics
///
/// Panics if the mutex is poisoned
pub fn pecos_get_pending_operations() -> Option<ByteMessage> {
    let mut state = CALLBACK_STATE.lock().unwrap();
    if let Some(ref mut state) = *state {
        state.outgoing_operations.pop_front()
    } else {
        None
    }
}

/// PECOS calls this to provide measurement results to the Bridge
///
/// # Panics
///
/// Panics if the mutex is poisoned
pub fn pecos_provide_measurements(message: ByteMessage) {
    let mut state = CALLBACK_STATE.lock().unwrap();
    if let Some(ref mut state) = *state {
        state.incoming_measurements.push_back(message);
        log::trace!("[PECOS] Provided measurements to Bridge");
    }
}

/// PECOS calls this to check if Bridge is waiting for measurements
///
/// # Panics
///
/// Panics if the mutex is poisoned
pub fn pecos_is_bridge_waiting() -> bool {
    let state = CALLBACK_STATE.lock().unwrap();
    if let Some(ref state) = *state {
        state.waiting_for_measurements
    } else {
        false
    }
}

/// PECOS calls this to check if execution is complete
///
/// # Panics
///
/// Panics if the mutex is poisoned
pub fn pecos_is_execution_complete() -> bool {
    let state = CALLBACK_STATE.lock().unwrap();
    if let Some(ref state) = *state {
        state.execution_complete
    } else {
        false
    }
}

/// Reset the callback state for a new shot
///
/// # Panics
///
/// Panics if the mutex is poisoned
pub fn pecos_reset_callback_state() {
    let mut state = CALLBACK_STATE.lock().unwrap();
    if let Some(ref mut state) = *state {
        state.outgoing_operations.clear();
        state.incoming_measurements.clear();
        state.waiting_for_measurements = false;
        state.execution_complete = false;
        log::debug!("[PECOS] Reset callback state for new shot");
    }
}
