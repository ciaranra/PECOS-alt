// Correct Selene stub functions matching the exact FFI interface
// Based on selene/selene-sim/c/include/selene/selene.h

#include <stdio.h>
#include <stdint.h>
#include <stdbool.h>

// Exact type definitions from Selene
typedef struct SeleneInstance SeleneInstance;

typedef struct selene_u64_result_t {
    uint32_t error_code;
    uint64_t value;
} selene_u64_result_t;

typedef struct selene_void_result_t {
    uint32_t error_code;
} selene_void_result_t;

typedef struct selene_bool_result_t {
    uint32_t error_code;
    bool value;
} selene_bool_result_t;

typedef struct selene_future_result_t {
    uint32_t error_code;
    uint64_t reference;
} selene_future_result_t;

typedef struct selene_f64_result_t {
    uint32_t error_code;
    double value;
} selene_f64_result_t;

typedef struct selene_u32_result_t {
    uint32_t error_code;
    uint32_t value;
} selene_u32_result_t;

typedef struct selene_string_t {
    const char *data;
    uint64_t length;
    bool owned;
} selene_string_t;

// Global counters
static uint64_t next_qubit_id = 0;
static uint64_t next_result_id = 0;
static uint64_t current_shot = 0;
static uint64_t time_cursor = 0;

// Core quantum operations - matching exact Selene signatures
struct selene_u64_result_t selene_qalloc(struct SeleneInstance *instance) {
    printf("=== SELENE STUB: selene_qalloc called with instance=%p ===\n", instance);
    fflush(stdout);
    struct selene_u64_result_t result = { .error_code = 0, .value = next_qubit_id++ };
    printf("=== SELENE STUB: selene_qalloc returning qubit_id=%lu ===\n", result.value);
    fflush(stdout);
    return result;
}

struct selene_void_result_t selene_qfree(struct SeleneInstance *instance, uint64_t q) {
    printf("=== SELENE STUB: selene_qfree called with instance=%p, qubit=%lu ===\n", instance, q);
    fflush(stdout);
    struct selene_void_result_t result = { .error_code = 0 };
    return result;
}

struct selene_bool_result_t selene_qubit_measure(struct SeleneInstance *instance, uint64_t q) {
    printf("=== SELENE STUB: selene_qubit_measure called with instance=%p, qubit=%lu ===\n", instance, q);
    fflush(stdout);
    // Return alternating values for testing
    static bool counter = false;
    counter = !counter;
    struct selene_bool_result_t result = { .error_code = 0, .value = counter };
    printf("=== SELENE STUB: selene_qubit_measure returning %s ===\n", counter ? "true" : "false");
    fflush(stdout);
    return result;
}

struct selene_future_result_t selene_qubit_lazy_measure(struct SeleneInstance *instance, uint64_t q) {
    printf("=== SELENE STUB: selene_qubit_lazy_measure called with instance=%p, qubit=%lu ===\n", instance, q);
    fflush(stdout);
    struct selene_future_result_t result = { .error_code = 0, .reference = next_result_id++ };
    printf("=== SELENE STUB: selene_qubit_lazy_measure returning reference=%lu ===\n", result.reference);
    fflush(stdout);
    return result;
}

struct selene_future_result_t selene_qubit_lazy_measure_leaked(struct SeleneInstance *instance, uint64_t q) {
    printf("=== SELENE STUB: selene_qubit_lazy_measure_leaked called with instance=%p, qubit=%lu ===\n", instance, q);
    fflush(stdout);
    struct selene_future_result_t result = { .error_code = 0, .reference = next_result_id++ };
    return result;
}

struct selene_void_result_t selene_qubit_reset(struct SeleneInstance *instance, uint64_t q) {
    printf("=== SELENE STUB: selene_qubit_reset called with instance=%p, qubit=%lu ===\n", instance, q);
    fflush(stdout);
    struct selene_void_result_t result = { .error_code = 0 };
    return result;
}

// Future reading functions
struct selene_bool_result_t selene_future_read_bool(struct SeleneInstance *instance, uint64_t r) {
    printf("=== SELENE STUB: selene_future_read_bool called with instance=%p, reference=%lu ===\n", instance, r);
    fflush(stdout);
    // Return alternating values for testing
    static bool counter = false;
    counter = !counter;
    struct selene_bool_result_t result = { .error_code = 0, .value = counter };
    printf("=== SELENE STUB: selene_future_read_bool returning %s ===\n", counter ? "true" : "false");
    fflush(stdout);
    return result;
}

struct selene_u64_result_t selene_future_read_u64(struct SeleneInstance *instance, uint64_t r) {
    printf("=== SELENE STUB: selene_future_read_u64 called with instance=%p, reference=%lu ===\n", instance, r);
    fflush(stdout);
    struct selene_u64_result_t result = { .error_code = 0, .value = 42 };
    return result;
}

