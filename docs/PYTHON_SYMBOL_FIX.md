# Python Symbol Conflict Fix

## Problem

When running tests from Python, the code was encountering segmentation faults due to symbol conflicts between:
1. The Python extension (`_pecos_rslib.abi3.so`) which has `__quantum__rt__*` and `__quantum__qis__*` symbols statically linked from the rlib
2. The dynamically loaded cdylib (`libpecos_qis_ffi.so`) which exports the same symbols with `RTLD_GLOBAL`

This caused symbol resolution conflicts and segfaults.

## Root Cause

The pecos-qis-selene executor was unconditionally loading `libpecos_qis_ffi.so` using `dlopen` with `RTLD_GLOBAL`, which made its symbols globally available. However, when running from Python, the Python extension already had these symbols statically linked, creating a conflict.

## Solution

### Architecture Changes

1. **pecos-qis-ffi Cargo.toml** (`crates/pecos-qis-ffi/Cargo.toml:14-18`)
   - Changed crate-type from `["rlib", "staticlib"]` to `["rlib", "cdylib"]`
   - The cdylib provides the `__quantum__*` symbols for dynamic loading by Rust binaries
   - The rlib provides the same functionality for static linking in the Python extension

2. **pecos-qis-selene Cargo.toml** (`crates/pecos-qis-selene/Cargo.toml:14-16`)
   - Changed crate-type from `["cdylib", "rlib"]` to just `["rlib"]`
   - The interface layer no longer needs to be a cdylib - only the FFI layer does

### Code Changes

#### 1. Added Helper Functions to pecos-qis-ffi (`crates/pecos-qis-ffi/src/lib.rs:140-148`)

```rust
/// Get a clone of the thread-local operation collector
pub fn get_interface_clone() -> OperationCollector {
    with_interface(|interface| interface.clone())
}

/// Set measurement results in the thread-local operation collector
pub fn set_measurements(measurements: HashMap<usize, bool>) {
    with_interface(|interface| interface.set_measurement_results(measurements));
}
```

These functions allow direct access to the rlib functionality without going through the FFI.

#### 2. Updated executor.rs - execute_program() Method (`crates/pecos-qis-selene/src/executor.rs:196-350`)

Added Python detection and conditional library loading:

```rust
// Detect if running from Python
let is_python = std::env::current_exe()
    .ok()
    .and_then(|exe| exe.file_name().map(|n| n.to_string_lossy().contains("python")))
    .unwrap_or(false);

// Load libpecos_qis_ffi.so (or skip if Python)
let pecos_qis_lib = if !is_python {
    // Running from Rust binary - dynamically load the cdylib
    Some(unsafe { Library::new(&pecos_qis_lib_path)? })
} else {
    // Running from Python - symbols already available, no need to load
    None
};

// Reset interface (use either cdylib or rlib)
if let Some(ref lib) = pecos_qis_lib {
    let reset_interface_fn: Symbol<ResetInterfaceFn> = unsafe {
        lib.get(b"pecos_qis_reset_interface\0")?
    };
    unsafe { reset_interface_fn() };
} else {
    pecos_qis_ffi::reset_interface();
}

// ... execute program ...

// Collect operations (use either cdylib or rlib)
let operations = if let Some(ref lib) = pecos_qis_lib {
    // Get from dynamically loaded cdylib
    let get_operations_fn: Symbol<GetOperationsFn> = unsafe {
        lib.get(b"pecos_qis_get_operations\0")?
    };
    let operations_ptr = unsafe { get_operations_fn() };
    let operations = unsafe { Box::from_raw(operations_ptr) };
    *operations
} else {
    // Get directly from rlib (Python case)
    pecos_qis_ffi::get_interface_clone()
};
```

#### 3. Updated executor.rs - execute_with_measurements() Method (`crates/pecos-qis-selene/src/executor.rs:403-556`)

Applied the same Python detection pattern:

```rust
// Detect if running from Python
let is_python = std::env::current_exe()
    .ok()
    .and_then(|exe| exe.file_name().map(|n| n.to_string_lossy().contains("python")))
    .unwrap_or(false);

// Conditionally load library
let pecos_qis_lib = if !is_python {
    Some(unsafe { Library::new(&pecos_qis_lib_path)? })
} else {
    None
};

// Set measurements (use either cdylib or rlib)
if let Some(ref lib) = pecos_qis_lib {
    let set_measurements_fn: Symbol<SetMeasurementsFn> = unsafe {
        lib.get(b"pecos_qis_set_measurements\0")?
    };
    let measurements_vec: Vec<(usize, bool)> = measurements.into_iter().collect();
    unsafe {
        set_measurements_fn(measurements_vec.as_ptr(), measurements_vec.len());
    }
} else {
    pecos_qis_ffi::set_measurements(measurements);
}

// ... execute program ...

// Collect operations (use either cdylib or rlib)
let operations = if let Some(ref lib) = pecos_qis_lib {
    // Get from cdylib
    let get_operations_fn: Symbol<GetOperationsFn> = unsafe {
        lib.get(b"pecos_qis_get_operations\0")?
    };
    let operations_ptr = unsafe { get_operations_fn() };
    let operations = unsafe { Box::from_raw(operations_ptr) };
    *operations
} else {
    // Get from rlib
    pecos_qis_ffi::get_interface_clone()
};
```

## How It Works

### When Running from Rust Binary

1. Executor detects it's NOT running from Python (`is_python = false`)
2. Dynamically loads `libpecos_qis_ffi.so` with `RTLD_GLOBAL`
3. Calls FFI functions through dlopen/libloading symbols
4. QIS programs can resolve `__quantum__*` symbols from the globally loaded cdylib

### When Running from Python

1. Executor detects it's running from Python (`is_python = true`)
2. Skips dynamic library loading (symbols already available in Python extension)
3. Calls rlib functions directly (same implementation, different linking)
4. QIS programs resolve `__quantum__*` symbols from the Python extension's statically linked symbols

## Benefits

1. **No Symbol Conflicts**: Python and Rust use different code paths, avoiding symbol conflicts
2. **Same Implementation**: Both paths use the same underlying Rust code (rlib vs cdylib are built from same source)
3. **Unified Architecture**: Single source of truth for QIS FFI symbols in `pecos-qis-ffi`
4. **Maintainable**: Changes to QIS interface automatically apply to both Python and Rust execution

## Testing

### Rust Tests
All 8 bell_state tests pass:
```
cargo test --test bell_state_tests --release
```

### Python Tests
All 9 HUGR integration tests pass:
```
uv run pytest python/pecos-rslib/tests/test_hugr_integration.py -v
```

The Python tests exercise the Guppy → HUGR → Helios → QIS pipeline, which is the primary use case for the Helios interface from Python.

## Future Improvements

1. More robust Python detection (e.g., check for Python in process name or use an environment variable)
2. Explicit configuration option to choose between cdylib and rlib paths
3. Potential unification with similar patterns in other interfaces (if any)
