use anyhow::Result;
use clap::{Parser, Subcommand};
use std::fs;
use std::path::PathBuf;

use pecos_hugr::compiler::HugrCompiler;

#[derive(Parser)]
#[command(name = "pecos-hugr-cli")]
#[command(about = "PECOS HUGR compiler CLI", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Compile HUGR to LLVM IR
    Compile {
        /// Input HUGR file
        input: PathBuf,

        /// Output LLVM file
        #[arg(short, long)]
        output: PathBuf,

        /// Verbose output
        #[arg(short, long)]
        verbose: bool,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Compile {
            input,
            output,
            verbose,
        } => {
            if verbose {
                println!("Compiling HUGR file: {input:?}");
            }

            // Read HUGR file
            let hugr_bytes = fs::read(&input)?;

            // Create compiler
            let compiler = HugrCompiler::new();

            // Compile to LLVM
            let llvm_ir = compiler.compile_hugr_bytes_to_string(&hugr_bytes)?;

            // Write output
            fs::write(&output, llvm_ir)?;

            if verbose {
                println!("Successfully compiled to: {output:?}");
            }

            Ok(())
        }
    }
}