// Gate operations
struct selene_void_result_t selene_rxy(struct SeleneInstance *instance, uint64_t qubit_id, double theta, double phi) {
    printf("=== SELENE STUB: selene_rxy called with instance=%p, qubit=%lu, theta=%f, phi=%f ===\n", 
           instance, qubit_id, theta, phi);
    fflush(stdout);
    struct selene_void_result_t result = { .error_code = 0 };
    return result;
}

struct selene_void_result_t selene_rz(struct SeleneInstance *instance, uint64_t qubit_id, double theta) {
    printf("=== SELENE STUB: selene_rz called with instance=%p, qubit=%lu, theta=%f ===\n", 
           instance, qubit_id, theta);
    fflush(stdout);
    struct selene_void_result_t result = { .error_code = 0 };
    return result;
}

struct selene_void_result_t selene_rzz(struct SeleneInstance *instance, uint64_t qubit_id, uint64_t qubit_id2, double theta) {
    printf("=== SELENE STUB: selene_rzz called with instance=%p, qubit1=%lu, qubit2=%lu, theta=%f ===\n", 
           instance, qubit_id, qubit_id2, theta);
    fflush(stdout);
    struct selene_void_result_t result = { .error_code = 0 };
    return result;
}

// Shot management
struct selene_void_result_t selene_on_shot_start(struct SeleneInstance *instance, uint64_t shot_index) {
    printf("=== SELENE STUB: selene_on_shot_start called with instance=%p, shot_index=%lu ===\n", 
           instance, shot_index);
    fflush(stdout);
    current_shot = shot_index;
    struct selene_void_result_t result = { .error_code = 0 };
    return result;
}

struct selene_void_result_t selene_on_shot_end(struct SeleneInstance *instance) {
    printf("=== SELENE STUB: selene_on_shot_end called with instance=%p ===\n", instance);
    fflush(stdout);
    struct selene_void_result_t result = { .error_code = 0 };
    return result;
}

struct selene_u64_result_t selene_get_current_shot(struct SeleneInstance *instance) {
    printf("=== SELENE STUB: selene_get_current_shot called with instance=%p ===\n", instance);
    fflush(stdout);
    struct selene_u64_result_t result = { .error_code = 0, .value = current_shot };
    return result;
}

// Exit/cleanup
struct selene_void_result_t selene_exit(struct SeleneInstance *instance) {
    printf("=== SELENE STUB: selene_exit called with instance=%p ===\n", instance);
    fflush(stdout);
    struct selene_void_result_t result = { .error_code = 0 };
    return result;
}

// Time cursor
struct selene_u64_result_t selene_get_tc(struct SeleneInstance *instance) {
    printf("=== SELENE STUB: selene_get_tc called with instance=%p ===\n", instance);
    fflush(stdout);
    struct selene_u64_result_t result = { .error_code = 0, .value = time_cursor };
    return result;
}

struct selene_void_result_t selene_set_tc(struct SeleneInstance *instance, uint64_t tc) {
    printf("=== SELENE STUB: selene_set_tc called with instance=%p, tc=%lu ===\n", instance, tc);
    fflush(stdout);
    time_cursor = tc;
    struct selene_void_result_t result = { .error_code = 0 };
    return result;
}

// Barrier operations
struct selene_void_result_t selene_local_barrier(struct SeleneInstance *instance, const uint64_t *qubit_ids, uint64_t qubit_ids_length, uint64_t sleep_time) {
    printf("=== SELENE STUB: selene_local_barrier called with instance=%p, num_qubits=%lu, sleep_time=%lu ===\n", 
           instance, qubit_ids_length, sleep_time);
    fflush(stdout);
    struct selene_void_result_t result = { .error_code = 0 };
    return result;
}

struct selene_void_result_t selene_global_barrier(struct SeleneInstance *instance, uint64_t sleep_time) {
    printf("=== SELENE STUB: selene_global_barrier called with instance=%p, sleep_time=%lu ===\n", 
           instance, sleep_time);
    fflush(stdout);
    struct selene_void_result_t result = { .error_code = 0 };
    return result;
}

// Print functions
struct selene_void_result_t selene_print_bool(struct SeleneInstance *instance, struct selene_string_t tag, bool value) {
    printf("=== SELENE STUB: selene_print_bool called with instance=%p, value=%s ===\n", 
           instance, value ? "true" : "false");
    fflush(stdout);
    struct selene_void_result_t result = { .error_code = 0 };
    return result;
}

struct selene_void_result_t selene_print_f64(struct SeleneInstance *instance, struct selene_string_t tag, double value) {
    printf("=== SELENE STUB: selene_print_f64 called with instance=%p, value=%f ===\n", instance, value);
    fflush(stdout);
    struct selene_void_result_t result = { .error_code = 0 };
    return result;
}

