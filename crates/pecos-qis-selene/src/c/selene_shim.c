/**
 * PECOS Selene Runtime Shim
 *
 * This C library implements the selene_* API and forwards calls to PECOS's
 * thread-local QIS interface for operation collection.
 *
 * Architecture:
 *   program.x → ___qalloc() [from libhelios.a]
 *             → selene_qalloc() [from this shim]
 *             → pecos_collect_operation() [calls Rust FFI]
 *             → pecos_qis_interface::with_interface()
 */

#include <stdint.h>
#include <stdbool.h>
#include <stdio.h>
#include <string.h>

// Selene API types (matching selene.h)
typedef struct SeleneInstance {
    int dummy;  // Opaque struct - we don't use it
} SeleneInstance;

typedef struct {
    uint32_t error_code;
} selene_void_result_t;

typedef struct {
    uint32_t error_code;
    uint64_t value;
} selene_u64_result_t;

typedef struct {
    uint32_t error_code;
    uint32_t value;
} selene_u32_result_t;

typedef struct {
    uint32_t error_code;
    double value;
} selene_f64_result_t;

typedef struct {
    uint32_t error_code;
    bool value;
} selene_bool_result_t;

typedef struct {
    uint32_t error_code;
    uint64_t reference;
} selene_future_result_t;

typedef struct {
    const char *data;
    uint64_t length;
    bool owned;
} selene_string_t;

// =============================================================================
// Forward declarations of PECOS FFI functions
// These will be provided by the Rust pecos-qis-interface crate
// =============================================================================

// These functions are implemented in pecos-qis-interface/src/ffi.rs
// and exported with #[unsafe(no_mangle)]
extern int64_t __quantum__rt__qubit_allocate(void);
extern void __quantum__rt__qubit_release(int64_t qubit);
extern void __quantum__qis__rxy__body(double theta, double phi, int64_t qubit);
extern void __quantum__qis__rz__body(double theta, int64_t qubit);
extern void __quantum__qis__rzz__body(double theta, int64_t qubit1, int64_t qubit2);
extern void __quantum__qis__reset__body(int64_t qubit);
extern int32_t __quantum__qis__m__body(int64_t qubit, int64_t result);
extern int64_t __quantum__rt__result_allocate(void);

// Note: For lazy measurement and future operations, we need special handling
// since PECOS doesn't have native support yet. For now, we'll use placeholders.

// =============================================================================
// Helper macros
// =============================================================================

#define SUCCESS(type) ((type){.error_code = 0})
#define SUCCESS_VAL(type, val) ((type){.error_code = 0, .value = val})
#define SUCCESS_REF(type, ref) ((type){.error_code = 0, .reference = ref})

// =============================================================================
// Qubit allocation and deallocation
// =============================================================================

selene_u64_result_t selene_qalloc(SeleneInstance *instance) {
    (void)instance;  // Unused - we use thread-local storage
    int64_t qubit_id = __quantum__rt__qubit_allocate();
    return SUCCESS_VAL(selene_u64_result_t, (uint64_t)qubit_id);
}

selene_void_result_t selene_qfree(SeleneInstance *instance, uint64_t q) {
    (void)instance;
    __quantum__rt__qubit_release((int64_t)q);
    return SUCCESS(selene_void_result_t);
}

// =============================================================================
// Quantum gates
// =============================================================================

selene_void_result_t selene_rxy(SeleneInstance *instance, uint64_t q, double theta, double phi) {
    (void)instance;
    // Note: pecos-qis-interface uses r1xy which takes (theta, phi, qubit)
    // We need to check the signature - looking at ffi.rs it's:
    // pub unsafe extern "C" fn __quantum__qis__r1xy__body(theta: f64, phi: f64, qubit: i64)
    extern void __quantum__qis__r1xy__body(double theta, double phi, int64_t qubit);
    __quantum__qis__r1xy__body(theta, phi, (int64_t)q);
    return SUCCESS(selene_void_result_t);
}

selene_void_result_t selene_rz(SeleneInstance *instance, uint64_t q, double theta) {
    (void)instance;
    __quantum__qis__rz__body(theta, (int64_t)q);
    return SUCCESS(selene_void_result_t);
}

selene_void_result_t selene_rzz(SeleneInstance *instance, uint64_t q1, uint64_t q2, double theta) {
    (void)instance;
    __quantum__qis__rzz__body(theta, (int64_t)q1, (int64_t)q2);
    return SUCCESS(selene_void_result_t);
}

selene_void_result_t selene_qubit_reset(SeleneInstance *instance, uint64_t q) {
    (void)instance;
    __quantum__qis__reset__body((int64_t)q);
    return SUCCESS(selene_void_result_t);
}

// =============================================================================
// Measurement
// =============================================================================

