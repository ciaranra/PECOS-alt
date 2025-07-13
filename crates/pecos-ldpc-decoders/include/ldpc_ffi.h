#pragma once

#include <memory>
#include <vector>
#include <cstdint>
#include "rust/cxx.h"

// We don't need to forward declare the LDPC types since we use void*

// Forward declarations of types that will be defined by cxx
struct DecodingResult;
struct SparseMatrixRepr;

// Complete type definitions for cxx
class BpOsdDecoder {
public:
    // Use raw pointers to avoid needing complete types in header
    void* pcm;
    void* bp_decoder;
    void* osd_decoder;
    std::vector<double> channel_probs;
    bool use_osd;
    
    // Store decoder parameters
    int32_t max_iter;
    int32_t bp_method;
    int32_t bp_schedule;
    double ms_scaling_factor;
    int32_t osd_method;
    int32_t osd_order;
    int32_t input_vector_type;
    int32_t omp_thread_count;
    int32_t random_schedule_seed;
    
    // Declare destructor but don't define it here
    ~BpOsdDecoder();
};

class BpLsdDecoder {
public:
    // Use raw pointers to avoid needing complete types in header
    void* pcm;
    void* bp_decoder;
    void* lsd_decoder;
    std::vector<double> channel_probs;
    int bits_per_step;
    
    // Store decoder parameters
    int32_t max_iter;
    int32_t bp_method;
    int32_t bp_schedule;
    double ms_scaling_factor;
    int32_t lsd_method;
    int32_t lsd_order;
    int32_t input_vector_type;
    int32_t omp_thread_count;
    int32_t random_schedule_seed;
    
    // Declare destructor but don't define it here
    ~BpLsdDecoder();
};

// Function declarations
std::unique_ptr<BpOsdDecoder> create_bp_osd_decoder(
    const SparseMatrixRepr& pcm,
    rust::Slice<const double> channel_probs,
    int32_t max_iter,
    int32_t bp_method,
    int32_t bp_schedule,
    double ms_scaling_factor,
    int32_t osd_method,
    int32_t osd_order,
    int32_t input_vector_type,
    int32_t omp_thread_count,
    rust::Slice<const int32_t> serial_schedule_order,
    int32_t random_schedule_seed
);

DecodingResult decode_bp_osd(
    BpOsdDecoder& decoder,
    rust::Slice<const uint8_t> input_vector
);

rust::Vec<double> get_log_prob_ratios_osd(const BpOsdDecoder& decoder);

// Getter functions for BP+OSD decoder
uint32_t get_check_count_osd(const BpOsdDecoder& decoder);
uint32_t get_bit_count_osd(const BpOsdDecoder& decoder);
rust::Vec<double> get_channel_probs_osd(const BpOsdDecoder& decoder);
int32_t get_max_iter_osd(const BpOsdDecoder& decoder);
int32_t get_bp_method_osd(const BpOsdDecoder& decoder);
int32_t get_bp_schedule_osd(const BpOsdDecoder& decoder);
double get_ms_scaling_factor_osd(const BpOsdDecoder& decoder);
int32_t get_osd_method_osd(const BpOsdDecoder& decoder);
int32_t get_osd_order_osd(const BpOsdDecoder& decoder);
bool get_converged_osd(const BpOsdDecoder& decoder);
int32_t get_iterations_osd(const BpOsdDecoder& decoder);
rust::Vec<uint8_t> get_bp_decoding_osd(const BpOsdDecoder& decoder);
int32_t get_input_vector_type_osd(const BpOsdDecoder& decoder);
int32_t get_omp_thread_count_osd(const BpOsdDecoder& decoder);
int32_t get_random_schedule_seed_osd(const BpOsdDecoder& decoder);

std::unique_ptr<BpLsdDecoder> create_bp_lsd_decoder(
    const SparseMatrixRepr& pcm,
    rust::Slice<const double> channel_probs,
    int32_t max_iter,
    int32_t bp_method,
    int32_t bp_schedule,
    double ms_scaling_factor,
    int32_t lsd_method,
    int32_t lsd_order,
    int32_t bits_per_step,
    int32_t input_vector_type,
    int32_t omp_thread_count,
    rust::Slice<const int32_t> serial_schedule_order,
    int32_t random_schedule_seed
);

DecodingResult decode_bp_lsd(
    BpLsdDecoder& decoder,
    rust::Slice<const uint8_t> input_vector
);

rust::Vec<double> get_log_prob_ratios_lsd(const BpLsdDecoder& decoder);

// Getter functions for BP+LSD decoder
uint32_t get_check_count_lsd(const BpLsdDecoder& decoder);
uint32_t get_bit_count_lsd(const BpLsdDecoder& decoder);
rust::Vec<double> get_channel_probs_lsd(const BpLsdDecoder& decoder);
int32_t get_max_iter_lsd(const BpLsdDecoder& decoder);
int32_t get_bp_method_lsd(const BpLsdDecoder& decoder);
int32_t get_bp_schedule_lsd(const BpLsdDecoder& decoder);
double get_ms_scaling_factor_lsd(const BpLsdDecoder& decoder);
int32_t get_lsd_method_lsd(const BpLsdDecoder& decoder);
int32_t get_lsd_order_lsd(const BpLsdDecoder& decoder);
int32_t get_bits_per_step_lsd(const BpLsdDecoder& decoder);
bool get_converged_lsd(const BpLsdDecoder& decoder);
int32_t get_iterations_lsd(const BpLsdDecoder& decoder);
int32_t get_input_vector_type_lsd(const BpLsdDecoder& decoder);
int32_t get_omp_thread_count_lsd(const BpLsdDecoder& decoder);
int32_t get_random_schedule_seed_lsd(const BpLsdDecoder& decoder);

