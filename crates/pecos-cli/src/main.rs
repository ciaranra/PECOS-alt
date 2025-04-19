use clap::{Args, Parser, Subcommand};
use env_logger::Env;
use pecos::prelude::*;
use std::error::Error;

#[derive(Parser)]
#[command(
    name = "pecos",
    version = env!("CARGO_PKG_VERSION"),
    about = "A quantum error correction simulator",
    long_about = None
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Compile QIR program to native code
    Compile(CompileArgs),
    /// Run quantum program (supports QIR and PHIR/JSON formats)
    Run(RunArgs),
}

#[derive(Args)]
struct CompileArgs {
    /// Path to the quantum program (LLVM IR)
    program: String,
}

#[derive(Args)]
struct RunArgs {
    /// Path to the quantum program (LLVM IR or JSON)
    program: String,

    /// Number of shots for parallel execution
    #[arg(short, long, default_value_t = 1)]
    shots: usize,

    /// Number of parallel workers
    #[arg(short, long, default_value_t = 1)]
    workers: usize,

    /// Depolarizing noise probability (between 0 and 1)
    #[arg(short = 'p', long = "noise", value_parser = parse_noise_probability)]
    noise_probability: Option<f64>,

    /// Seed for random number generation (for reproducible results)
    #[arg(short = 'd', long)]
    seed: Option<u64>,
}

fn parse_noise_probability(arg: &str) -> Result<f64, String> {
    let prob: f64 = arg
        .parse()
        .map_err(|_| "Must be a valid floating point number")?;
    if !(0.0..=1.0).contains(&prob) {
        return Err("Noise probability must be between 0 and 1".into());
    }
    Ok(prob)
}

fn run_program(args: &RunArgs) -> Result<(), Box<dyn Error>> {
    let program_path = get_program_path(&args.program)?;
    let prob = args.noise_probability.unwrap_or(0.0);

    let classical_engine = setup_engine(&program_path, Some(args.shots.div_ceil(args.workers)))?;

    let results = MonteCarloEngine::run_with_classical_engine(
        classical_engine,
        prob,
        args.shots,
        args.workers,
        args.seed,
    )?;

    results.print();

    Ok(())
}

fn main() -> Result<(), Box<dyn Error>> {
    // Initialize logger with default "info" level if not specified
    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();

    let cli = Cli::parse();

    match &cli.command {
        Commands::Compile(args) => {
            let program_path = get_program_path(&args.program)?;
            match detect_program_type(&program_path)? {
                ProgramType::QIR => {
                    let engine = setup_engine(&program_path, None)?;
                    engine.compile()?;
                }
                ProgramType::PHIR => {
                    println!("PHIR/JSON programs don't require compilation");
                }
            }
        }
        Commands::Run(args) => run_program(args)?,
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn verify_cli_seed_argument() {
        let cmd = Cli::parse_from([
            "pecos",
            "run",
            "program.json",
            "-d",
            "42",
            "-s",
            "100",
            "-w",
            "2",
        ]);

        match cmd.command {
            Commands::Run(args) => {
                assert_eq!(args.seed, Some(42));
                assert_eq!(args.shots, 100);
                assert_eq!(args.workers, 2);
            }
            Commands::Compile(_) => panic!("Expected Run command"),
        }
    }

    #[test]
    fn verify_cli_no_seed_argument() {
        let cmd = Cli::parse_from(["pecos", "run", "program.json", "-s", "100", "-w", "2"]);

        match cmd.command {
            Commands::Run(args) => {
                assert_eq!(args.seed, None);
                assert_eq!(args.shots, 100);
                assert_eq!(args.workers, 2);
            }
            Commands::Compile(_) => panic!("Expected Run command"),
        }
    }
}
