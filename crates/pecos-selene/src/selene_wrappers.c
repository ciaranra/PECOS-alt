// Wrapper functions that intercept plugin calls and forward to real Selene
// This allows us to use the real libselene.so while handling NULL instance pointers

#define _GNU_SOURCE
#include <stdio.h>
#include <stdlib.h>
#include <stdint.h>
#include <stdbool.h>
#include <dlfcn.h>
#include <pthread.h>
#include <string.h>
#include <unistd.h>
#include <fcntl.h>

// Selene type definitions
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

typedef struct selene_string_t {
    const char *data;
    uint64_t length;
    bool owned;
} selene_string_t;

// Thread-local storage for the current Selene instance
static __thread SeleneInstance *current_instance = NULL;
static __thread bool instance_initialized = false;

// Handle to the real libselene.so
static void *selene_lib_handle = NULL;

// Function pointer to bridge's set_engine_callbacks function
static void (*bridge_set_callbacks)(void *, void *, void *) = NULL;

// Stub callbacks for bridge to use
static int stub_send_operation(void *context, const uint8_t *data, size_t len) {
    printf("*** WRAPPER: Bridge sending ByteMessage (%zu bytes) ***\n", len);
    // For now, just acknowledge - real implementation would forward to engine
    return 0;
}

static int stub_receive_measurements(void *context, uint8_t **data, size_t *len) {
    printf("*** WRAPPER: Bridge requesting measurements ***\n");

    // Return NULL/0 to trigger fallback in bridge
    *data = NULL;
    *len = 0;
    return 0;
}

// Function pointers to real Selene functions
static struct selene_u64_result_t (*real_selene_qalloc)(struct SeleneInstance *) = NULL;
static struct selene_void_result_t (*real_selene_qfree)(struct SeleneInstance *, uint64_t) = NULL;
static struct selene_void_result_t (*real_selene_qubit_reset)(struct SeleneInstance *, uint64_t) = NULL;
static struct selene_bool_result_t (*real_selene_qubit_measure)(struct SeleneInstance *, uint64_t) = NULL;
static struct selene_future_result_t (*real_selene_qubit_lazy_measure)(struct SeleneInstance *, uint64_t) = NULL;
static struct selene_bool_result_t (*real_selene_future_read_bool)(struct SeleneInstance *, uint64_t) = NULL;
static struct selene_void_result_t (*real_selene_rxy)(struct SeleneInstance *, uint64_t, double, double) = NULL;
static struct selene_void_result_t (*real_selene_rz)(struct SeleneInstance *, uint64_t, double) = NULL;
static struct selene_void_result_t (*real_selene_rzz)(struct SeleneInstance *, uint64_t, uint64_t, double) = NULL;
static struct selene_u64_result_t (*real_selene_get_tc)(struct SeleneInstance *) = NULL;
static struct selene_void_result_t (*real_selene_set_tc)(struct SeleneInstance *, uint64_t) = NULL;
static struct selene_void_result_t (*real_selene_load_config)(struct SeleneInstance **, const char *) = NULL;
static struct selene_void_result_t (*real_selene_on_shot_start)(struct SeleneInstance *, uint64_t) = NULL;
static struct selene_void_result_t (*real_selene_on_shot_end)(struct SeleneInstance *) = NULL;
static struct selene_void_result_t (*real_selene_exit)(struct SeleneInstance *) = NULL;