struct selene_void_result_t selene_print_u64(struct SeleneInstance *instance, struct selene_string_t tag, uint64_t value) {
    printf("=== SELENE STUB: selene_print_u64 called with instance=%p, value=%lu ===\n", instance, value);
    fflush(stdout);
    struct selene_void_result_t result = { .error_code = 0 };
    return result;
}

struct selene_void_result_t selene_print_i64(struct SeleneInstance *instance, struct selene_string_t tag, int64_t value) {
    printf("=== SELENE STUB: selene_print_i64 called with instance=%p, value=%ld ===\n", instance, value);
    fflush(stdout);
    struct selene_void_result_t result = { .error_code = 0 };
    return result;
}

// Array print functions
struct selene_void_result_t selene_print_bool_array(struct SeleneInstance *instance, struct selene_string_t tag, const bool *ptr, uint64_t length) {
    printf("=== SELENE STUB: selene_print_bool_array called with instance=%p, length=%lu ===\n", instance, length);
    fflush(stdout);
    struct selene_void_result_t result = { .error_code = 0 };
    return result;
}

struct selene_void_result_t selene_print_f64_array(struct SeleneInstance *instance, struct selene_string_t tag, const double *ptr, uint64_t length) {
    printf("=== SELENE STUB: selene_print_f64_array called with instance=%p, length=%lu ===\n", instance, length);
    fflush(stdout);
    struct selene_void_result_t result = { .error_code = 0 };
    return result;
}

struct selene_void_result_t selene_print_u64_array(struct SeleneInstance *instance, struct selene_string_t tag, const uint64_t *ptr, uint64_t length) {
    printf("=== SELENE STUB: selene_print_u64_array called with instance=%p, length=%lu ===\n", instance, length);
    fflush(stdout);
    struct selene_void_result_t result = { .error_code = 0 };
    return result;
}

struct selene_void_result_t selene_print_i64_array(struct SeleneInstance *instance, struct selene_string_t tag, const int64_t *ptr, uint64_t length) {
    printf("=== SELENE STUB: selene_print_i64_array called with instance=%p, length=%lu ===\n", instance, length);
    fflush(stdout);
    struct selene_void_result_t result = { .error_code = 0 };
    return result;
}

// State dump
struct selene_void_result_t selene_dump_state(struct SeleneInstance *instance, struct selene_string_t message, const uint64_t *qubits, uint64_t qubits_length) {
    printf("=== SELENE STUB: selene_dump_state called with instance=%p, num_qubits=%lu ===\n", instance, qubits_length);
    fflush(stdout);
    struct selene_void_result_t result = { .error_code = 0 };
    return result;
}

// Random number generation
struct selene_void_result_t selene_random_seed(struct SeleneInstance *instance, uint64_t seed) {
    printf("=== SELENE STUB: selene_random_seed called with instance=%p, seed=%lu ===\n", instance, seed);
    fflush(stdout);
    struct selene_void_result_t result = { .error_code = 0 };
    return result;
}

struct selene_u32_result_t selene_random_u32(struct SeleneInstance *instance) {
    printf("=== SELENE STUB: selene_random_u32 called with instance=%p ===\n", instance);
    fflush(stdout);
    struct selene_u32_result_t result = { .error_code = 0, .value = 12345 };
    return result;
}

struct selene_f64_result_t selene_random_f64(struct SeleneInstance *instance) {
    printf("=== SELENE STUB: selene_random_f64 called with instance=%p ===\n", instance);
    fflush(stdout);
    struct selene_f64_result_t result = { .error_code = 0, .value = 0.5 };
    return result;
}

// Custom runtime call
struct selene_u64_result_t selene_custom_runtime_call(struct SeleneInstance *instance, uint64_t tag, const uint8_t *data, uint64_t data_length) {
    printf("=== SELENE STUB: selene_custom_runtime_call called with instance=%p, tag=%lu, data_length=%lu ===\n", 
           instance, tag, data_length);
    fflush(stdout);
    struct selene_u64_result_t result = { .error_code = 1, .value = 0 }; // Not supported
    return result;
}

// Reference counting functions
struct selene_void_result_t selene_refcount_increment(struct SeleneInstance *instance, uint64_t reference) {
    printf("=== SELENE STUB: selene_refcount_increment called with instance=%p, reference=%lu ===\n", instance, reference);
    fflush(stdout);
    struct selene_void_result_t result = { .error_code = 0 };
    return result;
}

struct selene_void_result_t selene_refcount_decrement(struct SeleneInstance *instance, uint64_t reference) {
    printf("=== SELENE STUB: selene_refcount_decrement called with instance=%p, reference=%lu ===\n", instance, reference);
    fflush(stdout);
    struct selene_void_result_t result = { .error_code = 0 };
    return result;
}