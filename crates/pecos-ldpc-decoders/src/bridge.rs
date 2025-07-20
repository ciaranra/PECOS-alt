//! CXX bridge definitions for LDPC decoders

#[cxx::bridge]
#[allow(clippy::too_many_arguments)]
pub mod ffi {
    #[derive(Debug, Clone)]
    struct DecodingResult {
        pub decoding: Vec<u8>,
        pub converged: bool,
        pub iterations: i32,
    }

    #[derive(Debug)]
    struct SparseMatrixRepr {
        pub rows: u32,
        pub cols: u32,
        pub row_indices: Vec<u32>,
        pub col_indices: Vec<u32>,
    }

    unsafe extern "C++" {
        include!("ldpc_ffi.h");

        type BpOsdDecoder;
        type BpLsdDecoder;
        type SoftInfoBpDecoder;
        type FlipDecoder;
        type UnionFindDecoder;

        // BP+OSD Decoder functions
        fn create_bp_osd_decoder(
            pcm: &SparseMatrixRepr,
            channel_probs: &[f64],
            max_iter: i32,
            bp_method: i32,
            bp_schedule: i32,
            ms_scaling_factor: f64,
            osd_method: i32,
            osd_order: i32,
            input_vector_type: i32,
            omp_thread_count: i32,
            serial_schedule_order: &[i32],
            random_schedule_seed: i32,
        ) -> Result<UniquePtr<BpOsdDecoder>>;

        fn decode_bp_osd(
            decoder: Pin<&mut BpOsdDecoder>,
            input_vector: &[u8],
        ) -> Result<DecodingResult>;

        fn get_log_prob_ratios_osd(decoder: &BpOsdDecoder) -> Vec<f64>;

        // Getter functions for BP+OSD decoder
        fn get_check_count_osd(decoder: &BpOsdDecoder) -> u32;
        fn get_bit_count_osd(decoder: &BpOsdDecoder) -> u32;
        fn get_channel_probs_osd(decoder: &BpOsdDecoder) -> Vec<f64>;
        fn get_max_iter_osd(decoder: &BpOsdDecoder) -> i32;
        fn get_bp_method_osd(decoder: &BpOsdDecoder) -> i32;
        fn get_bp_schedule_osd(decoder: &BpOsdDecoder) -> i32;
        fn get_ms_scaling_factor_osd(decoder: &BpOsdDecoder) -> f64;
        fn get_osd_method_osd(decoder: &BpOsdDecoder) -> i32;
        fn get_osd_order_osd(decoder: &BpOsdDecoder) -> i32;
        fn get_converged_osd(decoder: &BpOsdDecoder) -> bool;
        fn get_iterations_osd(decoder: &BpOsdDecoder) -> i32;
        fn get_bp_decoding_osd(decoder: &BpOsdDecoder) -> Vec<u8>;
        fn get_input_vector_type_osd(decoder: &BpOsdDecoder) -> i32;
        fn get_omp_thread_count_osd(decoder: &BpOsdDecoder) -> i32;
        fn get_random_schedule_seed_osd(decoder: &BpOsdDecoder) -> i32;

        // BP+LSD Decoder functions
        fn create_bp_lsd_decoder(
            pcm: &SparseMatrixRepr,
            channel_probs: &[f64],
            max_iter: i32,
            bp_method: i32,
            bp_schedule: i32,
            ms_scaling_factor: f64,
            lsd_method: i32,
            lsd_order: i32,
            bits_per_step: i32,
            input_vector_type: i32,
            omp_thread_count: i32,
            serial_schedule_order: &[i32],
            random_schedule_seed: i32,
        ) -> Result<UniquePtr<BpLsdDecoder>>;

        fn decode_bp_lsd(
            decoder: Pin<&mut BpLsdDecoder>,
            input_vector: &[u8],
        ) -> Result<DecodingResult>;

        fn get_log_prob_ratios_lsd(decoder: &BpLsdDecoder) -> Vec<f64>;

        // Getter functions for BP+LSD decoder
        fn get_check_count_lsd(decoder: &BpLsdDecoder) -> u32;
        fn get_bit_count_lsd(decoder: &BpLsdDecoder) -> u32;
        fn get_channel_probs_lsd(decoder: &BpLsdDecoder) -> Vec<f64>;
        fn get_max_iter_lsd(decoder: &BpLsdDecoder) -> i32;
        fn get_bp_method_lsd(decoder: &BpLsdDecoder) -> i32;
        fn get_bp_schedule_lsd(decoder: &BpLsdDecoder) -> i32;
        fn get_ms_scaling_factor_lsd(decoder: &BpLsdDecoder) -> f64;
        fn get_lsd_method_lsd(decoder: &BpLsdDecoder) -> i32;
        fn get_lsd_order_lsd(decoder: &BpLsdDecoder) -> i32;
        fn get_bits_per_step_lsd(decoder: &BpLsdDecoder) -> i32;
        fn get_converged_lsd(decoder: &BpLsdDecoder) -> bool;
        fn get_iterations_lsd(decoder: &BpLsdDecoder) -> i32;
        fn get_input_vector_type_lsd(decoder: &BpLsdDecoder) -> i32;
        fn get_omp_thread_count_lsd(decoder: &BpLsdDecoder) -> i32;
        fn get_random_schedule_seed_lsd(decoder: &BpLsdDecoder) -> i32;

        // Statistics functions for BP+LSD
        fn set_do_stats_lsd(decoder: Pin<&mut BpLsdDecoder>, enable: bool);
        fn get_do_stats_lsd(decoder: &BpLsdDecoder) -> bool;
        fn get_statistics_json_lsd(decoder: &BpLsdDecoder) -> String;

        // Soft Information BP Decoder functions
        fn create_soft_info_bp_decoder(
            pcm: &SparseMatrixRepr,
            channel_probs: &[f64],
            max_iter: i32,
            bp_method: i32,
            ms_scaling_factor: f64,
            omp_thread_count: i32,
            serial_schedule_order: &[i32],
            random_schedule_seed: i32,
        ) -> Result<UniquePtr<SoftInfoBpDecoder>>;

        fn decode_soft_info_bp(
            decoder: Pin<&mut SoftInfoBpDecoder>,
            soft_syndrome: &[f64],
            cutoff: f64,
            sigma: f64,
        ) -> Result<DecodingResult>;

        // Getter functions for Soft Info BP decoder
        fn get_check_count_soft(decoder: &SoftInfoBpDecoder) -> u32;
        fn get_bit_count_soft(decoder: &SoftInfoBpDecoder) -> u32;
        fn get_channel_probs_soft(decoder: &SoftInfoBpDecoder) -> Vec<f64>;
        fn get_max_iter_soft(decoder: &SoftInfoBpDecoder) -> i32;
        fn get_bp_method_soft(decoder: &SoftInfoBpDecoder) -> i32;
        fn get_ms_scaling_factor_soft(decoder: &SoftInfoBpDecoder) -> f64;
        fn get_converged_soft(decoder: &SoftInfoBpDecoder) -> bool;
        fn get_iterations_soft(decoder: &SoftInfoBpDecoder) -> i32;
        fn get_omp_thread_count_soft(decoder: &SoftInfoBpDecoder) -> i32;
        fn get_random_schedule_seed_soft(decoder: &SoftInfoBpDecoder) -> i32;
        fn get_log_prob_ratios_soft(decoder: &SoftInfoBpDecoder) -> Vec<f64>;

        // Flip Decoder functions
        fn create_flip_decoder(
            pcm: &SparseMatrixRepr,
            max_iter: i32,
            pfreq: i32,
            seed: i32,
        ) -> Result<UniquePtr<FlipDecoder>>;

        fn decode_flip(decoder: Pin<&mut FlipDecoder>, syndrome: &[u8]) -> Result<DecodingResult>;

        // Getter functions for Flip decoder
        fn get_check_count_flip(decoder: &FlipDecoder) -> u32;
        fn get_bit_count_flip(decoder: &FlipDecoder) -> u32;
        fn get_max_iter_flip(decoder: &FlipDecoder) -> i32;
        fn get_converged_flip(decoder: &FlipDecoder) -> bool;
        fn get_iterations_flip(decoder: &FlipDecoder) -> i32;

        // Union Find Decoder functions
        fn create_union_find_decoder(
            pcm: &SparseMatrixRepr,
            uf_method: i32,
        ) -> Result<UniquePtr<UnionFindDecoder>>;

        fn decode_union_find(
            decoder: Pin<&mut UnionFindDecoder>,
            syndrome: &[u8],
            llrs: &[f64],
            bits_per_step: i32,
        ) -> Result<DecodingResult>;

        // Getter functions for Union Find decoder
        fn get_check_count_uf(decoder: &UnionFindDecoder) -> u32;
        fn get_bit_count_uf(decoder: &UnionFindDecoder) -> u32;

        // MBP Decoder for quantum codes
        type MbpDecoder;

        fn create_mbp_decoder(
            hx: &SparseMatrixRepr,
            hz: &SparseMatrixRepr,
            error_rate: f64,
            xyz_bias: &[f64],
            max_iter: i32,
            bp_method: i32,
            ms_scaling_factor: f64,
            omp_thread_count: i32,
        ) -> Result<UniquePtr<MbpDecoder>>;

        fn decode_mbp(decoder: Pin<&mut MbpDecoder>, syndrome: &[u8]) -> Result<DecodingResult>;

        // Getter functions for MBP decoder
        fn get_check_count_mbp(decoder: &MbpDecoder) -> u32;
        fn get_bit_count_mbp(decoder: &MbpDecoder) -> u32;
        fn get_max_iter_mbp(decoder: &MbpDecoder) -> i32;
        fn get_converged_mbp(decoder: &MbpDecoder) -> bool;
        fn get_iterations_mbp(decoder: &MbpDecoder) -> i32;
    }
}
