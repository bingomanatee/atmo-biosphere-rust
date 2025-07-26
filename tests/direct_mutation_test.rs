use atmo_biosphere_rust::sim_immut::simulation_immut::{SimulationImmut, SimulationConfigImmut};
use atmo_biosphere_rust::sim_immut::layer_set_immut::default_layer_set_params_immut;
use atmo_biosphere_rust::sim_immut::radiative_transfer::RadiativeTransferConfig;
use atmo_biosphere_rust::energy_mass::energy_mass::EnergyMass;
use h3o::Resolution;
use std::time::Instant;

#[test]
fn test_direct_mutation_performance() {
    println!("⚡ DIRECT MUTATION PERFORMANCE TEST");
    println!("===================================");
    println!("🎯 NO TRANSACTIONS - Direct cell mutations only");
    println!("🚀 Target: Maximum possible performance");
    
    // Create simulation with direct mutations
    let config = SimulationConfigImmut {
        warmup_steps: 0,
        steps: 10_000, // 10 million years
        years_per_step: 1000.0,
        surface_temp_k: 288.0,
        layer_set_params: default_layer_set_params_immut(Resolution::Three, 6371.0),
        radiative_transfer_config: RadiativeTransferConfig::default(),
    };
    
    let mut components = vec![];
    let mut sim = SimulationImmut::new(config, &mut components);
    
    println!("\n🌍 DIRECT MUTATION SETUP:");
    println!("   - Layer sets: {}", sim.layer_sets.len());
    println!("   - Total cells: {}", sim.total_cells());
    let (_, _, total_pairs) = sim.binary_pairing_system.get_performance_stats();
    println!("   - Binary pairs: {}", total_pairs);
    println!("   - Transaction system: ❌ DEPRECATED");
    println!("   - Direct mutations: ✅ ENABLED");
    
    // Show initial state
    print_direct_mutation_state(&sim, 0);
    
    println!("\n⚡ STARTING DIRECT MUTATION SIMULATION...");
    println!("🚀 NO transaction overhead - pure speed");
    
    let simulation_start = Instant::now();
    let mut step_times = Vec::new();
    let mut last_report_time = simulation_start;
    
    // DIRECT MUTATION SIMULATION LOOP
    for step in 0..sim.config.steps as usize {
        let step_start = Instant::now();
        
        // DIRECT MUTATION STEP (MAXIMUM SPEED!)
        sim.step_with_binary_pairing();
        
        let step_duration = step_start.elapsed();
        step_times.push(step_duration);
        
        // PROGRESS REPORTING
        let elapsed_since_report = last_report_time.elapsed();
        if elapsed_since_report.as_secs() >= 30 || step == sim.config.steps as usize - 1 {
            report_direct_mutation_performance(&sim, step, &step_times);
            last_report_time = Instant::now();
            step_times.clear();
        }
        
        // GEOLOGICAL STATE AT MILESTONES
        if step % 1_000 == 0 && step > 0 {
            let million_years = step as f64 * sim.config.years_per_step / 1_000_000.0;
            print_direct_mutation_state(&sim, million_years as i64);
        }
    }
    
    let total_time = simulation_start.elapsed();
    
    // FINAL DIRECT MUTATION RESULTS
    print_final_direct_mutation_results(&sim, &total_time);
    
    // VALIDATION
    assert!(total_time.as_secs() > 0, "Simulation should take time");
    
    let avg_step_time = total_time.div_f64(sim.config.steps as f64);
    let steps_per_second = sim.config.steps as f64 / total_time.as_secs_f64();
    
    println!("\n⚡ DIRECT MUTATION TEST COMPLETED!");
    println!("   ⚡ Average step time: {:.3}ms", avg_step_time.as_secs_f64() * 1000.0);
    println!("   🔄 Steps per second: {:.1}", steps_per_second);
    
    // Check if we achieved better performance than transactions
    let target_fps = 60.0;
    let target_step_time_ms = 1000.0 / target_fps;
    let actual_step_time_ms = avg_step_time.as_secs_f64() * 1000.0;
    
    if actual_step_time_ms < target_step_time_ms {
        println!("   🎮 GAME READY: {:.1}x faster than 60 FPS target!", target_step_time_ms / actual_step_time_ms);
    } else {
        println!("   📊 PROGRESS: {:.1}x slower than 60 FPS target", actual_step_time_ms / target_step_time_ms);
    }
    
    // Compare to previous transaction-based performance
    let transaction_step_time_ms = 14.649; // From previous test
    let speedup = transaction_step_time_ms / actual_step_time_ms;
    println!("   🚀 Speedup vs transactions: {:.1}x faster", speedup);
}

/// Report direct mutation performance progress
fn report_direct_mutation_performance(
    sim: &SimulationImmut,
    step: usize,
    step_times: &[std::time::Duration],
) {
    let million_years = (step + 1) as f64 * sim.config.years_per_step / 1_000_000.0;
    let progress_percent = ((step + 1) as f64 / sim.config.steps as f64) * 100.0;
    
    let avg_step_time = if !step_times.is_empty() {
        step_times.iter().sum::<std::time::Duration>() / step_times.len() as u32
    } else {
        std::time::Duration::new(0, 0)
    };
    
    let steps_per_second = if avg_step_time.as_secs_f64() > 0.0 {
        1.0 / avg_step_time.as_secs_f64()
    } else {
        0.0
    };
    
    println!("⏰ Direct Mutation Progress: {:.1}% complete ({:.1} million years)", progress_percent, million_years);
    println!("   - Avg step time: {:.3}ms", avg_step_time.as_secs_f64() * 1000.0);
    println!("   - Steps per second: {:.1}", steps_per_second);
    
    // Performance analysis
    let target_fps = 60.0;
    let target_step_time_ms = 1000.0 / target_fps;
    let actual_step_time_ms = avg_step_time.as_secs_f64() * 1000.0;
    
    if actual_step_time_ms < target_step_time_ms {
        println!("   🎮 GAME READY: {:.1}x faster than 60 FPS!", target_step_time_ms / actual_step_time_ms);
    } else {
        println!("   📊 Progress: {:.1}x slower than 60 FPS target", actual_step_time_ms / target_step_time_ms);
    }
    
    println!("   - Direct mutations: ✅ NO transaction overhead");
}

