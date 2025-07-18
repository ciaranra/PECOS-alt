use pecos_core::errors::PecosError;
use pecos_engines::prelude::*;

fn main() -> Result<(), PecosError> {
    // Create some quantum measurement data
    let mut shot_vec = ShotVec::new();

    for i in 0..5 {
        let mut shot = Shot::default();
        shot.add_register("q", i % 8, 3);
        shot.add_register("ancilla", i % 2, 1);
        shot.add_register("syndrome", (i * 3) % 4, 2);
        shot_vec.shots.push(shot);
    }

    println!("=== ShotMap Display Options ===\n");

    // Convert to ShotMap for display and analysis
    let shot_map = shot_vec.try_as_shot_map()?;

    // Default is decimal
    println!("1. Default (decimal):");
    println!("{}", shot_map.display());

    // Easy to get other formats
    println!("\n2. Binary format:");
    println!("{}", shot_map.display().bitvec_binary());

    println!("\n3. Hexadecimal format:");
    println!("{}", shot_map.display().bitvec_hex());

    // Show with limited shots
    println!("\n4. Hexadecimal with max 3 shots:");
    println!("{}", shot_map.display().bitvec_hex().max_shots(3));

    Ok(())
}
