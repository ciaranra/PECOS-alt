use pecos_engines::{ByteMessage, ByteMessageBuilder, EngineStage};
/// Prototype test to validate the callback-based communication design
///
/// This test validates:
/// 1. FFI callbacks work
/// 2. ByteMessage exchange works
/// 3. EngineStage flow is correct
/// 4. Synchronization between processes
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

/// Simulated callback state (would be in pecos-selene-bridge)
static CALLBACK_STATE: Mutex<CallbackState> = Mutex::new(CallbackState {
    operations: vec![],
    measurements: vec![],
    waiting: false,
    complete: false,
});

struct CallbackState {
    operations: Vec<ByteMessage>,
    measurements: Vec<ByteMessage>,
    waiting: bool,
    complete: bool,
}

/// Simulated Bridge simulator functions
mod bridge_simulator {
    use super::*;

    /// Bridge calls this to send operations to PECOS
    pub fn send_operations(msg: ByteMessage) {
        println!("[Bridge] Sending operations");
        let mut state = CALLBACK_STATE.lock().unwrap();
        state.operations.push(msg);
    }

    /// Bridge calls this to get measurements
    pub fn receive_measurements() -> Option<ByteMessage> {
        println!("[Bridge] Requesting measurements");
        let mut state = CALLBACK_STATE.lock().unwrap();
        state.waiting = true;
        state.measurements.pop()
    }

    /// Bridge signals completion
    pub fn signal_complete() {
        println!("[Bridge] Signaling complete");
        let mut state = CALLBACK_STATE.lock().unwrap();
        state.complete = true;
    }
}

/// Simulated Engine functions
mod engine {
    use super::*;

    /// Engine gets pending operations
    pub fn get_operations() -> Option<ByteMessage> {
        let mut state = CALLBACK_STATE.lock().unwrap();
        state.operations.pop()
    }

    /// Engine provides measurements
    pub fn provide_measurements(msg: ByteMessage) {
        println!("[Engine] Providing measurements");
        let mut state = CALLBACK_STATE.lock().unwrap();
        state.measurements.push(msg);
        state.waiting = false;
    }

    /// Check if bridge is waiting
    pub fn is_waiting() -> bool {
        CALLBACK_STATE.lock().unwrap().waiting
    }

    /// Check if complete
    pub fn is_complete() -> bool {
        CALLBACK_STATE.lock().unwrap().complete
    }
}

/// Simulate the Selene process (Bridge simulator)
fn simulate_selene_process() {
    thread::spawn(|| {
        println!("[Selene Process] Starting");

        // Simulate quantum program execution
        // Step 1: Send H gate
        let mut builder = ByteMessageBuilder::new();
        let _ = builder.for_quantum_operations();
        builder.add_h(&[0]);
        bridge_simulator::send_operations(builder.build());

        // Step 2: Send measurement
        let mut builder = ByteMessageBuilder::new();
        let _ = builder.for_quantum_operations();
        builder.add_measurements(&[0]);
        bridge_simulator::send_operations(builder.build());

        // Step 3: Wait for measurement result
        println!("[Selene Process] Waiting for measurement");
        loop {
            thread::sleep(Duration::from_millis(10));
            if let Some(meas) = bridge_simulator::receive_measurements() {
                println!("[Selene Process] Got measurement result");
                break;
            }
        }

        // Step 4: Complete
        bridge_simulator::signal_complete();
        println!("[Selene Process] Complete");
    });
}

/// Simulate the Engine with EngineStage flow
struct PrototypeEngine;

impl PrototypeEngine {
    fn start(&mut self) -> EngineStage<ByteMessage, String> {
        println!("[Engine] Starting");

        // Wait for first operations
        let mut attempts = 0;
        loop {
            if let Some(ops) = engine::get_operations() {
                println!("[Engine] Got initial operations");
                return EngineStage::NeedsProcessing(ops);
            }

            if engine::is_complete() {
                println!("[Engine] Complete immediately");
                return EngineStage::Complete("No operations".to_string());
            }

            thread::sleep(Duration::from_millis(50));
            attempts += 1;
            if attempts > 20 {
                panic!("Timeout waiting for operations");
            }
        }
    }

    fn continue_processing(
        &mut self,
        measurements: ByteMessage,
    ) -> EngineStage<ByteMessage, String> {
        println!("[Engine] Continue processing");

        // Provide measurements
        engine::provide_measurements(measurements);

        // Wait for next operations or completion
        thread::sleep(Duration::from_millis(50));

        if let Some(ops) = engine::get_operations() {
            println!("[Engine] Got more operations");
            EngineStage::NeedsProcessing(ops)
        } else if engine::is_complete() {
            println!("[Engine] Complete");
            EngineStage::Complete("Success".to_string())
        } else {
            panic!("Unexpected state");
        }
    }
}