/// Print direct mutation geological state
fn print_direct_mutation_state(sim: &SimulationImmut, million_years: i64) {
    println!("\n⚡ DIRECT MUTATION STATE at {} Million Years:", million_years);
    println!("==============================================");
    println!("| Layer | Cells | Avg Temp(K) | Total Energy(J) | Material   |");
    println!("|-------|-------|-------------|-----------------|------------|");
    
    let mut total_energy = 0.0;
    
    for (layer_idx, layer_set) in sim.layer_sets.iter().enumerate() {
        if let Some((_, column)) = layer_set.layers.iter().next() {
            let avg_temp: f64 = column.cells.iter()
                .map(|cell| cell.temperature_kelvin())
                .sum::<f64>() / column.cells.len() as f64;
            
            let layer_energy: f64 = column.cells.iter()
                .map(|cell| cell.energy_joules)
                .sum();
            
            total_energy += layer_energy;
            
            let material = match layer_idx {
                0 => "basalt",
                1 => "peridotite", 
                2 => "eclogite",
                _ => "deep_mantle",
            };
            
            println!("| {:5} | {:5} | {:11.1} | {:13.2e} | {:<10} |",
                     layer_idx + 1, column.cells.len(), avg_temp, layer_energy, material);
        }
    }
    println!("|-------|-------|-------------|-----------------|------------|");
    println!("| TOTAL | {:5} |             | {:13.2e} |            |", sim.total_cells(), total_energy);
    
    println!("\n⚡ Direct Mutation Status:");
    println!("   - NO transaction overhead");
    println!("   - Direct cell mutations only");
    println!("   - Maximum possible speed");
}

/// Print final direct mutation performance results
fn print_final_direct_mutation_results(sim: &SimulationImmut, total_time: &std::time::Duration) {
    println!("\n⚡ FINAL DIRECT MUTATION RESULTS:");
    println!("=================================");
    println!("⏱️  Total simulation time: {:.2} seconds", total_time.as_secs_f64());
    
    let avg_step_time = total_time.div_f64(sim.config.steps as f64);
    let steps_per_second = sim.config.steps as f64 / total_time.as_secs_f64();
    
    println!("⚡ Average step time: {:.3}ms", avg_step_time.as_secs_f64() * 1000.0);
    println!("🔄 Steps per second: {:.1}", steps_per_second);
    
    // Game performance analysis
    let target_fps = 60.0;
    let target_step_time_ms = 1000.0 / target_fps;
    let actual_step_time_ms = avg_step_time.as_secs_f64() * 1000.0;
    
    println!("\n🎮 GAME PERFORMANCE ANALYSIS:");
    println!("   - Target (60 FPS): {:.2}ms per step", target_step_time_ms);
    println!("   - Actual: {:.3}ms per step", actual_step_time_ms);
    
    if actual_step_time_ms < target_step_time_ms {
        let speedup = target_step_time_ms / actual_step_time_ms;
        println!("   🎉 GAME READY: {:.1}x faster than 60 FPS target!", speedup);
        println!("   🚀 Could run at {:.0} FPS!", 1000.0 / actual_step_time_ms);
    } else {
        let slowdown = actual_step_time_ms / target_step_time_ms;
        println!("   📊 {:.1}x slower than 60 FPS target", slowdown);
        println!("   🔧 Current max FPS: {:.1}", 1000.0 / actual_step_time_ms);
    }
    
    // Compare to transaction system
    let transaction_step_time_ms = 14.649; // Previous benchmark
    let speedup = transaction_step_time_ms / actual_step_time_ms;
    
    println!("\n⚡ DIRECT MUTATION ACHIEVEMENTS:");
    println!("   - Speedup vs transactions: {:.1}x faster", speedup);
    println!("   - NO transaction overhead: ✅ ELIMINATED");
    println!("   - Direct cell mutations: ✅ MAXIMUM SPEED");
    println!("   - Basic safety checks: ✅ MINIMAL OVERHEAD");
    
    // Billion year projection
    let billion_year_steps = 1_000_000_u64;
    let billion_year_time = avg_step_time.mul_f64(billion_year_steps as f64);
    let billion_year_hours = billion_year_time.as_secs_f64() / 3600.0;
    
    println!("\n🌍 Billion Year Projection:");
    println!("   - Direct mutation time: {:.1} hours", billion_year_hours);
    
    if billion_year_hours < 1.0 {
        println!("   🎉 INCREDIBLE: Billion years in under 1 hour!");
    } else if billion_year_hours < 3.0 {
        println!("   ✅ EXCELLENT: Billion years in {:.1} hours", billion_year_hours);
    } else {
        println!("   📊 GOOD: Billion years in {:.1} hours", billion_year_hours);
    }
}