selene_bool_result_t selene_qubit_measure(SeleneInstance *instance, uint64_t q) {
    (void)instance;
    // For immediate measurement, we allocate a result and measure
    int64_t result_id = __quantum__rt__result_allocate();
    int32_t result = __quantum__qis__m__body((int64_t)q, result_id);
    return (selene_bool_result_t){.error_code = 0, .value = (bool)result};
}

selene_future_result_t selene_qubit_lazy_measure(SeleneInstance *instance, uint64_t q) {
    (void)instance;
    // For lazy measurement, we allocate a result ID and queue the measurement
    // The actual measurement result will be retrieved later
    int64_t result_id = __quantum__rt__result_allocate();
    __quantum__qis__m__body((int64_t)q, result_id);
    // Return the result ID as the future reference
    return SUCCESS_REF(selene_future_result_t, (uint64_t)result_id);
}

selene_future_result_t selene_qubit_lazy_measure_leaked(SeleneInstance *instance, uint64_t q) {
    // Same as lazy_measure for now
    return selene_qubit_lazy_measure(instance, q);
}

// =============================================================================
// Future operations
// =============================================================================

selene_bool_result_t selene_future_read_bool(SeleneInstance *instance, uint64_t r) {
    (void)instance;
    // Read the measurement result
    // We need a function to retrieve stored results
    extern int32_t __quantum__rt__result_get_one(int64_t result);
    int32_t value = __quantum__rt__result_get_one((int64_t)r);
    return (selene_bool_result_t){.error_code = 0, .value = (bool)value};
}

selene_u64_result_t selene_future_read_u64(SeleneInstance *instance, uint64_t r) {
    (void)instance;
    // For now, treat as bool and convert to u64
    extern int32_t __quantum__rt__result_get_one(int64_t result);
    int32_t value = __quantum__rt__result_get_one((int64_t)r);
    return SUCCESS_VAL(selene_u64_result_t, (uint64_t)value);
}

selene_void_result_t selene_refcount_increment(SeleneInstance *instance, uint64_t r) {
    (void)instance;
    (void)r;
    // No-op for PECOS - we don't do refcounting
    return SUCCESS(selene_void_result_t);
}

selene_void_result_t selene_refcount_decrement(SeleneInstance *instance, uint64_t r) {
    (void)instance;
    (void)r;
    // No-op for PECOS - we don't do refcounting
    return SUCCESS(selene_void_result_t);
}

// =============================================================================
// Print operations (for debug/output)
// =============================================================================

selene_void_result_t selene_print_bool(SeleneInstance *instance, selene_string_t tag, bool value) {
    (void)instance;
    // Use the print_bool FFI function if available
    // Signature: pub unsafe extern "C" fn print_bool(label_ptr: *const u8, label_len: i64, value: bool)
    extern void print_bool(const uint8_t *label_ptr, int64_t label_len, bool value);
    print_bool((const uint8_t*)tag.data, (int64_t)tag.length, value);
    return SUCCESS(selene_void_result_t);
}

selene_void_result_t selene_print_i64(SeleneInstance *instance, selene_string_t tag, int64_t value) {
    (void)instance;
    printf("%.*s: %ld\n", (int)tag.length, tag.data, value);
    return SUCCESS(selene_void_result_t);
}

selene_void_result_t selene_print_u64(SeleneInstance *instance, selene_string_t tag, uint64_t value) {
    (void)instance;
    printf("%.*s: %lu\n", (int)tag.length, tag.data, value);
    return SUCCESS(selene_void_result_t);
}

selene_void_result_t selene_print_f64(SeleneInstance *instance, selene_string_t tag, double value) {
    (void)instance;
    printf("%.*s: %f\n", (int)tag.length, tag.data, value);
    return SUCCESS(selene_void_result_t);
}

selene_void_result_t selene_print_bool_array(SeleneInstance *instance, selene_string_t tag,
                                             const bool *ptr, uint64_t length) {
    (void)instance;
    printf("%.*s: [", (int)tag.length, tag.data);
    for (uint64_t i = 0; i < length; i++) {
        printf("%s%s", ptr[i] ? "true" : "false", (i < length - 1) ? ", " : "");
    }
    printf("]\n");
    return SUCCESS(selene_void_result_t);
}

selene_void_result_t selene_print_i64_array(SeleneInstance *instance, selene_string_t tag,
                                            const int64_t *ptr, uint64_t length) {
    (void)instance;
    printf("%.*s: [", (int)tag.length, tag.data);
    for (uint64_t i = 0; i < length; i++) {
        printf("%ld%s", ptr[i], (i < length - 1) ? ", " : "");
    }
    printf("]\n");
    return SUCCESS(selene_void_result_t);
}

selene_void_result_t selene_print_u64_array(SeleneInstance *instance, selene_string_t tag,
                                            const uint64_t *ptr, uint64_t length) {
    (void)instance;
    printf("%.*s: [", (int)tag.length, tag.data);
    for (uint64_t i = 0; i < length; i++) {
        printf("%lu%s", ptr[i], (i < length - 1) ? ", " : "");
    }
    printf("]\n");
    return SUCCESS(selene_void_result_t);
}

