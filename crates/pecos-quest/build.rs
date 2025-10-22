//! Build script for pecos-quest

mod build_quest;

fn main() {
    // Initialize logger for build script
    env_logger::init();
    // Download and build QuEST
    let download_info = pecos_build_utils::quest_download_info();

    // Download if needed
    if let Err(e) = pecos_build_utils::download_all_cached(vec![download_info]) {
        log::warn!("Download failed: {e}, continuing with build");
    }

    // Build QuEST
    build_quest::build().expect("QuEST build failed");
}