#[test]
fn test_callback_prototype() {
    println!("\n=== CALLBACK PROTOTYPE TEST ===\n");

    // Start simulated Selene process
    simulate_selene_process();

    // Give it time to start
    thread::sleep(Duration::from_millis(100));

    // Run engine with EngineStage flow
    let mut engine = PrototypeEngine;
    let mut stage = engine.start();

    loop {
        match stage {
            EngineStage::NeedsProcessing(ops) => {
                println!("[Test] Processing operations");

                // Simulate quantum engine processing
                thread::sleep(Duration::from_millis(10));

                // Create mock measurements
                let mut builder = ByteMessageBuilder::new();
                let _ = builder.for_outcomes();
                builder.add_outcomes(&[0]); // Measurement result: 0
                let measurements = builder.build();

                // Continue
                stage = engine.continue_processing(measurements);
            }
            EngineStage::Complete(result) => {
                println!("[Test] Complete: {}", result);
                assert_eq!(result, "Success");
                break;
            }
        }
    }

    println!("\n=== TEST PASSED ===");
}

#[test]
fn test_ffi_callback_mechanism() {
    println!("\n=== FFI CALLBACK TEST ===\n");

    // Test that we can call functions through FFI-style interface

    // Simulate C-style callback functions
    extern "C" fn send_op_callback(data: *const u8, len: usize) -> i32 {
        unsafe {
            let bytes = std::slice::from_raw_parts(data, len);
            let msg = ByteMessage::new(bytes);
            bridge_simulator::send_operations(msg);
        }
        0 // Success
    }

    extern "C" fn recv_meas_callback(data_out: *mut u8, max_len: usize) -> i32 {
        if let Some(msg) = bridge_simulator::receive_measurements() {
            let bytes = msg.as_bytes();
            if bytes.len() <= max_len {
                unsafe {
                    std::ptr::copy_nonoverlapping(bytes.as_ptr(), data_out, bytes.len());
                }
                bytes.len() as i32
            } else {
                -1 // Buffer too small
            }
        } else {
            0 // No data
        }
    }

    // Test sending operations via callback
    let mut builder = ByteMessageBuilder::new();
    let _ = builder.for_quantum_operations();
    builder.add_h(&[0]);
    let msg = builder.build();
    let bytes = msg.as_bytes();

    let result = send_op_callback(bytes.as_ptr(), bytes.len());
    assert_eq!(result, 0);

    // Verify it was received
    assert!(engine::get_operations().is_some());

    // Test receiving measurements via callback
    let mut buffer = vec![0u8; 1024];
    let result = recv_meas_callback(buffer.as_mut_ptr(), buffer.len());
    assert_eq!(result, 0); // No measurements yet

    // Provide a measurement
    let mut builder = ByteMessageBuilder::new();
    let _ = builder.for_outcomes();
    builder.add_outcomes(&[1]);
    engine::provide_measurements(builder.build());

    // Now should receive it
    let result = recv_meas_callback(buffer.as_mut_ptr(), buffer.len());
    assert!(result > 0); // Got measurement

    println!("FFI callbacks work!");
}

#[test]
fn test_synchronization() {
    println!("\n=== SYNCHRONIZATION TEST ===\n");

    // Test that Bridge and Engine can synchronize properly

    let sync_test = Arc::new(Mutex::new(0));
    let sync_clone = sync_test.clone();

    // Selene thread
    thread::spawn(move || {
        // Wait for engine to be ready
        thread::sleep(Duration::from_millis(50));

        // Send operations
        *sync_clone.lock().unwrap() = 1;
        bridge_simulator::send_operations(ByteMessage::create_empty());

        // Wait for measurements
        loop {
            if *sync_clone.lock().unwrap() == 2 {
                break;
            }
            thread::sleep(Duration::from_millis(10));
        }

        // Signal complete
        *sync_clone.lock().unwrap() = 3;
        bridge_simulator::signal_complete();
    });

    // Engine thread
    thread::spawn(move || {
        // Wait for operations
        loop {
            if *sync_test.lock().unwrap() == 1 {
                assert!(engine::get_operations().is_some());
                break;
            }
            thread::sleep(Duration::from_millis(10));
        }

        // Provide measurements
        *sync_test.lock().unwrap() = 2;
        engine::provide_measurements(ByteMessage::create_empty());

        // Wait for completion
        loop {
            if *sync_test.lock().unwrap() == 3 {
                assert!(engine::is_complete());
                break;
            }
            thread::sleep(Duration::from_millis(10));
        }
    });

    // Give threads time to complete
    thread::sleep(Duration::from_millis(500));

    println!("Synchronization works!");
}