// Initialize the wrapper library
__attribute__((constructor))
static void init_wrapper() {
    // Try to load the real libselene.so
    const char *selene_paths[] = {
        // Try Python environment first
        "/home/ciaranra/Repos/cl_projects/gup/PECOS/.venv/lib/python3.12/site-packages/selene_sim/_dist/lib/libselene.so",
        // Fallback paths
        "libselene.so",
        NULL
    };

    for (int i = 0; selene_paths[i] != NULL; i++) {
        selene_lib_handle = dlopen(selene_paths[i], RTLD_NOW | RTLD_LOCAL);
        if (selene_lib_handle) {
            printf("*** WRAPPER: Loaded real Selene library from: %s ***\n", selene_paths[i]);
            break;
        }
    }

    if (!selene_lib_handle) {
        printf("*** WRAPPER: WARNING - Could not load real libselene.so, using stub behavior ***\n");
        return;
    }

    // Load function pointers
    real_selene_qalloc = dlsym(selene_lib_handle, "selene_qalloc");
    real_selene_qfree = dlsym(selene_lib_handle, "selene_qfree");
    real_selene_qubit_reset = dlsym(selene_lib_handle, "selene_qubit_reset");
    real_selene_qubit_measure = dlsym(selene_lib_handle, "selene_qubit_measure");
    real_selene_qubit_lazy_measure = dlsym(selene_lib_handle, "selene_qubit_lazy_measure");
    real_selene_future_read_bool = dlsym(selene_lib_handle, "selene_future_read_bool");
    real_selene_rxy = dlsym(selene_lib_handle, "selene_rxy");
    real_selene_rz = dlsym(selene_lib_handle, "selene_rz");
    real_selene_rzz = dlsym(selene_lib_handle, "selene_rzz");
    real_selene_get_tc = dlsym(selene_lib_handle, "selene_get_tc");
    real_selene_set_tc = dlsym(selene_lib_handle, "selene_set_tc");
    real_selene_load_config = dlsym(selene_lib_handle, "selene_load_config");
    real_selene_on_shot_start = dlsym(selene_lib_handle, "selene_on_shot_start");
    real_selene_on_shot_end = dlsym(selene_lib_handle, "selene_on_shot_end");
    real_selene_exit = dlsym(selene_lib_handle, "selene_exit");

    printf("*** WRAPPER: Function pointers loaded ***\n");

    // Also try to load the bridge plugin and set up callbacks
    void *bridge_handle = dlopen("/home/ciaranra/Repos/cl_projects/gup/PECOS/target/debug/libpecos_selene_bridge.so", RTLD_NOW | RTLD_GLOBAL);
    if (bridge_handle) {
        printf("*** WRAPPER: Loaded bridge plugin ***\n");
        bridge_set_callbacks = dlsym(bridge_handle, "pecos_bridge_set_engine_callbacks");
        if (bridge_set_callbacks) {
            printf("*** WRAPPER: Found bridge callback setter ***\n");
            // Set up the callbacks so bridge can communicate
            bridge_set_callbacks(NULL, stub_send_operation, stub_receive_measurements);
            printf("*** WRAPPER: Bridge callbacks configured ***\n");
        }
    }
}

// Clean up on exit
__attribute__((destructor))
static void cleanup_wrapper() {
    if (current_instance && real_selene_exit) {
        real_selene_exit(current_instance);
    }
    if (selene_lib_handle) {
        dlclose(selene_lib_handle);
    }
}

// Helper to initialize Selene if needed
static void ensure_initialized() {
    if (!instance_initialized && real_selene_load_config) {
        // Create a minimal config file matching Selene's expected format
        // Using Quest simulator plugin with proper format
        const char *config_content =
            "n_qubits: 10\n"
            "shots:\n"
            "  count: 1\n"
            "  offset: 0\n"
            "  increment: 1\n"
            "simulator:\n"
            "  name: \"pecos_rslib.bridge_simulator.PecosSeleneBridgeSimulator\"\n"
            "  file: \"/home/ciaranra/Repos/cl_projects/gup/PECOS/target/debug/libpecos_selene_bridge.so\"\n"
            "  args: []\n"
            "error_model:\n"
            "  name: \"selene_ideal_error_model_plugin.plugin.IdealErrorModelPlugin\"\n"
            "  file: \"/home/ciaranra/Repos/cl_projects/gup/PECOS/.venv/lib/python3.12/site-packages/selene_ideal_error_model_plugin/_dist/lib/libselene_ideal_plugin.so\"\n"
            "  args: []\n"
            "runtime:\n"
            "  name: \"selene_simple_runtime_plugin.plugin.SimpleRuntimePlugin\"\n"
            "  file: \"/home/ciaranra/Repos/cl_projects/gup/PECOS/.venv/lib/python3.12/site-packages/selene_simple_runtime_plugin/_dist/lib/libselene_simple_runtime.so\"\n"
            "  args: []\n"
            "artifact_dir: \"/tmp/selene_artifacts\"\n"
            "output_stream: \"file:///tmp/selene_output.log\"\n"  // File output stream
            "event_hooks:\n"
            "  metrics: false\n";

        // Write config to temp file
        char config_path[] = "/tmp/selene_config_XXXXXX.yaml";
        int fd = mkstemps(config_path, 5);
        if (fd != -1) {
            write(fd, config_content, strlen(config_content));
            close(fd);

            // Initialize Selene
            struct selene_void_result_t result = real_selene_load_config(&current_instance, config_path);
            if (result.error_code == 0 && current_instance) {
                printf("*** WRAPPER: Successfully initialized Selene with real libselene.so! ***\n");
                printf("*** WRAPPER: Config file: %s ***\n", config_path);
                printf("*** WRAPPER: Instance pointer: %p ***\n", current_instance);

                // Start the first shot
                real_selene_on_shot_start(current_instance, 0);
                instance_initialized = true;
            } else {
                printf("*** WRAPPER: Failed to initialize Selene, error: %u ***\n", result.error_code);
            }
        }
    }
}

// API function to set the current instance from Rust
void selene_wrapper_set_instance(SeleneInstance *instance) {
    current_instance = instance;
    instance_initialized = (instance != NULL);
    printf("*** WRAPPER: Instance set to %p ***\n", instance);
}

// Wrapper functions that intercept calls and forward to real Selene