selene_void_result_t selene_print_f64_array(SeleneInstance *instance, selene_string_t tag,
                                            const double *ptr, uint64_t length) {
    (void)instance;
    printf("%.*s: [", (int)tag.length, tag.data);
    for (uint64_t i = 0; i < length; i++) {
        printf("%f%s", ptr[i], (i < length - 1) ? ", " : "");
    }
    printf("]\n");
    return SUCCESS(selene_void_result_t);
}

selene_void_result_t selene_print_panic(SeleneInstance *instance, selene_string_t message,
                                       uint32_t error_code) {
    (void)instance;
    fprintf(stderr, "PANIC [%u]: %.*s\n", error_code, (int)message.length, message.data);
    return SUCCESS(selene_void_result_t);
}

// =============================================================================
// Stub implementations for functions we don't need yet
// =============================================================================

selene_void_result_t selene_dump_state(SeleneInstance *instance, selene_string_t message,
                                       const uint64_t *qubits, uint64_t qubits_length) {
    (void)instance; (void)message; (void)qubits; (void)qubits_length;
    // No-op - state dumping not supported in operation collection mode
    return SUCCESS(selene_void_result_t);
}

selene_void_result_t selene_set_tc(SeleneInstance *instance, uint64_t time_cursor) {
    (void)instance; (void)time_cursor;
    // No-op - time cursor not used
    return SUCCESS(selene_void_result_t);
}

selene_u64_result_t selene_get_tc(SeleneInstance *instance) {
    (void)instance;
    return SUCCESS_VAL(selene_u64_result_t, 0);
}

selene_u64_result_t selene_get_current_shot(SeleneInstance *instance) {
    (void)instance;
    return SUCCESS_VAL(selene_u64_result_t, 0);
}

selene_void_result_t selene_local_barrier(SeleneInstance *instance, const uint64_t *qubit_ids,
                                         uint64_t qubit_ids_length, uint64_t sleep_time) {
    (void)instance; (void)qubit_ids; (void)qubit_ids_length; (void)sleep_time;
    // No-op - barriers not needed for operation collection
    return SUCCESS(selene_void_result_t);
}

selene_void_result_t selene_global_barrier(SeleneInstance *instance, uint64_t sleep_time) {
    (void)instance; (void)sleep_time;
    return SUCCESS(selene_void_result_t);
}

selene_u64_result_t selene_shot_count(SeleneInstance *instance) {
    (void)instance;
    // Return 1 shot for operation collection mode
    return SUCCESS_VAL(selene_u64_result_t, 1);
}

selene_void_result_t selene_on_shot_start(SeleneInstance *instance, uint64_t shot_index) {
    (void)instance; (void)shot_index;
    return SUCCESS(selene_void_result_t);
}

selene_void_result_t selene_on_shot_end(SeleneInstance *instance) {
    (void)instance;
    return SUCCESS(selene_void_result_t);
}

selene_void_result_t selene_load_config(SeleneInstance **instance, const char *config_file) {
    (void)config_file;
    // Return a dummy instance pointer - we don't actually use it
    static SeleneInstance dummy_instance;
    *instance = &dummy_instance;
    return SUCCESS(selene_void_result_t);
}

selene_void_result_t selene_exit(SeleneInstance *instance) {
    (void)instance;
    return SUCCESS(selene_void_result_t);
}

selene_void_result_t selene_print_exit(SeleneInstance *instance, selene_string_t message,
                                      uint32_t error_code) {
    (void)instance;
    fprintf(stderr, "EXIT [%u]: %.*s\n", error_code, (int)message.length, message.data);
    return SUCCESS(selene_void_result_t);
}

// Random number generation stubs
selene_void_result_t selene_random_seed(SeleneInstance *instance, uint64_t seed) {
    (void)instance; (void)seed;
    return SUCCESS(selene_void_result_t);
}

selene_void_result_t selene_random_advance(SeleneInstance *instance, uint64_t delta) {
    (void)instance; (void)delta;
    return SUCCESS(selene_void_result_t);
}

selene_u32_result_t selene_random_u32(SeleneInstance *instance) {
    (void)instance;
    return (selene_u32_result_t){.error_code = 0, .value = 0};
}

selene_u32_result_t selene_random_u32_bounded(SeleneInstance *instance, uint32_t bound) {
    (void)instance; (void)bound;
    return (selene_u32_result_t){.error_code = 0, .value = 0};
}

selene_f64_result_t selene_random_f64(SeleneInstance *instance) {
    (void)instance;
    return (selene_f64_result_t){.error_code = 0, .value = 0.0};
}

selene_u64_result_t selene_custom_runtime_call(SeleneInstance *instance, uint64_t tag,
                                               const uint8_t *data, uint64_t data_length) {
    (void)instance; (void)tag; (void)data; (void)data_length;
    return SUCCESS_VAL(selene_u64_result_t, 0);
}