// Statistics functions for BP+LSD
void set_do_stats_lsd(BpLsdDecoder& decoder, bool enable);
bool get_do_stats_lsd(const BpLsdDecoder& decoder);
rust::String get_statistics_json_lsd(const BpLsdDecoder& decoder);

// Soft Information BP Decoder
class SoftInfoBpDecoder {
public:
    // Use raw pointers to avoid needing complete types in header
    void* pcm;
    void* bp_decoder;
    std::vector<double> channel_probs;
    
    // Store decoder parameters
    int32_t max_iter;
    int32_t bp_method;
    double ms_scaling_factor;
    int32_t omp_thread_count;
    int32_t random_schedule_seed;
    
    // Declare destructor but don't define it here
    ~SoftInfoBpDecoder();
};

// Flip Decoder
class FlipDecoder {
public:
    void* pcm;
    void* flip_decoder;
    
    // Store decoder parameters
    int32_t max_iter;
    int32_t pfreq;
    int32_t seed;
    
    ~FlipDecoder();
};

// Union Find Decoder
class UnionFindDecoder {
public:
    void* pcm;
    void* uf_decoder;
    
    // Store decoder parameters
    int32_t uf_method;
    
    ~UnionFindDecoder();
};

// Soft Info BP functions
std::unique_ptr<SoftInfoBpDecoder> create_soft_info_bp_decoder(
    const SparseMatrixRepr& pcm,
    rust::Slice<const double> channel_probs,
    int32_t max_iter,
    int32_t bp_method,
    double ms_scaling_factor,
    int32_t omp_thread_count,
    rust::Slice<const int32_t> serial_schedule_order,
    int32_t random_schedule_seed
);

DecodingResult decode_soft_info_bp(
    SoftInfoBpDecoder& decoder,
    rust::Slice<const double> soft_syndrome,
    double cutoff,
    double sigma
);

// Getter functions for Soft Info BP decoder
uint32_t get_check_count_soft(const SoftInfoBpDecoder& decoder);
uint32_t get_bit_count_soft(const SoftInfoBpDecoder& decoder);
rust::Vec<double> get_channel_probs_soft(const SoftInfoBpDecoder& decoder);
int32_t get_max_iter_soft(const SoftInfoBpDecoder& decoder);
int32_t get_bp_method_soft(const SoftInfoBpDecoder& decoder);
double get_ms_scaling_factor_soft(const SoftInfoBpDecoder& decoder);
bool get_converged_soft(const SoftInfoBpDecoder& decoder);
int32_t get_iterations_soft(const SoftInfoBpDecoder& decoder);
int32_t get_omp_thread_count_soft(const SoftInfoBpDecoder& decoder);
int32_t get_random_schedule_seed_soft(const SoftInfoBpDecoder& decoder);
rust::Vec<double> get_log_prob_ratios_soft(const SoftInfoBpDecoder& decoder);

// Flip Decoder functions
std::unique_ptr<FlipDecoder> create_flip_decoder(
    const SparseMatrixRepr& pcm,
    int32_t max_iter,
    int32_t pfreq,
    int32_t seed
);

DecodingResult decode_flip(
    FlipDecoder& decoder,
    rust::Slice<const uint8_t> syndrome
);

// Getter functions for Flip decoder
uint32_t get_check_count_flip(const FlipDecoder& decoder);
uint32_t get_bit_count_flip(const FlipDecoder& decoder);
int32_t get_max_iter_flip(const FlipDecoder& decoder);
bool get_converged_flip(const FlipDecoder& decoder);
int32_t get_iterations_flip(const FlipDecoder& decoder);

// Union Find Decoder functions
std::unique_ptr<UnionFindDecoder> create_union_find_decoder(
    const SparseMatrixRepr& pcm,
    int32_t uf_method
);

DecodingResult decode_union_find(
    UnionFindDecoder& decoder,
    rust::Slice<const uint8_t> syndrome,
    rust::Slice<const double> llrs,
    int32_t bits_per_step
);

// Getter functions for Union Find decoder
uint32_t get_check_count_uf(const UnionFindDecoder& decoder);
uint32_t get_bit_count_uf(const UnionFindDecoder& decoder);

// MBP Decoder for quantum codes
class MbpDecoder {
public:
    void* pcm;      // GF(4) parity check matrix
    void* pcmx;     // X stabilizer matrix
    void* pcmz;     // Z stabilizer matrix
    void* mbp_decoder;
    
    // Store decoder parameters
    int32_t max_iter;
    int32_t bp_method;
    double ms_scaling_factor;
    int32_t qubit_count;
    int32_t stab_count;
    
    ~MbpDecoder();
};

// MBP Decoder functions
std::unique_ptr<MbpDecoder> create_mbp_decoder(
    const SparseMatrixRepr& hx,
    const SparseMatrixRepr& hz,
    double error_rate,
    rust::Slice<const double> xyz_bias,
    int32_t max_iter,
    int32_t bp_method,
    double ms_scaling_factor,
    int32_t omp_thread_count
);

DecodingResult decode_mbp(
    MbpDecoder& decoder,
    rust::Slice<const uint8_t> syndrome
);

// Getter functions for MBP decoder
uint32_t get_check_count_mbp(const MbpDecoder& decoder);
uint32_t get_bit_count_mbp(const MbpDecoder& decoder);
int32_t get_max_iter_mbp(const MbpDecoder& decoder);
bool get_converged_mbp(const MbpDecoder& decoder);
int32_t get_iterations_mbp(const MbpDecoder& decoder);