struct selene_u64_result_t selene_qalloc(struct SeleneInstance *instance) {
    ensure_initialized();

    if (!real_selene_qalloc || !current_instance) {
        fprintf(stderr, "*** WRAPPER ERROR: Selene not initialized, cannot allocate qubit ***\n");
        struct selene_u64_result_t result = { .error_code = 1, .value = 0 };
        return result;
    }

    return real_selene_qalloc(current_instance);
}

struct selene_void_result_t selene_qfree(struct SeleneInstance *instance, uint64_t q) {
    if (!real_selene_qfree || !current_instance) {
        fprintf(stderr, "*** WRAPPER ERROR: Selene not initialized ***\n");
        struct selene_void_result_t result = { .error_code = 1 };
        return result;
    }

    return real_selene_qfree(current_instance, q);
}

struct selene_void_result_t selene_qubit_reset(struct SeleneInstance *instance, uint64_t q) {
    if (!real_selene_qubit_reset || !current_instance) {
        fprintf(stderr, "*** WRAPPER ERROR: Selene not initialized ***\n");
        struct selene_void_result_t result = { .error_code = 1 };
        return result;
    }

    return real_selene_qubit_reset(current_instance, q);
}

struct selene_bool_result_t selene_qubit_measure(struct SeleneInstance *instance, uint64_t q) {
    if (!real_selene_qubit_measure || !current_instance) {
        fprintf(stderr, "*** WRAPPER ERROR: Selene not initialized ***\n");
        struct selene_bool_result_t result = { .error_code = 1, .value = false };
        return result;
    }

    return real_selene_qubit_measure(current_instance, q);
}

struct selene_future_result_t selene_qubit_lazy_measure(struct SeleneInstance *instance, uint64_t q) {
    if (!real_selene_qubit_lazy_measure || !current_instance) {
        fprintf(stderr, "*** WRAPPER ERROR: Selene not initialized ***\n");
        struct selene_future_result_t result = { .error_code = 1, .reference = 0 };
        return result;
    }

    return real_selene_qubit_lazy_measure(current_instance, q);
}

struct selene_bool_result_t selene_future_read_bool(struct SeleneInstance *instance, uint64_t r) {
    if (!real_selene_future_read_bool || !current_instance) {
        fprintf(stderr, "*** WRAPPER ERROR: Selene not initialized ***\n");
        struct selene_bool_result_t result = { .error_code = 1, .value = false };
        return result;
    }

    return real_selene_future_read_bool(current_instance, r);
}

struct selene_void_result_t selene_rxy(struct SeleneInstance *instance, uint64_t q, double theta, double phi) {
    if (!real_selene_rxy || !current_instance) {
        fprintf(stderr, "*** WRAPPER ERROR: Selene not initialized ***\n");
        struct selene_void_result_t result = { .error_code = 1 };
        return result;
    }

    return real_selene_rxy(current_instance, q, theta, phi);
}

struct selene_void_result_t selene_rz(struct SeleneInstance *instance, uint64_t q, double theta) {
    if (!real_selene_rz || !current_instance) {
        fprintf(stderr, "*** WRAPPER ERROR: Selene not initialized ***\n");
        struct selene_void_result_t result = { .error_code = 1 };
        return result;
    }

    return real_selene_rz(current_instance, q, theta);
}

struct selene_void_result_t selene_rzz(struct SeleneInstance *instance, uint64_t q1, uint64_t q2, double theta) {
    if (!real_selene_rzz || !current_instance) {
        fprintf(stderr, "*** WRAPPER ERROR: Selene not initialized ***\n");
        struct selene_void_result_t result = { .error_code = 1 };
        return result;
    }

    return real_selene_rzz(current_instance, q1, q2, theta);
}

struct selene_u64_result_t selene_get_tc(struct SeleneInstance *instance) {
    if (!real_selene_get_tc || !current_instance) {
        // For get_tc, return 0 as default which is valid
        struct selene_u64_result_t result = { .error_code = 0, .value = 0 };
        return result;
    }

    return real_selene_get_tc(current_instance);
}

struct selene_void_result_t selene_set_tc(struct SeleneInstance *instance, uint64_t tc) {
    if (!real_selene_set_tc || !current_instance) {
        // For set_tc, we can ignore if not initialized yet
        struct selene_void_result_t result = { .error_code = 0 };
        return result;
    }

    return real_selene_set_tc(current_instance, tc);
}

// Reference counting functions (may not exist in real Selene)
struct selene_void_result_t selene_refcount_increment(struct SeleneInstance *instance, uint64_t reference) {
    // These might not exist in real Selene, so just return success
    struct selene_void_result_t result = { .error_code = 0 };
    return result;
}

struct selene_void_result_t selene_refcount_decrement(struct SeleneInstance *instance, uint64_t reference) {
    // These might not exist in real Selene, so just return success
    struct selene_void_result_t result = { .error_code = 0 };
    return result;
}
