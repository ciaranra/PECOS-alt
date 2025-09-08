/// CLI tool to test HUGR 0.13 loading from guppylang
use anyhow::Result;
use std::fs;
use std::path::PathBuf;

#[cfg(feature = "hugr-013")]
use pecos_selene::hugr_013_support::load_hugr_013_package;

#[cfg(feature = "hugr-013")]
use hugr_core_013::hugr::views::HugrView;

fn main() -> Result<()> {
    env_logger::init();

    let args: Vec<String> = std::env::args().collect();
    if args.len() != 2 {
        eprintln!("Usage: {} <hugr-file>", args[0]);
        std::process::exit(1);
    }

    let hugr_path = PathBuf::from(&args[1]);

    #[cfg(feature = "hugr-013")]
    {
        println!("Loading HUGR 0.13 file: {}", hugr_path.display());

        let hugr_bytes = fs::read(&hugr_path)?;
        println!("Read {} bytes", hugr_bytes.len());

        match load_hugr_013_package(&hugr_bytes) {
            Ok(package) => {
                println!("✓ Successfully loaded HUGR 0.13 package!");

                // Print some basic info about the package
                println!("  Modules: {}", package.modules.len());
                println!("  Extensions: {}", package.extensions.len());

                for (idx, module) in package.modules.iter().enumerate() {
                    println!("    - Module {}: {} nodes", idx, module.node_count());
                }

                Ok(())
            }
            Err(e) => {
                eprintln!("✗ Failed to load HUGR 0.13: {}", e);
                std::process::exit(1);
            }
        }
    }

    #[cfg(not(feature = "hugr-013"))]
    {
        eprintln!("HUGR 0.13 support not enabled. Rebuild with --features hugr-013");
        std::process::exit(1);
    }
}
