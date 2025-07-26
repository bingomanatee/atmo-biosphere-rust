use atmo_biosphere_rust::sim_immut::simulation_immut::{SimulationImmut, SimulationConfigImmut};
use atmo_biosphere_rust::sim_immut::layer_set_immut::default_layer_set_params_immut;
use atmo_biosphere_rust::sim_immut::radiative_transfer::RadiativeTransferConfig;
use atmo_biosphere_rust::energy_mass::energy_mass::EnergyMass;
use atmo_biosphere_rust::profiling::ComponentProfiler;
use atmo_biosphere_rust::reporting::GeologicalReporter;
use h3o::Resolution;
use std::time::Instant;

fn main() {
    println!("🌍 Billion Year Geological Simulation");
    println!("=====================================");

    // Check for summary-only flag (detailed is now default)
    let args: Vec<String> = std::env::args().collect();
    let show_detailed = !args.contains(&"--summary".to_string());

    // Create simulation configuration
    let config = SimulationConfigImmut {
        warmup_steps: 0,
        steps: 1_000, 
        years_per_step: 1000.0,
        surface_temp_k: 288.0,
        layer_set_params: default_layer_set_params_immut(Resolution::Three, 6371.0),
        radiative_transfer_config: RadiativeTransferConfig::default(),
    };

    // Create simulation with geological components
    let mut components: Vec<Box<dyn atmo_biosphere_rust::component::SimComponent>> = vec![
        Box::new(atmo_biosphere_rust::component::SurfaceEmissionComponent::new()),
        Box::new(atmo_biosphere_rust::component::CoreHeatComponent::new()),
    ];
    let mut sim = SimulationImmut::new(config, &mut components);

    println!("Simulating {} steps ({} million years)", sim.config.steps, sim.config.steps as f64 * sim.config.years_per_step / 1_000_000.0);
    if show_detailed {
        println!("Detailed cell-by-cell reporting enabled (use --summary for layer averages only)");
        // Show initial state
        GeologicalReporter::print_detailed_thermal_structure(&sim, 0.0);
    } else {
        println!("Summary reporting mode (use default for detailed cell-by-cell reporting)");
    }
    println!("Starting simulation...\n");

    let simulation_start = Instant::now();
    let mut last_progress_time = simulation_start;
    
    // Main simulation loop
    while sim.steps < sim.config.steps {
        // Run one simulation step
        sim.step_with_binary_pairing();

        // Progress reporting every 15 seconds
        if last_progress_time.elapsed().as_secs() >= 15 {
            let progress_percent = sim.steps as f64 / sim.config.steps as f64 * 100.0;
            let elapsed = simulation_start.elapsed().as_secs_f64();
            let eta = if sim.steps > 0 {
                elapsed * (sim.config.steps as f64 / sim.steps as f64 - 1.0)
            } else {
                0.0
            };

            println!("Progress: {:.1}% complete, ETA: {:.1}s", progress_percent, eta);
            last_progress_time = Instant::now();
        }
    }

    let total_time = simulation_start.elapsed();

    // Calculate performance metrics
    let avg_step_time_ms = total_time.as_secs_f64() * 1000.0 / sim.steps as f64;
    let steps_per_second = sim.steps as f64 / total_time.as_secs_f64();

    // Final results
    println!("\n🎉 Simulation Complete!");
    println!("⏱️  Total time: {:.1}s", total_time.as_secs_f64());
    println!("⚡ Average step time: {:.2}ms", avg_step_time_ms);
    println!("🚀 Performance: {:.1} FPS", steps_per_second);

    // Game performance analysis
    let target_fps = 60.0;
    let target_step_time_ms = 1000.0 / target_fps;

    if avg_step_time_ms < target_step_time_ms {
        let speedup = target_step_time_ms / avg_step_time_ms;
        println!("🎮 Game ready: {:.1}x faster than 60 FPS target", speedup);
    } else {
        let slowdown = avg_step_time_ms / target_step_time_ms;
        println!("🎮 Need {:.1}x speedup for 60 FPS target", slowdown);
    }

    // Show final geological state
    if show_detailed {
        let final_million_years = sim.config.steps as f64 * sim.config.years_per_step / 1_000_000.0;
        GeologicalReporter::print_detailed_thermal_structure(&sim, final_million_years);
    } else {
        let final_million_years = sim.config.steps as f64 * sim.config.years_per_step / 1_000_000.0;
        GeologicalReporter::print_geological_state_summary(&sim, final_million_years);
    }

}


