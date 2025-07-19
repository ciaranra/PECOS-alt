//! Build script for pecos-ldpc-decoders

mod build_ldpc;

fn main() {
    // Download and build LDPC
    let download_info = pecos_build_utils::ldpc_download_info();

    // Download if needed
    if let Err(e) = pecos_build_utils::download_all_cached(vec![download_info]) {
        println!("cargo:warning=Download failed: {e}, continuing with build");
    }

    // Build LDPC
    build_ldpc::build().expect("LDPC build failed");
}
