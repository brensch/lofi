mod args;

use args::Args;
use lofi_sim::{Simulation, GROUP_SIZE, SAMPLE_RATE};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    let mut sim = Simulation::new(
        args.nodes,
        args.seed,
        args.sync_start_ms * 1_000,
        args.group_join_ms * 1_000,
    );
    sim.schedule_demo_drop();
    if args.settle_ms > 0 {
        sim.run(args.settle_ms * 1_000);
    }
    let initial_stats = sim.phase_stats();
    let samples = sim.render(args.duration_ms * 1_000);
    let left_stats = sim.phase_stats_for(0..GROUP_SIZE);
    let right_stats = sim.phase_stats_for(GROUP_SIZE..GROUP_SIZE * 2);
    let final_stats = sim.phase_stats();
    println!(
        "initial: max sync phase spread={}us, mean abs error={}us",
        initial_stats.max_spread_us, initial_stats.mean_abs_error_us
    );
    println!(
        "final left cluster: max sync phase spread={}us, mean abs error={}us",
        left_stats.max_spread_us, left_stats.mean_abs_error_us
    );
    println!(
        "final right cluster: max sync phase spread={}us, mean abs error={}us",
        right_stats.max_spread_us, right_stats.mean_abs_error_us
    );
    println!(
        "final global: max sync phase spread={}us, mean abs error={}us",
        final_stats.max_spread_us, final_stats.mean_abs_error_us
    );
    lofi_sim::wav::write_wav(&args.wav_path, SAMPLE_RATE, &samples)?;
    println!("wrote {}", args.wav_path.display());
    Ok(())
}
