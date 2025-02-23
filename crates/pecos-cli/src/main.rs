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

    // Create Monte Carlo engine
    let mut mc_engine = MonteCarloEngine::new(program_path, args.workers);

    // Set up noise model if requested
    if let Some(prob) = args.noise_probability {
        let noise_model = DepolarizingNoise::new(prob);
        mc_engine.set_noise_model(Some(Box::new(noise_model)));
    }

    // Run simulation
    let results = mc_engine.run_program(args.shots)?;

    // Print results
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
                    let engine = setup_engine(&program_path)?;
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
